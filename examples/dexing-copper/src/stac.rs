use crate::config::{GDAL_TRANSLATE, MAX_LAT, MAX_LON, MIN_LAT, MIN_LON};
use geo_core::errors::{GeoError, GeoResult};
use geo_raster::grid::RasterBand;

// ── STAC 搜索 ──────────────────────────────────────────

pub(crate) async fn search_sentinel2_scenes(
    client: &geo_adapters_io::stac::StacClient,
    year: u16,
    verbose: bool,
) -> GeoResult<Vec<geo_adapters_io::stac::StacItem>> {
    let date_from = format!("{year}-06-01");
    let date_to = format!("{year}-08-31");

    if verbose {
        println!("  [STAC] 搜索 {year} 年 6-8 月 Sentinel-2 L2A...");
    }

    let items = client
        .search(
            "sentinel-2-l2a",
            MIN_LON,
            MIN_LAT,
            MAX_LON,
            MAX_LAT,
            &date_from,
            &date_to,
            10,
        )
        .await?;

    if verbose {
        for item in &items {
            let cc = item.cloud_cover.map_or("?".into(), |c| format!("{c:.1}%"));
            let dt = item.datetime.as_deref().unwrap_or("?");
            println!("    {} | {} | 云量: {}", item.id, dt, cc);
        }
        println!("    找到 {} 景", items.len());
    }

    Ok(items)
}

// ── 真实 Sentinel-2 波段下载 + GeoTIFF 读取 ─────────────

/// 从 STAC item 提取 B04 (Red) 和 B08 (NIR) 的 HTTPS 下载 URL。
fn extract_band_hrefs(
    item: &geo_adapters_io::stac::StacItem,
) -> (Option<String>, Option<String>, Option<String>) {
    let assets = match &item.assets {
        Some(a) => a,
        None => return (None, None, None),
    };
    let b4 = assets["B04"]["href"].as_str().map(|s| s.to_string());
    let b8 = assets["B08"]["href"].as_str().map(|s| s.to_string());
    let scl = assets["SCL"]["href"].as_str().map(|s| s.to_string());
    (b4, b8, scl)
}

/// 从 HTTPS URL 下载 Sentinel-2 COG 波段到本地 GeoTIFF。
/// 优先使用 GDAL /vsicurl/ (HTTP range reads, 只下载 AOI 区域, 极快)。
/// 如果 GDAL 不可用, 回退到 reqwest 全量下载。
async fn download_band_cog(url: &str, path: &std::path::Path, label: &str) -> GeoResult<()> {
    if path.exists() {
        println!("    [cached] {}", label);
        return Ok(());
    }

    // 方案 A: GDAL /vsicurl/ → 裁剪到 AOI (只下载 ~2MB)
    let gdal_path = std::path::Path::new(GDAL_TRANSLATE);
    if gdal_path.exists() {
        println!("    下载 {label} (GDAL /vsicurl/) ...");
        let vsi_url = format!("/vsicurl/{url}");
        // gdal_translate -projwin ulx uly lrx lry 来裁剪
        let result = tokio::process::Command::new(gdal_path)
            .args([
                "-of",
                "GTiff",
                "-co",
                "COMPRESS=LZW",
                "-projwin",
                &format!("{}", MIN_LON),
                &format!("{}", MAX_LAT),
                &format!("{}", MAX_LON),
                &format!("{}", MIN_LAT),
                "-projwin_srs",
                "EPSG:4326",
                &vsi_url,
                &path.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| GeoError::ExternalProcess {
                command: "gdal_translate".into(),
                message: e.to_string(),
            })?;

        if result.status.success() {
            let meta = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let size_mb = meta as f64 / 1_048_576.0;
            println!("    ✓ {label} ({size_mb:.1} MB, AOI 裁剪)");
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&result.stderr);
        println!("    ⚠ GDAL 失败: {}", stderr.lines().last().unwrap_or(""));
        println!("    回退到 HTTP 全量下载...");
    }

    // 方案 B: reqwest 全量下载
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .map_err(|e| GeoError::ExternalProcess {
            command: "HTTP GET".into(),
            message: e.to_string(),
        })?;

    if !resp.status().is_success() {
        return Err(GeoError::ExternalProcess {
            command: "HTTP GET".into(),
            message: format!("HTTP {} for {label}", resp.status()),
        });
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| GeoError::Io(std::io::Error::other(e)))?;

    std::fs::write(path, &bytes)?;
    let size_mb = bytes.len() as f64 / 1_048_576.0;
    println!("    ✓ {label} ({size_mb:.1} MB)");
    Ok(())
}

/// 从本地 GeoTIFF 文件读取像素到 RasterBand (f64)。
/// 自动处理 UInt16 (Sentinel-2 L2A) 和 Float32。
fn read_geotiff_to_band(path: &std::path::Path, band_name: &str) -> GeoResult<RasterBand> {
    // 尝试用 tiff crate 直接解码
    match read_tiff_crate(path, band_name) {
        Ok(band) => return Ok(band),
        Err(e) => {
            eprintln!("    tiff crate 解码失败 ({}), 尝试 GDAL 转换...", e);
        }
    }

    // 回退: GDAL 转换 → 再读
    let tmp_path = path.with_extension("tmp.tif");
    let gdal = std::path::Path::new(GDAL_TRANSLATE);
    if !gdal.exists() {
        return Err(GeoError::Other(
            "No TIFF decoder available (tiff crate failed, gdal_translate not found)".into(),
        ));
    }

    let result = std::process::Command::new(gdal)
        .args([
            "-of",
            "GTiff",
            "-co",
            "COMPRESS=NONE",
            "-ot",
            "Float32",
            &path.to_string_lossy(),
            &tmp_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| GeoError::ExternalProcess {
            command: "gdal_translate".into(),
            message: e.to_string(),
        })?;

    if !result.status.success() {
        return Err(GeoError::Other("gdal_translate conversion failed".into()));
    }

    let band = read_tiff_crate(&tmp_path, band_name)?;
    let _ = std::fs::remove_file(&tmp_path);
    Ok(band)
}

fn read_tiff_crate(path: &std::path::Path, band_name: &str) -> GeoResult<RasterBand> {
    use std::io::BufReader;
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut decoder = tiff::decoder::Decoder::new(reader)
        .map_err(|e| GeoError::Other(format!("TIFF decode: {e}")))?;

    let (width, height) = decoder
        .dimensions()
        .map_err(|e| GeoError::Other(format!("TIFF dims: {e}")))?;

    let cols = width as usize;
    let rows = height as usize;
    let n_total = rows * cols;

    // 检测数据类型
    let bits_per_sample = decoder
        .find_tag_unsigned_vec(tiff::tags::Tag::BitsPerSample)
        .map_err(|e| GeoError::Other(format!("TIFF bits: {e}")))?;
    let sample_format = decoder
        .find_tag_unsigned_vec(tiff::tags::Tag::SampleFormat)
        .unwrap_or_default();

    let is_float = sample_format.as_ref().and_then(|v| v.first()) == Some(&3); // 3 = IEEE floating point
    let _is_uint16 = bits_per_sample.as_ref().and_then(|v| v.first()) == Some(&16) && !is_float;

    // 读取图像
    let img_result = decoder.read_image();
    let img = match img_result {
        Ok(tiff::decoder::DecodingResult::U16(data)) => {
            // Sentinel-2 L2A UInt16, 需要除以 10000 转反射率
            let mut out = Vec::with_capacity(n_total);
            let scale = 1.0 / 10000.0;
            for v in data.iter().take(n_total) {
                if *v == 0 {
                    out.push(-999.0); // nodata
                } else {
                    out.push(*v as f64 * scale);
                }
            }
            out
        }
        Ok(tiff::decoder::DecodingResult::U8(data)) => data
            .iter()
            .take(n_total)
            .map(|v| *v as f64 / 255.0)
            .collect(),
        Ok(tiff::decoder::DecodingResult::F32(data)) => {
            // Float32, 直接使用
            data.iter().take(n_total).map(|v| *v as f64).collect()
        }
        Ok(tiff::decoder::DecodingResult::F64(data)) => data.to_vec(),
        other => {
            return Err(GeoError::Other(format!(
                "Unsupported TIFF format: {:?}",
                other.map(|_| ())
            )));
        }
    };

    let data = if img.len() < n_total {
        let mut padded = img;
        padded.resize(n_total, -999.0);
        padded
    } else if img.len() > n_total {
        img[..n_total].to_vec()
    } else {
        img
    };

    Ok(RasterBand::new(band_name, rows, cols, data, -999.0))
}

/// 对 Planetary Computer Azure Blob 资产 URL 进行 SAS 签名。
///
/// URL 格式: `https://{account}.blob.core.windows.net/{container}/{rest...}`
/// 签名 API: `GET https://planetarycomputer.microsoft.com/api/sas/v1/token/{account}/{container}/{rest...}`
/// 返回: `{ "url": "https://...?sv=...&se=...&sr=...&sig=..." }`
async fn sign_pc_asset_url(href: &str) -> Result<String, String> {
    let parsed = url::Url::parse(href).map_err(|e| format!("URL parse: {e}"))?;
    let host = parsed.host_str().ok_or("no host")?;
    let account = host.split('.').next().ok_or("no account")?;
    let path = parsed.path().trim_start_matches('/');
    // path 格式: {container}/{blob...} → 取 container 部分
    let container = path.split('/').next().ok_or("no container in path")?;
    if container.is_empty() {
        return Err("empty container".into());
    }

    // SAS 签名 API: /sas/v1/token/{account}/{container}
    let sign_url =
        format!("https://planetarycomputer.microsoft.com/api/sas/v1/token/{account}/{container}");

    let resp = reqwest::get(&sign_url)
        .await
        .map_err(|e| format!("SAS sign request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("SAS sign HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("SAS sign JSON: {e}"))?;

    // 尝试多种可能的字段名: url, signed_url, token (然后拼接)
    if let Some(url) = body["url"].as_str() {
        return Ok(url.to_string());
    }
    if let Some(token) = body["token"].as_str() {
        // 用原始 href + token 拼接
        let sep = if href.contains('?') { "&" } else { "?" };
        return Ok(format!("{href}{sep}{token}"));
    }
    Err("no url or token in SAS response".into())
}

/// 从 Planetary Computer 下载 S2B 场景 + SCL 云掩膜 + 多景中值合成。
/// 仅使用 S2B (统一传感器), 自动遮蔽云/云影, 多景取中值。
pub(crate) async fn download_with_scl(
    scenes: &[geo_adapters_io::stac::StacItem],
    year: u16,
    output_dir: &std::path::Path,
) -> Option<(RasterBand, RasterBand)> {
    let s2b_scenes: Vec<_> = scenes.iter().filter(|s| s.id.contains("S2B")).collect();
    if s2b_scenes.is_empty() {
        eprintln!("  ⚠ {year} 无 S2B 场景");
        return None;
    }

    let pc_client = geo_adapters_io::stac::StacClient::new(
        "https://planetarycomputer.microsoft.com/api/stac/v1",
    );

    const SCL_CLOUD: &[u8] = &[3, 7, 8, 9, 10];

    let mut all_red: Vec<RasterBand> = Vec::new();
    let mut all_nir: Vec<RasterBand> = Vec::new();

    for scene in &s2b_scenes {
        let id_s = scene.id.replace([':', '/', '\\', ' '], "_");
        println!("  [{year}] {id_s} ...");

        let full = pc_client.get_item("sentinel-2-l2a", &scene.id).await.ok()?;
        let (b4_h, b8_h, scl_h) = extract_band_hrefs(&full);
        if b4_h.is_none() || b8_h.is_none() || scl_h.is_none() {
            continue;
        }

        let b4_s = sign_pc_asset_url(&b4_h?).await.ok()?;
        let b8_s = sign_pc_asset_url(&b8_h?).await.ok()?;
        let scl_s = sign_pc_asset_url(&scl_h?).await.ok()?;

        let dir = output_dir.join(format!("sentinel2_{year}"));
        std::fs::create_dir_all(&dir).ok()?;

        let b4_p = dir.join(format!("{id_s}_B04.tif"));
        let b8_p = dir.join(format!("{id_s}_B08.tif"));
        let scl_p = dir.join(format!("{id_s}_SCL.tif"));

        download_band_cog(&b4_s, &b4_p, &format!("B04 {year}"))
            .await
            .ok()?;
        download_band_cog(&b8_s, &b8_p, &format!("B08 {year}"))
            .await
            .ok()?;
        download_band_cog(&scl_s, &scl_p, &format!("SCL {year}"))
            .await
            .ok()?;

        let mut red = read_geotiff_to_band(&b4_p, "B04").ok()?;
        let mut nir = read_geotiff_to_band(&b8_p, "B08").ok()?;
        let scl_data = read_geotiff_to_band(&scl_p, "SCL")
            .map(|b| b.data)
            .unwrap_or_default();

        let mut masked = 0usize;
        let total = red.data.len();
        for i in 0..total {
            let v = scl_data.get(i).copied().unwrap_or(0.0) as u8;
            if SCL_CLOUD.contains(&v) {
                red.data[i] = red.nodata;
                nir.data[i] = nir.nodata;
                masked += 1;
            }
        }
        if masked > 0 {
            println!(
                "    SCL掩膜: {masked}/{total} ({:.1}%)",
                masked as f64 / total as f64 * 100.0
            );
        }

        all_red.push(red);
        all_nir.push(nir);
    }

    if all_red.is_empty() {
        return None;
    }

    if all_red.len() == 1 {
        let (r, c) = (all_red[0].rows, all_red[0].cols);
        println!("  ✓ {year} S2B {r}x{c} px");
        return Some((all_red.remove(0), all_nir.remove(0)));
    }

    // 多景中值合成
    let rows = all_red[0].rows;
    let cols = all_red[0].cols;
    let nd = all_red[0].nodata;
    let n_scenes = all_red.len();

    let composite = |bands: &[RasterBand]| -> RasterBand {
        let mut data = Vec::with_capacity(rows * cols);
        for i in 0..rows * cols {
            let mut vals: Vec<f64> = bands
                .iter()
                .map(|b| b.data[i])
                .filter(|v| *v != nd && !v.is_nan())
                .collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            data.push(if vals.is_empty() {
                nd
            } else {
                vals[vals.len() / 2]
            });
        }
        RasterBand::new("composite", rows, cols, data, nd)
    };

    let red_c = composite(&all_red);
    let nir_c = composite(&all_nir);
    println!("  ✓ {year} {n_scenes}景中值合成 ({rows}x{cols} px)");
    Some((red_c, nir_c))
}

/// 从 Planetary Computer STAC items 下载真实 Sentinel-2 波段 (含 SAS 签名)。
pub(crate) fn generate_simulated_bands(
    rows: usize,
    cols: usize,
    seed: u64,
    restored_factor_nir: f64,
    restored_factor_red: f64,
) -> (RasterBand, RasterBand) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();

    // Simple PRNG
    fn prng(seed: &mut u64) -> f64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);

        (*seed >> 32) as f64 / u32::MAX as f64
    }

    let mut s = hash;
    let (cx, cy) = (cols as f64 * 0.47, rows as f64 * 0.52); // 矿区中心
    let (mine_sx, mine_sy) = (cols as f64 * 0.16, rows as f64 * 0.14);

    let n_pixels = rows * cols;
    let mut red_data = Vec::with_capacity(n_pixels);
    let mut nir_data = Vec::with_capacity(n_pixels);

    for r in 0..rows {
        for c in 0..cols {
            // 距离矿区中心的高斯权重
            let dx = (c as f64 - cx) / mine_sx;
            let dy = (r as f64 - cy) / mine_sy;
            let dist2 = dx * dx + dy * dy;
            let mine_weight = (-dist2 / 2.0).exp();
            let natural_weight = 1.0 - mine_weight;

            let noise = prng(&mut s) * 0.04;

            // 自然植被光谱
            let red_nat = 0.05 + prng(&mut s) * 0.03;
            let nir_nat = 0.50 + prng(&mut s) * 0.18;

            // 矿区光谱
            let red_mine = 0.20 + prng(&mut s) * 0.15;
            let nir_mine = 0.10 + prng(&mut s) * 0.10;

            // 混合
            let red =
                (red_nat * natural_weight + red_mine * mine_weight) * restored_factor_red + noise;
            let nir = (nir_nat * natural_weight + nir_mine * mine_weight) * restored_factor_nir
                + noise * 2.0;

            red_data.push(red.clamp(0.01, 0.45));
            nir_data.push(nir.clamp(0.02, 0.85));
        }
    }

    let red_band = RasterBand::new("B4_RED".to_string(), rows, cols, red_data, -999.0);
    let nir_band = RasterBand::new("B8_NIR".to_string(), rows, cols, nir_data, -999.0);

    (red_band, nir_band)
}

//! Validate all Chinese provincial capital coordinates using geo-core.
//! Outputs JSON summary for the population map.
//!
//! Usage: cargo run --example validate_china_coords

use geo_core::types::{validate_coord, point, BBox};
use serde::Serialize;

#[derive(Serialize)]
struct ProvinceResult {
    name: String,
    lon: f64,
    lat: f64,
    valid: bool,
    population_wan: f64,
}

#[derive(Serialize)]
struct Report {
    total: usize,
    valid: usize,
    invalid: usize,
    china_bbox: BBox,
    crs: String,
    provinces: Vec<ProvinceResult>,
}

fn main() {
    // All 34 provincial capital coordinates (WGS84)
    let provinces: Vec<(&str, f64, f64, f64)> = vec![
        ("北京市",   116.4074, 39.9042,  2188.6),
        ("天津市",   117.1902, 39.1252,  1367.0),
        ("河北省",   114.5020, 38.0455,  7420.0),
        ("山西省",   112.5492, 37.8570,  3481.0),
        ("内蒙古",   111.6708, 40.8183,  2401.0),
        ("辽宁省",   123.4291, 41.7968,  4259.0),
        ("吉林省",   125.3245, 43.8868,  2407.0),
        ("黑龙江省", 126.6424, 45.7570,  3125.0),
        ("上海市",   121.4737, 31.2304,  2475.0),
        ("江苏省",   118.7674, 32.0415,  8515.0),
        ("浙江省",   120.1536, 30.2875,  6577.0),
        ("安徽省",   117.2830, 31.8612,  6121.0),
        ("福建省",   119.3062, 26.0753,  4187.0),
        ("江西省",   115.8922, 28.6765,  4517.0),
        ("山东省",   117.0009, 36.6758, 10163.0),
        ("河南省",   113.6654, 34.7580,  9815.0),
        ("湖北省",   114.2986, 30.5844,  5838.0),
        ("湖南省",   112.9823, 28.1941,  6604.0),
        ("广东省",   113.2644, 23.1291, 12706.0),
        ("广西",     108.3200, 22.8240,  5047.0),
        ("海南省",   110.3312, 20.0320,  1027.0),
        ("重庆市",   106.5044, 29.5332,  3191.0),
        ("四川省",   104.0657, 30.6595,  8374.0),
        ("贵州省",   106.7135, 26.5783,  3856.0),
        ("云南省",   102.7123, 25.0406,  4693.0),
        ("西藏",     91.1322,  29.6604,   364.0),
        ("陕西省",   108.9480, 34.2632,  3954.0),
        ("甘肃省",   103.8236, 36.0580,  2501.0),
        ("青海省",   101.7789, 36.6232,   592.0),
        ("宁夏",     106.2782, 38.4662,   728.0),
        ("新疆",     87.6168,  43.8256,  2587.0),
        ("台湾省",   121.5090, 25.0443,  2357.0),
        ("香港",     114.1734, 22.3193,   750.0),
        ("澳门",     113.5491, 22.1987,    68.0),
    ];

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  geo-core 坐标验证 — 中国 34 省/地区省会坐标                ║");
    println!("║  CRS: EPSG:4326 (WGS84)                                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut valid_count = 0;
    let mut invalid_count = 0;

    // Compute China bounding box
    let mut min_lon = f64::MAX;
    let mut min_lat = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut max_lat = f64::MIN;

    let mut results: Vec<ProvinceResult> = Vec::new();

    println!("{:<8} {:<12} {:<10} {:<10} {:<10} {:>12}", 
             "状态", "省份", "经度", "纬度", "合法性", "人口(万)");
    println!("{}", "-".repeat(70));

    for (name, lon, lat, pop) in &provinces {
        let result = validate_coord(*lon, *lat);
        let valid = result.is_ok();

        if valid {
            valid_count += 1;
            min_lon = min_lon.min(*lon);
            min_lat = min_lat.min(*lat);
            max_lon = max_lon.max(*lon);
            max_lat = max_lat.max(*lat);
        } else {
            invalid_count += 1;
        }

        let status = if valid { "✅" } else { "❌" };
        let validity = if valid { "通过" } else { "失败" };

        println!("{:<6} {:<12} {:<10.4} {:<10.4} {:<10} {:>10.1}", 
                 status, name, lon, lat, validity, pop);

        results.push(ProvinceResult {
            name: name.to_string(),
            lon: *lon,
            lat: *lat,
            valid,
            population_wan: *pop,
        });
    }

    // Build China bounding box
    let china_bbox = BBox::new(min_lon, min_lat, max_lon, max_lat);

    println!("\n{}", "=".repeat(70));
    println!("📊 验证汇总");
    println!("{}", "=".repeat(70));
    println!("  总计: {} 个区域", provinces.len());
    println!("  通过: {} ✅", valid_count);
    println!("  失败: {} ❌", invalid_count);

    println!("\n🔲 中国陆地包围盒 (省级中心点):");
    println!("  EPSG:4326 WGS84");
    println!("  min_x = {:.4}  (最西: 西藏)", min_lon);
    println!("  max_x = {:.4}  (最东: 黑龙江)", max_lon);
    println!("  min_y = {:.4}  (最南: 海南)", min_lat);
    println!("  max_y = {:.4}  (最北: 黑龙江)", max_lat);
    println!("  宽度 = {:.4}° 经度", china_bbox.width());
    println!("  高度 = {:.4}° 纬度", china_bbox.height());
    println!("\n  BBox::contains(116.4, 39.9) = {} (北京)",
             china_bbox.contains(116.4074, 39.9042));
    println!("  BBox::contains(121.5, 31.2) = {} (上海)",
             china_bbox.contains(121.4737, 31.2304));
    println!("  BBox::contains(87.6, 43.8)  = {} (乌鲁木齐)",
             china_bbox.contains(87.6168, 43.8256));
    println!("  BBox::contains(0.0, 0.0)    = {} (不在中国)",
             china_bbox.contains(0.0, 0.0));

    // Also test Point construction via geo-core
    println!("\n🧪 point() 构造测试:");
    match point(116.4074, 39.9042) {
        Ok(p) => println!("  point(116.4074, 39.9042) = Point({}, {}) ✅ (北京)", p.x(), p.y()),
        Err(e) => println!("  ❌ {}", e),
    }
    match point(200.0, 50.0) {
        Ok(_) => println!("  point(200, 50) 预期失败但通过了 ❌"),
        Err(e) => println!("  point(200, 50) → {} ✅ (越界被拒绝)", e),
    }

    // Output JSON for HTML integration
    println!("\n{}", "=".repeat(70));
    println!("📋 JSON 输出 (供地图使用):");
    println!("{}", "=".repeat(70));

    let report = Report {
        total: provinces.len(),
        valid: valid_count,
        invalid: invalid_count,
        china_bbox: china_bbox,
        crs: "EPSG:4326 WGS84".into(),
        provinces: results,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("JSON序列化失败: {}", e),
    }
}

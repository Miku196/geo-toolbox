/// DSSAT 作物模型输入文件生成适配器。
///
/// 生成 .WTH（天气）/ .SOIL（土壤）/ .CUL（品种）/ .FILEX 等标准输入文件。
/// 纯 Rust，无外部依赖。

use serde::{Deserialize, Serialize};

/// DSSAT 气象站信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherStation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation_m: f64,
    pub wmo_code: String,
}

/// DSSAT 逐日气象数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyWeather {
    pub julian_day: u16,
    pub solar_rad_mj_m2: f64,
    pub tmax_c: f64,
    pub tmin_c: f64,
    pub rainfall_mm: f64,
}

/// DSSAT 土壤层。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilLayer {
    pub depth_cm: f64,
    pub clay_pct: f64,
    pub silt_pct: f64,
    pub sand_pct: f64,
    pub organic_c_pct: f64,
    pub bulk_density_g_cm3: f64,
    pub ph: f64,
    pub ll: f64,
    pub dul: f64,
    pub sat: f64,
    pub ks: f64,
}

/// DSSAT 土壤剖面。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilProfile {
    pub soil_id: String,
    pub soil_name: String,
    pub layers: Vec<SoilLayer>,
    pub albedo: f64,
    pub evaporation: f64,
}

/// DSSAT 品种参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultivarParams {
    pub cultivar_name: String,
    pub ecotype: String,
    pub p1: f64,
    pub p2: f64,
    pub p5: f64,
    pub g2: f64,
    pub g3: f64,
    pub phint: f64,
}

/// 生成 DSSAT .WTH 天气文件。
pub fn generate_wth(station: &WeatherStation, daily_data: &[DailyWeather]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "*WEATHER DATA : {}\n",
        station.name
    ));
    out.push_str(&format!(
        "@ INSI      LAT     LONG  ELEV   TAV   AMP REFHT WNDHT\n"
    ));
    out.push_str(&format!(
        "  {} {:>8.2} {:>8.2} {:>5.0}  -99   -99  2.00  2.00\n",
        &station.wmo_code[..std::cmp::min(4, station.wmo_code.len())],
        station.latitude,
        station.longitude,
        station.elevation_m,
    ));
    out.push_str("@DATE  SRAD  TMAX  TMIN  RAIN\n");
    for day in daily_data {
        out.push_str(&format!(
            "{:>5} {:>5.1} {:>5.1} {:>5.1} {:>5.1}\n",
            format!("{:0>3}", day.julian_day),
            day.solar_rad_mj_m2,
            day.tmax_c,
            day.tmin_c,
            if day.rainfall_mm < 0.0 { -99.0 } else { day.rainfall_mm },
        ));
    }
    out
}

/// 生成 DSSAT .SOL 土壤文件。
pub fn generate_sol(profile: &SoilProfile) -> String {
    let mut out = String::new();
    out.push_str("*SOILS: Soil Profile Data\n\n");
    out.push_str(&format!(
        "@SITE        COUNTRY          LAT     LONG SCS FAMILY\n"
    ));
    out.push_str(&format!(
        "  {:<12} {:<16} {:>7.2} {:>7.2} -\n",
        profile.soil_id, profile.soil_name,
        0.0, 0.0,
    ));
    out.push_str("@  SLB  SLMH  SLLL  SDUL  SSAT  SRGF  SSKS  SBDM  SLOC  SLCL  SLSI  SLCF  SLNI  SLHW\n");
    for (i, layer) in profile.layers.iter().enumerate() {
        out.push_str(&format!(
            "  {:>3} {:>5.0} {:>5.0} {:>5.0} {:>5.0} {:>5.2} {:>5.1} {:>5.2} {:>5.2} {:>5.0} {:>5.0} {:>5.0} {:>5.3} {:>5.1}\n",
            i + 1,
            layer.depth_cm,
            layer.ll * 100.0,
            layer.dul * 100.0,
            layer.sat * 100.0,
            if i == 0 { 1.0 } else { 0.5 },
            layer.ks * 100.0,
            layer.bulk_density_g_cm3,
            layer.organic_c_pct,
            layer.clay_pct,
            layer.silt_pct,
            layer.sand_pct,
            0.0,
            layer.ph,
        ));
    }
    out
}

/// 生成 DSSAT .CUL 品种文件。
pub fn generate_cul(params: &CultivarParams) -> String {
    let mut out = String::new();
    out.push_str("*CULTIVAR COEFFICIENTS\n");
    out.push_str("@  VAR#  VAR-NAME……  EXPNO   ECO#  P1  P2  P5  G2  G3  PHINT\n");
    out.push_str(&format!(
        "  {:>5}  {:<15}  {:>5}  {:>5}  {:>4.0} {:>4.0} {:>4.0} {:>4.0} {:>4.0} {:>5.0}\n",
        1,
        params.cultivar_name,
        1,
        params.ecotype,
        params.p1,
        params.p2,
        params.p5,
        params.g2,
        params.g3,
        params.phint,
    ));
    out
}

/// 将月平均气象数据分解为逐日数据（简化版）。
pub fn monthly_to_daily_wth(
    tmax_monthly: &[f64],
    tmin_monthly: &[f64],
    rain_monthly: &[f64],
    latitude: f64,
    elevation_m: f64,
) -> Vec<DailyWeather> {
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut result = Vec::new();
    let mut julian = 1;
    for m in 0..12 {
        let days = month_days[m];
        for d in 0..days {
            // Linear interpolation within month
            let tmax = tmax_monthly[m] + (d as f64 / days as f64 - 0.5) * 2.0;
            let tmin = tmin_monthly[m] + (d as f64 / days as f64 - 0.5) * 2.0;
            // Distribute rainfall (simplified: even distribution with some variability)
            let rain_day = if rain_monthly[m] > 0.0 {
                let factor = (julian as f64 * 7.0).sin().abs() * 2.0;
                rain_monthly[m] / days as f64 * factor
            } else {
                0.0
            };
            // Estimate solar radiation from temperature range
            let t_range = tmax - tmin;
            let ext_rad = extraterrestrial_radiation(julian, latitude);
            let solar_rad = (ext_rad * (1.0 - (t_range * 0.004).min(0.7))).max(0.0).min(ext_rad);
            result.push(DailyWeather {
                julian_day: julian,
                solar_rad_mj_m2: solar_rad,
                tmax_c: tmax,
                tmin_c: tmin,
                rainfall_mm: rain_day,
            });
            julian += 1;
        }
    }
    result
}

/// 计算大气层外太阳辐射 (MJ/m²/day)。
fn extraterrestrial_radiation(julian: u16, latitude: f64) -> f64 {
    let lat_rad = latitude.to_radians();
    let j = julian as f64;
    let declination = 23.45_f64.to_radians() * ((284.0 + j) * 2.0 * std::f64::consts::PI / 365.0).sin();
    let ws = (-lat_rad.tan() * declination.tan()).acos();
    let dr = 1.0 + 0.033 * (2.0 * std::f64::consts::PI * j / 365.0).cos();
    let gsc = 0.0820; // solar constant MJ/m²/min
    let ra = (24.0 * 60.0 / std::f64::consts::PI) * gsc * dr
        * (ws * lat_rad.sin() * declination.sin() + lat_rad.cos() * declination.cos() * ws.sin());
    ra
}

/// 从 SCS 土壤分组生成 DSSAT 土壤剖面。
pub fn soil_from_scs_group(soil_id: &str, group: &str, lat: f64, lon: f64) -> SoilProfile {
    let (sand, clay, silt, om) = match group {
        "A" => (85.0, 5.0, 10.0, 1.0),
        "B" => (60.0, 15.0, 25.0, 2.0),
        "C" => (35.0, 30.0, 35.0, 1.5),
        "D" => (25.0, 40.0, 35.0, 2.5),
        _ => (50.0, 20.0, 30.0, 1.5),
    };
    let _ = lat;
    let _ = lon;
    let top_layer = SoilLayer {
        depth_cm: 30.0,
        clay_pct: clay,
        silt_pct: silt,
        sand_pct: sand,
        organic_c_pct: om * 0.58,
        bulk_density_g_cm3: 1.3,
        ph: 6.5,
        ll: 0.10,
        dul: 0.25,
        sat: 0.45,
        ks: 0.5,
    };
    let sub_layer = SoilLayer {
        depth_cm: 100.0,
        clay_pct: clay * 1.2,
        silt_pct: silt * 0.9,
        sand_pct: sand * 0.95,
        organic_c_pct: om * 0.58 * 0.3,
        bulk_density_g_cm3: 1.4,
        ph: 6.8,
        ll: 0.12,
        dul: 0.28,
        sat: 0.42,
        ks: 0.3,
    };
    SoilProfile {
        soil_id: soil_id.to_string(),
        soil_name: format!("SCS_{}", group),
        layers: vec![top_layer, sub_layer],
        albedo: 0.13,
        evaporation: 0.50,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wth() {
        let station = WeatherStation {
            name: "Test Station".into(),
            latitude: 30.0, longitude: 104.0,
            elevation_m: 500.0, wmo_code: "TEST".into(),
        };
        let data = vec![DailyWeather {
            julian_day: 1, solar_rad_mj_m2: 20.0,
            tmax_c: 30.0, tmin_c: 20.0, rainfall_mm: 0.0,
        }];
        let wth = generate_wth(&station, &data);
        assert!(wth.contains("30.0"));
        assert!(wth.contains("20.0"));
    }

    #[test]
    fn test_generate_sol() {
        let profile = SoilProfile {
            soil_id: "IB001".into(), soil_name: "Test Soil".into(),
            layers: vec![SoilLayer {
                depth_cm: 30.0, clay_pct: 20.0, silt_pct: 30.0, sand_pct: 50.0,
                organic_c_pct: 1.0, bulk_density_g_cm3: 1.3, ph: 6.5,
                ll: 0.10, dul: 0.25, sat: 0.45, ks: 0.5,
            }],
            albedo: 0.13, evaporation: 0.50,
        };
        let sol = generate_sol(&profile);
        assert!(sol.contains("IB001"));
    }

    #[test]
    fn test_generate_cul() {
        let params = CultivarParams {
            cultivar_name: "Cultivar1".into(), ecotype: "IB0001".into(),
            p1: 200.0, p2: 0.0, p5: 600.0, g2: 0.02, g3: 1.0, phint: 100.0,
        };
        let cul = generate_cul(&params);
        assert!(cul.contains("Cultivar1"));
        assert!(cul.contains("IB0001"));
    }

    #[test]
    fn test_monthly_to_daily() {
        let tmax = vec![15.0; 12];
        let tmin = vec![5.0; 12];
        let rain = vec![50.0; 12];
        let daily = monthly_to_daily_wth(&tmax, &tmin, &rain, 30.0, 500.0);
        assert_eq!(daily.len(), 365);
        assert!(daily[0].solar_rad_mj_m2 > 0.0);
        assert!(daily[0].rainfall_mm >= 0.0);
    }

    #[test]
    fn test_soil_from_scs_group() {
        let soil = soil_from_scs_group("CN001", "B", 30.0, 104.0);
        assert_eq!(soil.layers.len(), 2);
        assert!((soil.layers[0].sand_pct - 60.0).abs() < 1.0);
    }

    #[test]
    fn test_extraterrestrial_radiation() {
        let ra = extraterrestrial_radiation(180, 30.0);
        assert!(ra > 20.0);
        assert!(ra < 45.0);
    }
}

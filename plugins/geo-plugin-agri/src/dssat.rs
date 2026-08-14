/// DSSAT 数据类型的重新导出 — 从 geo-core traits 引入。
/// 这些类型由 geo-adapter-dssat 实现，但通过 trait 方法访问。
pub use geo_core::traits::{
    CultivarParams, DailyWeather, SoilLayer, SoilProfile, WeatherStation,
};

/// 生成 DSSAT .WTH 天气文件。
pub fn generate_wth(station: &WeatherStation, daily_data: &[DailyWeather]) -> String {
    let mut out = String::new();
    out.push_str(&format!("*WEATHER DATA : {}\n", station.name));
    out.push_str("@ INSI      LAT     LONG  ELEV   TAV   AMP REFHT WNDHT\n");
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
            if day.rainfall_mm < 0.0 {
                -99.0
            } else {
                day.rainfall_mm
            },
        ));
    }
    out
}

/// 生成 DSSAT .SOL 土壤文件。
pub fn generate_sol(profile: &SoilProfile) -> String {
    let mut out = String::new();
    out.push_str("*SOILS: Soil Profile Data\n\n");
    out.push_str("@SITE        COUNTRY          LAT     LONG SCS FAMILY\n");
    out.push_str(&format!(
        "  {:<12} {:<16} {:>7.2} {:>7.2} -\n",
        profile.soil_id, profile.soil_name, 0.0, 0.0,
    ));
    out.push_str(
        "@  SLB  SLMH  SLLL  SDUL  SSAT  SRGF  SSKS  SBDM  SLOC  SLCL  SLSI  SLCF  SLNI  SLHW\n",
    );
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
    _elevation_m: f64,
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
            let solar_rad = (ext_rad * (1.0 - (t_range * 0.004).min(0.7)))
                .max(0.0)
                .min(ext_rad);
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
    let declination =
        23.45_f64.to_radians() * ((284.0 + j) * 2.0 * std::f64::consts::PI / 365.0).sin();
    let ws = (-lat_rad.tan() * declination.tan()).acos();
    let dr = 1.0 + 0.033 * (2.0 * std::f64::consts::PI * j / 365.0).cos();
    let gsc = 0.0820; // solar constant MJ/m²/min
    (24.0 * 60.0 / std::f64::consts::PI)
        * gsc
        * dr
        * (ws * lat_rad.sin() * declination.sin() + lat_rad.cos() * declination.cos() * ws.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wth_basic() {
        let station = WeatherStation {
            name: "Test Station".to_string(),
            latitude: 30.5,
            longitude: 114.3,
            elevation_m: 50.0,
            wmo_code: "CN001".to_string(),
        };
        let data = vec![
            DailyWeather {
                julian_day: 1,
                solar_rad_mj_m2: 12.5,
                tmax_c: 28.0,
                tmin_c: 18.0,
                rainfall_mm: 0.0,
            },
            DailyWeather {
                julian_day: 2,
                solar_rad_mj_m2: 14.0,
                tmax_c: 30.0,
                tmin_c: 20.0,
                rainfall_mm: 5.0,
            },
        ];
        let output = generate_wth(&station, &data);
        assert!(output.contains("*WEATHER DATA"));
        assert!(output.contains("CN00"));
    }

    #[test]
    fn test_generate_sol() {
        let profile = SoilProfile {
            soil_id: "CN001".to_string(),
            soil_name: "CHINA".to_string(),
            albedo: 0.13,
            evaporation: 0.5,
            layers: vec![SoilLayer {
                depth_cm: 20.0,
                clay_pct: 20.0,
                silt_pct: 30.0,
                sand_pct: 45.0,
                organic_c_pct: 1.2,
                bulk_density_g_cm3: 1.35,
                ph: 6.5,
                ll: 0.15,
                dul: 0.30,
                sat: 0.45,
                ks: 0.5,
            }],
        };
        let output = generate_sol(&profile);
        assert!(output.contains("*SOILS"));
        assert!(output.contains("CN001"));
    }

    #[test]
    fn test_generate_cul() {
        let params = CultivarParams {
            cultivar_name: "Generic Maize".to_string(),
            ecotype: "000001".to_string(),
            p1: 200.0,
            p2: 0.5,
            p5: 800.0,
            g2: 600.0,
            g3: 8.0,
            phint: 38.0,
        };
        let output = generate_cul(&params);
        assert!(output.contains("*CULTIVAR"));
    }

    #[test]
    fn test_monthly_to_daily() {
        let tmax = vec![
            10.0, 12.0, 16.0, 20.0, 25.0, 28.0, 30.0, 29.0, 25.0, 20.0, 15.0, 11.0,
        ];
        let tmin = vec![
            0.0, 2.0, 6.0, 10.0, 15.0, 18.0, 20.0, 19.0, 15.0, 10.0, 5.0, 1.0,
        ];
        let rain = vec![
            50.0, 45.0, 80.0, 100.0, 150.0, 200.0, 180.0, 140.0, 100.0, 70.0, 55.0, 40.0,
        ];
        let daily = monthly_to_daily_wth(&tmax, &tmin, &rain, 30.0, 50.0);
        assert_eq!(daily.len(), 365);
        assert!(!daily.is_empty());
    }

    #[test]
    fn test_serde_weather() {
        let w = DailyWeather {
            julian_day: 150,
            solar_rad_mj_m2: 20.0,
            tmax_c: 35.0,
            tmin_c: 25.0,
            rainfall_mm: 0.0,
        };
        let json = serde_json::to_string(&w).unwrap();
        let deser: DailyWeather = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.julian_day, 150);
    }
}

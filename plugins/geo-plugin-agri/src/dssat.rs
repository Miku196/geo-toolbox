/// DSSAT 作物模型输入文件生成器 — 委托到 geo-adapter-dssat
pub use geo_adapter_dssat::*;

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

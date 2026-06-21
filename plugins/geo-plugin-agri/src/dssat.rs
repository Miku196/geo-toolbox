/// DSSAT 作物模型输入文件生成器 — 委托到 geo-adapter-dssat
pub use geo_adapter_dssat::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wth_basic() {
        let station = WeatherStation {
            insi: "CN001".to_string(),
            site: "CNSITE".to_string(),
            name: "Test Station".to_string(),
            lat: 30.5,
            lon: 114.3,
            elev: 50.0,
            tav: 16.5,
            tamp: 10.2,
            refht: 2.0,
            wndht: 10.0,
        };
        let data = vec![
            DailyWeather {
                year: 2024,
                doy: 1,
                srad: 12.5,
                tmax: 28.0,
                tmin: 18.0,
                rain: 0.0,
                wind: Some(2.5),
                rhum: Some(65.0),
            },
            DailyWeather {
                year: 2024,
                doy: 2,
                srad: 14.0,
                tmax: 30.0,
                tmin: 20.0,
                rain: 5.0,
                wind: Some(3.0),
                rhum: Some(70.0),
            },
        ];
        let output = generate_wth(&station, &data);
        assert!(output.contains("*WEATHER DATA"));
        assert!(output.contains("CN001"));
    }

    #[test]
    fn test_generate_sol() {
        let profile = SoilProfile {
            site_id: "CN001".to_string(),
            country: "CHINA".to_string(),
            lat: 30.5,
            lon: 114.3,
            slope: 0.02,
            drainage: 3,
            albedo: 0.13,
            evaporation: 0.5,
            runoff: 0.5,
            color: 3,
            layers: vec![SoilLayer {
                depth_bottom_cm: 20.0,
                clay_pct: 20.0,
                silt_pct: 30.0,
                stones_pct: 5.0,
                organic_c_pct: 1.2,
                bd: 1.35,
                ph: 6.5,
                cec: 15.0,
                bsn: 50.0,
                sat: 0.45,
                ll: 0.15,
                dul: 0.30,
                ssks: 0.5,
                rv: 0.0,
                ssp: 0.0,
            }],
        };
        let output = generate_sol(&profile);
        assert!(output.contains("*SOIL")) | assert!(output.contains("CN001"));
    }

    #[test]
    fn test_generate_cul() {
        let params = CultivarParams {
            cultivar_id: "IB0001".to_string(),
            cultivar_name: "Generic Maize".to_string(),
            ecotype: "000001".to_string(),
            p1: 200.0,
            p2: 0.5,
            p5: 800.0,
            g2: 600.0,
            g3: 8.0,
            phint: 38.0,
            ax: 5.0,
        };
        let output = generate_cul(&params);
        assert!(output.contains("*CULTIVAR"));
    }

    #[test]
    fn test_monthly_to_daily() {
        let monthly = vec![
            50.0, 45.0, 80.0, 100.0, 150.0, 200.0, 180.0, 140.0, 100.0, 70.0, 55.0, 40.0,
        ];
        let daily = monthly_to_daily_wth(&monthly, 2024, 30.0);
        assert_eq!(daily.len(), 366);
        assert!(!daily.is_empty());
    }

    #[test]
    fn test_serde_weather() {
        let w = DailyWeather {
            year: 2024,
            doy: 150,
            srad: 20.0,
            tmax: 35.0,
            tmin: 25.0,
            rain: 0.0,
            wind: Some(3.0),
            rhum: Some(60.0),
        };
        let json = serde_json::to_string(&w).unwrap();
        let deser: DailyWeather = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.doy, 150);
        assert!((deser.wind.unwrap() - 3.0).abs() < 1e-10);
    }
}

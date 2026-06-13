//! Integration test: full ecology assessment pipeline.
//!
//! Verifies that EcologyPlugin::assess_restoration produces correct output
//! with realistic GeoJSON AOI + two-period NDVI rasters.
//!
//! Uses the ProcessPlugin trait to validate the seam works.

use geo_core::plugin::{Plugin, ProcessPlugin};
use geo_plugin_ecology::ecology::{AssessmentInput, EcologyPlugin};
use geo_plugin_ecology::EcologyConfig;
use geo_raster::RasterBand;
use serde_json::json;

fn make_band(data: Vec<f64>) -> RasterBand {
    RasterBand::new("band", 1, data.len(), data, -999.0)
}

fn test_config() -> EcologyConfig {
    toml::from_str(
        r#"
        [plugin]
        name = "ecology"
        version = "0.1.0"
        description = "test"

        [ndvi]
        healthy_min = 0.5
        degraded_max = 0.2
        improvement_threshold = 0.1
        degradation_threshold = -0.1

        [carbon]
        source = "IPCC_2019"
        forest = -5.0
        grassland = -1.2
        built_up = 2.0
        bare = 0.0
        wetland = -8.5
        cropland = 0.5
    "#,
    )
    .unwrap()
}

fn test_aoi() -> String {
    r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": {"class": "forest", "area_ha": 50.0},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[104.0,30.5],[104.1,30.5],[104.1,30.6],[104.0,30.6],[104.0,30.5]]]
                }
            },
            {
                "type": "Feature",
                "properties": {"class": "grassland", "area_ha": 30.0},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[104.1,30.5],[104.2,30.5],[104.2,30.6],[104.1,30.6],[104.1,30.5]]]
                }
            }
        ]
    }"#.to_string()
}

#[test]
fn test_full_ecology_pipeline() {
    let config = test_config();
    let plugin = EcologyPlugin::new(config);

    // Verify Plugin trait
    assert_eq!(plugin.name(), "ecology");
    assert_eq!(plugin.category(), geo_core::plugin::PluginCategory::Process);
    assert!(plugin.is_healthy());

    // Verify ProcessPlugin trait
    assert_eq!(plugin.process_type(), "ecology_assess");

    // 2020 baseline: degraded land (high red reflectance → low NDVI)
    let red_2020 = make_band(vec![0.40, 0.45, 0.42, 0.44, 0.40, 0.38, 0.41, 0.43]);
    let nir_2020 = make_band(vec![0.15, 0.18, 0.16, 0.17, 0.14, 0.13, 0.15, 0.16]);

    // 2025 after restoration: healthy vegetation (low red, high NIR → high NDVI)
    let red_2025 = make_band(vec![0.05, 0.06, 0.07, 0.08, 0.05, 0.04, 0.06, 0.07]);
    let nir_2025 = make_band(vec![0.55, 0.58, 0.56, 0.57, 0.54, 0.53, 0.55, 0.56]);

    let aoi = test_aoi();
    let input = AssessmentInput {
        aoi_name: "Test Mining Zone",
        aoi_geojson: &aoi,
        baseline_red: &red_2020,
        baseline_nir: &nir_2020,
        assessment_red: &red_2025,
        assessment_nir: &nir_2025,
        baseline_year: 2020,
        assessment_year: 2025,
    };

    let assessment = plugin
        .assess_restoration(&input)
        .expect("assessment should succeed");

    // ── Structural assertions ──
    assert_eq!(assessment.aoi_name, "Test Mining Zone");
    assert_eq!(assessment.baseline_year, 2020);
    assert_eq!(assessment.assessment_year, 2025);

    // NDVI should improve (2025 > 2020)
    let base_ndvi = assessment.baseline_ndvi.mean_ndvi.unwrap_or(0.0);
    let assess_ndvi = assessment.assessment_ndvi.mean_ndvi.unwrap_or(0.0);
    assert!(
        assess_ndvi > base_ndvi,
        "restored NDVI ({assess_ndvi:.3}) should exceed baseline ({base_ndvi:.3})"
    );

    // Carbon sink should be net-negative (carbon removal)
    assert!(
        assessment.carbon.total_emission_tco2e < 0.0,
        "should be net carbon sink, got {}",
        assessment.carbon.total_emission_tco2e
    );

    // Carbon classes should include forest and grassland
    let class_names: Vec<&str> = assessment
        .carbon
        .classes
        .iter()
        .map(|c| c.landcover_class.as_str())
        .collect();
    assert!(class_names.contains(&"forest"), "should have forest class");
    assert!(
        class_names.contains(&"grassland"),
        "should have grassland class"
    );

    // Conclusion should be set
    assert!(!assessment.conclusion.grade.is_empty());
    assert!(!assessment.conclusion.summary.is_empty());
    assert!(assessment.conclusion.carbon_sink_tco2_per_yr > 0.0);

    // ── Report generation ──
    let report = plugin
        .generate_report(&assessment)
        .expect("report generation should succeed");
    assert!(report.contains("Test Mining Zone"));
    assert!(report.contains("NDVI"));
    assert!(report.contains("tCO₂"));
}

#[tokio::test]
async fn test_process_plugin_trait_execute() {
    let config = test_config();
    let plugin = EcologyPlugin::new(config);

    // Call through the ProcessPlugin trait
    let result = plugin
        .execute(json!({
            "aoi_name": "Test via trait",
            "baseline_year": 2020,
            "assessment_year": 2025,
        }))
        .await
        .expect("trait execute should succeed");

    assert_eq!(result["aoi_name"], "Test via trait");
    assert_eq!(result["baseline_year"], 2020);
    assert!(result["carbon_sink_tco2_per_yr"].as_f64().unwrap() > 0.0);
    assert!(!result["conclusion"]["grade"].as_str().unwrap().is_empty());
}

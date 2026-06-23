# Progress

## Status
✅ ALL 15 FEATURES COMPLETE (1001 tests, 0 failures)

## 15 Features Across 9 Plugins

### geo-plugin-survey ✅ (33 tests)
- [x] utm.rs — UTM zone calculation, lat/lon ↔ UTM conversion
- [x] vincenty.rs — Vincenty's geodesic formulae (inverse, direct) + haversine helper
- MCP tools: `survey_utm_zone_info`, `survey_latlon_to_utm`, `survey_utm_to_latlon`, `survey_vincenty_inverse`, `survey_vincenty_direct`, `survey_haversine_distance`

### geo-plugin-geohazard ✅ (41 tests)
- [x] newmark.rs — Newmark displacement (Jibson 2007) standalone module
- MCP tools: `geohazard_newmark`

### geo-plugin-carbon ✅ (38 tests)
- [x] carbon_price.rs — Carbon price scenarios (EU ETS, China, California, voluntary)
- [x] vcs_gs.rs — VCS additionality/validation, Gold Standard SDG mapping
- MCP tools: `carbon_price_scenario`, `carbon_offset_revenue`, `carbon_vcs_additionality`, `carbon_vcs_validation`, `carbon_gold_standard_sdg`

### geo-plugin-hydro ✅ (87 tests)
- [x] tr55.rs — NRCS TR-55 tabular method (Tc, travel time, Ia, peak discharge)
- [x] muskingum.rs — Muskingum flood routing (X, K, celerity computation)
- MCP tools: `hydro_tr55_tc`, `hydro_tr55_peak_discharge`, `hydro_tr55_unit_hydrograph`, `hydro_muskingum_routing`, `hydro_muskingum_parameters`

### geo-plugin-urban ✅ (33 tests)
- [x] urban_flood.rs — Urban flood simulation (SCS-runoff + pipe network capacity check)
- [x] accessibility.rs — 15-minute city accessibility (POI reachable within time/speed)
- MCP tools: `urban_flood_simulate`, `urban_flood_pipe_network`, `urban_accessibility`, `urban_accessibility_isochrone`

### geo-plugin-coastal ✅ (66 tests)
- [x] slr.rs — Sea Level Rise bathtub model (IPCC AR6 SSP scenarios)
- [x] cvi.rs — Coastal Vulnerability Index (Gornitz 1991)
- MCP tools: `coastal_slr_bathtub`, `coastal_slr_inundation`, `coastal_cvi`

### geo-plugin-ecology ✅ (67 tests)
- [x] musle.rs — MUSLE 事件版土壤流失 (assess_musle, event_assessment, annual_average)
- MCP tools: `ecology_musle_single`, `ecology_musle_assessment`, `ecology_musle_annual`

### geo-plugin-energy ✅ (48 tests)
- [x] wake.rs — Jensen/Frandsen 尾流效应 (cumulative_wake, farm_wake_efficiency, farm_aep_with_wake)
- [x] turbine.rs — 风力机功率曲线 (V80/V164/G114 presets, AEP, wind shear)
- MCP tools: `energy_turbine_power`, `energy_turbine_aep`, `energy_jensen_wake`, `energy_farm_wake_efficiency`, `energy_wind_shear`

### geo-plugin-remote-sensing ✅ (15 tests, NEW)
- [x] radiometric.rs — TOA 辐射亮度/反射率、DOS 大气校正、云检测
- [x] insar.rs — 相干性计算、相位解缠 (Goldstein)、LOS 形变估计
- MCP tools: `remote_toa_radiance`, `remote_full_pipeline`, `remote_cloud_mask`, `remote_insar_coherence`, `remote_insar_full`, `remote_insar_displacement_class`

### geo-plugin-forestry ✅ (32 tests)
- [x] site_index.rs — Richards/Logistic site index curves for 6 species
- [x] harvest.rs — Selective/clearcut harvest, sustainable yield (AAC), carbon debt/payback
- MCP tools: `forestry_site_index`, `forestry_site_class`, `forestry_harvest_selective`, `forestry_harvest_clearcut`, `forestry_sustainable_yield`, `forestry_harvest_carbon_impact`

## GeoConfig System ✅
- [x] core/geo-core/src/config.rs (568 lines)
- [x] GeoConfig: 14 adapter configs + 4 plugin configs + MCP server + logging
- [x] config.json project-wide config file
- [x] env var overrides (GEO_*_* pattern)
- [x] geo-wiring/lib.rs integration
- [x] geo-cli/main.rs + geo-server/registry.rs integration

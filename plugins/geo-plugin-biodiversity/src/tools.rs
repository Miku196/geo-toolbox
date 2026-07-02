use geo_registry::register_plugin;
use geo_registry::registry::ToolResult;
use geo_registry::PluginRegistry;

pub fn register_tools(registry: &mut PluginRegistry) {
    register_plugin!(registry, "biodiversity", "Biodiversity assessment — SDM, habitat, GAP analysis",
    PluginCategory::Process, [
        sync "biodiversity_fit_sdm" => "Fit bioclimatic envelope SDM from occurrence records"
            ; serde_json::json!({"type":"object","properties":{"species":{"type":"string"},"occurrences":{"type":"array","items":{"type":"object","properties":{"lon":{"type":"number"},"lat":{"type":"number"},"env_values":{"type":"array","items":{"type":"number"}}},"required":["lon","lat","env_values"]}},"var_names":{"type":"array","items":{"type":"string"}}},"required":["species","occurrences","var_names"]})
            => |args| -> ToolResult {
                use crate::biodiversity::Occurrence;
                let species = args["species"].as_str().unwrap_or("");
                let var_names: Vec<String> = args["var_names"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let occurrences: Vec<Occurrence> = args["occurrences"].as_array()
                    .map(|a| a.iter().filter_map(|v| {
                        Some(Occurrence {
                            lon: v["lon"].as_f64()?,
                            lat: v["lat"].as_f64()?,
                            env_values: v["env_values"].as_array()?.iter().filter_map(|x| x.as_f64()).collect(),
                        })
                    }).collect())
                    .unwrap_or_default();
                let plugin = crate::BiodiversityPlugin::new(Default::default());
                let model = plugin.fit_sdm(species, &occurrences, &var_names)?;
                serde_json::to_value(model).map_err(geo_core::GeoError::Serde)
            },
        sync "biodiversity_assess_habitat" => "Compute landscape patch metrics from habitat raster"
            ; serde_json::json!({"type":"object","properties":{"habitat":{"type":"array","items":{"type":"integer","minimum":0,"maximum":1}},"rows":{"type":"integer"},"cols":{"type":"integer"},"cell_area":{"type":"number"}},"required":["habitat","rows","cols","cell_area"]})
            => |args| -> ToolResult {
                let habitat: Vec<u8> = args["habitat"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_u64().map(|x| x as u8)).collect())
                    .unwrap_or_default();
                let rows = args["rows"].as_u64().unwrap_or(1) as usize;
                let cols = args["cols"].as_u64().unwrap_or(1) as usize;
                let cell_area = args["cell_area"].as_f64().unwrap_or(1.0);
                let plugin = crate::BiodiversityPlugin::new(Default::default());
                let metrics = plugin.assess_habitat(&habitat, rows, cols, cell_area)?;
                serde_json::to_value(metrics).map_err(geo_core::GeoError::Serde)
            },
        sync "biodiversity_gap_analysis" => "GAP analysis: how much of a species range is protected"
            ; serde_json::json!({"type":"object","properties":{"species":{"type":"string"},"range_bboxes":{"type":"array","items":{"type":"object","properties":{"min_x":{"type":"number"},"min_y":{"type":"number"},"max_x":{"type":"number"},"max_y":{"type":"number"}},"required":["min_x","min_y","max_x","max_y"]}},"protected_areas":{"type":"array","items":{"type":"object","properties":{"min_x":{"type":"number"},"min_y":{"type":"number"},"max_x":{"type":"number"},"max_y":{"type":"number"}},"required":["min_x","min_y","max_x","max_y"]}}},"required":["species","range_bboxes","protected_areas"]})
            => |args| -> ToolResult {
                use geo_core::types::BBox;
                let parse_bboxes = |key: &str| -> Vec<BBox> {
                    args[key].as_array().map(|a| a.iter().filter_map(|v| {
                        Some(BBox::new(v["min_x"].as_f64()?, v["min_y"].as_f64()?, v["max_x"].as_f64()?, v["max_y"].as_f64()?))
                    }).collect()).unwrap_or_default()
                };
                let species = args["species"].as_str().unwrap_or("");
                let range = parse_bboxes("range_bboxes");
                let pas = parse_bboxes("protected_areas");
                let plugin = crate::BiodiversityPlugin::new(Default::default());
                let result = plugin.gap_analysis(species, &range, &pas);
                serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
            },
        sync "biodiversity_diversity" => "Compute Shannon/Simpson diversity indices from species abundances"
            ; serde_json::json!({"type":"object","properties":{"abundances":{"type":"array","items":{"type":"number"}}},"required":["abundances"]})
            => |args| -> ToolResult {
                let abundances: Vec<f64> = args["abundances"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();
                let plugin = crate::BiodiversityPlugin::new(Default::default());
                let result = plugin.diversity(&abundances);
                serde_json::to_value(result).map_err(geo_core::GeoError::Serde)
            },
        sync "biodiversity_connectivity" => "Compute connectivity index from patch bounding boxes"
            ; serde_json::json!({"type":"object","properties":{"patch_bboxes":{"type":"array","items":{"type":"object","properties":{"min_x":{"type":"number"},"min_y":{"type":"number"},"max_x":{"type":"number"},"max_y":{"type":"number"}},"required":["min_x","min_y","max_x","max_y"]}}},"required":["patch_bboxes"]})
            => |args| -> ToolResult {
                use geo_core::types::BBox;
                let patches: Vec<BBox> = args["patch_bboxes"].as_array().map(|a| a.iter().filter_map(|v| {
                    Some(BBox::new(v["min_x"].as_f64()?, v["min_y"].as_f64()?, v["max_x"].as_f64()?, v["max_y"].as_f64()?))
                }).collect()).unwrap_or_default();
                let ci = crate::biodiversity::connectivity_index(&patches);
                Ok(serde_json::json!({"connectivity_index": ci}))
            }
    ]);
}

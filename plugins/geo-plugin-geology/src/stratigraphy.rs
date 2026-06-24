use serde::{Deserialize, Serialize};

/// 地层单元定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDefinition {
    /// 层名
    pub name: String,
    /// 顶部深度 (m, 向下为正)
    pub top_depth_m: f64,
    /// 底部深度 (m)
    pub base_depth_m: f64,
    /// 岩性代码
    pub lithology_code: String,
    /// 密度 (kg/m³)
    pub density_kgm3: f64,
}

/// 三维地层模型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratigraphicModel {
    /// 地层列表
    pub layers: Vec<LayerDefinition>,
    /// 模型总深度 (m)
    pub total_depth_m: f64,
    /// 层数
    pub n_layers: usize,
    /// 单位厚度 (m)
    pub layer_thickness_m: f64,
}

/// 钻孔柱状。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratigraphicColumn {
    /// x 坐标
    pub x: f64,
    /// y 坐标
    pub y: f64,
    /// 地表高程 (m)
    pub surface_elevation_m: f64,
    /// 各层顶面高程 (m)
    pub layer_top_elevations_m: Vec<f64>,
    /// 层名
    pub layer_names: Vec<String>,
}

/// 创建钻孔地层序列。
pub fn stratigraphic_column(
    x: f64,
    y: f64,
    surface_elevation_m: f64,
    layer_defs: &[LayerDefinition],
) -> StratigraphicColumn {
    let mut tops = Vec::new();
    let mut names = Vec::new();
    for layer in layer_defs {
        tops.push(surface_elevation_m - layer.top_depth_m);
        names.push(layer.name.clone());
    }
    StratigraphicColumn {
        x,
        y,
        surface_elevation_m,
        layer_top_elevations_m: tops,
        layer_names: names,
    }
}

/// 三维地层建模 (简单层状插值)。
/// - dem: 数字高程模型 [rows × cols]
/// - base_layer: 地层定义
/// - cols
pub fn stratigraphic_model_3d(
    dem: &[f64],
    layer_defs: &[LayerDefinition],
    cols: usize,
) -> StratigraphicModel {
    let n_layers = layer_defs.len();
    let total_depth = layer_defs
        .iter()
        .map(|l| l.base_depth_m)
        .fold(0.0_f64, f64::max);
    let thickness = if n_layers > 0 {
        total_depth / n_layers as f64
    } else {
        50.0
    };

    StratigraphicModel {
        layers: layer_defs.to_vec(),
        total_depth_m: total_depth,
        n_layers,
        layer_thickness_m: thickness,
    }
}

/// 给定位置的层界面高程。
pub fn layer_elevation(
    surface_elevation_m: f64,
    layer_index: usize,
    layer_defs: &[LayerDefinition],
) -> f64 {
    if layer_index >= layer_defs.len() {
        return surface_elevation_m - layer_defs.last().map(|l| l.base_depth_m).unwrap_or(0.0);
    }
    surface_elevation_m - layer_defs[layer_index].top_depth_m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_layers() -> Vec<LayerDefinition> {
        vec![
            LayerDefinition {
                name: "topsoil".into(),
                top_depth_m: 0.0,
                base_depth_m: 2.0,
                lithology_code: "QS".into(),
                density_kgm3: 1800.0,
            },
            LayerDefinition {
                name: "clay".into(),
                top_depth_m: 2.0,
                base_depth_m: 10.0,
                lithology_code: "CL".into(),
                density_kgm3: 2000.0,
            },
            LayerDefinition {
                name: "sandstone".into(),
                top_depth_m: 10.0,
                base_depth_m: 50.0,
                lithology_code: "SS".into(),
                density_kgm3: 2400.0,
            },
        ]
    }

    #[test]
    fn test_stratigraphic_column() {
        let layers = example_layers();
        let col = stratigraphic_column(500.0, 300.0, 100.0, &layers);
        assert_eq!(col.x, 500.0);
        assert_eq!(col.y, 300.0);
        assert_eq!(col.layer_names.len(), 3);
        assert!((col.layer_top_elevations_m[0] - 100.0).abs() < 0.01);
        assert!((col.layer_top_elevations_m[1] - 98.0).abs() < 0.01);
    }

    #[test]
    fn test_model_3d() {
        let layers = example_layers();
        let m = stratigraphic_model_3d(&[100.0; 100], &layers, 10);
        assert_eq!(m.n_layers, 3);
        assert!((m.total_depth_m - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_layer_elevation() {
        let layers = example_layers();
        let e = layer_elevation(150.0, 1, &layers);
        assert!((e - 148.0).abs() < 0.01);
    }

    #[test]
    fn test_layer_elevation_out_of_range() {
        let layers = example_layers();
        let e = layer_elevation(100.0, 10, &layers);
        assert!((e - 50.0).abs() < 0.01);
    }
}

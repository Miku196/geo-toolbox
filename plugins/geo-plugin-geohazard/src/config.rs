use serde::Deserialize;

/// 地质灾害插件的顶级配置。
#[derive(Debug, Clone, Deserialize)]
pub struct GeohazardConfig {
    pub plugin: PluginMeta,

    /// 滑坡因子权重。
    #[serde(default)]
    pub landslide: LandslideWeights,

    /// 泥石流参数。
    #[serde(default)]
    pub debris_flow: DebrisFlowParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// 滑坡6因子权重 + 模糊隶属度参数。
#[derive(Debug, Clone, Deserialize)]
pub struct LandslideWeights {
    /// 坡度权重（度 → 归一化）。
    #[serde(default = "default_slope_weight")]
    pub slope_weight: f64,

    /// 坡向权重（朝南性）。
    #[serde(default = "default_aspect_weight")]
    pub aspect_weight: f64,

    /// 岩性权重。
    #[serde(default = "default_lithology_weight")]
    pub lithology_weight: f64,

    /// 降雨量权重。
    #[serde(default = "default_rainfall_weight")]
    pub rainfall_weight: f64,

    /// 距断层距离权重。
    #[serde(default = "default_fault_weight")]
    pub fault_weight: f64,

    /// 植被覆盖权重（NDVI）。
    #[serde(default = "default_vegetation_weight")]
    pub vegetation_weight: f64,

    // ── 隶属度参数 ──
    /// 坡度隶属度转折点1（°），低于此值安全。
    #[serde(default = "default_slope_a")]
    pub slope_a: f64,
    /// 坡度隶属度转折点2（°），高于此值高度危险。
    #[serde(default = "default_slope_b")]
    pub slope_b: f64,

    /// 降雨量隶属度转折点1（mm）。
    #[serde(default = "default_rainfall_a")]
    pub rainfall_a: f64,
    /// 降雨量隶属度转折点2（mm）。
    #[serde(default = "default_rainfall_b")]
    pub rainfall_b: f64,

    /// 断层距离隶属度转折点1（m）。
    #[serde(default = "default_fault_a")]
    pub fault_a: f64,
    /// 断层距离隶属度转折点2（m）。
    #[serde(default = "default_fault_b")]
    pub fault_b: f64,

    /// 植被 NDVI 低阈值（低于此值高危险）。
    #[serde(default = "default_veg_a")]
    pub veg_a: f64,
    /// 植被 NDVI 高阈值（高于此值安全）。
    #[serde(default = "default_veg_b")]
    pub veg_b: f64,
}

/// 泥石流危险性参数。
#[derive(Debug, Clone, Deserialize)]
pub struct DebrisFlowParams {
    /// 沟床比降危险性阈值（°），高于此值危险。
    #[serde(default = "default_channel_gradient_threshold")]
    pub channel_gradient_threshold: f64,

    /// 沟床比降极高值（°）。
    #[serde(default = "default_channel_gradient_max")]
    pub channel_gradient_max: f64,

    /// 松散物源量阈值（m³/km）。
    #[serde(default = "default_material_threshold")]
    pub material_threshold: f64,

    /// 松散物源量极高值（m³/km）。
    #[serde(default = "default_material_max")]
    pub material_max: f64,

    /// 降雨触发阈值（mm/24h）。
    #[serde(default = "default_rainfall_trigger")]
    pub rainfall_trigger: f64,

    /// 泥石流物质密度（t/m³）。
    #[serde(default = "default_debris_density")]
    pub debris_density: f64,

    /// 物质因子，反映土体可移动性 [0.5–2.0]。
    #[serde(default = "default_material_factor")]
    pub material_factor: f64,

    /// 体积经验系数：V = volume_factor × 0.5 × A × R × MF。
    #[serde(default = "default_volume_factor")]
    pub volume_factor: f64,

    /// 最小视摩擦角（°），防止极端长距离冲出。
    #[serde(default = "default_min_travel_angle")]
    pub min_travel_angle_deg: f64,
}

// ── 权重默认值 ──
fn default_slope_weight() -> f64 {
    0.25
}
fn default_aspect_weight() -> f64 {
    0.10
}
fn default_lithology_weight() -> f64 {
    0.20
}
fn default_rainfall_weight() -> f64 {
    0.20
}
fn default_fault_weight() -> f64 {
    0.10
}
fn default_vegetation_weight() -> f64 {
    0.15
}

// ── 隶属度默认值 ──
fn default_slope_a() -> f64 {
    15.0
} // 15°以下安全
fn default_slope_b() -> f64 {
    35.0
} // 35°以上高度危险
fn default_rainfall_a() -> f64 {
    100.0
} // <100mm 安全
fn default_rainfall_b() -> f64 {
    300.0
} // >300mm 高度危险
fn default_fault_a() -> f64 {
    200.0
} // <200m 高度危险
fn default_fault_b() -> f64 {
    1000.0
} // >1000m 安全
fn default_veg_a() -> f64 {
    0.2
} // NDVI < 0.2 高危险
fn default_veg_b() -> f64 {
    0.6
} // NDVI > 0.6 安全

// ── 泥石流默认值 ──
fn default_channel_gradient_threshold() -> f64 {
    15.0
}
fn default_channel_gradient_max() -> f64 {
    35.0
}
fn default_material_threshold() -> f64 {
    1000.0
}
fn default_material_max() -> f64 {
    10000.0
}
fn default_rainfall_trigger() -> f64 {
    50.0
}

fn default_debris_density() -> f64 {
    2.0
}
fn default_material_factor() -> f64 {
    1.0
}
fn default_volume_factor() -> f64 {
    1.0
}
fn default_min_travel_angle() -> f64 {
    5.0
}

impl Default for LandslideWeights {
    fn default() -> Self {
        Self {
            slope_weight: default_slope_weight(),
            aspect_weight: default_aspect_weight(),
            lithology_weight: default_lithology_weight(),
            rainfall_weight: default_rainfall_weight(),
            fault_weight: default_fault_weight(),
            vegetation_weight: default_vegetation_weight(),
            slope_a: default_slope_a(),
            slope_b: default_slope_b(),
            rainfall_a: default_rainfall_a(),
            rainfall_b: default_rainfall_b(),
            fault_a: default_fault_a(),
            fault_b: default_fault_b(),
            veg_a: default_veg_a(),
            veg_b: default_veg_b(),
        }
    }
}

impl Default for DebrisFlowParams {
    fn default() -> Self {
        Self {
            channel_gradient_threshold: default_channel_gradient_threshold(),
            channel_gradient_max: default_channel_gradient_max(),
            material_threshold: default_material_threshold(),
            material_max: default_material_max(),
            rainfall_trigger: default_rainfall_trigger(),
            debris_density: default_debris_density(),
            material_factor: default_material_factor(),
            volume_factor: default_volume_factor(),
            min_travel_angle_deg: default_min_travel_angle(),
        }
    }
}

geo_core::default_from_rules!(GeohazardConfig, "geohazard");

impl LandslideWeights {
    /// 检查权重之和是否合理（应接近 1.0）。
    pub fn total_weight(&self) -> f64 {
        self.slope_weight
            + self.aspect_weight
            + self.lithology_weight
            + self.rainfall_weight
            + self.fault_weight
            + self.vegetation_weight
    }
}

impl DebrisFlowParams {
    /// 沟床比降是否为危险区间。
    pub fn is_gradient_dangerous(&self, gradient_deg: f64) -> bool {
        gradient_deg >= self.channel_gradient_threshold
    }

    /// 松散物源量是否触发。
    pub fn is_material_enough(&self, material_volume: f64) -> bool {
        material_volume >= self.material_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GeohazardConfig::default();
        let w = &config.landslide;
        assert!(w.slope_weight > 0.0);
        assert!(w.aspect_weight > 0.0);
        let total = w.total_weight();
        assert!(
            (total - 1.0).abs() < 0.01,
            "weights must sum to ~1.0, got {total}"
        );
    }

    #[test]
    fn test_debris_flow_defaults() {
        let params = DebrisFlowParams::default();
        assert!(params.is_gradient_dangerous(20.0));
        assert!(!params.is_gradient_dangerous(10.0));
        assert!(params.is_material_enough(5000.0));
        assert!(!params.is_material_enough(500.0));
    }
}

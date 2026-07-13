//! Model input file generator traits.
//!
//! These traits define the interface for generating domain-specific
//! input files (DSSAT, MODFLOW, etc.).  Plugins depend on the trait;
//! Adapters implement it.  Wiring (in `geo-wiring`) injects concrete
//! impls at runtime.
//!
//! Architecture rule:  Core → trait definition (this file)
//!                      Plugin → depends on trait
//!                      Adapter → implements trait
//!                      Wiring → injects adapter into plugin

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════
// 1. DSSAT Crop Model File Generator
// ════════════════════════════════════════════════════════════════

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

/// DSSAT 输入文件生成器。
///
/// Implemented by `geo-adapter-dssat`.
pub trait DssatGenerator: Send + Sync {
    /// 生成 .WTH 天气文件。
    fn generate_wth(&self, station: &WeatherStation, daily_data: &[DailyWeather]) -> String;
    /// 生成 .SOL 土壤文件。
    fn generate_sol(&self, profile: &SoilProfile) -> String;
    /// 生成 .CUL 品种文件。
    fn generate_cul(&self, params: &CultivarParams) -> String;
    /// 月平均气象数据 → 逐日数据分解。
    fn monthly_to_daily_wth(
        &self,
        tmax_monthly: &[f64],
        tmin_monthly: &[f64],
        rain_monthly: &[f64],
        latitude: f64,
        elevation_m: f64,
    ) -> Vec<DailyWeather>;
    /// 从 SCS 水文土壤分组生成 DSSAT 土壤剖面。
    fn soil_from_scs_group(&self, soil_id: &str, group: &str, lat: f64, lon: f64) -> SoilProfile;
}

// ════════════════════════════════════════════════════════════════
// 2. MODFLOW Groundwater Model File Generator
// ════════════════════════════════════════════════════════════════

/// MODFLOW 离散化参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModflowGrid {
    pub nrow: usize,
    pub ncol: usize,
    pub nlay: usize,
    pub delr: f64,
    pub delc: f64,
    pub top: Vec<Vec<f64>>,
    pub bot: Vec<Vec<Vec<f64>>>,
}

/// MODFLOW 应力期。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModflowStressPeriod {
    pub nstp: usize,
    pub tsmult: f64,
}

/// MODFLOW 输入文件生成器。
///
/// Implemented by `geo-adapter-modflow`.
pub trait ModflowGenerator: Send + Sync {
    /// 生成 .NAM 名称文件。
    fn generate_nam(&self, model_name: &str, units: &[(&str, usize)]) -> String;
    /// 生成 .DIS 离散化文件。
    #[allow(clippy::too_many_arguments)]
    fn generate_dis(
        &self,
        nlay: usize,
        nrow: usize,
        ncol: usize,
        delr: f64,
        delc: f64,
        top: f64,
        bot: f64,
        nper: usize,
    ) -> String;
    /// 生成 .BAS6 基础文件。
    fn generate_bas6(&self, ibound_val: i32, strt: f64, nrow: usize, ncol: usize) -> String;
    /// 生成 .LPF 层属性流文件。
    fn generate_lpf(
        &self,
        hk: f64,
        vka: f64,
        ss: f64,
        sy: f64,
        nrow: usize,
        ncol: usize,
    ) -> String;
}

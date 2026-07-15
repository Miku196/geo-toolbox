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
    /// Station name (e.g., "Chengdu").
    pub name: String,
    /// Latitude in decimal degrees (WGS84).
    pub latitude: f64,
    /// Longitude in decimal degrees (WGS84).
    pub longitude: f64,
    /// Elevation above sea level (m).
    pub elevation_m: f64,
    /// WMO station identifier code.
    pub wmo_code: String,
}

/// DSSAT 逐日气象数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyWeather {
    /// Julian day of year (1--366).
    pub julian_day: u16,
    /// Daily solar radiation (MJ/m²).
    pub solar_rad_mj_m2: f64,
    /// Maximum daily temperature (°C).
    pub tmax_c: f64,
    /// Minimum daily temperature (°C).
    pub tmin_c: f64,
    /// Daily rainfall total (mm).
    pub rainfall_mm: f64,
}

/// DSSAT 土壤层。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilLayer {
    /// Depth to bottom of layer (cm).
    pub depth_cm: f64,
    /// Clay content (%).
    pub clay_pct: f64,
    /// Silt content (%).
    pub silt_pct: f64,
    /// Sand content (%).
    pub sand_pct: f64,
    /// Organic carbon content (%).
    pub organic_c_pct: f64,
    /// Bulk density (g/cm³).
    pub bulk_density_g_cm3: f64,
    /// Soil pH.
    pub ph: f64,
    /// Lower limit of plant-extractable soil water (cm³/cm³).
    pub ll: f64,
    /// Drained upper limit / field capacity (cm³/cm³).
    pub dul: f64,
    /// Saturated water content (cm³/cm³).
    pub sat: f64,
    /// Saturated hydraulic conductivity (cm/hr).
    pub ks: f64,
}

/// DSSAT 土壤剖面。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilProfile {
    /// Soil identifier code (DSSAT format, e.g. "IB00000001").
    pub soil_id: String,
    /// Descriptive soil name.
    pub soil_name: String,
    /// Ordered soil layers from surface downward.
    pub layers: Vec<SoilLayer>,
    /// Soil albedo (0.0--1.0).
    pub albedo: f64,
    /// Evaporation limit parameter (U in DSSAT).
    pub evaporation: f64,
}

/// DSSAT 品种参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultivarParams {
    /// Cultivar name or variety identifier.
    pub cultivar_name: String,
    /// DSSAT ecotype code.
    pub ecotype: String,
    /// Thermal time from emergence to end of juvenile phase (GDD).
    pub p1: f64,
    /// Critical photoperiod or daylength sensitivity coefficient.
    pub p2: f64,
    /// Thermal time from flowering to end of grain filling (GDD).
    pub p5: f64,
    /// Maximum kernel number per plant.
    pub g2: f64,
    /// Kernel growth rate during grain filling (mg/kernel/day).
    pub g3: f64,
    /// Phyllochron interval (GDD between leaf tip appearances).
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
    /// Number of rows.
    pub nrow: usize,
    /// Number of columns.
    pub ncol: usize,
    /// Number of layers.
    pub nlay: usize,
    /// Cell width along rows (L).
    pub delr: f64,
    /// Cell width along columns (L).
    pub delc: f64,
    /// Top elevation for each cell (row × col) (L).
    pub top: Vec<Vec<f64>>,
    /// Bottom elevation for each cell in each layer (lay × row × col) (L).
    pub bot: Vec<Vec<Vec<f64>>>,
}

/// MODFLOW 应力期。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModflowStressPeriod {
    /// Number of time steps in stress period.
    pub nstp: usize,
    /// Time step multiplier (geometric progression factor).
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

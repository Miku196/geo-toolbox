// ── 常量 ────────────────────────────────────────────────

pub(crate) const OUTPUT_DIR: &str = "output";
pub(crate) const AOI_NAME: &str = "德兴铜矿及周边生态修复区";
pub(crate) const STAC_ENDPOINT: &str = "https://planetarycomputer.microsoft.com/api/stac/v1";

// AOI bbox: 德兴铜矿
pub(crate) const MIN_LON: f64 = 117.49;
pub(crate) const MIN_LAT: f64 = 28.95;
pub(crate) const MAX_LON: f64 = 117.69;
pub(crate) const MAX_LAT: f64 = 29.12;

// IPPC Tier 1 排放因子 (tCO₂/ha/yr, 中国亚热带)
pub(crate) const FOREST_FACTOR: f64 = -6.5;
pub(crate) const GRASSLAND_FACTOR: f64 = -1.5;
pub(crate) const CROPLAND_FACTOR: f64 = 0.3;
pub(crate) const BUILT_UP_FACTOR: f64 = 2.5;
pub(crate) const BARE_FACTOR: f64 = 0.0;

pub(crate) const GDAL_TRANSLATE: &str = r"E:Program FilesQGISQT6 3.40.13ingdal_translate.exe";

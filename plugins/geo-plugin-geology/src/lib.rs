#![allow(missing_docs)]

pub mod config;
pub mod lithology;
pub mod stratigraphy;
pub mod structures;
pub mod tools;
pub mod trait_impl;

pub use config::GeologyConfig;
pub use lithology::{
    classify_lithology, engineering_parameters, lithology_from_code, LithologyClass,
    LithologyResult,
};
pub use stratigraphy::{
    layer_elevation, stratigraphic_column, stratigraphic_model_3d, LayerDefinition,
    StratigraphicModel,
};
pub use structures::{
    fault_plane_geometry, fold_geometry, structure_attitude, FaultGeometry, FoldGeometry,
    StructureAttitude,
};
pub use trait_impl::GeologyPlugin;

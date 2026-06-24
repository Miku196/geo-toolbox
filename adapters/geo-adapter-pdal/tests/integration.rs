//! Integration tests for geo-adapter-pdal.

use geo_adapter_pdal::PdalAdapter;

#[test]
fn test_adapter_construction() {
    let adapter = PdalAdapter::new();
    assert_eq!(adapter.name(), "pdal");
    assert!(adapter.description().contains("PDAL"));
}

#[test]
fn test_adapter_default() {
    let adapter = PdalAdapter::default();
    assert_eq!(adapter.version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_custom_bin() {
    let adapter = PdalAdapter::with_bin("/opt/pdal/bin/pdal");
    assert_eq!(adapter.name(), "pdal");
}

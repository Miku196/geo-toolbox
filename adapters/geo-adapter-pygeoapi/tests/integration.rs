//! Integration tests for geo-adapter-pygeoapi.

use geo_adapter_pygeoapi::PyGeoAdapter;

/// Verify adapter construction and Plugin trait implementation.
#[test]
fn test_adapter_construction() {
    let adapter = PyGeoAdapter::new();
    assert_eq!(adapter.name(), "pygeoapi");
    assert!(!adapter.version().is_empty());
    assert!(adapter.description().contains("PyO3"));
}

#[test]
fn test_adapter_default() {
    let adapter = PyGeoAdapter::default();
    assert_eq!(adapter.name(), "pygeoapi");
}

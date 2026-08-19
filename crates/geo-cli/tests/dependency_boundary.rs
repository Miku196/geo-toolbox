//! Dependency-boundary guard test.
//!
//! Enforces the five-layer single-direction dependency rule for geo-cli:
//! geo-cli must NOT depend on any adapter crate directly — adapters are only
//! reachable through the geo-wiring composition root.
//!
//! This is a Cargo-test-level guard because dependency edges can't be asserted
//! from inside Rust the way a source import can.  We scan geo-cli's own source
//! and manifest (via CARGO_MANIFEST_DIR) and fail if any adapter crate-name
//! leaks in, which would mean the boundary was bypassed.

use std::path::{Path, PathBuf};

/// Adapter crate names that geo-cli must never reference directly.
const ADAPTER_CRATES: &[&str] = &[
    "geo_adapters_io",
    "geo_adapters_geo",
    "geo_adapters_sim",
    "geo_adapter_qgis",
];

fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("entry");
        let p = entry.path();
        if p.is_dir() {
            walk_rs_files(&p, out);
        } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(p);
        }
    }
}

#[test]
fn geo_cli_source_does_not_import_adapter_crates() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let mut files = Vec::new();
    walk_rs_files(&src_dir, &mut files);

    assert!(
        !files.is_empty(),
        "no .rs source files found under {src_dir:?}"
    );

    let mut offenders: Vec<String> = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("read source file");
        for crate_name in ADAPTER_CRATES {
            // Only flag a concrete reference to the adapter crate path
            // (`geo_adapters_geo::...` / `geo_adapter_qgis::...`).  A bare
            // mention in a doc comment of the crate's concept is tolerated,
            // but the underscored crate name in source signals a direct use.
            if text.contains(&format!("{crate_name}::")) {
                offenders.push(format!("{}: uses {crate_name}::", file.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "geo-cli source must not reference adapter crates directly (route through geo-wiring):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn geo_cli_manifest_does_not_depend_on_adapter_crates() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml =
        std::fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("read geo-cli Cargo.toml");

    // Only inspect the [dependencies] section (up to [features] / next table).
    let deps_section = cargo_toml.split("[features]").next().unwrap_or(&cargo_toml);

    let offenders: Vec<&str> = ADAPTER_CRATES
        .iter()
        .copied()
        .filter(|c| deps_section.contains(c))
        .collect();

    assert!(
        offenders.is_empty(),
        "geo-cli Cargo.toml [dependencies] must not list adapter crates directly; got: {offenders:?} (access adapters via geo-wiring only)",
    );
}

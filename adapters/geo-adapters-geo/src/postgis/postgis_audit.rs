//! Audit trail for carbon accounting traceability.
//!
//! Every carbon calculation can be traced back to its source data
//! via DVC hashes and factor UUIDs. The audit trail is embedded in
//! each `EmissionResult` and can be queried from the database.

/// Full audit trail for a carbon calculation.
///
/// Links the remote sensing input (lc_dvc_hash), emission factor
/// (factor_set_id), and factor source version (factor_dvc_hash).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditTrail {
    /// DVC hash of the landcover / remote sensing data used.
    pub lc_dvc_hash: Option<String>,
    /// DVC hash of the emission factor dataset version.
    pub factor_dvc_hash: Option<String>,
    /// UUID of the factor_set row in `factor_registry`.
    pub factor_set_id: String,
}

impl AuditTrail {
    /// Returns true if all audit fields are present (fully traceable).
    pub fn is_complete(&self) -> bool {
        self.lc_dvc_hash.is_some()
            && self.factor_dvc_hash.is_some()
            && !self.factor_set_id.is_empty()
    }

    /// Summary for human-readable output.
    pub fn summary(&self) -> String {
        format!(
            "lc={} factor_set={} factor_version={}",
            self.lc_dvc_hash.as_deref().unwrap_or("?"),
            self.factor_set_id,
            self.factor_dvc_hash.as_deref().unwrap_or("?")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_complete() {
        let trail = AuditTrail {
            lc_dvc_hash: Some("abc".into()),
            factor_dvc_hash: Some("def".into()),
            factor_set_id: "ghi".into(),
        };
        assert!(trail.is_complete());
    }

    #[test]
    fn test_audit_incomplete() {
        let trail = AuditTrail {
            lc_dvc_hash: None,
            factor_dvc_hash: Some("def".into()),
            factor_set_id: "ghi".into(),
        };
        assert!(!trail.is_complete());
    }
}

//! Path-safety validation for the QGIS adapter.
//!
//! The adapter passes untrusted `output`/`input`/`overlay`/... parameters
//! straight to the `qgis_process` backend, which will happily read from and
//! write to any path — including `../../../../etc/evil`, `C:\Windows\...` or
//! `/etc/...`. This module confines every file path handled here to a set of
//! explicitly allowed root directories (the current workspace and the system
//! temporary directory) and refuses anything that escapes them via `..`,
//! absolute-path jumps, or symlinks.

use std::path::{Component, Path, PathBuf};

/// Canonicalize `path` if it exists; otherwise canonicalize its nearest
/// existing ancestor and re-append the remaining components. Resolving
/// symlinks here means a `..` or symlink cannot smuggle a path out of an
/// allowed root.
fn canonicalize_for_check(path: &Path) -> std::io::Result<PathBuf> {
    if let Ok(c) = std::fs::canonicalize(path) {
        return Ok(c);
    }
    // Fall back: canonicalize the longest existing ancestor, then re-append
    // the components that do not exist yet.
    let mut missing: Vec<PathBuf> = Vec::new();
    let mut ancestor = path.to_path_buf();
    loop {
        match std::fs::canonicalize(&ancestor) {
            Ok(c) => {
                let mut result = c;
                for component in missing.iter().rev() {
                    result.push(component);
                }
                return Ok(result);
            }
            Err(_) => match ancestor.parent() {
                Some(parent) if parent != ancestor => {
                    if let Some(name) = ancestor.file_name() {
                        missing.push(PathBuf::from(name));
                    }
                    ancestor = parent.to_path_buf();
                }
                _ => return Ok(path.to_path_buf()),
            },
        }
    }
}

/// Lexically normalize an absolute path, collapsing `.` and `..` components.
/// Returns `None` if a parent-directory (`..`) pops above the path root,
/// i.e. the path would escape via directory traversal.
fn lexical_normalize_absolute(path: &Path) -> Option<PathBuf> {
    let mut kept: Vec<Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                if matches!(kept.last(), Some(Component::Normal(_))) {
                    kept.pop();
                } else {
                    // A `..` right after the root/prefix escapes upward.
                    return None;
                }
            }
            Component::CurDir => {}
            other => kept.push(other),
        }
    }
    let mut out = PathBuf::new();
    for c in kept {
        out.push(c.as_os_str());
    }
    Some(out)
}

/// Validate that `path` is confined within one of `allowed_roots`.
///
/// Rules enforced (Windows and Unix alike):
///   * the path must be non-empty;
///   * it must not contain NUL or control characters;
///   * `..` directory-traversal that escapes the root is rejected;
///   * the (lexically normalized, symlink-resolved) path must resolve inside
///     at least one allowed root — so absolute paths outside the workspace and
///     relative paths pointing outside it are rejected.
///
/// On success returns the resolved absolute path as a `PathBuf`; on failure
/// returns a human-readable reason string.
pub fn validate_safe_path(path: &str, allowed_roots: &[PathBuf]) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is empty".into());
    }
    if path.contains('\0') || path.chars().any(char::is_control) {
        return Err(format!("path contains control characters: {path:?}"));
    }

    let p = Path::new(trimmed);

    // Build the absolute candidate (relative paths resolve against the cwd).
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("cannot determine working directory: {e}"))?;
        cwd.join(p)
    };

    // Lexical normalization — catches `..` traversal outright.
    let normalized = lexical_normalize_absolute(&abs)
        .ok_or_else(|| format!("path escapes working directory via '..': {path:?}"))?;

    // Resolve symlinks for a robust containment check.
    let canonical = canonicalize_for_check(&normalized)
        .map_err(|e| format!("cannot resolve path {path:?}: {e}"))?;

    // The canonical path must stay inside at least one allowed root.
    let mut allowed_ok = false;
    for root in allowed_roots {
        let root_canon = canonicalize_for_check(root).unwrap_or_else(|_| root.clone());
        if canonical.starts_with(&root_canon) {
            allowed_ok = true;
            break;
        }
    }
    if !allowed_ok {
        return Err(format!(
            "path {path:?} resolves outside allowed directories"
        ));
    }

    Ok(normalized)
}

/// Default set of directories the QGIS adapter is permitted to read/write:
/// the current working directory (workspace) plus the system temp directory.
pub fn default_allowed_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    roots.push(std::env::temp_dir());
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a validation against a temp-dir-only allowlist so tests are
    /// independent of the machine's cwd.
    fn temp_roots() -> Vec<PathBuf> {
        vec![std::env::temp_dir()]
    }

    #[test]
    fn rejects_empty_path() {
        assert!(validate_safe_path("", &temp_roots()).is_err());
        assert!(validate_safe_path("   ", &temp_roots()).is_err());
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(validate_safe_path("../../../../etc/evil", &temp_roots()).is_err());
        assert!(validate_safe_path("a/../../../../etc/evil", &temp_roots()).is_err());
    }

    #[test]
    fn rejects_absolute_system_paths() {
        // Unix-style absolute escapes outside the allowed (temp) root.
        assert!(validate_safe_path("/etc/passwd", &temp_roots()).is_err());
        // On Unix /tmp is usually the temp root, so /tmp/weird falls inside the
        // allowed root and is permitted; the escape checks that matter are the
        // /etc and parent-traversal cases above/below.
        assert!(validate_safe_path("C:\\Windows\\System32\\evil", &temp_roots()).is_err());
    }

    #[test]
    fn rejects_control_characters() {
        assert!(validate_safe_path("ok\0evil", &temp_roots()).is_err());
    }

    #[test]
    fn accepts_path_inside_allowed_root() {
        let root = std::env::temp_dir();
        // A direct child of the allowed root must be accepted.
        let child = root.join("validate_ok_child_dir").join("out.gpkg");
        let res = validate_safe_path(&child.to_string_lossy(), &[root.clone()]);
        assert!(res.is_ok(), "should accept: {:?}", res.err());
        // Even a non-existent child (write target) resolves lexically inside root.
        let ghost = root.join("nope_does_not_exist/out.gpkg");
        assert!(validate_safe_path(&ghost.to_string_lossy(), &temp_roots()).is_ok());
    }

    #[test]
    fn lexical_normalize_rejects_escape() {
        assert!(lexical_normalize_absolute(Path::new("/etc/../../x")).is_none());
        assert!(lexical_normalize_absolute(Path::new("C:/a/../../..")).is_none());
        // Plain ../ beyond the current dir's root should also fail.
        assert!(lexical_normalize_absolute(Path::new("/tmp/../../..")).is_none());
    }

    #[test]
    fn allows_relative_child_against_temp_root() {
        // A relative path that is a sibling of temp root should be rejected
        // because it escapes temp, while a plain filename resolves under cwd
        // (which is not in this test's allowlist) — verify it errs rather
        // than resolving outside the allowed root.
        let res = validate_safe_path("output.gpkg", &temp_roots());
        // With cwd == some real dir, "output.gpkg" resolves to cwd/output.gpkg,
        // which is not inside the temp-root allowlist, so it must be rejected.
        assert!(res.is_err());
    }
}

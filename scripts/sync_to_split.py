#!/usr/bin/env python3
"""Sync source files from the geo-toolbox monorepo (base) to the split/ release repos.

Base repo (D:/geo/geo-toolbox) is the single source of truth; the four split
repos under D:/geo/split (geo-toolbox-core/web/agent/edge) are release forms.
This script copies *source* files (src/**.rs, benches/**.rs, tests/**.rs and data
files) along a fixed mapping table, while deliberately NOT touching Cargo.toml /
Cargo.lock / .cargo/ (the split workspaces adapt their own dependency manifests).

Subcommands:
    sync      Compare sha256 and copy differing files (logs a copy manifest).
    dry-run   Report differences only (no writes).
    report    Per-mapping summary of differing file counts.

Options:
    --skip-git   Do not guard against overwriting locally-modified (dirty) target
                 files. By default a dirty target file is warned about and skipped.

Read-only and safe by default: never touch the split repos unless you pass sync
explicitly, and even then it refuses to overwrite a dirty (uncommitted) target
file unless --skip-git is given.
"""

import argparse
import hashlib
import subprocess
import sys
from pathlib import Path

BASE = Path(__file__).resolve().parents[1]          # D:/geo
SOURCE_ROOT = BASE / "geo-toolbox"                   # base monorepo
SPLIT_ROOT = BASE / "split"                          # split/ release repos

#: Directories (per crate) from which we sync `.rs` files.
_RUST_DIRS = {"src", "benches", "tests"}

#: Manifest / lock / cargo-config artifacts that must NEVER be synced
#: (split workspaces adapt their own dependency resolution).
_SKIP_NAMES = {"Cargo.toml", "Cargo.lock"}
_SKIP_DIRS = {".cargo", ".git", "target", "node_modules", "pkg", ".github"}

#: File extensions (lowercase) treated as "data files" worth syncing. This is the
#: safe allow-list; anything else (e.g. unknown binaries) is skipped.
_DATA_EXTENSIONS = {
    ".tera", ".sql", ".toml", ".geojson", ".json", ".csv", ".tsv", ".yaml",
    ".yml", ".py", ".html", ".md", ".rst", ".txt", ".example", ".qgs", ".sh",
    ".ts", ".js", ".wasm", ".tif", ".tiff", ".png", ".jpg", ".jpeg", ".svg",
}


class Mapping:
    """One source directory -> one-or-more split target directories."""

    def __init__(self, source_rel, targets):
        self.source_rel = source_rel                  # e.g. "core"
        self.targets = targets                        # e.g. [("geo-toolbox-core", "core")]

    @property
    def source(self):
        return SOURCE_ROOT / self.source_rel

    def target_paths(self):
        return [(SPLIT_ROOT / repo / rel, repo) for repo, rel in self.targets]


#: The fixed mapping table (base path -> split repo + path).
MAPPINGS = [
    Mapping("core",              [("geo-toolbox-core",  "core")]),
    Mapping("plugins",           [("geo-toolbox-core",  "plugins")]),
    Mapping("examples",          [("geo-toolbox-core",  "examples")]),
    Mapping("crates/geo-wasm",   [("geo-toolbox-web",   "crates/geo-wasm")]),
    Mapping("crates/geo-server", [("geo-toolbox-agent", "crates/geo-server")]),
    Mapping("crates/geo-wiring", [("geo-toolbox-agent", "crates/geo-wiring")]),
    Mapping("adapters",          [("geo-toolbox-agent", "adapters")]),
    Mapping("crates/geo-cli",    [("geo-toolbox-edge",  "crates/geo-cli"),
                                  ("geo-toolbox-agent", "crates/geo-cli")]),
]


def should_sync(rel_parts):
    """Decide whether a file should be synced.

    rel_parts: path segments relative to a mapping source dir.

    Rules:
      * skip Cargo.toml / Cargo.lock (by name) and anything under .cargo/ .git/
        target/ node_modules/ pkg/ .github/ (by directory segment);
      * sync `.rs` only when it lives under src/ benches/ tests/;
      * sync non-`.rs` files only when their extension is a known data-file kind.
    """
    name = rel_parts[-1]
    if name in _SKIP_NAMES:
        return False
    for part in rel_parts[:-1]:
        if part in _SKIP_DIRS:
            return False
    if name.endswith(".rs"):
        return any(part in _RUST_DIRS for part in rel_parts[:-1])
    return Path(name).suffix.lower() in _DATA_EXTENSIONS


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def iter_candidate_files(mapping):
    """Yield (source_abs, rel_parts) for every sync candidate under a mapping."""
    src = mapping.source
    if not src.is_dir():
        return
    for p in sorted(src.rglob("*")):
        if not p.is_file():
            continue
        try:
            rel_parts = p.relative_to(src).parts
        except ValueError:
            continue
        if should_sync(rel_parts):
            yield p, rel_parts


class Diff:
    def __init__(self, source, target, rel, reason, dirty=False):
        self.source = source
        self.target = target
        self.rel = rel
        self.reason = reason
        self.dirty = dirty


def collect_dirty_files(repo_root):
    """Return the set of dirty (uncommitted) file absolute paths in a repo, or an
    empty set on any git failure (safe default: treat as clean)."""
    if not (repo_root / ".git").exists():
        return set()
    try:
        out = subprocess.run(
            ["git", "-C", str(repo_root), "status", "--porcelain", "-z"],
            capture_output=True, text=True, check=True,
        ).stdout
    except (subprocess.SubprocessError, OSError):
        return set()
    dirty = set()
    for entry in out.split("\0"):
        if not entry:
            continue
        # Entry layout: "XY" status (2 chars, 3 for renames) then a space then path.
        rest = entry[3:]
        if " -> " in rest:
            rest = rest.split(" -> ", 1)[1]           # rename: take the new name
        if rest:
            dirty.add((repo_root / rest).resolve())
    return dirty


def files_differ(src_abs, target_abs):
    if not target_abs.exists():
        return True
    if src_abs.stat().st_size != target_abs.stat().st_size:
        return True
    return sha256(src_abs) != sha256(target_abs)


def compute_diffs(mappings, skip_git):
    """Walk every mapping and yield Diff objects for files needing a copy."""
    dirty_cache = {}
    for mapping in mappings:
        for target_root, repo in mapping.target_paths():
            dirty = None
            if not skip_git:
                if repo not in dirty_cache:
                    dirty_cache[repo] = collect_dirty_files(target_root)
                dirty = dirty_cache[repo]
            for src_abs, rel_parts in iter_candidate_files(mapping):
                rel = "/".join(rel_parts)
                target_abs = target_root / Path(*rel_parts)
                if not files_differ(src_abs, target_abs):
                    continue
                if dirty is not None and target_abs.resolve() in dirty:
                    yield Diff(src_abs, target_abs, rel, "dirty", dirty=True)
                else:
                    yield Diff(src_abs, target_abs, rel,
                               "added" if not target_abs.exists() else "modified")


def run_sync(mappings, skip_git):
    diffs = list(compute_diffs(mappings, skip_git))
    copied, dirty_skipped = [], []
    for d in diffs:
        if d.dirty:
            dirty_skipped.append(d)
            continue
        d.target.parent.mkdir(parents=True, exist_ok=True)
        with open(d.source, "rb") as r, open(d.target, "wb") as w:
            w.write(r.read())
        copied.append(d)
    for d in copied:
        print("  copied  %s  (%s)" % (d.rel, d.reason))
    for d in dirty_skipped:
        print("  SKIPPED %s  (target has uncommitted changes)" % d.rel)
    print()
    print("sync: %d copied, %d skipped (dirty)" % (len(copied), len(dirty_skipped)))
    return 0


def run_dry_run(mappings, skip_git):
    diffs = list(compute_diffs(mappings, skip_git))
    if not diffs:
        print("dry-run: no differences (all mapped files identical).")
        return 0
    for d in diffs:
        tag = "DIRTY " if d.dirty else d.reason.upper()
        try:
            dest = d.target.relative_to(BASE)
        except ValueError:
            dest = d.target
        print("  [%-7s] %s  ->  %s" % (tag, d.rel, dest))
    dirty_n = sum(1 for d in diffs if d.dirty)
    print()
    print("dry-run: %d file(s) would be copied (%d blocked by dirty target)."
          % (len(diffs), dirty_n))
    return 0


def run_report(mappings, skip_git):
    print("Per-mapping difference summary:")
    print("-" * 78)
    total = 0
    for mapping in mappings:
        for target_root, repo in mapping.target_paths():
            target_rel = mapping.targets[0][1]
            label = "%s/%s" % (repo, target_rel)
            added = modified = dirty_n = 0
            dirty = collect_dirty_files(target_root) if not skip_git else set()
            for src_abs, rel_parts in iter_candidate_files(mapping):
                target_abs = target_root / Path(*rel_parts)
                if not files_differ(src_abs, target_abs):
                    continue
                if not skip_git and target_abs.resolve() in dirty:
                    dirty_n += 1
                elif not target_abs.exists():
                    added += 1
                else:
                    modified += 1
            n = added + modified + dirty_n
            total += n
            print("  %-24s -> %-30s %4d diff  (added %d, modified %d, dirty %d)"
                  % (mapping.source_rel, label, n, added, modified, dirty_n))
    print("-" * 78)
    print("Total differing files: %d" % total)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="sync_to_split.py",
        description="Sync base-monorepo source files into the split/ release repos.",
    )
    parser.add_argument(
        "command", choices=["sync", "dry-run", "report"],
        help="sync = copy differences; dry-run = list only; report = summary counts",
    )
    parser.add_argument(
        "--skip-git", action="store_true",
        help="do not skip (warn-only) target files that have uncommitted changes",
    )
    args = parser.parse_args(argv)

    if args.command == "sync":
        return run_sync(MAPPINGS, args.skip_git)
    if args.command == "dry-run":
        return run_dry_run(MAPPINGS, args.skip_git)
    if args.command == "report":
        return run_report(MAPPINGS, args.skip_git)
    return 2


if __name__ == "__main__":
    sys.exit(main())

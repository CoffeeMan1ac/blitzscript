//! Discovery: find runnable commands that ALREADY EXIST in the user's repos.
//!
//! The repo is the source of truth. Everything here reads committed files
//! (package.json, Makefile, ...) and yields names + how to invoke them. The
//! SQLite registry is only a derived cache of what we find here.
//!
//! Adding a new source (justfile, Taskfile, ...) later means: add a parser
//! module that returns `Vec<DiscoveredCommand>` and call it from `scan_root`.

pub mod makefile;
pub mod package_json;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Where a discovered command came from. Extend this enum (not the call sites)
/// when adding new parsers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Npm,
    Make,
}

impl SourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceType::Npm => "npm",
            SourceType::Make => "make",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "npm" => Some(SourceType::Npm),
            "make" => Some(SourceType::Make),
            _ => None,
        }
    }
}

/// A single runnable command discovered in a repo.
///
/// `id` is a stable, deterministic identity derived from (source, cwd, name) so
/// that a rescan produces the same ids for unchanged commands. The DB is a
/// cache; this id is what we key it on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredCommand {
    pub id: String,
    /// Nearest ancestor dir that looks like a repo root (has .git or a manifest).
    pub repo: String,
    pub source: SourceType,
    pub name: String,
    pub description: Option<String>,
    /// The resolved invocation, e.g. `npm run build` or `make test`.
    pub invocation: String,
    /// Absolute working directory the command should run in.
    pub cwd: String,
}

impl DiscoveredCommand {
    pub fn new(
        repo: String,
        source: SourceType,
        name: String,
        description: Option<String>,
        invocation: String,
        cwd: String,
    ) -> Self {
        let id = stable_id(source, &cwd, &name);
        DiscoveredCommand {
            id,
            repo,
            source,
            name,
            description,
            invocation,
            cwd,
        }
    }
}

/// Deterministic id from the identity tuple. Not cryptographic — just a stable
/// key for the cache. (FNV-1a over the joined fields, hex-encoded.)
fn stable_id(source: SourceType, cwd: &str, name: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut mix = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    mix(source.as_str().as_bytes());
    mix(b"\0");
    mix(cwd.as_bytes());
    mix(b"\0");
    mix(name.as_bytes());
    format!("{}-{:016x}", source.as_str(), hash)
}

/// Directory names we never descend into.
const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "dist"];

/// Walk `root` recursively and collect every discovered command.
///
/// `repo` for each command is the nearest ancestor directory (at or above the
/// manifest's own dir) that contains a `.git` entry; if none is found we fall
/// back to the manifest's own directory.
pub fn scan_root(root: &Path) -> Vec<DiscoveredCommand> {
    let mut out = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Prune the heavy/irrelevant directories before descending.
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    if SKIP_DIRS.contains(&name) {
                        return false;
                    }
                }
            }
            true
        });

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        let manifest_dir = match path.parent() {
            Some(d) => d,
            None => continue,
        };
        let repo = nearest_repo_root(manifest_dir, root);
        let cwd = manifest_dir.to_string_lossy().to_string();

        match file_name {
            "package.json" => {
                if let Ok(text) = std::fs::read_to_string(path) {
                    out.extend(package_json::parse(&text, &repo, &cwd, manifest_dir));
                }
            }
            "Makefile" | "makefile" | "GNUmakefile" => {
                if let Ok(text) = std::fs::read_to_string(path) {
                    out.extend(makefile::parse(&text, &repo, &cwd));
                }
            }
            _ => {}
        }
    }

    out
}

/// Nearest ancestor of `start` (inclusive), not going above `root`, that
/// contains a `.git` entry. Falls back to `start` itself.
fn nearest_repo_root(start: &Path, root: &Path) -> String {
    let mut dir: Option<&Path> = Some(start);
    while let Some(d) = dir {
        if d.join(".git").exists() {
            return d.to_string_lossy().to_string();
        }
        if d == root {
            break;
        }
        dir = d.parent();
    }
    start.to_string_lossy().to_string()
}

/// Helper exposed for parsers that want a repo-relative-ish working dir string.
#[allow(dead_code)]
pub(crate) fn abs(path: &Path) -> PathBuf {
    path.to_path_buf()
}

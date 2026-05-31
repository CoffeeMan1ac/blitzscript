//! Parser for `package.json` `scripts`.
//!
//! Invocation is `<runner> run <name>`, where the runner is chosen by the
//! lockfile present alongside the manifest (pnpm/yarn), defaulting to npm.

use super::{DiscoveredCommand, SourceType};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct PackageJson {
    #[serde(default)]
    scripts: BTreeMap<String, String>,
}

/// The package manager used to run scripts, decided by lockfile presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runner {
    Npm,
    Pnpm,
    Yarn,
}

impl Runner {
    fn cmd(self) -> &'static str {
        match self {
            Runner::Npm => "npm",
            Runner::Pnpm => "pnpm",
            Runner::Yarn => "yarn",
        }
    }
}

/// Detect the runner from lockfiles in `dir`. pnpm and yarn win over npm when
/// their lockfile is present; otherwise default to npm.
pub fn detect_runner(dir: &Path) -> Runner {
    if dir.join("pnpm-lock.yaml").exists() {
        Runner::Pnpm
    } else if dir.join("yarn.lock").exists() {
        Runner::Yarn
    } else {
        // package-lock.json or nothing -> npm.
        Runner::Npm
    }
}

/// Build the invocation string for a script name under a given runner.
/// All three managers use `<runner> run <name>`.
fn invocation_for(runner: Runner, name: &str) -> String {
    format!("{} run {}", runner.cmd(), name)
}

/// Parse a `package.json` document into discovered commands.
///
/// `manifest_dir` is used to detect the lockfile; `cwd` is the absolute
/// working directory string we attach to each command.
pub fn parse(text: &str, repo: &str, cwd: &str, manifest_dir: &Path) -> Vec<DiscoveredCommand> {
    let parsed: PackageJson = match serde_json::from_str(text) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let runner = detect_runner(manifest_dir);

    parsed
        .scripts
        .into_iter()
        .map(|(name, body)| {
            // The script body is the closest thing to a description here; keep
            // it short so the UI stays scannable.
            let description = if body.trim().is_empty() {
                None
            } else {
                Some(body)
            };
            DiscoveredCommand::new(
                repo.to_string(),
                SourceType::Npm,
                name.clone(),
                description,
                invocation_for(runner, &name),
                cwd.to_string(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_scripts_map() {
        let json = r#"{
            "name": "demo",
            "scripts": {
                "build": "vite build",
                "test": "vitest run",
                "blank": ""
            }
        }"#;
        let dir = Path::new("/tmp/does-not-matter");
        let cmds = parse(json, "/repo", "/repo/app", dir);

        assert_eq!(cmds.len(), 3);

        let build = cmds.iter().find(|c| c.name == "build").unwrap();
        assert_eq!(build.invocation, "npm run build");
        assert_eq!(build.description.as_deref(), Some("vite build"));
        assert_eq!(build.source, SourceType::Npm);
        assert_eq!(build.cwd, "/repo/app");

        // Empty script body yields no description.
        let blank = cmds.iter().find(|c| c.name == "blank").unwrap();
        assert_eq!(blank.description, None);
    }

    #[test]
    fn missing_scripts_key_is_empty_not_error() {
        let json = r#"{ "name": "demo", "version": "1.0.0" }"#;
        let cmds = parse(json, "/repo", "/repo", Path::new("/tmp"));
        assert!(cmds.is_empty());
    }

    #[test]
    fn malformed_json_yields_nothing() {
        let cmds = parse("{ not json", "/repo", "/repo", Path::new("/tmp"));
        assert!(cmds.is_empty());
    }

    #[test]
    fn lockfile_detection_picks_runner() {
        let base = std::env::temp_dir().join(format!("bs-pkg-{}", std::process::id()));

        // npm by default (no lockfile).
        let npm_dir = base.join("npm");
        fs::create_dir_all(&npm_dir).unwrap();
        assert_eq!(detect_runner(&npm_dir), Runner::Npm);

        // pnpm-lock.yaml -> pnpm.
        let pnpm_dir = base.join("pnpm");
        fs::create_dir_all(&pnpm_dir).unwrap();
        fs::write(pnpm_dir.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();
        assert_eq!(detect_runner(&pnpm_dir), Runner::Pnpm);

        // yarn.lock -> yarn.
        let yarn_dir = base.join("yarn");
        fs::create_dir_all(&yarn_dir).unwrap();
        fs::write(yarn_dir.join("yarn.lock"), "# yarn lockfile v1").unwrap();
        assert_eq!(detect_runner(&yarn_dir), Runner::Yarn);

        // The invocation string follows the detected runner.
        let json = r#"{ "scripts": { "dev": "vite" } }"#;
        let cmds = parse(json, "/r", "/r", &pnpm_dir);
        assert_eq!(cmds[0].invocation, "pnpm run dev");

        let _ = fs::remove_dir_all(&base);
    }
}

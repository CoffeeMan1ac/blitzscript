//! Tauri command handlers — the bridge the frontend calls.
//!
//! These are thin: they validate inputs, delegate to discovery/db/exec/safety,
//! and shape results for the UI. No business logic of consequence lives here.

use crate::exec::{self, RunRegistry, RunRequest};
use blitzscript_core::db::{Db, HistoryRow};
use blitzscript_core::discovery::{self, DiscoveredCommand};
use blitzscript_core::safety::{self, SafetyAssessment};
use std::path::Path;
use tauri::{AppHandle, State};

const ROOT_KEY: &str = "root_dir";

/// Get the persisted root directory, if one was chosen before.
#[tauri::command]
pub fn get_root(db: State<Db>) -> Result<Option<String>, String> {
    db.get_setting(ROOT_KEY).map_err(stringify)
}

/// Persist the chosen root directory.
#[tauri::command]
pub fn set_root(db: State<Db>, root: String) -> Result<(), String> {
    db.set_setting(ROOT_KEY, &root).map_err(stringify)
}

/// Rescan `root`, REPLACING the discovered-commands cache, and return the
/// freshly discovered list. The cache is rebuilt wholesale so it can never
/// drift from the repos.
#[tauri::command]
pub fn scan(db: State<Db>, root: String) -> Result<Vec<DiscoveredCommand>, String> {
    let path = Path::new(&root);
    if !path.is_dir() {
        return Err(format!("not a directory: {root}"));
    }
    let cmds = discovery::scan_root(path);
    db.replace_discovered(&cmds).map_err(stringify)?;
    db.set_setting(ROOT_KEY, &root).map_err(stringify)?;
    Ok(cmds)
}

/// Return the cached discovered commands without rescanning (used on startup).
#[tauri::command]
pub fn list_commands(db: State<Db>) -> Result<Vec<DiscoveredCommand>, String> {
    db.list_discovered().map_err(stringify)
}

/// Run the operator-error heuristic against a resolved command string.
#[tauri::command]
pub fn assess_command(resolved_command: String) -> SafetyAssessment {
    safety::assess(&resolved_command)
}

/// Start a run. The frontend supplies `run_id` (so it can subscribe to this
/// run's events before we start) and the already-resolved command string.
/// Streaming, exit capture and history recording happen on a background thread.
#[tauri::command]
pub fn run_command(
    app: AppHandle,
    run_id: String,
    name: String,
    source: String,
    resolved_command: String,
    cwd: String,
) -> Result<(), String> {
    if !Path::new(&cwd).is_dir() {
        return Err(format!("working directory does not exist: {cwd}"));
    }
    exec::spawn(
        app,
        RunRequest {
            run_id,
            name,
            source,
            resolved_command,
            cwd,
        },
    )
}

/// Cancel an in-flight run by id.
#[tauri::command]
pub fn cancel_run(registry: State<RunRegistry>, run_id: String) -> bool {
    registry.cancel(&run_id)
}

/// History, newest-first, optionally filtered by a substring of the command,
/// cwd or name.
#[tauri::command]
pub fn list_history(
    db: State<Db>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<HistoryRow>, String> {
    let q = query.unwrap_or_default();
    let limit = limit.unwrap_or(200).clamp(1, 1000);
    db.list_history(&q, limit).map_err(stringify)
}

fn stringify<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

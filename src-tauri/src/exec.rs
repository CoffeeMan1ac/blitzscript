//! Execution via a PTY so child processes emit ANSI exactly as they would in a
//! real terminal. Output is streamed to the frontend over Tauri events; exit
//! code and wall-clock duration are captured and written to history.
//!
//! Commands run with the user's normal shell and inherited environment — this
//! app deliberately introduces no competing env/secret source.

use blitzscript_core::db::Db;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

/// Event name carrying a chunk of live output for a run.
pub const EVENT_OUTPUT: &str = "bs://output";
/// Event name signalling a run finished (with exit code + duration).
pub const EVENT_EXIT: &str = "bs://exit";

/// Registry of in-flight runs so they can be cancelled by id. Managed by Tauri.
#[derive(Default)]
pub struct RunRegistry {
    inner: Mutex<HashMap<String, Box<dyn ChildKiller + Send + Sync>>>,
}

impl RunRegistry {
    fn register(&self, id: String, killer: Box<dyn ChildKiller + Send + Sync>) {
        self.inner.lock().unwrap().insert(id, killer);
    }

    fn remove(&self, id: &str) {
        self.inner.lock().unwrap().remove(id);
    }

    /// Kill the run with this id if it is still tracked. Returns whether a run
    /// was found to cancel.
    pub fn cancel(&self, id: &str) -> bool {
        if let Some(mut killer) = self.inner.lock().unwrap().remove(id) {
            let _ = killer.kill();
            true
        } else {
            false
        }
    }
}

/// Everything needed to start one execution. The caller (a Tauri command) has
/// already resolved the command string and cwd and run the safety check.
pub struct RunRequest {
    pub run_id: String,
    pub name: String,
    pub source: String,
    pub resolved_command: String,
    pub cwd: String,
}

#[derive(Clone, Serialize)]
struct OutputEvent {
    run_id: String,
    chunk: String,
}

#[derive(Clone, Serialize)]
struct ExitEvent {
    run_id: String,
    exit_code: Option<i64>,
    duration_ms: i64,
}

/// Spawn the command in a PTY and start streaming. Returns immediately; the
/// run continues on a background thread that emits output, then the exit event,
/// and records the run in history.
pub fn spawn(app: AppHandle, req: RunRequest) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 30,
            cols: 110,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("failed to open pty: {e}"))?;

    let mut builder = CommandBuilder::new(default_shell());
    builder.arg("-c");
    builder.arg(&req.resolved_command);
    builder.cwd(&req.cwd);

    let mut child = pair
        .slave
        .spawn_command(builder)
        .map_err(|e| format!("failed to spawn command: {e}"))?;

    let killer = child.clone_killer();
    app.state::<RunRegistry>()
        .register(req.run_id.clone(), killer);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("failed to read pty: {e}"))?;

    // Move the master into the thread to keep the pty alive; drop our copy of
    // the slave so EOF arrives once the child (the only remaining holder) exits.
    let master = pair.master;
    drop(pair.slave);

    let started = Instant::now();
    let started_at = now_unix_ms();

    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF: child closed the pty
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = app.emit(
                        EVENT_OUTPUT,
                        OutputEvent {
                            run_id: req.run_id.clone(),
                            chunk,
                        },
                    );
                }
                Err(_) => break,
            }
        }

        let exit_code = child.wait().ok().map(|status| status.exit_code() as i64);
        let duration_ms = started.elapsed().as_millis() as i64;

        // Hold the master until output draining is done.
        drop(master);

        app.state::<RunRegistry>().remove(&req.run_id);

        // Record the run. History is a log of past actions, never consulted to
        // decide what commands exist.
        let db = app.state::<Db>();
        let _ = db.insert_history(
            &req.name,
            &req.source,
            &req.resolved_command,
            &req.cwd,
            exit_code,
            duration_ms,
            started_at,
        );

        let _ = app.emit(
            EVENT_EXIT,
            ExitEvent {
                run_id: req.run_id,
                exit_code,
                duration_ms,
            },
        );
    });

    Ok(())
}

/// The shell used to interpret the resolved command. Honour the user's $SHELL
/// (their normal environment) and fall back to a POSIX shell.
fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

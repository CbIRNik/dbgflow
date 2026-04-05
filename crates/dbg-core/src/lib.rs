//! Core runtime for the `dbgflow` graph debugger.
//!
//! This crate contains the in-memory session model, event collector, session
//! persistence helpers, and the embedded local UI server.
//!
//! # Module Structure
//!
//! - [`session`]: Core data types for sessions, nodes, edges, and events
//! - [`runtime`]: Runtime state management and event recording
//! - [`server`]: Embedded HTTP server for the debugger UI
//! - [`ui`]: Embedded UI assets
//!
//! # Quick Start
//!
//! ```ignore
//! use dbgflow_core::{reset_session, current_session, serve_session};
//!
//! reset_session("My Debug Session");
//! // ... run instrumented code ...
//! serve_session(current_session(), "127.0.0.1", 3000)?;
//! ```
#![warn(missing_docs)]

mod internal_runtime;
mod server;
mod session;
mod ui;

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public re-exports from session module
// ---------------------------------------------------------------------------

pub use session::{
    Edge, EdgeKind, Event, EventKind, FunctionMeta, Node, NodeKind, Session, TypeMeta, ValueSlot,
};

// ---------------------------------------------------------------------------
// Public re-exports from internal_runtime module
// ---------------------------------------------------------------------------

pub use internal_runtime::{UiDebugValue, current_session, reset_session, runtime, type_preview};

// ---------------------------------------------------------------------------
// Public re-exports from server module
// ---------------------------------------------------------------------------

pub use server::{serve_session, serve_session_with_rerun};

// ---------------------------------------------------------------------------
// Session persistence (convenience wrappers)
// ---------------------------------------------------------------------------

/// Writes the current session to a JSON file.
pub fn write_session_json(path: impl AsRef<Path>) -> std::io::Result<()> {
    let sess = current_session();
    session::write_session_json(&sess, path)
}

/// Reads a session from a JSON file.
pub fn read_session_json(path: impl AsRef<Path>) -> std::io::Result<Session> {
    session::read_session_json(path)
}

/// Writes the current session into a directory using a sanitized file name.
pub fn write_session_snapshot_in_dir(
    dir: impl AsRef<Path>,
    label: impl AsRef<str>,
) -> std::io::Result<PathBuf> {
    let sess = current_session();
    session::write_session_snapshot_in_dir(&sess, dir, label)
}

/// Writes the current session into the directory pointed to by `DBG_SESSION_DIR`.
///
/// Returns `Ok(None)` if the environment variable is not set.
pub fn write_session_snapshot_from_env(label: impl AsRef<str>) -> std::io::Result<Option<PathBuf>> {
    let sess = current_session();
    session::write_session_snapshot_from_env(&sess, label)
}

// ---------------------------------------------------------------------------
// Convenience capture functions
// ---------------------------------------------------------------------------

/// Runs a closure inside a fresh in-memory capture session.
///
/// This resets the global session state, runs the provided closure, and returns
/// its result. Use [`current_session`] to retrieve the captured session afterward.
pub fn capture_session<T>(title: impl Into<String>, run: impl FnOnce() -> T) -> T {
    reset_session(title);
    run()
}

/// Runs a closure, then writes the captured session to a JSON file.
///
/// This is a convenience function that combines [`capture_session`] with
/// [`write_session_json`].
pub fn capture_session_to_path<T>(
    title: impl Into<String>,
    path: impl AsRef<Path>,
    run: impl FnOnce() -> T,
) -> std::io::Result<T> {
    reset_session(title);
    let result = run();
    write_session_json(path)?;
    Ok(result)
}

/// Runs a closure, then serves the captured session over the local UI server.
///
/// This is a convenience function that combines [`capture_session`] with
/// [`serve_session`].
pub fn capture_session_and_serve<T>(
    title: impl Into<String>,
    host: &str,
    port: u16,
    run: impl FnOnce() -> T,
) -> std::io::Result<T> {
    reset_session(title);
    let result = run();
    serve_session(current_session(), host, port)?;
    Ok(result)
}

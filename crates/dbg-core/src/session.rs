//! Session types and persistence.
//!
//! This module defines the core data model for debugging sessions, including
//! graph nodes, edges, and recorded execution events. It also provides helpers
//! for reading and writing session files.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use serde::{Deserialize, Serialize};

/// Graph node metadata persisted in a debugging session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    /// Stable node identifier within a session.
    pub id: String,
    /// Human-readable label shown in the UI.
    pub label: String,
    /// Semantic node type.
    pub kind: NodeKind,
    /// Rust module path where the node originated.
    pub module_path: String,
    /// Source file path captured for the node.
    pub file: String,
    /// Source line captured for the node.
    pub line: u32,
    /// Optional source code snippet shown by the UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Supported node types in the session graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A traced function node.
    Function,
    /// A `#[ui_debug]` data node.
    Type,
    /// A test node emitted by `#[dbg_test]` or test helpers.
    Test,
}

/// Directed edge between two graph nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    /// Parent or source node identifier.
    pub from: String,
    /// Child or target node identifier.
    pub to: String,
    /// Semantic edge type used by the UI.
    #[serde(default)]
    pub kind: EdgeKind,
    /// Optional short label rendered by the UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Supported edge types in the session graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Control flow between traced functions.
    #[default]
    ControlFlow,
    /// Data flow from a function into a data node snapshot.
    DataFlow,
    /// Link from a test node to the node it failed or passed on.
    TestLink,
}

/// Named value preview attached to an event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValueSlot {
    /// Logical field name or label.
    pub name: String,
    /// String preview rendered by the UI.
    pub preview: String,
}

/// Single recorded execution event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// Monotonic sequence number within the session.
    pub seq: u64,
    /// Call identifier when the event belongs to a traced function invocation.
    pub call_id: Option<u64>,
    /// Parent call identifier when nested under another traced invocation.
    pub parent_call_id: Option<u64>,
    /// Node identifier this event belongs to.
    pub node_id: String,
    /// Event category.
    pub kind: EventKind,
    /// Short title shown in the UI timeline.
    pub title: String,
    /// Attached value previews.
    pub values: Vec<ValueSlot>,
}

/// Supported event kinds emitted by the runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Function entry event.
    FunctionEnter,
    /// Function exit or unwind event.
    FunctionExit,
    /// Snapshot of a `#[ui_debug]` value.
    ValueSnapshot,
    /// Test start event.
    TestStarted,
    /// Test success event.
    TestPassed,
    /// Test failure event.
    TestFailed,
}

/// Complete replayable debugging session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Session title shown in the UI.
    pub title: String,
    /// All discovered graph nodes.
    pub nodes: Vec<Node>,
    /// All graph edges.
    pub edges: Vec<Edge>,
    /// Ordered execution events.
    pub events: Vec<Event>,
}

impl Session {
    /// Creates an empty session with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            events: Vec::new(),
        }
    }
}

/// Static metadata generated for traced functions.
#[derive(Clone, Copy, Debug)]
pub struct FunctionMeta {
    /// Stable function identifier.
    pub id: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Rust module path.
    pub module_path: &'static str,
    /// Source file.
    pub file: &'static str,
    /// Source line.
    pub line: u32,
    /// Original source snippet captured from the macro input.
    pub source: &'static str,
}

/// Static metadata generated for `#[ui_debug]` types.
#[derive(Clone, Copy, Debug)]
pub struct TypeMeta {
    /// Stable type identifier.
    pub id: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Rust module path.
    pub module_path: &'static str,
    /// Source file.
    pub file: &'static str,
    /// Source line.
    pub line: u32,
    /// Original source snippet captured from the macro input.
    pub source: &'static str,
}

// ---------------------------------------------------------------------------
// Session persistence
// ---------------------------------------------------------------------------

/// Writes a session to a JSON file.
pub fn write_session_json(session: &Session, path: impl AsRef<Path>) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(session).map_err(std::io::Error::other)?;

    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, json)
}

/// Reads a session from a JSON file.
pub fn read_session_json(path: impl AsRef<Path>) -> std::io::Result<Session> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(std::io::Error::other)
}

/// Sanitizes a label string into a safe filename component.
fn sanitize_filename(label: &str) -> String {
    let sanitized: String = label
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '-',
        })
        .collect();

    sanitized.trim_matches('-').to_owned()
}

/// Writes a session into a directory using a sanitized file name.
pub fn write_session_snapshot_in_dir(
    session: &Session,
    dir: impl AsRef<Path>,
    label: impl AsRef<str>,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(&dir)?;

    let file_name = format!(
        "{}-{}.json",
        sanitize_filename(label.as_ref()),
        process::id()
    );
    let path = dir.as_ref().join(file_name);
    write_session_json(session, &path)?;
    Ok(path)
}

/// Writes a session into the directory pointed to by `DBG_SESSION_DIR`.
///
/// Returns `Ok(None)` if the environment variable is not set.
pub fn write_session_snapshot_from_env(
    session: &Session,
    label: impl AsRef<str>,
) -> std::io::Result<Option<PathBuf>> {
    match env::var("DBG_SESSION_DIR") {
        Ok(dir) => write_session_snapshot_in_dir(session, dir, label).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(std::io::Error::other(error)),
    }
}

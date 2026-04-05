//! Public facade for the `dbgflow` graph debugger.
//!
//! Most users should depend on this crate instead of wiring `dbgflow-core` and
//! `dbgflow-macros` manually.
#![warn(missing_docs)]

extern crate self as dbgflow;

use std::fs;
use std::path::Path;

/// Re-exported runtime and session types from `dbgflow-core`.
pub use dbgflow_core::{
    Edge, EdgeKind, Event, EventKind, FunctionMeta, Node, NodeKind, Session, TypeMeta,
    UiDebugValue, ValueSlot, capture_session, capture_session_and_serve, capture_session_to_path,
    current_session, read_session_json, reset_session, runtime, serve_session,
    serve_session_with_rerun, write_session_json, write_session_snapshot_from_env,
    write_session_snapshot_in_dir,
};
/// Re-exported procedural macros from `dbgflow-macros`.
pub use dbgflow_macros::{dbg_test, trace, ui_debug};

/// Common imports for user code.
pub mod prelude {
    pub use crate::{
        UiDebugValue, capture, capture_and_serve, capture_to_file, dbg_test, trace, ui_debug,
    };
}

/// Starts a fresh in-memory debugging session with the provided title.
pub fn init_session(title: impl Into<String>) {
    reset_session(title);
}

/// Writes the current in-memory session to a JSON file.
pub fn save_current_session(path: impl AsRef<Path>) -> std::io::Result<()> {
    write_session_json(path)
}

/// Captures a specific block of code into a fresh in-memory session.
pub fn capture<T>(title: impl Into<String>, run: impl FnOnce() -> T) -> T {
    capture_session(title, run)
}

/// Captures a specific block of code and writes its session to a JSON file.
pub fn capture_to_file<T>(
    title: impl Into<String>,
    path: impl AsRef<Path>,
    run: impl FnOnce() -> T,
) -> std::io::Result<T> {
    capture_session_to_path(title, path, run)
}

/// Captures a specific block of code and immediately serves its session in the browser UI.
pub fn capture_and_serve<T>(
    title: impl Into<String>,
    host: &str,
    port: u16,
    run: impl FnOnce() -> T,
) -> std::io::Result<T> {
    capture_session_and_serve(title, host, port, run)
}

/// Serves the current in-memory session over the embedded local HTTP server.
pub fn serve_current_session(host: &str, port: u16) -> std::io::Result<()> {
    serve_session(current_session(), host, port)
}

/// Reads a saved session file, or aggregates all JSON sessions in a directory.
pub fn load_saved_session(path: impl AsRef<Path>) -> std::io::Result<Session> {
    let path = path.as_ref();
    if path.is_dir() {
        load_saved_session_dir(path)
    } else {
        read_session_json(path)
    }
}

fn load_saved_session_dir(dir: &Path) -> std::io::Result<Session> {
    let mut session_paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            session_paths.push(path);
        }
    }

    session_paths.sort();

    if session_paths.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no session JSON files found in {}", dir.display()),
        ));
    }

    if session_paths.len() == 1 {
        return read_session_json(&session_paths[0]);
    }

    let title = dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name} ({} sessions)", session_paths.len()))
        .unwrap_or_else(|| format!("dbgflow session bundle ({})", session_paths.len()));

    let mut merged = Session::new(title);
    let mut next_seq = 1_u64;
    let mut next_call_id = 1_u64;

    for (index, session_path) in session_paths.iter().enumerate() {
        let session = read_session_json(session_path)?;
        let prefix = format!(
            "bundle:{}:{}::",
            index + 1,
            session_file_slug(session_path, &session)
        );
        let seq_offset = next_seq.saturating_sub(1);
        let call_id_offset = next_call_id.saturating_sub(1);

        let max_call_id = session.events.iter().filter_map(|event| event.call_id).max();

        merged.nodes.extend(session.nodes.into_iter().map(|mut node| {
            node.id = format!("{prefix}{}", node.id);
            node
        }));

        merged.edges.extend(session.edges.into_iter().map(|mut edge| {
            edge.from = format!("{prefix}{}", edge.from);
            edge.to = format!("{prefix}{}", edge.to);
            edge
        }));

        merged.events.extend(session.events.into_iter().map(|mut event| {
            event.seq += seq_offset;
            event.call_id = event.call_id.map(|call_id| call_id + call_id_offset);
            event.parent_call_id = event
                .parent_call_id
                .map(|call_id| call_id + call_id_offset);
            event.node_id = format!("{prefix}{}", event.node_id);
            event
        }));

        next_seq = merged.events.last().map(|event| event.seq + 1).unwrap_or(next_seq);
        next_call_id = max_call_id.map(|call_id| call_id + call_id_offset + 1).unwrap_or(next_call_id);
    }

    Ok(merged)
}

fn session_file_slug(path: &Path, session: &Session) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(&session.title)
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_lowercase(),
            _ => '-',
        })
        .collect()
}

/// Persists the current session if the `DBG_SESSION_DIR` environment variable is set.
///
/// This is primarily used by `#[dbg_test]` and `dbgflow test`.
pub fn persist_session_from_env(
    label: impl AsRef<str>,
) -> std::io::Result<Option<std::path::PathBuf>> {
    write_session_snapshot_from_env(label)
}

/// Extracts a human-readable panic message from a panic payload.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic without string payload".to_owned()
    }
}

/// Demo pipeline and helpers used by the built-in CLI demo.
pub mod demo {
    use std::path::Path;

    use super::{
        UiDebugValue, current_session, load_saved_session, reset_session, runtime, serve_session,
        trace, ui_debug, write_session_json,
    };

    /// Small demo state object that appears as a data node in the UI.
    #[ui_debug(name = "Pipeline State")]
    pub struct PipelineState {
        /// Raw input sentence for the demo pipeline.
        pub input: String,
        /// Tokenized input.
        pub tokens: Vec<String>,
        /// Normalized tokens.
        pub normalized: Vec<String>,
        /// Final demo verdict.
        pub verdict: Option<String>,
        /// Mock counter for network request recursion
        pub network_retries: usize,
    }

    impl PipelineState {
        /// Creates the sample input used by the built-in demo.
        pub fn sample() -> Self {
            Self {
                input: "Trace UIDebug test network failure".to_owned(),
                tokens: Vec::new(),
                normalized: Vec::new(),
                verdict: None,
                network_retries: 0,
            }
        }

        /// Creates a second sample used to demonstrate switching between pipelines.
        pub fn review_sample() -> Self {
            Self {
                input: "Review snapshot playback stability".to_owned(),
                tokens: Vec::new(),
                normalized: Vec::new(),
                verdict: None,
                network_retries: 0,
            }
        }
    }

    /// Runs the complete demo pipeline.
    #[trace(name = "Run Pipeline")]
    pub fn run_pipeline(state: &mut PipelineState) {
        ingest(state);
        normalize(state);
        let status = fetch_data_recursively(state, 3);
        evaluate(state, status);
    }

    /// Runs the second demo pipeline used for chain switching.
    #[trace(name = "Run Review Pipeline")]
    pub fn run_review_pipeline(state: &mut PipelineState) {
        ingest(state);
        normalize(state);
        summarize(state);
    }

    /// Recursively attempts a mock network fetch.
    #[trace(name = "Network Fetch")]
    pub fn fetch_data_recursively(state: &mut PipelineState, attempts_left: usize) -> bool {
        state.network_retries += 1;
        state.emit_snapshot("sending request...");

        if attempts_left <= 1 {
            state.emit_snapshot("request succeeded");
            true
        } else {
            state.emit_snapshot("request failed, retrying");
            fetch_data_recursively(state, attempts_left - 1)
        }
    }

    /// Tokenizes the input string.
    #[trace(name = "Ingest Input")]
    pub fn ingest(state: &mut PipelineState) {
        state.tokens = state.input.split_whitespace().map(str::to_owned).collect();
        state.emit_snapshot("input tokenized");
    }

    /// Normalizes tokens for the demo pipeline.
    #[trace(name = "Normalize Tokens")]
    pub fn normalize(state: &mut PipelineState) {
        state.normalized = state
            .tokens
            .iter()
            .map(|token| token.to_lowercase())
            .collect();
        state.emit_snapshot("tokens normalized");
    }

    /// Computes a final verdict for the demo pipeline.
    #[trace(name = "Evaluate Verdict")]
    pub fn evaluate(state: &mut PipelineState, network_ok: bool) {
        let has_debug = state.normalized.iter().any(|token| token.contains("debug"));
        state.verdict = Some(if has_debug && network_ok {
            "interactive graph (online)".to_owned()
        } else {
            "raw trace only (offline)".to_owned()
        });
        state.emit_snapshot("verdict computed");
    }

    /// Produces a human-readable summary for the second demo pipeline.
    #[trace(name = "Summarize Playback")]
    pub fn summarize(state: &mut PipelineState) {
        state.verdict = Some(format!(
            "{} tokens ready for review",
            state.normalized.len()
        ));
        state.emit_snapshot("review summary prepared");
    }

    /// Adds a synthetic failing test event to the demo session.
    pub fn simulate_test_failure() {
        runtime::record_test_started(
            "pipeline::renders_failed_node",
            concat!(module_path!(), "::evaluate"),
        );
        runtime::record_test_failed(
            "pipeline::renders_failed_node",
            concat!(module_path!(), "::evaluate"),
            "assertion failed: expected verdict to mention failing node overlay",
        );
    }

    /// Adds a synthetic passing test event for the second demo pipeline.
    pub fn simulate_test_success() {
        runtime::record_test_started(
            "pipeline::renders_review_pipeline",
            concat!(module_path!(), "::summarize"),
        );
        runtime::record_test_passed(
            "pipeline::renders_review_pipeline",
            concat!(module_path!(), "::summarize"),
        );
    }

    /// Builds the in-memory demo session.
    pub fn build_session() {
        reset_session("dbgflow demo: graph debugger session");

        let mut state = PipelineState::sample();
        run_pipeline(&mut state);
        simulate_test_failure();

        let mut review_state = PipelineState::review_sample();
        run_review_pipeline(&mut review_state);
        simulate_test_success();
    }

    /// Runs the demo, persists it to disk, and optionally serves it.
    pub fn run(output: impl AsRef<Path>, serve: bool, port: u16) -> std::io::Result<()> {
        build_session();
        write_session_json(&output)?;
        println!("Session written to {}", output.as_ref().display());

        if serve {
            serve_session(current_session(), "127.0.0.1", port)?;
        }

        Ok(())
    }

    /// Serves a previously saved session JSON file.
    pub fn serve_saved(path: impl AsRef<Path>, port: u16) -> std::io::Result<()> {
        let session = load_saved_session(path)?;
        serve_session(session, "127.0.0.1", port)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{EventKind, current_session, demo, load_saved_session};

    #[test]
    fn demo_populates_graph_and_test_failure() {
        demo::build_session();
        let session = current_session();

        assert!(
            session
                .nodes
                .iter()
                .any(|node| node.id == "dbgflow::demo::run_pipeline")
        );
        assert!(session.edges.iter().any(|edge| {
            edge.from == "dbgflow::demo::run_pipeline" && edge.to == "dbgflow::demo::evaluate"
        }));
        assert!(
            session
                .nodes
                .iter()
                .any(|node| node.id == "dbgflow::demo::run_review_pipeline")
        );
        assert!(
            session
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::ValueSnapshot))
        );
        assert!(
            session
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::TestFailed))
        );
        assert!(
            session
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::TestPassed))
        );
    }

    #[test]
    fn load_saved_session_merges_directory_sessions_into_multiple_runs() {
        let temp_root = std::env::temp_dir().join(format!(
            "dbgflow-session-merge-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("temp dir should be created");

        let first_path = temp_root.join("alpha_session.json");
        let second_path = temp_root.join("beta_session.json");

        let first_json = r#"{
  "title": "Alpha Pipeline",
  "nodes": [
    {
      "id": "alpha::run",
      "label": "Run Alpha",
      "kind": "function",
      "module_path": "alpha",
      "file": "examples/alpha.rs",
      "line": 1,
      "source": "fn run_alpha() {}"
    }
  ],
  "edges": [],
  "events": [
    {
      "seq": 1,
      "call_id": 1,
      "parent_call_id": null,
      "node_id": "alpha::run",
      "kind": "function_enter",
      "title": "enter Run Alpha",
      "values": []
    },
    {
      "seq": 2,
      "call_id": 1,
      "parent_call_id": null,
      "node_id": "alpha::run",
      "kind": "function_exit",
      "title": "return run_alpha",
      "values": []
    }
  ]
}"#;
        let second_json = r#"{
  "title": "Beta Pipeline",
  "nodes": [
    {
      "id": "beta::run",
      "label": "Run Beta",
      "kind": "function",
      "module_path": "beta",
      "file": "examples/beta.rs",
      "line": 1,
      "source": "fn run_beta() {}"
    },
    {
      "id": "beta::test",
      "label": "renders beta",
      "kind": "test",
      "module_path": "beta",
      "file": "examples/beta.rs",
      "line": 2,
      "source": null
    }
  ],
  "edges": [
    {
      "from": "beta::test",
      "to": "beta::run",
      "kind": "test_link",
      "label": null
    }
  ],
  "events": [
    {
      "seq": 1,
      "call_id": 1,
      "parent_call_id": null,
      "node_id": "beta::run",
      "kind": "function_enter",
      "title": "enter Run Beta",
      "values": []
    },
    {
      "seq": 2,
      "call_id": null,
      "parent_call_id": null,
      "node_id": "beta::test",
      "kind": "test_started",
      "title": "test beta",
      "values": []
    },
    {
      "seq": 3,
      "call_id": 1,
      "parent_call_id": null,
      "node_id": "beta::run",
      "kind": "function_exit",
      "title": "return run_beta",
      "values": []
    }
  ]
}"#;

        std::fs::write(&first_path, first_json).expect("first session should be written");
        std::fs::write(&second_path, second_json).expect("second session should be written");

        let merged = load_saved_session(&temp_root).expect("directory sessions should merge");

        assert_eq!(merged.nodes.len(), 3);
        assert_eq!(merged.edges.len(), 1);
        assert_eq!(merged.events.len(), 5);
        assert_eq!(
            merged.events.iter().map(|event| event.seq).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            merged
                .events
                .iter()
                .filter_map(|event| event.call_id)
                .collect::<Vec<_>>(),
            vec![1, 1, 2, 2]
        );
        assert!(merged
            .nodes
            .iter()
            .all(|node| node.id.starts_with("bundle:")));

        std::fs::remove_dir_all(temp_root).expect("temp dir should be removed");
    }
}

//! Public facade for the `dbgflow` graph debugger.
//!
//! Most users should depend on this crate instead of wiring `dbgflow-core` and
//! `dbgflow-macros` manually.
#![warn(missing_docs)]

extern crate self as dbgflow;

use std::path::Path;

/// Re-exported runtime and session types from `dbgflow-core`.
pub use dbgflow_core::{
    Edge, Event, EventKind, FunctionMeta, Node, NodeKind, Session, TypeMeta, UiDebugValue,
    ValueSlot, current_session, read_session_json, reset_session, runtime, serve_session,
    write_session_json, write_session_snapshot_from_env, write_session_snapshot_in_dir,
};
/// Re-exported procedural macros from `dbgflow-macros`.
pub use dbgflow_macros::{dbg_test, trace, ui_debug};

/// Common imports for user code.
pub mod prelude {
    pub use crate::{UiDebugValue, dbg_test, trace, ui_debug};
}

/// Starts a fresh in-memory debugging session with the provided title.
pub fn init_session(title: impl Into<String>) {
    reset_session(title);
}

/// Writes the current in-memory session to a JSON file.
pub fn save_current_session(path: impl AsRef<Path>) -> std::io::Result<()> {
    write_session_json(path)
}

/// Serves the current in-memory session over the embedded local HTTP server.
pub fn serve_current_session(host: &str, port: u16) -> std::io::Result<()> {
    serve_session(current_session(), host, port)
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
        UiDebugValue, current_session, read_session_json, reset_session, runtime, serve_session,
        trace, ui_debug, write_session_json,
    };

    /// Small demo state object that appears as a data node in the UI.
    #[ui_debug]
    pub struct PipelineState {
        /// Raw input sentence for the demo pipeline.
        pub input: String,
        /// Tokenized input.
        pub tokens: Vec<String>,
        /// Normalized tokens.
        pub normalized: Vec<String>,
        /// Final demo verdict.
        pub verdict: Option<String>,
    }

    impl PipelineState {
        /// Creates the sample input used by the built-in demo.
        pub fn sample() -> Self {
            Self {
                input: "Trace UIDebug test failure".to_owned(),
                tokens: Vec::new(),
                normalized: Vec::new(),
                verdict: None,
            }
        }
    }

    /// Runs the complete demo pipeline.
    #[trace]
    pub fn run_pipeline(state: &mut PipelineState) {
        ingest(state);
        normalize(state);
        evaluate(state);
    }

    /// Tokenizes the input string.
    #[trace]
    pub fn ingest(state: &mut PipelineState) {
        state.tokens = state.input.split_whitespace().map(str::to_owned).collect();
        state.emit_snapshot("input tokenized");
    }

    /// Normalizes tokens for the demo pipeline.
    #[trace]
    pub fn normalize(state: &mut PipelineState) {
        state.normalized = state
            .tokens
            .iter()
            .map(|token| token.to_lowercase())
            .collect();
        state.emit_snapshot("tokens normalized");
    }

    /// Computes a final verdict for the demo pipeline.
    #[trace]
    pub fn evaluate(state: &mut PipelineState) {
        let has_debug = state.normalized.iter().any(|token| token.contains("debug"));
        state.verdict = Some(if has_debug {
            "interactive graph".to_owned()
        } else {
            "raw trace only".to_owned()
        });
        state.emit_snapshot("verdict computed");
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

    /// Builds the in-memory demo session.
    pub fn build_session() {
        reset_session("dbgflow demo: graph debugger session");

        let mut state = PipelineState::sample();
        state.emit_snapshot("initial state");
        run_pipeline(&mut state);
        simulate_test_failure();
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
        let session = read_session_json(path)?;
        serve_session(session, "127.0.0.1", port)
    }
}

#[cfg(test)]
mod tests {
    use super::{EventKind, current_session, demo};

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
    }
}

//! Core runtime for the `dbgflow` graph debugger.
//!
//! This crate contains the in-memory session model, event collector, session
//! persistence helpers, and the embedded local UI server.
#![warn(missing_docs)]

use std::any::type_name_of_val;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Debug;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

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
}

#[derive(Clone, Debug)]
struct CallFrame {
    call_id: u64,
    node_id: String,
}

#[derive(Default)]
struct RuntimeState {
    title: String,
    nodes: BTreeMap<String, Node>,
    edges: BTreeSet<(String, String)>,
    events: Vec<Event>,
    last_event_node_id: Option<String>,
    next_seq: AtomicU64,
    next_call_id: AtomicU64,
}

thread_local! {
    static CALL_STACK: RefCell<Vec<CallFrame>> = const { RefCell::new(Vec::new()) };
}

static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

fn state() -> &'static Mutex<RuntimeState> {
    STATE.get_or_init(|| Mutex::new(RuntimeState::default()))
}

fn lock_state() -> MutexGuard<'static, RuntimeState> {
    state().lock().expect("dbgflow-core runtime mutex poisoned")
}

fn next_seq(state: &RuntimeState) -> u64 {
    state.next_seq.fetch_add(1, Ordering::Relaxed) + 1
}

fn next_call_id(state: &RuntimeState) -> u64 {
    state.next_call_id.fetch_add(1, Ordering::Relaxed) + 1
}

fn push_event(state: &mut RuntimeState, mut event: Event) {
    event.seq = next_seq(state);
    state.last_event_node_id = Some(event.node_id.clone());
    state.events.push(event);
}

fn ensure_node(state: &mut RuntimeState, node: Node) {
    state.nodes.entry(node.id.clone()).or_insert(node);
}

fn ensure_edge(state: &mut RuntimeState, from: &str, to: &str) {
    if from == to {
        return;
    }

    state.edges.insert((from.to_owned(), to.to_owned()));
}

/// Clears the runtime and starts a new in-memory session.
pub fn reset_session(title: impl Into<String>) {
    let mut state = lock_state();
    state.title = title.into();
    state.nodes.clear();
    state.edges.clear();
    state.events.clear();
    state.last_event_node_id = None;
    state.next_seq.store(0, Ordering::Relaxed);
    state.next_call_id.store(0, Ordering::Relaxed);
    CALL_STACK.with(|stack| stack.borrow_mut().clear());
}

/// Returns a snapshot of the current in-memory session.
pub fn current_session() -> Session {
    let state = lock_state();
    let nodes = state.nodes.values().cloned().collect();
    let edges = state
        .edges
        .iter()
        .map(|(from, to)| Edge {
            from: from.clone(),
            to: to.clone(),
        })
        .collect();

    Session {
        title: state.title.clone(),
        nodes,
        edges,
        events: state.events.clone(),
    }
}

/// Writes the current session to a JSON file.
pub fn write_session_json(path: impl AsRef<Path>) -> std::io::Result<()> {
    let session = current_session();
    let json = serde_json::to_string_pretty(&session).map_err(std::io::Error::other)?;

    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, json)
}

/// Reads a session from a JSON file.
pub fn read_session_json(path: impl AsRef<Path>) -> std::io::Result<Session> {
    let content = fs::read_to_string(path)?;
    let session = serde_json::from_str(&content).map_err(std::io::Error::other)?;
    Ok(session)
}

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

/// Writes the current session into a directory using a sanitized file name.
pub fn write_session_snapshot_in_dir(
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
    write_session_json(&path)?;
    Ok(path)
}

/// Writes the current session into the directory pointed to by `DBG_SESSION_DIR`.
pub fn write_session_snapshot_from_env(label: impl AsRef<str>) -> std::io::Result<Option<PathBuf>> {
    match env::var("DBG_SESSION_DIR") {
        Ok(dir) => write_session_snapshot_in_dir(dir, label).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(std::io::Error::other(error)),
    }
}

/// Produces a cheap type-oriented preview for values captured by trace arguments.
pub fn type_preview<T>(value: &T) -> String {
    format!("type {}", type_name_of_val(value))
}

/// Trait implemented for values that should appear as UI data nodes.
pub trait UiDebugValue: Debug + Sized {
    /// Returns the static metadata for the type.
    fn ui_debug_type_meta() -> TypeMeta;

    /// Returns the text snapshot shown in the UI.
    fn ui_debug_snapshot(&self) -> String {
        format!("{self:#?}")
    }

    /// Emits a value snapshot event for the current session.
    fn emit_snapshot(&self, label: impl Into<String>) {
        runtime::record_type_snapshot(self, label.into());
    }
}

/// Runtime helpers used by generated macros and advanced callers.
pub mod runtime {
    use super::{
        CALL_STACK, CallFrame, Event, EventKind, FunctionMeta, Node, NodeKind, UiDebugValue,
        ValueSlot, ensure_edge, ensure_node, lock_state, next_call_id, push_event,
    };

    /// Guard object representing an active traced function invocation.
    #[derive(Debug)]
    pub struct TraceFrame {
        call_id: u64,
        parent_call_id: Option<u64>,
        node_id: &'static str,
        finished: bool,
    }

    impl TraceFrame {
        /// Enters a traced function and records the corresponding event.
        pub fn enter(meta: FunctionMeta, values: Vec<ValueSlot>) -> Self {
            let (call_id, parent_call_id) = {
                let mut state = lock_state();

                ensure_node(
                    &mut state,
                    Node {
                        id: meta.id.to_owned(),
                        label: meta.label.to_owned(),
                        kind: NodeKind::Function,
                        module_path: meta.module_path.to_owned(),
                        file: meta.file.to_owned(),
                        line: meta.line,
                    },
                );

                let call_id = next_call_id(&state);
                let parent_call_id = CALL_STACK.with(|stack| {
                    let stack = stack.borrow();
                    stack.last().map(|frame| frame.call_id)
                });

                let parent_node = CALL_STACK.with(|stack| {
                    let stack = stack.borrow();
                    stack.last().map(|frame| frame.node_id.clone())
                });

                if let Some(parent_node) = parent_node {
                    ensure_edge(&mut state, &parent_node, meta.id);
                }

                push_event(
                    &mut state,
                    Event {
                        seq: 0,
                        call_id: Some(call_id),
                        parent_call_id,
                        node_id: meta.id.to_owned(),
                        kind: EventKind::FunctionEnter,
                        title: format!("enter {}", meta.label),
                        values,
                    },
                );

                (call_id, parent_call_id)
            };

            CALL_STACK.with(|stack| {
                stack.borrow_mut().push(CallFrame {
                    call_id,
                    node_id: meta.id.to_owned(),
                });
            });

            Self {
                call_id,
                parent_call_id,
                node_id: meta.id,
                finished: false,
            }
        }

        /// Records a successful function return.
        pub fn finish_return<T>(&mut self, result: &T) {
            if self.finished {
                return;
            }

            {
                let mut state = lock_state();
                push_event(
                    &mut state,
                    Event {
                        seq: 0,
                        call_id: Some(self.call_id),
                        parent_call_id: self.parent_call_id,
                        node_id: self.node_id.to_owned(),
                        kind: EventKind::FunctionExit,
                        title: format!(
                            "return {}",
                            self.node_id.rsplit("::").next().unwrap_or(self.node_id)
                        ),
                        values: vec![ValueSlot {
                            name: "result".to_owned(),
                            preview: super::type_preview(result),
                        }],
                    },
                );
            }

            self.finished = true;
            pop_stack(self.call_id);
        }

        /// Records an error or unwind outcome for the current frame.
        pub fn finish_error(&mut self, message: impl Into<String>) {
            if self.finished {
                return;
            }

            {
                let mut state = lock_state();
                push_event(
                    &mut state,
                    Event {
                        seq: 0,
                        call_id: Some(self.call_id),
                        parent_call_id: self.parent_call_id,
                        node_id: self.node_id.to_owned(),
                        kind: EventKind::FunctionExit,
                        title: format!(
                            "panic {}",
                            self.node_id.rsplit("::").next().unwrap_or(self.node_id)
                        ),
                        values: vec![ValueSlot {
                            name: "status".to_owned(),
                            preview: message.into(),
                        }],
                    },
                );
            }

            self.finished = true;
            pop_stack(self.call_id);
        }
    }

    impl Drop for TraceFrame {
        fn drop(&mut self) {
            if self.finished {
                return;
            }

            self.finish_error("unwound before explicit return");
        }
    }

    fn pop_stack(call_id: u64) {
        CALL_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            if stack.last().map(|frame| frame.call_id) == Some(call_id) {
                stack.pop();
            }
        });
    }

    /// Builds a value preview for a traced function argument.
    pub fn preview_argument<T>(name: impl Into<String>, value: &T) -> ValueSlot {
        ValueSlot {
            name: name.into(),
            preview: super::type_preview(value),
        }
    }

    /// Records a snapshot for a `#[ui_debug]` value.
    pub fn record_type_snapshot<T: UiDebugValue>(value: &T, label: impl Into<String>) {
        let meta = T::ui_debug_type_meta();
        let mut state = lock_state();

        ensure_node(
            &mut state,
            Node {
                id: meta.id.to_owned(),
                label: meta.label.to_owned(),
                kind: NodeKind::Type,
                module_path: meta.module_path.to_owned(),
                file: meta.file.to_owned(),
                line: meta.line,
            },
        );

        if let Some(parent_node) = CALL_STACK.with(|stack| {
            let stack = stack.borrow();
            stack.last().map(|frame| frame.node_id.clone())
        }) {
            ensure_edge(&mut state, &parent_node, meta.id);
        }

        push_event(
            &mut state,
            Event {
                seq: 0,
                call_id: CALL_STACK.with(|stack| {
                    let stack = stack.borrow();
                    stack.last().map(|frame| frame.call_id)
                }),
                parent_call_id: None,
                node_id: meta.id.to_owned(),
                kind: EventKind::ValueSnapshot,
                title: label.into(),
                values: vec![ValueSlot {
                    name: meta.label.to_owned(),
                    preview: value.ui_debug_snapshot(),
                }],
            },
        );
    }

    /// Records that a test started and explicitly links it to a node.
    pub fn record_test_started(test_name: impl Into<String>, node_id: impl Into<String>) {
        record_test_event(
            EventKind::TestStarted,
            test_name.into(),
            node_id.into(),
            Vec::new(),
        );
    }

    /// Records that a test passed and explicitly links it to a node.
    pub fn record_test_passed(test_name: impl Into<String>, node_id: impl Into<String>) {
        record_test_event(
            EventKind::TestPassed,
            test_name.into(),
            node_id.into(),
            Vec::new(),
        );
    }

    /// Records that a test failed and explicitly links it to a node.
    pub fn record_test_failed(
        test_name: impl Into<String>,
        node_id: impl Into<String>,
        failure: impl Into<String>,
    ) {
        record_test_event(
            EventKind::TestFailed,
            test_name.into(),
            node_id.into(),
            vec![ValueSlot {
                name: "failure".to_owned(),
                preview: failure.into(),
            }],
        );
    }

    /// Returns the node id attached to the most recently recorded event.
    pub fn latest_node_id() -> Option<String> {
        let state = lock_state();
        state.last_event_node_id.clone()
    }

    /// Records that a test started and links it to the latest active node.
    pub fn record_test_started_latest(test_name: impl Into<String>) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        record_test_event(EventKind::TestStarted, test_name, node_id, Vec::new());
    }

    /// Records that a test passed and links it to the latest active node.
    pub fn record_test_passed_latest(test_name: impl Into<String>) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        record_test_event(EventKind::TestPassed, test_name, node_id, Vec::new());
    }

    /// Records that a test failed and links it to the latest active node.
    pub fn record_test_failed_latest(test_name: impl Into<String>, failure: impl Into<String>) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        record_test_event(
            EventKind::TestFailed,
            test_name,
            node_id,
            vec![ValueSlot {
                name: "failure".to_owned(),
                preview: failure.into(),
            }],
        );
    }

    fn record_test_event(
        kind: EventKind,
        test_name: String,
        node_id: String,
        values: Vec<ValueSlot>,
    ) {
        let test_id = format!("test::{test_name}");
        let mut state = lock_state();

        ensure_node(
            &mut state,
            Node {
                id: test_id.clone(),
                label: test_name.clone(),
                kind: NodeKind::Test,
                module_path: "cargo::test".to_owned(),
                file: "<runner>".to_owned(),
                line: 0,
            },
        );
        ensure_edge(&mut state, &test_id, &node_id);

        push_event(
            &mut state,
            Event {
                seq: 0,
                call_id: None,
                parent_call_id: None,
                node_id: test_id,
                kind,
                title: test_name,
                values,
            },
        );
    }
}

fn content_type(path: &str) -> &'static str {
    match path {
        "/" => "text/html; charset=utf-8",
        "/app.js" => "application/javascript; charset=utf-8",
        "/app.css" => "text/css; charset=utf-8",
        "/session.json" => "application/json; charset=utf-8",
        _ => "text/plain; charset=utf-8",
    }
}

fn write_response(
    stream: &mut TcpStream,
    method: &str,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = if method == "HEAD" {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
    } else {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    };
    stream.write_all(response.as_bytes())
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<(String, String)> {
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let line = request.lines().next().unwrap_or_default();
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    Ok((method.to_owned(), path.to_owned()))
}

/// Serves a session over the embedded local HTTP server.
pub fn serve_session(session: Session, host: &str, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind((host, port))?;
    let json = serde_json::to_string(&session).map_err(std::io::Error::other)?;
    let html = ui::index_html();
    let app_js = ui::app_js();
    let app_css = ui::app_css();

    println!("Debugger UI: http://{host}:{port}");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        let (method, path) = match read_request(&mut stream) {
            Ok(request) => request,
            Err(_) => continue,
        };

        let result = match path.as_str() {
            "/" => write_response(&mut stream, &method, "200 OK", content_type("/"), &html),
            "/app.js" => write_response(
                &mut stream,
                &method,
                "200 OK",
                content_type("/app.js"),
                &app_js,
            ),
            "/app.css" => write_response(
                &mut stream,
                &method,
                "200 OK",
                content_type("/app.css"),
                &app_css,
            ),
            "/session.json" => write_response(
                &mut stream,
                &method,
                "200 OK",
                content_type("/session.json"),
                &json,
            ),
            _ => write_response(
                &mut stream,
                &method,
                "404 Not Found",
                content_type(""),
                "not found",
            ),
        };

        if let Err(error) = result {
            if !matches!(
                error.kind(),
                std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
            ) {
                return Err(error);
            }
        }
    }

    Ok(())
}

mod ui {
    pub fn index_html() -> String {
        include_str!("../ui/index.html").to_owned()
    }

    pub fn app_js() -> String {
        include_str!("../ui/app.js").to_owned()
    }

    pub fn app_css() -> String {
        include_str!("../ui/app.css").to_owned()
    }
}

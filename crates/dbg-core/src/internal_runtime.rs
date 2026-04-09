//! Runtime state and event recording.
//!
//! This module manages the global runtime state for the debugger, including
//! the call stack, event recording, and node/edge management. It provides
//! the low-level API used by the trace macros.

use std::any::type_name_of_val;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::session::{
    Edge, EdgeKind, Event, EventKind, FunctionMeta, Node, NodeKind, Session, TypeMeta, ValueSlot,
};

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Stack frame representing an active traced function invocation.
#[derive(Clone, Debug)]
struct CallFrame {
    call_id: u64,
    node_id: String,
}

/// Global mutable runtime state.
#[derive(Default)]
struct RuntimeState {
    title: String,
    nodes: BTreeMap<String, Node>,
    edges: BTreeMap<(String, String), Edge>,
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
    ensure_edge_kind(state, from, to, EdgeKind::ControlFlow, None);
}

fn ensure_edge_kind(
    state: &mut RuntimeState,
    from: &str,
    to: &str,
    kind: EdgeKind,
    label: Option<String>,
) {
    if from == to {
        return;
    }

    state.edges.insert(
        (from.to_owned(), to.to_owned()),
        Edge {
            from: from.to_owned(),
            to: to.to_owned(),
            kind,
            label,
        },
    );
}

// ---------------------------------------------------------------------------
// Public session management
// ---------------------------------------------------------------------------

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
    let edges = state.edges.values().cloned().collect();

    Session {
        title: state.title.clone(),
        nodes,
        edges,
        events: state.events.clone(),
    }
}

/// Produces a cheap type-oriented preview for values captured by trace arguments.
pub fn type_preview<T>(value: &T) -> String {
    format!("type {}", type_name_of_val(value))
}

// ---------------------------------------------------------------------------
// UiDebugValue trait
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Runtime helpers (public module)
// ---------------------------------------------------------------------------

/// Runtime helpers used by generated macros and advanced callers.
pub mod runtime {
    use super::{
        CALL_STACK, CallFrame, EdgeKind, Event, EventKind, FunctionMeta, Node, NodeKind,
        UiDebugValue, ValueSlot, ensure_edge, ensure_edge_kind, ensure_node, lock_state,
        next_call_id, push_event,
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
                function_id: None,
                call_id: None,
                        label: meta.label.to_owned(),
                        kind: NodeKind::Function,
                        module_path: meta.module_path.to_owned(),
                        file: meta.file.to_owned(),
                        line: meta.line,
                        source: Some(meta.source.to_owned()),
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
        let label = label.into();
        let mut state = lock_state();

        ensure_node(
            &mut state,
            Node {
                id: meta.id.to_owned(),
                function_id: None,
                call_id: None,
                label: meta.label.to_owned(),
                kind: NodeKind::Type,
                module_path: meta.module_path.to_owned(),
                file: meta.file.to_owned(),
                line: meta.line,
                source: Some(meta.source.to_owned()),
            },
        );

        if let Some(parent_node) = CALL_STACK.with(|stack| {
            let stack = stack.borrow();
            stack.last().map(|frame| frame.node_id.clone())
        }) {
            ensure_edge_kind(
                &mut state,
                &parent_node,
                meta.id,
                EdgeKind::DataFlow,
                Some(label.clone()),
            );
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
                title: label,
                values: vec![ValueSlot {
                    name: meta.label.to_owned(),
                    preview: value.ui_debug_snapshot(),
                }],
            },
        );
    }

    /// Records that a test started and explicitly links it to a node.
    pub fn record_test_started(test_name: impl Into<String>, node_id: impl Into<String>) {
        let test_name = test_name.into();
        record_test_event(
            EventKind::TestStarted,
            test_name.clone(),
            short_label(&test_name),
            node_id.into(),
            Vec::new(),
        );
    }

    /// Records that a test passed and explicitly links it to a node.
    pub fn record_test_passed(test_name: impl Into<String>, node_id: impl Into<String>) {
        let test_name = test_name.into();
        record_test_event(
            EventKind::TestPassed,
            test_name.clone(),
            short_label(&test_name),
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
        let test_name = test_name.into();
        record_test_event(
            EventKind::TestFailed,
            test_name.clone(),
            short_label(&test_name),
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
        let label = short_label(&test_name);
        record_test_event(
            EventKind::TestStarted,
            test_name,
            label,
            node_id,
            Vec::new(),
        );
    }

    /// Records that a test passed and links it to the latest active node.
    pub fn record_test_passed_latest(test_name: impl Into<String>) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        let label = short_label(&test_name);
        record_test_event(EventKind::TestPassed, test_name, label, node_id, Vec::new());
    }

    /// Records that a test failed and links it to the latest active node.
    pub fn record_test_failed_latest(test_name: impl Into<String>, failure: impl Into<String>) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        let label = short_label(&test_name);
        record_test_event(
            EventKind::TestFailed,
            test_name,
            label,
            node_id,
            vec![ValueSlot {
                name: "failure".to_owned(),
                preview: failure.into(),
            }],
        );
    }

    /// Records that a test started and links it to the latest active node using a custom label.
    pub fn record_test_started_latest_with_label(
        test_name: impl Into<String>,
        label: impl Into<String>,
    ) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        record_test_event(
            EventKind::TestStarted,
            test_name,
            label.into(),
            node_id,
            Vec::new(),
        );
    }

    /// Records that a test passed and links it to the latest active node using a custom label.
    pub fn record_test_passed_latest_with_label(
        test_name: impl Into<String>,
        label: impl Into<String>,
    ) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        record_test_event(
            EventKind::TestPassed,
            test_name,
            label.into(),
            node_id,
            Vec::new(),
        );
    }

    /// Records that a test failed and links it to the latest active node using a custom label.
    pub fn record_test_failed_latest_with_label(
        test_name: impl Into<String>,
        label: impl Into<String>,
        failure: impl Into<String>,
    ) {
        let test_name = test_name.into();
        let node_id = latest_node_id().unwrap_or_else(|| format!("test::{test_name}"));
        record_test_event(
            EventKind::TestFailed,
            test_name,
            label.into(),
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
        label: String,
        node_id: String,
        values: Vec<ValueSlot>,
    ) {
        let test_id = format!("test::{test_name}");
        let mut state = lock_state();

        ensure_node(
            &mut state,
            Node {
                id: test_id.clone(),
                function_id: None,
                call_id: None,
                label,
                kind: NodeKind::Test,
                module_path: "cargo::test".to_owned(),
                file: "<runner>".to_owned(),
                line: 0,
                source: None,
            },
        );
        ensure_edge_kind(&mut state, &test_id, &node_id, EdgeKind::TestLink, None);

        push_event(
            &mut state,
            Event {
                seq: 0,
                call_id: None,
                parent_call_id: None,
                node_id: test_id,
                kind,
                title: short_label(&test_name),
                values,
            },
        );
    }

    /// Extracts the last segment of a module path for use as a short label.
    fn short_label(path: &str) -> String {
        path.rsplit("::").next().unwrap_or(path).to_owned()
    }

    // ---------------------------------------------------------------------------
    // Advanced async support
    // ---------------------------------------------------------------------------

    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    pin_project_lite::pin_project! {
        /// A wrapper that traces an asynchronous Future's execution.
        pub struct InstrumentedFuture<F> {
            #[pin]
            inner: F,
            meta: FunctionMeta,
            values: Option<Vec<ValueSlot>>,
            call_id: Option<u64>,
            parent_call_id: Option<u64>,
            node_id: &'static str,
            finished: bool,
        }
    }

    /// Wraps a future to record its execution spanning multiple await points.
    pub fn trace_future<F: Future>(
        meta: FunctionMeta,
        values: Vec<ValueSlot>,
        inner: F,
    ) -> InstrumentedFuture<F> {
        let parent_call_id = CALL_STACK.with(|stack| {
            stack.borrow().last().map(|frame| frame.call_id)
        });

        InstrumentedFuture {
            inner,
            meta,
            values: Some(values),
            call_id: None,
            parent_call_id,
            node_id: meta.id,
            finished: false,
        }
    }

    impl<F: Future> Future for InstrumentedFuture<F> {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();

            if this.call_id.is_none() {
                let mut state = lock_state();

                ensure_node(
                    &mut state,
                    Node {
                        id: this.meta.id.to_owned(),
                        function_id: None,
                        call_id: None,
                        label: this.meta.label.to_owned(),
                        kind: NodeKind::Function,
                        module_path: this.meta.module_path.to_owned(),
                        file: this.meta.file.to_owned(),
                        line: this.meta.line,
                        source: Some(this.meta.source.to_owned()),
                    },
                );

                let new_call_id = next_call_id(&state);
                *this.call_id = Some(new_call_id);

                let parent_node = CALL_STACK.with(|stack| {
                    let stack = stack.borrow();
                    stack.last().map(|frame| frame.node_id.clone())
                });

                if let Some(parent_node) = parent_node {
                    ensure_edge(&mut state, &parent_node, this.meta.id);
                }

                push_event(
                    &mut state,
                    Event {
                        seq: 0,
                        call_id: Some(new_call_id),
                        parent_call_id: *this.parent_call_id,
                        node_id: this.meta.id.to_owned(),
                        kind: EventKind::FunctionEnter,
                        title: format!("enter {}", this.meta.label),
                        values: this.values.take().unwrap_or_default(),
                    },
                );
            }

            let call_id = this.call_id.unwrap();

            CALL_STACK.with(|stack| {
                stack.borrow_mut().push(CallFrame {
                    call_id,
                    node_id: this.node_id.to_owned(),
                });
            });

            let res = this.inner.poll(cx);

            CALL_STACK.with(|stack| {
                stack.borrow_mut().pop();
            });

            if res.is_ready() && !*this.finished {
                *this.finished = true;
                let mut state = lock_state();
                push_event(
                    &mut state,
                    Event {
                        seq: 0,
                        call_id: Some(call_id),
                        parent_call_id: *this.parent_call_id,
                        node_id: this.node_id.to_owned(),
                        kind: EventKind::FunctionExit,
                        title: format!("return {}", this.node_id.rsplit("::").next().unwrap_or(this.node_id)),
                        values: vec![ValueSlot {
                            name: "result".to_owned(),
                            preview: match &res {
                                std::task::Poll::Ready(val) => super::type_preview(val),
                                _ => unreachable!(),
                            },
                        }],
                    },
                );
            }

            res
        }
    }
}

import re
with open('crates/dbg-core/src/internal_runtime.rs', 'r') as f:
    content = f.read()

# Fix Event missing parent_call_id in record_type_snapshot
content = re.sub(
    r'(Event\s*\{\n\s*seq:\s*0,\n\s*call_id:\s*CALL_STACK.with\([^)]+\),\n)',
    r'\1                parent_call_id: None,\n',
    content
)

# Append InstrumentedFuture perfectly.
future_code = """
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
"""

content = content.replace("}\n", "}\n" + future_code)
# wait, replacing "}\n" replaces ALL `}\n`. We just want to append before the last `}`.
with open('crates/dbg-core/src/internal_runtime.rs', 'r') as f:
    text = f.read()

# We replace `fn short_label(path: &str) -> String {\n        path.rsplit("::").next().unwrap_or(path).to_owned()\n    }\n}`
target = 'fn short_label(path: &str) -> String {\n        path.rsplit("::").next().unwrap_or(path).to_owned()\n    }\n'
if target in text:
    text = text.replace(target, target + future_code)
    with open('crates/dbg-core/src/internal_runtime.rs', 'w') as f:
        f.write(text)
else:
    print("could not find target")

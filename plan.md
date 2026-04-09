# Async support implementation plan

## Phase 1: Foundation (Core Tracing)
- [ ] Replace thread-local `CALL_STACK` with a task-local or concurrent-friendly construct (e.g. `tokio::task_local!`).
- [ ] Modify `internal_runtime.rs` to support `AsyncTraceFrame` and associate async scopes with a task or span ID.
- [ ] Incorporate `tracing` crate dependencies optionally (if going the `tracing` route).
- [ ] Update `Event` struct to include `task_id` or similar metadata to differentiate parallel execution tracks.

## Phase 2: Macros & Syntax
- [ ] Update `#[dbg]` / `#[trace]` macro in `dbg-macros/src/lib.rs` to optionally handle `async fn` and wrap the asynchronous body.
- [ ] Add `spawn` tracking (e.g. tracking `tokio::spawn` calls to link parent and child tasks).

## Phase 3: Web Visualization
- [ ] Modify `web/src/utils/graphUtils.js` and React components to render parallel tasks correctly (branching graphs for concurrent tracks).
- [ ] Ensure events with the same `parent_call_id` but different `task_id` are laid out parallelly.

## Phase 4: Integrations (Tokio Console)
- [ ] Add Tokio console subscriber compatibility layer to output DBG events into `tokio-console` if needed, or vice-versa.

# Architecture

## Target shape

The product is split into four layers:

1. Instrumentation layer:
   `#[trace]` wraps function bodies and emits execution events.
   `#[ui_debug]` marks data types that should appear as nodes and produce snapshots.
2. Runtime layer:
   A lightweight collector keeps the current session in memory, tracks the call stack, emits nodes, edges, value snapshots, and test events.
3. Transport layer:
   Sessions are serialized to JSON so they can be streamed, persisted, or replayed later.
4. Presentation layer:
   A browser UI renders the graph, execution timeline, and test-to-node failures.
   The current prototype uses React Flow for node graph rendering and `dagre` for directed auto-layout.

## Why this decomposition

- Proc macros give an ergonomic API inside user code.
- The `dbg` package exposes both a CLI binary and a library facade, so consumers can depend on one crate instead of wiring `dbg-core` and `dbg-macros` manually.
- The runtime stays library-sized and can later be embedded in `cargo test`, integration tests, binaries, or a language server.
- A JSON session format keeps the UI decoupled from execution. This makes live mode and replay mode the same product surface.

## Event model

The current runtime emits:

- function enter
- function exit
- value snapshot
- test started
- test passed
- test failed

That is intentionally enough to prove the UI contract before adding stepping and breakpoints.

## Practical roadmap

### Phase 1

- Done: workspace split into `dbgflow-core`, `dbgflow-macros`, and `dbgflow`
- Done: browser UI for graph and timeline
- Done: manual test failure linkage to nodes

### Phase 2

- Add `dbg test` wrapper around `cargo test --message-format json`
- Correlate failing test frames with traced call stack
- Stream events over websocket instead of only after-the-fact JSON dump

### Phase 3

- Fine-grained mutation events for fields and collections
- Breakpoints and pause/step controls
- Deterministic session replay and diffing between successful and failing test runs

## Constraints to keep in mind

- `#[trace]` must stay cheap enough for local development builds.
- Value capture should degrade gracefully when a type is not serializable.
- Async functions and multithreaded tests will require per-task or per-thread execution contexts, not only a single local stack.

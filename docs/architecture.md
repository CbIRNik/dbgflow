# Architecture

## System Overview

dbgflow captures program execution as a graph and renders it in an interactive browser UI.

```
┌─────────────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
│  Instrumentation    │────▶│      Runtime        │────▶│    Presentation     │
│  (#[trace], etc.)   │     │  (Session Model)    │     │    (Browser UI)     │
└─────────────────────┘     └─────────────────────┘     └─────────────────────┘
         │                           │                           │
    proc macros              in-memory collector         React + React Flow
    code generation          JSON serialization          dagre layout
```

## Layer Details

### 1. Instrumentation Layer

Procedural macros transform user code to emit events:

- **`#[trace]`** — Wraps function bodies with enter/exit events
- **`#[ui_debug]`** — Derives snapshot capability for types
- **`#[dbg_test]`** — Wraps tests with session capture and failure linking

### 2. Runtime Layer

The `dbgflow-core` crate provides:

- **Session Model** — Nodes, edges, and events as serializable structs
- **Collector** — Thread-local state tracking current call stack
- **Snapshot API** — `emit_snapshot()` for capturing data state

### 3. Transport Layer

Sessions serialize to JSON for:

- File persistence (`dbgflow test`, `capture_to_file`)
- HTTP response (`dbgflow serve`, embedded server)
- Session merging (multiple tests into one UI)

### 4. Presentation Layer

The browser UI provides:

- **Graph Canvas** — React Flow with custom node components
- **Auto-Layout** — Dagre for directed graph positioning
- **Playback Controls** — Timeline scrubbing and animated playback
- **Details Panel** — Node inspection with syntax-highlighted source

## Package Structure

```
dbgflow (crates/dbg-cli)
├── re-exports dbgflow-core types
├── re-exports dbgflow-macros macros
├── CLI binary (demo, serve, test commands)
└── session loading and merging

dbgflow-core (crates/dbg-core)
├── Session, Node, Edge, Event types
├── Runtime state and event recording
├── Embedded HTTP server
└── Built UI assets (app.js, app.css)

dbgflow-macros (crates/dbg-macros)
├── #[trace] — function instrumentation
├── #[ui_debug] — type snapshot derivation
└── #[dbg_test] — test wrapper
```

## Data Flow

1. User code calls a `#[trace]` function
2. Macro-generated code records `FunctionEnter` event
3. Function body executes, possibly calling `emit_snapshot()`
4. Macro-generated code records `FunctionExit` event
5. Session accumulates in thread-local storage
6. Test completion or explicit save writes JSON
7. CLI serves JSON via embedded HTTP server
8. Browser fetches session and renders graph

## Event Model

Events form an ordered sequence representing execution:

| Event | Description |
|-------|-------------|
| `function_enter` | Call started, captures arguments |
| `function_exit` | Call returned, captures return value |
| `value_snapshot` | Explicit data capture via `emit_snapshot()` |
| `test_started` | Test began execution |
| `test_passed` | Test completed successfully |
| `test_failed` | Test assertion failed |

Events reference nodes by ID and include call stack context via `call_id` and `parent_call_id`.

## UI Architecture

The React application manages:

- **Session State** — Fetched from `/session.json` endpoint
- **Pipeline Derivation** — Splits session into separate execution chains
- **Graph Layout** — Computes node positions via dagre
- **Playback State** — Current step, playing/paused, speed
- **Selection State** — Active node, details panel visibility

Key components:

- `WorkflowCanvas` — React Flow wrapper with custom node types
- `GraphNode` — Node renderer with status indicators
- `NodeDetailsPanel` — Resizable sidebar with input/output display
- `PlaybackControls` — Timeline and playback buttons

## Design Decisions

**Why proc macros?** — Ergonomic API without runtime overhead for uninstrumented code paths.

**Why JSON sessions?** — Human-readable, easy to diff, works with standard tools.

**Why embedded UI?** — Zero external dependencies for `dbgflow serve`.

**Why dagre layout?** — Handles directed graphs well, produces readable left-to-right flows.

**Why thread-local storage?** — Simple model that works for single-threaded tests (most common case).

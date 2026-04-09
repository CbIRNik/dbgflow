# dbgflow

A graph-first debugger for Rust that visualizes program execution as an interactive node graph.

## Features

- **Visual Execution Graph** — See function calls as nodes and their relationships as edges
- **Time-Travel Playback** — Step through execution events with animated playback controls
- **Value Snapshots** — Capture and inspect data state at any point during execution
- **Test Integration** — Link test failures directly to the traced function that caused them
- **Browser UI** — Modern React-based interface with syntax highlighting and collapsible data views

## Demo

![Demo](./demo.mp4)

## Packages

dbgflow is published on crates.io as three packages:

| Package | Description |
|---------|-------------|
| [`dbgflow`](https://crates.io/crates/dbgflow) | Main crate with library and CLI binary |
| [`dbgflow-core`](https://crates.io/crates/dbgflow-core) | Runtime, session model, and embedded UI server |
| [`dbgflow-macros`](https://crates.io/crates/dbgflow-macros) | Procedural macros (`#[trace]`, `#[ui_debug]`, `#[dbg_test]`) |

Documentation: [docs.rs/dbgflow](https://docs.rs/dbgflow)

## Installation

Add the library to your project and install the CLI:

```bash
cargo add dbgflow
cargo install dbgflow
```

## Quick Start

### 1. Run the Demo

See dbgflow in action with the built-in demo:

```bash
dbgflow demo --serve
```

This generates a sample session and opens the browser UI at `http://127.0.0.1:3000`.

### 2. Instrument Your Code

```rust
use dbgflow::prelude::*;

#[ui_debug]
struct State {
    counter: usize,
}

#[trace]
fn step(state: &mut State) {
    state.counter += 1;
    state.emit_snapshot("after step");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[dbg_test]
    fn state_flow() {
        let mut state = State { counter: 0 };
        step(&mut state);
        assert_eq!(state.counter, 2); // This will fail
    }
}
```

### 3. Capture Test Sessions

```bash
dbgflow test --manifest-path /path/to/project/Cargo.toml --serve -- --lib
```

The `--serve` flag opens the UI immediately after tests complete, focused on the first failing test.

## Macros

### `#[trace]`

Marks functions that appear as nodes in the execution graph:

```rust
#[trace]
fn process_data(input: &str) -> Result<Output, Error> {
    // Function calls are recorded as enter/exit events
}

#[trace(name = "Custom Label")]
fn internal_step() {
    // Override the display name in the UI
}
```

### `#[ui_debug]`

Marks types for value inspection in the UI:

```rust
#[ui_debug]
struct Pipeline {
    stage: String,
    items: Vec<Item>,
}

impl Pipeline {
    fn advance(&mut self) {
        self.stage = "processing".into();
        self.emit_snapshot("stage changed"); // Captures current state
    }
}
```

The `emit_snapshot` method is automatically available on `#[ui_debug]` types.

### `#[dbg_test]`

Wraps tests to capture execution sessions:

```rust
#[dbg_test]
fn my_test() {
    // Test execution is captured as a session
    // Failures link to the last traced function
}
```

## CLI Commands

### `dbgflow demo`

Generate and optionally serve the built-in demo session:

```bash
dbgflow demo                          # Generate demo-session.json
dbgflow demo --serve                  # Generate and open in browser
dbgflow demo --output my-demo.json    # Custom output path
dbgflow demo --port 8080              # Custom port
```

### `dbgflow serve`

Serve a saved session or directory of sessions:

```bash
dbgflow serve artifacts/session.json       # Single session
dbgflow serve artifacts/test-sessions/     # Merge all sessions in directory
dbgflow serve session.json --port 8080     # Custom port
```

When serving a directory, all JSON files are merged into a single session with multiple pipelines.

### `dbgflow test`

Run cargo test with session capture:

```bash
dbgflow test                                           # Test current project
dbgflow test --manifest-path /path/to/Cargo.toml       # Test specific project
dbgflow test --serve                                   # Open UI after tests
dbgflow test --output-dir ./my-sessions                # Custom session directory
dbgflow test -- --lib                                  # Pass args to cargo test
dbgflow test -- --test integration_tests               # Run specific tests
```

Sessions are saved to `artifacts/test-sessions/run-<timestamp>/` by default.

## Browser UI

The dbgflow UI provides an interactive visualization of your execution sessions.

### Graph Canvas

- **Function Nodes** — Display traced functions with `fn` badge and call signature
- **Data Nodes** — Display `#[ui_debug]` types with `db` badge
- **Test Nodes** — Display tests with `t` badge, linked to their last traced function
- **Edges** — Show call relationships and test-to-function links
- **Status Indicators** — Color-coded dots show idle (gray), running (orange), success (green), or failure (red)

### Left Panel (Node Details)

Click any node to open the details panel:

- **Node Info** — Type badge and execution status
- **Source Code** — Collapsible view of the traced function source (with Rust syntax highlighting)
- **Input Section** — Captured function arguments and their values
- **Output Section** — Return values and snapshots emitted during execution
- **Resizable** — Drag the panel edge to adjust width

The panel automatically follows the active node during playback. Click the canvas background or press the X button to dismiss.

### Playback Controls

The bottom control bar provides:

- **Play/Pause** — Start or stop animated playback through events
- **Step Controls** — Jump to start or end of the timeline
- **Pipeline Selector** — Switch between multiple pipelines in a merged session
- **Step Selector** — Jump to a specific event by number
- **Speed Control** — Adjust playback speed (0.25x to 4x)
- **Canvas Mode Toggle** — Switch between "Pan" (drag canvas) and "Nodes" (drag individual nodes)
- **Timeline Slider** — Scrub through events with animated transitions

### Keyboard & Mouse

- Click a node to select it and open details
- Click the canvas background to deselect
- In Pan mode: drag to pan the viewport
- In Nodes mode: drag nodes to reposition them
- Scroll to zoom in/out

## Programmatic Capture

For non-test scenarios, use the capture helpers:

```rust
use dbgflow::prelude::*;

fn main() -> std::io::Result<()> {
    // Capture and immediately serve
    capture_and_serve("my session", "127.0.0.1", 3000, || {
        run_pipeline();
    })?;
    Ok(())
}
```

```rust
// Capture to a file
dbgflow::capture_to_file("my session", "artifacts/session.json", || {
    run_pipeline();
})?;
```

```rust
// Manual session management
dbgflow::init_session("my session");
run_pipeline();
dbgflow::save_current_session("artifacts/session.json")?;
```

## Session Format

Sessions are stored as JSON files containing:

| Field | Description |
|-------|-------------|
| `nodes` | Function, type, and test nodes with metadata |
| `edges` | Relationships between nodes (calls, test links) |
| `events` | Ordered sequence of execution events |

### Node Types

- `function` — Traced function
- `type` — `#[ui_debug]` data type
- `test` — `#[dbg_test]` test case

### Event Types

- `function_enter` — Function call started
- `function_exit` — Function call returned
- `value_snapshot` — Data state captured via `emit_snapshot`
- `test_started` — Test execution began
- `test_passed` — Test completed successfully
- `test_failed` — Test assertion failed

## Project Structure

```
crates/
├── dbg-cli/       # dbgflow library and CLI binary
├── dbg-core/      # Runtime, session model, embedded UI server
└── dbg-macros/    # Procedural macros
web/               # React Flow UI sources
examples/          # Example projects with traced code
docs/              # Architecture and release documentation
```

## Building from Source

Clone the repository and build:

```bash
git clone https://github.com/yourname/dbgflow
cd dbgflow
cargo build --release
```

The CLI binary is at `target/release/dbgflow`.

### UI Development

The UI assets are pre-built in `crates/dbg-core/ui`. To modify the frontend:

```bash
cd web
bun install
bun run build
```

This writes `app.js` and `app.css` to `crates/dbg-core/ui`.

The UI uses:

- `@xyflow/react` for graph rendering
- `@dagrejs/dagre` for automatic graph layout
- `prismjs` for syntax highlighting

## Limitations

- `#[dbg_test]` does not support async tests
- `dbgflow test` runs tests with `RUST_TEST_THREADS=1` for stable capture
- Value snapshots use `Debug` formatting, not structured diffs
- The UI is replay-oriented; live streaming is not yet supported

## License

MIT

# dbgflow

`dbgflow` is a graph-first debugger for Rust.

It gives you:

- a Rust library crate named `dbgflow`
- a CLI binary named `dbgflow`
- attribute macros to trace functions, data nodes, and tests
- a browser UI that renders execution as a graph with a timeline and failing test nodes

The public package name, Rust crate name, and CLI name are all `dbgflow`.

## What It Does

`dbgflow` is aimed at the workflow where you want to look at program execution as a graph instead of a text log.

Core concepts:

- `#[trace]` marks functions that should appear as executable graph nodes.
- `#[ui_debug]` marks structs or enums that should appear as data nodes and support value snapshots.
- `#[dbg_test]` wraps tests so a session is persisted per test and failures are linked to the latest traced node.
- `dbgflow test` runs `cargo test`, collects session JSON files, and can immediately serve the captured failing run.
- `dbgflow serve` opens any saved session in the local browser UI.

## Install

### From crates.io

```bash
cargo add dbgflow
cargo install dbgflow
```

This adds the library to your project and installs the `dbgflow` CLI into your Cargo bin directory.

### From this repository

If you want to work from source instead of crates.io:

```toml
[dependencies]
dbgflow = { path = "/absolute/path/to/dbg/crates/dbg-cli" }
```

```toml
[dev-dependencies]
dbgflow = { path = "/absolute/path/to/dbg/crates/dbg-cli" }
```

## Quickstart

### Library usage

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
        assert_eq!(state.counter, 2);
    }
}
```

### CLI usage

Run the built-in demo:

```bash
dbgflow demo --serve
```

Serve an existing session:

```bash
dbgflow serve artifacts/demo-session.json
```

Run a real project and capture traced test sessions:

```bash
dbgflow test --manifest-path /path/to/project/Cargo.toml -- --lib
```

If you want the UI immediately on the preferred captured session:

```bash
dbgflow test --manifest-path /path/to/project/Cargo.toml --serve -- --lib
```

## Real Project Flow

1. Add the dependency:

```bash
cargo add dbgflow
```

2. Import the prelude:

```rust
use dbgflow::prelude::*;
```

3. Mark the code you care about:

- put `#[trace]` on functions you want to see as execution nodes
- put `#[ui_debug]` on structs/enums whose state you want to snapshot
- put `#[dbg_test]` on tests you want captured as sessions

4. Run the test capture command:

```bash
dbgflow test --manifest-path /path/to/project/Cargo.toml -- --lib
```

5. Inspect the output. The CLI prints:

- the run directory under `artifacts/test-sessions/run-<timestamp>`
- every captured session file
- a ready-to-run `dbgflow serve /abs/path/to/session.json` command for the preferred session

6. Open the UI:

```bash
dbgflow serve /abs/path/to/failing-session.json
```

## Manual Capture

If you want to trace a specific block of code without wrapping it into `#[dbg_test]`, use the capture helpers:

```rust
use dbgflow::prelude::*;

fn main() -> std::io::Result<()> {
    capture_and_serve("checkout flow", "127.0.0.1", 3000, || {
        let mut state = State { counter: 0 };
        step(&mut state);
        classify(&state);
    })?;

    Ok(())
}
```

For file-based capture instead of serving immediately:

```rust
dbgflow::capture_to_file("checkout flow", "artifacts/manual-session.json", || {
    // traced code here
})?;
```

## Session Model

Each run produces a JSON session that contains:

- nodes
- edges
- events

Current node types:

- `function`
- `type`
- `test`

Current event types:

- `function_enter`
- `function_exit`
- `value_snapshot`
- `test_started`
- `test_passed`
- `test_failed`

## Workspace Layout

- `crates/dbg-cli`: package `dbgflow`, exposing the `dbgflow` library crate and `dbgflow` CLI binary
- `crates/dbg-core`: package `dbgflow-core`, containing runtime, session model, and embedded UI server
- `crates/dbg-macros`: package `dbgflow-macros`, containing `#[trace]`, `#[ui_debug]`, and `#[dbg_test]`
- `web`: React Flow UI sources, built with `bun`
- `docs/architecture.md`: system architecture notes
- `docs/publishing.md`: release checklist for future versions
- `examples/pipelines`: standalone example workspace with traced pipeline binaries and saved session JSON files

## UI Build

The repository includes built UI assets in `crates/dbg-core/ui`, so the Rust side works directly after clone.

If you change the frontend:

```bash
cd web
bun install
bun run build
```

That writes `app.js` and `app.css` into `crates/dbg-core/ui`.

The current UI uses:

- `@xyflow/react` for graph rendering
- `@dagrejs/dagre` for directed graph layout

## Current Limitations

- `#[dbg_test]` does not support async tests yet.
- `dbgflow test` forces `RUST_TEST_THREADS=1` to keep per-test capture stable in the current implementation.
- The test node is linked to the latest traced node seen during the test, not a full assertion stack.
- Value previews are mostly `Debug` snapshots and type names, not structured semantic diffs yet.
- The embedded UI is replay-oriented today; it is not yet a live streaming debugger.

## Development

Useful commands:

```bash
cargo test
bun run --cwd web build
cargo run -p dbgflow -- demo --serve
cargo run --manifest-path examples/pipelines/Cargo.toml --example loops
cargo run -p dbgflow -- serve examples/pipelines
```

## Published Packages

- crate: [crates.io/crates/dbgflow](https://crates.io/crates/dbgflow)
- core runtime: [crates.io/crates/dbgflow-core](https://crates.io/crates/dbgflow-core)
- proc macros: [crates.io/crates/dbgflow-macros](https://crates.io/crates/dbgflow-macros)
- docs: [docs.rs/dbgflow](https://docs.rs/dbgflow)

For release steps for the next version, see [docs/publishing.md](docs/publishing.md).

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

### CLI from the current repository

```bash
cargo install --path crates/dbg-cli
```

This installs the `dbgflow` binary into your Cargo bin directory.

### Library from crates.io

Once published, the dependency line for a real project will look like this:

```toml
[dependencies]
dbgflow = "0.1.0"
```

### Temporary local dependency

Before publication, use a path dependency:

```toml
[dependencies]
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

```toml
[dependencies]
dbgflow = "0.1.0"
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

- `crates/dbg-cli`: publishable package `dbgflow`, exposing the `dbgflow` library crate and `dbgflow` CLI binary
- `crates/dbg-core`: publishable package `dbgflow-core`, containing runtime, session model, and embedded UI server
- `crates/dbg-macros`: publishable package `dbgflow-macros`, containing `#[trace]`, `#[ui_debug]`, and `#[dbg_test]`
- `web`: React Flow UI sources, built with `bun`
- `docs/architecture.md`: system architecture notes
- `docs/publishing.md`: crates.io and Homebrew publication checklist
- `examples/real-project`: checked fixture demonstrating real-project integration

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
cargo run -p dbgflow -- test --manifest-path examples/real-project/Cargo.toml -- --lib
```

## Publication

The package name `dbg` is already taken on crates.io, so publication is prepared under:

- `dbgflow`
- `dbgflow-core`
- `dbgflow-macros`

The library crate name and binary are also `dbgflow`.

For the exact publishing steps, see [docs/publishing.md](docs/publishing.md).

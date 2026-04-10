# Releasing

This document is the release checklist for the next `dbgflow` release.

Current release line: `0.3.0`.

The first public release is already live:

- [crates.io/crates/dbgflow](https://crates.io/crates/dbgflow)
- [crates.io/crates/dbgflow-core](https://crates.io/crates/dbgflow-core)
- [crates.io/crates/dbgflow-macros](https://crates.io/crates/dbgflow-macros)

## Pre-release Checklist

1. Build frontend assets:

```bash
cd web
bun install
bun run build
```

2. Run tests:

```bash
cargo test
cargo check --manifest-path examples/pipelines/Cargo.toml --examples
cargo run --manifest-path examples/pipelines/Cargo.toml --example loops
```

3. Verify package contents:

```bash
cargo package -p dbgflow --allow-dirty --list
cargo package -p dbgflow-core --allow-dirty --list
cargo package -p dbgflow-macros --allow-dirty --list
```

4. Dry-run packaging:

```bash
cargo package -p dbgflow-core --allow-dirty
cargo package -p dbgflow-macros --allow-dirty
cargo package -p dbgflow --allow-dirty --list
```

Important:

- `dbgflow-core` and `dbgflow-macros` can be fully verified locally.
- `dbgflow` depends on those packages as published registry dependencies, so local packaging of the top-level crate is only meaningful after the dependency versions exist on crates.io.
- Use `cargo package -p dbgflow --allow-dirty --list` as the fast local contents check before release.

5. Optional local install smoke test:

```bash
cargo install --path crates/dbg-cli --force
dbgflow demo --serve
```

## Version Bump

Update the shared workspace version in the root `Cargo.toml` and make sure internal dependency versions still match:

- `dbgflow-core`
- `dbgflow-macros`
- `dbgflow`

## Publish Order

The top-level package depends on the two internal packages, so publish in this order:

1. `dbgflow-core`
2. `dbgflow-macros`
3. `dbgflow`

Commands:

```bash
cargo publish -p dbgflow-core
cargo publish -p dbgflow-macros
cargo publish -p dbgflow
```

If crates.io indexing has not propagated yet, wait briefly between publishes.

## docs.rs

Expected docs.rs pages:

- `https://docs.rs/dbgflow`
- `https://docs.rs/dbgflow-core`
- `https://docs.rs/dbgflow-macros`

The manifests already include `package.metadata.docs.rs`.

If docs are missing right after publish, check:

- `https://docs.rs/releases/queue?expand=1`
- `https://docs.rs/releases/search?query=dbgflow`

docs.rs often lags behind crates.io for a while after a fresh publish.

## Homebrew

Homebrew is possible, but not as the primary publication channel yet.

To publish on Homebrew you still need:

- a public Git repository
- versioned release tarballs
- release SHA256 values
- either a personal tap or a formula accepted by Homebrew/homebrew-core

In practice, the simplest standard installation path for this project is:

- publish packages to crates.io
- let users install the CLI via `cargo install dbgflow`

After public releases exist, you can add a Homebrew formula that builds the Rust binary from the release tarball.

## Formula Template

Create a tap repository such as `yourname/homebrew-tap` and add `Formula/dbgflow.rb`:

```ruby
class Dbgflow < Formula
  desc "Graph-first Rust debugger with trace macros and a browser UI"
  homepage "https://github.com/yourname/dbgflow"
  url "https://github.com/yourname/dbgflow/archive/refs/tags/v0.3.0.tar.gz"
  sha256 "REPLACE_WITH_RELEASE_TARBALL_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    cd "crates/dbg-cli" do
      system "cargo", "install", *std_cargo_args(path: ".")
    end
  end

  test do
    assert_match "Graph-first Rust debugger", shell_output("#{bin}/dbgflow --help")
  end
end
```

Then users can install from the tap with:

```bash
brew install yourname/tap/dbgflow
```

## Release Hygiene

- versioning policy
- release tagging convention such as `v0.3.0`

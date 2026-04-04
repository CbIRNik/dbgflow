# Publishing

This document is the release checklist for publishing `dbgflow`.

## Package Names

The original package name `dbg` is already occupied on crates.io.

Prepared publishable package names:

- `dbgflow`
- `dbgflow-core`
- `dbgflow-macros`

Important:

- the crates.io package name is `dbgflow`
- the Rust library crate name is `dbgflow`
- the CLI binary name is `dbgflow`

That means users will write:

```toml
[dependencies]
dbgflow = "0.1.0"
```

## Pre-publish Checklist

1. Build frontend assets:

```bash
cd web
bun install
bun run build
```

2. Run tests:

```bash
cargo test
cargo test --manifest-path examples/real-project/Cargo.toml
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
- `dbgflow` depends on those packages as published registry dependencies, so `cargo package -p dbgflow` will fail until `dbgflow-core` and `dbgflow-macros` exist on crates.io.
- For the top-level crate, use `cargo package -p dbgflow --allow-dirty --list` as the local contents check before publication.

5. Optional local install smoke test:

```bash
cargo install --path crates/dbg-cli --force
dbgflow demo --serve
```

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
  url "https://github.com/yourname/dbgflow/archive/refs/tags/v0.1.0.tar.gz"
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

Before the first public release, still decide:

- versioning policy
- release tagging convention such as `v0.1.0`

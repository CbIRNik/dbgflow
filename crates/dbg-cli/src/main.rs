//! CLI binary for the dbgflow graph debugger.
//!
//! This provides commands for running demos, serving sessions, and
//! running tests with session capture.

use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use dbgflow::{EventKind, read_session_json};

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

/// Graph-first Rust debugger for Rust code.
#[derive(Parser)]
#[command(name = "dbgflow", about = "Graph-first Rust debugger for Rust code")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Available subcommands.
#[derive(Subcommand)]
enum Command {
    /// Run the built-in demo pipeline and optionally serve it.
    Demo {
        /// Output path for the demo session JSON.
        #[arg(long, default_value = "artifacts/demo-session.json")]
        output: PathBuf,

        /// Start the UI server after generating the session.
        #[arg(long)]
        serve: bool,

        /// Port for the UI server.
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },

    /// Serve a saved session JSON file or aggregate all JSON sessions in a directory.
    Serve {
        /// Path to a session JSON file, or a directory containing session JSON files.
        session: PathBuf,

        /// Port for the UI server.
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },

    /// Run cargo test with session capture and optionally serve results.
    Test {
        /// Path to Cargo.toml for the project to test.
        #[arg(long)]
        manifest_path: Option<PathBuf>,

        /// Directory to store captured sessions.
        #[arg(long, default_value = "artifacts/test-sessions")]
        output_dir: PathBuf,

        /// Start the UI server after tests complete.
        #[arg(long)]
        serve: bool,

        /// Port for the UI server.
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Additional arguments to pass to cargo test.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Demo {
            output,
            serve,
            port,
        } => dbgflow::demo::run(output, serve, port),
        Command::Serve { session, port } => dbgflow::demo::serve_saved(session, port),
        Command::Test {
            manifest_path,
            output_dir,
            serve,
            port,
            cargo_args,
        } => run_test(manifest_path, output_dir, serve, port, cargo_args),
    }
}

// ---------------------------------------------------------------------------
// Test command implementation
// ---------------------------------------------------------------------------

/// Configuration for a test run.
#[derive(Clone)]
struct TestRunConfig {
    manifest_path: Option<PathBuf>,
    output_dir: PathBuf,
    cargo_args: Vec<String>,
}

/// Output from a single test run.
struct TestRunOutput {
    status: ExitStatus,
    run_dir: PathBuf,
    session_paths: Vec<PathBuf>,
    preferred_session: Option<PathBuf>,
}

/// Runs tests with session capture.
fn run_test(
    manifest_path: Option<PathBuf>,
    output_dir: PathBuf,
    serve: bool,
    port: u16,
    cargo_args: Vec<String>,
) -> std::io::Result<()> {
    let config = TestRunConfig {
        manifest_path,
        output_dir: std::env::current_dir()?.join(output_dir),
        cargo_args,
    };

    let first_run = execute_test_run(&config)?;
    print_test_summary(&first_run);

    if first_run.session_paths.is_empty() {
        println!(
            "No sessions were captured. Annotate tests with #[dbg_test] from dbgflow to emit per-test sessions."
        );
        return Ok(());
    }

    let preferred_session = first_run
        .preferred_session
        .clone()
        .expect("preferred session should exist when at least one session was captured");

    println!("Open a captured session with:");
    println!("  dbgflow serve {}", preferred_session.display());

    if serve {
        let initial_session = read_session_json(&preferred_session)?;
        let rerun_config = config.clone();
        println!("Serving {}", preferred_session.display());
        dbgflow::serve_session_with_rerun(initial_session, "127.0.0.1", port, move || {
            let rerun = execute_test_run(&rerun_config)?;
            print_test_summary(&rerun);
            if rerun.session_paths.is_empty() {
                return Err(std::io::Error::other(
                    "rerun finished without dbgflow sessions; make sure #[dbg_test] is present",
                ));
            }
            let preferred_session = rerun.preferred_session.ok_or_else(|| {
                std::io::Error::other("rerun finished without a preferred dbgflow session")
            })?;
            read_session_json(preferred_session)
        })?;
    }

    if !first_run.status.success() {
        return Err(std::io::Error::other(format!(
            "cargo test exited with status {}",
            first_run.status
        )));
    }

    Ok(())
}

/// Executes a test run and collects session files.
fn execute_test_run(config: &TestRunConfig) -> std::io::Result<TestRunOutput> {
    let run_dir = config
        .output_dir
        .join(format!("run-{}", unix_timestamp_millis()));
    fs::create_dir_all(&run_dir)?;

    let mut command = ProcessCommand::new("cargo");
    command.arg("test");
    if let Some(manifest_path) = &config.manifest_path {
        command.arg("--manifest-path").arg(manifest_path);
    }
    command.args(&config.cargo_args);
    command.env("DBG_SESSION_DIR", &run_dir);
    command.env("RUST_TEST_THREADS", "1");
    let status = command.status()?;

    let mut session_paths = collect_session_files(&run_dir)?;
    session_paths.sort();

    let preferred_session =
        find_failed_session(&session_paths)?.or_else(|| session_paths.first().cloned());

    Ok(TestRunOutput {
        status,
        run_dir,
        session_paths,
        preferred_session,
    })
}

/// Prints a summary of a test run to stdout.
fn print_test_summary(run: &TestRunOutput) {
    println!(
        "Captured {} dbgflow session(s) in {}",
        run.session_paths.len(),
        run.run_dir.display()
    );
    for path in &run.session_paths {
        println!("  {}", path.display());
    }
}

/// Collects all JSON session files from a directory.
fn collect_session_files(dir: &PathBuf) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// Finds the first session that contains a test failure event.
fn find_failed_session(paths: &[PathBuf]) -> std::io::Result<Option<PathBuf>> {
    for path in paths {
        let session = read_session_json(path)?;
        let has_failure = session
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::TestFailed));
        if has_failure {
            return Ok(Some(path.clone()));
        }
    }
    Ok(None)
}

/// Returns the current Unix timestamp in milliseconds.
fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use dbg::{EventKind, read_session_json};

#[derive(Parser)]
#[command(name = "dbg", about = "Graph-first Rust debugger for Rust code")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Demo {
        #[arg(long, default_value = "artifacts/demo-session.json")]
        output: PathBuf,
        #[arg(long)]
        serve: bool,
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },
    Serve {
        session: PathBuf,
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },
    Test {
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        #[arg(long, default_value = "artifacts/test-sessions")]
        output_dir: PathBuf,
        #[arg(long)]
        serve: bool,
        #[arg(long, default_value_t = 3000)]
        port: u16,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Demo {
            output,
            serve,
            port,
        } => dbg::demo::run(output, serve, port),
        Command::Serve { session, port } => dbg::demo::serve_saved(session, port),
        Command::Test {
            manifest_path,
            output_dir,
            serve,
            port,
            cargo_args,
        } => run_test(manifest_path, output_dir, serve, port, cargo_args),
    }
}

fn run_test(
    manifest_path: Option<PathBuf>,
    output_dir: PathBuf,
    serve: bool,
    port: u16,
    cargo_args: Vec<String>,
) -> std::io::Result<()> {
    let run_dir = std::env::current_dir()?
        .join(output_dir)
        .join(format!("run-{}", unix_timestamp()));
    fs::create_dir_all(&run_dir)?;

    let mut command = ProcessCommand::new("cargo");
    command.arg("test");
    if let Some(manifest_path) = manifest_path {
        command.arg("--manifest-path").arg(manifest_path);
    }
    command.args(&cargo_args);
    command.env("DBG_SESSION_DIR", &run_dir);
    command.env("RUST_TEST_THREADS", "1");
    let status = command.status()?;

    let mut session_paths = collect_session_paths(&run_dir)?;
    session_paths.sort();

    println!(
        "Captured {} dbg session(s) in {}",
        session_paths.len(),
        run_dir.display()
    );
    for path in &session_paths {
        println!("  {}", path.display());
    }

    if session_paths.is_empty() {
        println!(
            "No sessions were captured. Annotate tests with #[dbg::dbg_test] to emit per-test sessions."
        );
        return Ok(());
    }

    let preferred_session =
        pick_session_to_serve(&session_paths)?.unwrap_or_else(|| session_paths[0].clone());

    println!("Open a captured session with:");
    println!("  dbg serve {}", preferred_session.display());

    if serve {
        let session_path = preferred_session;
        println!("Serving {}", session_path.display());
        dbg::demo::serve_saved(session_path, port)?;
    }

    if !status.success() {
        return Err(std::io::Error::other(format!(
            "cargo test exited with status {status}"
        )));
    }

    Ok(())
}

fn collect_session_paths(dir: &PathBuf) -> std::io::Result<Vec<PathBuf>> {
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

fn pick_session_to_serve(paths: &[PathBuf]) -> std::io::Result<Option<PathBuf>> {
    for path in paths {
        let session = read_session_json(path)?;
        if session
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::TestFailed))
        {
            return Ok(Some(path.clone()));
        }
    }
    Ok(paths.first().cloned())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

//! Example: Network Request Pipeline
//!
//! Demonstrates tracing a pipeline that performs real network operations
//! with retry logic and timeout handling.

use std::process::{Command, Stdio};
use std::time::Duration;

use dbgflow::prelude::*;

#[ui_debug(name = "Network State")]
struct NetworkState {
    target_host: String,
    attempts: usize,
    max_attempts: usize,
    last_response_time_ms: Option<u64>,
    status: NetworkStatus,
}

#[ui_debug(name = "Network Status")]
enum NetworkStatus {
    Pending,
    Connecting,
    Success,
    Failed(String),
    Timeout,
}

impl NetworkState {
    fn new(host: &str, max_attempts: usize) -> Self {
        Self {
            target_host: host.to_owned(),
            attempts: 0,
            max_attempts,
            last_response_time_ms: None,
            status: NetworkStatus::Pending,
        }
    }
}

#[trace(name = "Ping Host")]
fn ping_host(state: &mut NetworkState) -> bool {
    state.status = NetworkStatus::Connecting;
    state.emit_snapshot("initiating ping");

    // Use ping command (works on macOS/Linux)
    let output = Command::new("ping")
        .args(["-c", "1", "-t", "2", &state.target_host])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(result) if result.status.success() => {
            // Extract response time from ping output
            let stdout = String::from_utf8_lossy(&result.stdout);
            let time_ms = parse_ping_time(&stdout);
            state.last_response_time_ms = time_ms;
            state.status = NetworkStatus::Success;
            state.emit_snapshot("ping successful");
            true
        }
        Ok(_) => {
            state.status = NetworkStatus::Failed("Host unreachable".to_owned());
            state.emit_snapshot("ping failed");
            false
        }
        Err(e) => {
            state.status = NetworkStatus::Failed(e.to_string());
            state.emit_snapshot("ping error");
            false
        }
    }
}

fn parse_ping_time(output: &str) -> Option<u64> {
    // Parse "time=X.Y ms" from ping output
    for line in output.lines() {
        if let Some(time_idx) = line.find("time=") {
            let after_time = &line[time_idx + 5..];
            let time_str: String = after_time
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            return time_str.parse::<f64>().ok().map(|t| t as u64);
        }
    }
    None
}

#[trace(name = "Retry Network Request")]
fn retry_request(state: &mut NetworkState) -> bool {
    while state.attempts < state.max_attempts {
        state.attempts += 1;
        state.emit_snapshot(&format!("attempt {}/{}", state.attempts, state.max_attempts));

        if ping_host(state) {
            return true;
        }

        if state.attempts < state.max_attempts {
            state.emit_snapshot("waiting before retry");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    state.status = NetworkStatus::Timeout;
    state.emit_snapshot("all attempts exhausted");
    false
}

#[trace(name = "HTTP Check")]
fn http_check(state: &mut NetworkState) -> Option<u16> {
    state.emit_snapshot("checking HTTP endpoint");

    // Try a simple HTTP request using curl
    let output = Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-m",
            "5",
            &format!("https://{}", state.target_host),
        ])
        .output();

    match output {
        Ok(result) => {
            let code_str = String::from_utf8_lossy(&result.stdout);
            let status_code = code_str.trim().parse::<u16>().ok();
            state.emit_snapshot(&format!("HTTP status: {:?}", status_code));
            status_code
        }
        Err(e) => {
            state.emit_snapshot(&format!("HTTP check failed: {}", e));
            None
        }
    }
}

#[trace(name = "Network Diagnostics Pipeline")]
fn run_diagnostics(state: &mut NetworkState) -> DiagnosticResult {
    state.emit_snapshot("starting network diagnostics");

    // Step 1: Ping test with retries
    let ping_ok = retry_request(state);

    // Step 2: HTTP check if ping succeeded
    let http_status = if ping_ok {
        http_check(state)
    } else {
        None
    };

    let result = DiagnosticResult {
        host: state.target_host.clone(),
        ping_success: ping_ok,
        ping_latency_ms: state.last_response_time_ms,
        http_status_code: http_status,
    };

    state.emit_snapshot("diagnostics complete");
    result
}

#[ui_debug(name = "Diagnostic Result")]
struct DiagnosticResult {
    host: String,
    ping_success: bool,
    ping_latency_ms: Option<u64>,
    http_status_code: Option<u16>,
}

fn main() -> std::io::Result<()> {
    let result = dbgflow::capture_to_file(
        "Network Request Pipeline",
        "network_session.json",
        || {
            let mut state = NetworkState::new("google.com", 3);
            run_diagnostics(&mut state)
        },
    )?;

    println!("Network Diagnostics Complete:");
    println!("  Host: {}", result.host);
    println!("  Ping: {}", if result.ping_success { "OK" } else { "FAILED" });
    if let Some(latency) = result.ping_latency_ms {
        println!("  Latency: {}ms", latency);
    }
    if let Some(status) = result.http_status_code {
        println!("  HTTP Status: {}", status);
    }

    println!("\nSession saved to network_session.json");
    Ok(())
}

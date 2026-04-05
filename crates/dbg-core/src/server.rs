//! Embedded HTTP server for serving the debugger UI.
//!
//! This module provides a minimal HTTP server that serves the browser-based
//! debugging interface and exposes API endpoints for session data and reruns.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;

use crate::session::Session;

/// Type alias for rerun handler closures.
pub type RerunHandler = Arc<dyn Fn() -> std::io::Result<Session> + Send + Sync>;

/// Internal server state shared across request handlers.
struct ServerState {
    session: Session,
    generation: u64,
    running: bool,
    last_error: Option<String>,
}

/// Status response returned by the `/api/status` endpoint.
#[derive(Serialize)]
struct StatusResponse<'a> {
    running: bool,
    can_rerun: bool,
    generation: u64,
    session_title: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

/// Returns the MIME content type for a given request path.
fn content_type_for_path(path: &str) -> &'static str {
    match path {
        "/" => "text/html; charset=utf-8",
        "/app.js" => "application/javascript; charset=utf-8",
        "/app.css" => "text/css; charset=utf-8",
        "/globals.css" => "text/css; charset=utf-8",
        "/session.json" => "application/json; charset=utf-8",
        "/api/status" => "application/json; charset=utf-8",
        "/api/rerun" => "application/json; charset=utf-8",
        _ => "text/plain; charset=utf-8",
    }
}

/// Writes an HTTP response to the given stream.
fn write_response(
    stream: &mut TcpStream,
    method: &str,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let body_len = body.len();
    let response = if method == "HEAD" {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {body_len}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n"
        )
    } else {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {body_len}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}"
        )
    };
    stream.write_all(response.as_bytes())
}

/// Parses an HTTP request and returns the method and path.
fn parse_request(stream: &mut TcpStream) -> std::io::Result<(String, String)> {
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let line = request.lines().next().unwrap_or_default();
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let raw_path = parts.next().unwrap_or("/");
    let path = raw_path.split(['?', '#']).next().unwrap_or("/");
    Ok((method.to_owned(), path.to_owned()))
}

/// Serializes a value to a JSON string.
fn json_body<T: Serialize>(value: &T) -> std::io::Result<String> {
    serde_json::to_string(value).map_err(std::io::Error::other)
}

// ---------------------------------------------------------------------------
// Server implementation
// ---------------------------------------------------------------------------

/// Core server loop implementation.
fn serve_session_inner(
    session: Session,
    host: &str,
    port: u16,
    rerun: Option<RerunHandler>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind((host, port))?;
    let shared = Arc::new(Mutex::new(ServerState {
        session,
        generation: 0,
        running: false,
        last_error: None,
    }));

    let html = super::ui::index_html();
    let app_js = super::ui::app_js();
    let app_css = super::ui::app_css();
    let globals_css = super::ui::globals_css();

    println!("Debugger UI: http://{host}:{port}");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        let (method, path) = match parse_request(&mut stream) {
            Ok(request) => request,
            Err(_) => continue,
        };

        let result = handle_request(
            &mut stream,
            &method,
            &path,
            &shared,
            &rerun,
            &html,
            &app_js,
            &app_css,
            &globals_css,
        );

        if let Err(error) = result {
            // Ignore broken pipe and connection reset errors
            if !matches!(
                error.kind(),
                std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
            ) {
                return Err(error);
            }
        }
    }

    Ok(())
}

/// Handles a single HTTP request.
#[allow(clippy::too_many_arguments)]
fn handle_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    shared: &Arc<Mutex<ServerState>>,
    rerun: &Option<RerunHandler>,
    html: &str,
    app_js: &str,
    app_css: &str,
    globals_css: &str,
) -> std::io::Result<()> {
    match (method, path) {
        (_, "/") => write_response(stream, method, "200 OK", content_type_for_path("/"), html),
        (_, "/app.js") => write_response(
            stream,
            method,
            "200 OK",
            content_type_for_path("/app.js"),
            app_js,
        ),
        (_, "/app.css") => write_response(
            stream,
            method,
            "200 OK",
            content_type_for_path("/app.css"),
            app_css,
        ),
        (_, "/globals.css") => write_response(
            stream,
            method,
            "200 OK",
            content_type_for_path("/globals.css"),
            globals_css,
        ),
        (_, "/session.json") => {
            let body = {
                let state = shared
                    .lock()
                    .expect("dbgflow-core serve session mutex poisoned");
                json_body(&state.session)?
            };
            write_response(
                stream,
                method,
                "200 OK",
                content_type_for_path("/session.json"),
                &body,
            )
        }
        (_, "/api/status") => {
            let body = {
                let state = shared
                    .lock()
                    .expect("dbgflow-core serve session mutex poisoned");
                let status = StatusResponse {
                    running: state.running,
                    can_rerun: rerun.is_some(),
                    generation: state.generation,
                    session_title: &state.session.title,
                    last_error: state.last_error.as_deref(),
                };
                json_body(&status)?
            };
            write_response(
                stream,
                method,
                "200 OK",
                content_type_for_path("/api/status"),
                &body,
            )
        }
        ("POST", "/api/rerun") => handle_rerun_request(stream, method, shared, rerun),
        _ => write_response(
            stream,
            method,
            "404 Not Found",
            content_type_for_path(""),
            "not found",
        ),
    }
}

/// Handles a POST request to trigger a rerun.
fn handle_rerun_request(
    stream: &mut TcpStream,
    method: &str,
    shared: &Arc<Mutex<ServerState>>,
    rerun: &Option<RerunHandler>,
) -> std::io::Result<()> {
    let body = if let Some(rerun_handler) = rerun.clone() {
        let should_spawn = {
            let mut state = shared
                .lock()
                .expect("dbgflow-core serve session mutex poisoned");
            if state.running {
                false
            } else {
                state.running = true;
                state.last_error = None;
                true
            }
        };

        if should_spawn {
            let shared = Arc::clone(shared);
            thread::spawn(move || {
                let rerun_result = rerun_handler();
                let mut state = shared
                    .lock()
                    .expect("dbgflow-core serve session mutex poisoned");
                state.running = false;
                match rerun_result {
                    Ok(session) => {
                        state.session = session;
                        state.generation += 1;
                    }
                    Err(error) => {
                        state.last_error = Some(error.to_string());
                    }
                }
            });
        }

        let state = shared
            .lock()
            .expect("dbgflow-core serve session mutex poisoned");
        let status = StatusResponse {
            running: state.running,
            can_rerun: true,
            generation: state.generation,
            session_title: &state.session.title,
            last_error: state.last_error.as_deref(),
        };
        json_body(&status)?
    } else {
        json_body(&serde_json::json!({
            "running": false,
            "can_rerun": false,
            "generation": 0_u64,
            "last_error": "rerun is not available for this session"
        }))?
    };

    let status = if rerun.is_some() {
        "202 Accepted"
    } else {
        "405 Method Not Allowed"
    };

    write_response(
        stream,
        method,
        status,
        content_type_for_path("/api/rerun"),
        &body,
    )
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Serves a session over the embedded local HTTP server.
pub fn serve_session(session: Session, host: &str, port: u16) -> std::io::Result<()> {
    serve_session_inner(session, host, port, None)
}

/// Serves a session over the embedded local HTTP server and exposes a rerun API.
///
/// The UI can call `POST /api/rerun` to request a fresh session. The provided
/// closure must run the underlying pipeline or test command and return the new
/// captured session snapshot.
pub fn serve_session_with_rerun(
    session: Session,
    host: &str,
    port: u16,
    rerun: impl Fn() -> std::io::Result<Session> + Send + Sync + 'static,
) -> std::io::Result<()> {
    serve_session_inner(session, host, port, Some(Arc::new(rerun)))
}

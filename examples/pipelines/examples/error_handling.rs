//! Example: Error Handling Pipeline
//!
//! Demonstrates tracing error handling patterns including Result/Option types,
//! error propagation, and recovery strategies.

use std::collections::HashMap;
use std::fmt;

use dbgflow::prelude::*;

// ============================================================================
// Custom error types
// ============================================================================

#[ui_debug(name = "App Error")]
enum AppError {
    NotFound { resource: String, id: String },
    ValidationFailed { field: String, message: String },
    PermissionDenied { action: String, user: String },
    NetworkError { url: String, status: Option<u16> },
    DatabaseError { query: String, details: String },
    RateLimited { retry_after_secs: u64 },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound { resource, id } =>
                write!(f, "{} not found: {}", resource, id),
            AppError::ValidationFailed { field, message } =>
                write!(f, "Validation failed for {}: {}", field, message),
            AppError::PermissionDenied { action, user } =>
                write!(f, "Permission denied: {} cannot {}", user, action),
            AppError::NetworkError { url, status } =>
                write!(f, "Network error for {}: {:?}", url, status),
            AppError::DatabaseError { query, details } =>
                write!(f, "Database error in '{}': {}", query, details),
            AppError::RateLimited { retry_after_secs } =>
                write!(f, "Rate limited, retry after {}s", retry_after_secs),
        }
    }
}

// ============================================================================
// State tracking
// ============================================================================

#[ui_debug(name = "Operation State")]
struct OperationState {
    operation_name: String,
    attempts: usize,
    errors_encountered: Vec<String>,
    recovered: bool,
    final_status: OperationStatus,
}

#[ui_debug(name = "Operation Status")]
enum OperationStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    PartialSuccess,
}

impl OperationState {
    fn new(name: &str) -> Self {
        Self {
            operation_name: name.to_owned(),
            attempts: 0,
            errors_encountered: Vec::new(),
            recovered: false,
            final_status: OperationStatus::Pending,
        }
    }

    fn record_error(&mut self, error: &str) {
        self.errors_encountered.push(error.to_owned());
    }
}

// ============================================================================
// Mock database
// ============================================================================

#[ui_debug(name = "Database")]
struct MockDatabase {
    users: HashMap<u64, String>,
    posts: HashMap<u64, (u64, String)>,
    query_count: usize,
}

impl MockDatabase {
    fn new() -> Self {
        let mut users = HashMap::new();
        users.insert(1, "alice".to_owned());
        users.insert(2, "bob".to_owned());

        let mut posts = HashMap::new();
        posts.insert(100, (1, "Hello World".to_owned()));
        posts.insert(101, (1, "Rust is awesome".to_owned()));
        posts.insert(102, (2, "Learning dbgflow".to_owned()));

        Self { users, posts, query_count: 0 }
    }
}

// ============================================================================
// Operations with error handling
// ============================================================================

#[trace(name = "Find User")]
fn find_user(db: &mut MockDatabase, user_id: u64, state: &mut OperationState) -> Result<String, AppError> {
    db.query_count += 1;
    state.attempts += 1;
    state.final_status = OperationStatus::InProgress;
    state.emit_snapshot(&format!("looking up user {}", user_id));

    match db.users.get(&user_id) {
        Some(username) => {
            state.emit_snapshot(&format!("found user: {}", username));
            Ok(username.clone())
        }
        None => {
            let error = AppError::NotFound {
                resource: "User".to_owned(),
                id: user_id.to_string(),
            };
            state.record_error(&error.to_string());
            state.emit_snapshot("user not found");
            Err(error)
        }
    }
}

#[trace(name = "Find Post")]
fn find_post(db: &mut MockDatabase, post_id: u64, state: &mut OperationState) -> Result<(u64, String), AppError> {
    db.query_count += 1;
    state.attempts += 1;
    state.emit_snapshot(&format!("looking up post {}", post_id));

    match db.posts.get(&post_id) {
        Some(post) => {
            state.emit_snapshot(&format!("found post: '{}'", post.1));
            Ok(post.clone())
        }
        None => {
            let error = AppError::NotFound {
                resource: "Post".to_owned(),
                id: post_id.to_string(),
            };
            state.record_error(&error.to_string());
            state.emit_snapshot("post not found");
            Err(error)
        }
    }
}

#[trace(name = "Validate Input")]
fn validate_input(input: &str, state: &mut OperationState) -> Result<String, AppError> {
    state.attempts += 1;
    state.emit_snapshot(&format!("validating input: '{}'", input));

    if input.is_empty() {
        let error = AppError::ValidationFailed {
            field: "input".to_owned(),
            message: "cannot be empty".to_owned(),
        };
        state.record_error(&error.to_string());
        state.emit_snapshot("validation failed: empty input");
        return Err(error);
    }

    if input.len() > 100 {
        let error = AppError::ValidationFailed {
            field: "input".to_owned(),
            message: "exceeds maximum length of 100".to_owned(),
        };
        state.record_error(&error.to_string());
        state.emit_snapshot("validation failed: too long");
        return Err(error);
    }

    state.emit_snapshot("validation passed");
    Ok(input.trim().to_owned())
}

#[trace(name = "Check Permission")]
fn check_permission(user: &str, action: &str, state: &mut OperationState) -> Result<(), AppError> {
    state.attempts += 1;
    state.emit_snapshot(&format!("checking if '{}' can '{}'", user, action));

    let allowed_actions = ["read", "write", "comment"];
    if allowed_actions.contains(&action) {
        state.emit_snapshot("permission granted");
        Ok(())
    } else {
        let error = AppError::PermissionDenied {
            action: action.to_owned(),
            user: user.to_owned(),
        };
        state.record_error(&error.to_string());
        state.emit_snapshot("permission denied");
        Err(error)
    }
}

// ============================================================================
// Error propagation with ?
// ============================================================================

#[trace(name = "Get Post With Author")]
fn get_post_with_author(
    db: &mut MockDatabase,
    post_id: u64,
    state: &mut OperationState,
) -> Result<(String, String), AppError> {
    state.emit_snapshot("fetching post with author info");

    let (author_id, content) = find_post(db, post_id, state)?;
    let author_name = find_user(db, author_id, state)?;

    state.final_status = OperationStatus::Success;
    state.emit_snapshot(&format!("got post by {}: '{}'", author_name, content));
    Ok((author_name, content))
}

#[trace(name = "Create Comment")]
fn create_comment(
    db: &mut MockDatabase,
    user_id: u64,
    post_id: u64,
    comment: &str,
    state: &mut OperationState,
) -> Result<String, AppError> {
    state.emit_snapshot("creating comment");

    // Step 1: Find user
    let username = find_user(db, user_id, state)?;

    // Step 2: Check permission
    check_permission(&username, "comment", state)?;

    // Step 3: Find post
    let _ = find_post(db, post_id, state)?;

    // Step 4: Validate comment
    let validated = validate_input(comment, state)?;

    state.final_status = OperationStatus::Success;
    let result = format!("{} commented: '{}'", username, validated);
    state.emit_snapshot(&result);
    Ok(result)
}

// ============================================================================
// Option handling
// ============================================================================

#[ui_debug(name = "Config")]
struct Config {
    database_url: Option<String>,
    api_key: Option<String>,
    timeout_secs: Option<u64>,
    retry_count: Option<u32>,
}

#[trace(name = "Load Config")]
fn load_config(state: &mut OperationState) -> Config {
    state.emit_snapshot("loading configuration");

    let config = Config {
        database_url: Some("postgres://localhost/app".to_owned()),
        api_key: None, // Simulating missing config
        timeout_secs: Some(30),
        retry_count: None,
    };

    state.emit_snapshot("config loaded");
    config
}

#[trace(name = "Get Database URL")]
fn get_database_url(config: &Config, state: &mut OperationState) -> Result<String, AppError> {
    state.emit_snapshot("getting database URL");

    config.database_url.clone().ok_or_else(|| {
        let error = AppError::ValidationFailed {
            field: "database_url".to_owned(),
            message: "not configured".to_owned(),
        };
        state.record_error(&error.to_string());
        state.emit_snapshot("database URL not found");
        error
    })
}

#[trace(name = "Get API Key")]
fn get_api_key(config: &Config, state: &mut OperationState) -> Result<String, AppError> {
    state.emit_snapshot("getting API key");

    config.api_key.clone().ok_or_else(|| {
        let error = AppError::ValidationFailed {
            field: "api_key".to_owned(),
            message: "not configured".to_owned(),
        };
        state.record_error(&error.to_string());
        state.emit_snapshot("API key not found");
        error
    })
}

#[trace(name = "Get Timeout")]
fn get_timeout_or_default(config: &Config, state: &mut OperationState) -> u64 {
    state.emit_snapshot("getting timeout");

    let timeout = config.timeout_secs.unwrap_or_else(|| {
        state.emit_snapshot("using default timeout");
        60
    });

    state.emit_snapshot(&format!("timeout: {}s", timeout));
    timeout
}

// ============================================================================
// Recovery patterns
// ============================================================================

#[trace(name = "Fetch With Retry")]
fn fetch_with_retry(
    url: &str,
    max_retries: u32,
    state: &mut OperationState,
) -> Result<String, AppError> {
    state.emit_snapshot(&format!("fetching {} with {} retries", url, max_retries));

    for attempt in 1..=max_retries {
        state.attempts = attempt as usize;
        state.emit_snapshot(&format!("attempt {}/{}", attempt, max_retries));

        // Simulate failure on first attempts
        if attempt < max_retries {
            let error = AppError::NetworkError {
                url: url.to_owned(),
                status: Some(503),
            };
            state.record_error(&error.to_string());
            state.emit_snapshot("request failed, will retry");
            continue;
        }

        // Success on last attempt
        state.recovered = true;
        state.final_status = OperationStatus::Success;
        state.emit_snapshot("request succeeded after retries");
        return Ok(format!("Response from {}", url));
    }

    state.final_status = OperationStatus::Failed;
    Err(AppError::NetworkError {
        url: url.to_owned(),
        status: None,
    })
}

#[trace(name = "Fallback Strategy")]
fn with_fallback<T, F1, F2>(
    primary: F1,
    fallback: F2,
    state: &mut OperationState,
) -> Result<T, AppError>
where
    F1: FnOnce(&mut OperationState) -> Result<T, AppError>,
    F2: FnOnce(&mut OperationState) -> Result<T, AppError>,
{
    state.emit_snapshot("trying primary strategy");

    match primary(state) {
        Ok(result) => {
            state.emit_snapshot("primary succeeded");
            Ok(result)
        }
        Err(e) => {
            state.record_error(&e.to_string());
            state.emit_snapshot("primary failed, trying fallback");
            fallback(state)
        }
    }
}

// ============================================================================
// Main pipeline
// ============================================================================

#[trace(name = "Run Error Handling Examples")]
fn run_examples() {
    let mut db = MockDatabase::new();

    // Example 1: Successful operation
    println!("--- Example 1: Successful lookup ---");
    let mut state1 = OperationState::new("user_lookup");
    match find_user(&mut db, 1, &mut state1) {
        Ok(user) => println!("Found user: {}", user),
        Err(e) => println!("Error: {}", e),
    }

    // Example 2: Not found error
    println!("\n--- Example 2: Not found error ---");
    let mut state2 = OperationState::new("missing_user");
    match find_user(&mut db, 999, &mut state2) {
        Ok(user) => println!("Found user: {}", user),
        Err(e) => println!("Error: {}", e),
    }

    // Example 3: Error propagation
    println!("\n--- Example 3: Error propagation ---");
    let mut state3 = OperationState::new("post_with_author");
    match get_post_with_author(&mut db, 100, &mut state3) {
        Ok((author, content)) => println!("Post by {}: '{}'", author, content),
        Err(e) => println!("Error: {}", e),
    }

    // Example 4: Chain of validations
    println!("\n--- Example 4: Comment creation ---");
    let mut state4 = OperationState::new("create_comment");
    match create_comment(&mut db, 1, 100, "Great post!", &mut state4) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Example 5: Comment with missing post
    println!("\n--- Example 5: Comment on missing post ---");
    let mut state5 = OperationState::new("comment_missing_post");
    match create_comment(&mut db, 1, 999, "Comment", &mut state5) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Example 6: Option handling
    println!("\n--- Example 6: Config options ---");
    let mut state6 = OperationState::new("config");
    let config = load_config(&mut state6);

    match get_database_url(&config, &mut state6) {
        Ok(url) => println!("DB URL: {}", url),
        Err(e) => println!("Error: {}", e),
    }

    match get_api_key(&config, &mut state6) {
        Ok(key) => println!("API Key: {}", key),
        Err(e) => println!("Error: {} (expected)", e),
    }

    let timeout = get_timeout_or_default(&config, &mut state6);
    println!("Timeout: {}s", timeout);

    // Example 7: Retry pattern
    println!("\n--- Example 7: Retry pattern ---");
    let mut state7 = OperationState::new("fetch_with_retry");
    match fetch_with_retry("https://api.example.com/data", 3, &mut state7) {
        Ok(response) => println!("Got: {}", response),
        Err(e) => println!("Error: {}", e),
    }
    println!("Recovered: {}, Errors: {:?}", state7.recovered, state7.errors_encountered);

    // Example 8: Fallback strategy
    println!("\n--- Example 8: Fallback strategy ---");
    let mut state8 = OperationState::new("fallback");
    let result = with_fallback(
        |s| {
            s.emit_snapshot("primary: checking cache");
            Err(AppError::NotFound {
                resource: "cache".to_owned(),
                id: "key123".to_owned(),
            })
        },
        |s| {
            s.emit_snapshot("fallback: querying database");
            Ok("fallback_value".to_owned())
        },
        &mut state8,
    );
    println!("Fallback result: {:?}", result);

    println!("\nTotal database queries: {}", db.query_count);
}

fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file(
        "Error Handling Pipeline",
        "error_handling_session.json",
        run_examples,
    )?;

    println!("\nSession saved to error_handling_session.json");
    Ok(())
}

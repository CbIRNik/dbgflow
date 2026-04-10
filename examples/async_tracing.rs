use dbgflow::trace;

#[trace]
async fn child_task(name: &str) -> String {
    format!("done {}", name)
}

#[trace]
async fn main_task() {
    let a = child_task("A").await;
    let b = child_task("B").await;
    println!("results: {}, {}", a, b);
}

#[tokio::main]
async fn main() {
    dbgflow::init_session("Async Session");
    main_task().await;
    let _ = dbgflow::persist_session("async_session");
}

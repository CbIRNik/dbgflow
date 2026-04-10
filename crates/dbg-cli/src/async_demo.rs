use dbgflow_macros::trace;

#[trace(name = "Async Child Task")]
pub async fn child_task(name: &str, delay_ms: u64) -> String {
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    format!("child {} done", name)
}

#[trace(name = "Async Parent Pipeline")]
pub async fn parent_pipeline() {
    let t1 = tokio::spawn(child_task("A", 10));
    let t2 = tokio::spawn(child_task("B", 15));

    let _ = t1.await;
    let _ = t2.await;
}

pub fn run_async_demo() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(parent_pipeline());
}

import re
with open('crates/dbg-cli/src/lib.rs', 'r') as f:
    content = f.read()

async_code = """
    /// A simulated async task that waits and returns a string
    #[trace(name = "Async Child Task")]
    pub async fn async_child_task(name: &str, delay_ms: u64) -> String {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        format!("child {} done", name)
    }

    /// An async pipeline that spawns parallel tasks
    #[trace(name = "Async Parent Pipeline")]
    pub async fn async_parent_pipeline() {
        let t1 = tokio::spawn(async_child_task("NetworkRequest_A", 10));
        let t2 = tokio::spawn(async_child_task("DatabaseQuery_B", 15));
        let t3 = tokio::spawn(async_child_task("FileRead_C", 5));
        
        let _ = tokio::join!(t1, t2, t3);
    }
"""

# inject async code before `pub fn simulate_test_failure`
content = content.replace("    pub fn simulate_test_failure", async_code + "    pub fn simulate_test_failure")

# inject tokio runtime into `build_session`
build_session_code = """
        let mut review_state = PipelineState::review_sample();
        run_review_pipeline(&mut review_state);
        simulate_test_success();
        
        // Run async parallel tasks
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        rt.block_on(async_parent_pipeline());
        
        // Add a mock test outcome for the async pipeline
        super::runtime::record_test_started(
            "pipeline::async_parallel_execution",
            concat!(module_path!(), "::async_parent_pipeline"),
        );
        super::runtime::record_test_passed(
            "pipeline::async_parallel_execution",
            concat!(module_path!(), "::async_parent_pipeline"),
        );
"""

content = content.replace("""        let mut review_state = PipelineState::review_sample();
        run_review_pipeline(&mut review_state);
        simulate_test_success();""", build_session_code)

with open('crates/dbg-cli/src/lib.rs', 'w') as f:
    f.write(content)

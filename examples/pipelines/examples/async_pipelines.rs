use dbgflow::{trace, ui_debug};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[ui_debug(name = "Parallel Pipeline State")]
#[derive(Default, Clone)]
pub struct PipelineState {
    pub tasks_started: usize,
    pub tasks_completed: usize,
    pub data_fetched: Vec<String>,
}

#[trace(name = "Simulated Request")]
async fn fetch_data(state: Arc<Mutex<PipelineState>>, id: usize, delay: u64) -> String {
    {
        let mut st = state.lock().await;
        st.tasks_started += 1;
        st.emit_snapshot(format!("Starting task {}", id));
    }

    sleep(Duration::from_millis(delay)).await;

    let result = format!("Data from task {}", id);
    {
        let mut st = state.lock().await;
        st.tasks_completed += 1;
        st.data_fetched.push(result.clone());
        st.emit_snapshot(format!("Completed task {}", id));
    }
    
    result
}

#[trace(name = "Parallel Execution Controller")]
async fn run_parallel(state: Arc<Mutex<PipelineState>>) {
    let s1 = state.clone();
    let s2 = state.clone();
    let s3 = state.clone();

    let t1 = tokio::spawn(fetch_data(s1, 1, 30));
    let t2 = tokio::spawn(fetch_data(s2, 2, 50));
    let t3 = tokio::spawn(fetch_data(s3, 3, 20));

    let _ = tokio::join!(t1, t2, t3);
    
    {
        let mut st = state.lock().await;
        st.emit_snapshot("All parallel tasks finished".to_string());
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file("dbgflow demo: async pipeline", "artifacts/async-session.json", || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let state = Arc::new(Mutex::new(PipelineState::default()));
            run_parallel(state).await;
        });
    });
    println!("Session written to artifacts/async-session.json");
    Ok(())
}

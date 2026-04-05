//! Example: Loop Tracing Pipeline
//!
//! Demonstrates tracing different loop patterns with state changes,
//! including for loops, while loops, and iterators.

use dbgflow::prelude::*;

#[ui_debug(name = "Loop State")]
struct LoopState {
    iteration: usize,
    total_iterations: usize,
    accumulated_value: i64,
    items_processed: Vec<String>,
}

impl LoopState {
    fn new() -> Self {
        Self {
            iteration: 0,
            total_iterations: 0,
            accumulated_value: 0,
            items_processed: Vec::new(),
        }
    }
}

// ============================================================================
// For loop with range
// ============================================================================

#[trace(name = "Sum Range")]
fn sum_range(start: i64, end: i64, state: &mut LoopState) -> i64 {
    state.emit_snapshot(&format!("summing range {}..{}", start, end));

    for i in start..=end {
        state.iteration += 1;
        state.total_iterations += 1;
        state.accumulated_value += i;
        state.emit_snapshot(&format!("iter {}: added {}, sum = {}",
                                      state.iteration, i, state.accumulated_value));
    }

    state.emit_snapshot("range sum complete");
    state.accumulated_value
}

// ============================================================================
// While loop with condition
// ============================================================================

#[ui_debug(name = "Collatz State")]
struct CollatzState {
    current_value: u64,
    step_count: usize,
    sequence: Vec<u64>,
    max_value_seen: u64,
}

#[trace(name = "Collatz Sequence")]
fn collatz_sequence(start: u64, state: &mut CollatzState) -> usize {
    state.current_value = start;
    state.sequence.push(start);
    state.max_value_seen = start;
    state.emit_snapshot(&format!("starting collatz from {}", start));

    while state.current_value != 1 {
        state.step_count += 1;

        if state.current_value % 2 == 0 {
            state.current_value /= 2;
        } else {
            state.current_value = 3 * state.current_value + 1;
        }

        state.sequence.push(state.current_value);
        if state.current_value > state.max_value_seen {
            state.max_value_seen = state.current_value;
        }

        state.emit_snapshot(&format!("step {}: value = {}", state.step_count, state.current_value));
    }

    state.emit_snapshot(&format!("reached 1 in {} steps, max = {}", state.step_count, state.max_value_seen));
    state.step_count
}

// ============================================================================
// Iterator-based processing
// ============================================================================

#[ui_debug(name = "Filter State")]
struct FilterState {
    source_count: usize,
    filtered_count: usize,
    filter_criteria: String,
    results: Vec<i32>,
}

#[trace(name = "Filter and Transform")]
fn filter_and_transform(numbers: &[i32], threshold: i32, state: &mut FilterState) -> Vec<i32> {
    state.source_count = numbers.len();
    state.filter_criteria = format!("n > {} && n % 2 == 0", threshold);
    state.emit_snapshot("starting filter operation");

    let results: Vec<i32> = numbers
        .iter()
        .filter(|&&n| {
            let passes = n > threshold && n % 2 == 0;
            if passes {
                state.filtered_count += 1;
            }
            passes
        })
        .map(|&n| {
            let transformed = n * 2;
            state.results.push(transformed);
            transformed
        })
        .collect();

    state.emit_snapshot(&format!("filtered {} -> {} items", state.source_count, state.filtered_count));
    results
}

// ============================================================================
// Nested loops
// ============================================================================

#[ui_debug(name = "Matrix State")]
struct MatrixState {
    rows: usize,
    cols: usize,
    current_row: usize,
    current_col: usize,
    cells_processed: usize,
}

#[trace(name = "Process Matrix")]
fn process_matrix(rows: usize, cols: usize, state: &mut MatrixState) -> Vec<Vec<i32>> {
    state.rows = rows;
    state.cols = cols;
    state.emit_snapshot(&format!("creating {}x{} matrix", rows, cols));

    let mut matrix = Vec::with_capacity(rows);

    for row in 0..rows {
        state.current_row = row;
        let mut row_data = Vec::with_capacity(cols);

        for col in 0..cols {
            state.current_col = col;
            state.cells_processed += 1;

            let value = (row * cols + col) as i32;
            row_data.push(value);

            if state.cells_processed % 5 == 0 || (row == rows - 1 && col == cols - 1) {
                state.emit_snapshot(&format!("processed {} cells, current [{},{}] = {}",
                                              state.cells_processed, row, col, value));
            }
        }

        matrix.push(row_data);
    }

    state.emit_snapshot("matrix complete");
    matrix
}

// ============================================================================
// Loop with early exit
// ============================================================================

#[ui_debug(name = "Search State")]
struct SearchState {
    target: i32,
    elements_checked: usize,
    found_at_index: Option<usize>,
}

#[trace(name = "Linear Search")]
fn linear_search(haystack: &[i32], needle: i32, state: &mut SearchState) -> Option<usize> {
    state.target = needle;
    state.emit_snapshot(&format!("searching for {} in {} elements", needle, haystack.len()));

    for (idx, &value) in haystack.iter().enumerate() {
        state.elements_checked += 1;

        if value == needle {
            state.found_at_index = Some(idx);
            state.emit_snapshot(&format!("found {} at index {} after {} checks",
                                          needle, idx, state.elements_checked));
            return Some(idx);
        }

        if state.elements_checked % 10 == 0 {
            state.emit_snapshot(&format!("checked {} elements, not found yet", state.elements_checked));
        }
    }

    state.emit_snapshot(&format!("{} not found after {} checks", needle, state.elements_checked));
    None
}

// ============================================================================
// Loop with accumulator pattern
// ============================================================================

#[ui_debug(name = "Stats State")]
struct StatsState {
    count: usize,
    sum: f64,
    min: Option<f64>,
    max: Option<f64>,
    mean: f64,
}

#[trace(name = "Compute Statistics")]
fn compute_statistics(values: &[f64], state: &mut StatsState) -> f64 {
    state.emit_snapshot(&format!("computing stats for {} values", values.len()));

    for &value in values {
        state.count += 1;
        state.sum += value;

        state.min = Some(state.min.map_or(value, |m| m.min(value)));
        state.max = Some(state.max.map_or(value, |m| m.max(value)));
        state.mean = state.sum / state.count as f64;

        if state.count % 5 == 0 || state.count == values.len() {
            state.emit_snapshot(&format!("n={}, mean={:.2}, min={:.2}, max={:.2}",
                                          state.count, state.mean,
                                          state.min.unwrap_or(0.0),
                                          state.max.unwrap_or(0.0)));
        }
    }

    state.mean
}

// ============================================================================
// Main pipeline
// ============================================================================

#[trace(name = "Run Loop Examples")]
fn run_examples() {
    // For loop: sum range
    let mut loop_state = LoopState::new();
    let sum = sum_range(1, 10, &mut loop_state);
    println!("Sum of 1..=10 = {}", sum);

    // While loop: Collatz sequence
    let mut collatz_state = CollatzState {
        current_value: 0,
        step_count: 0,
        sequence: Vec::new(),
        max_value_seen: 0,
    };
    let steps = collatz_sequence(27, &mut collatz_state);
    println!("Collatz(27) took {} steps, max value = {}", steps, collatz_state.max_value_seen);

    // Iterator-based filtering
    let numbers: Vec<i32> = (1..=20).collect();
    let mut filter_state = FilterState {
        source_count: 0,
        filtered_count: 0,
        filter_criteria: String::new(),
        results: Vec::new(),
    };
    let filtered = filter_and_transform(&numbers, 5, &mut filter_state);
    println!("Filtered and transformed: {:?}", filtered);

    // Nested loops: matrix
    let mut matrix_state = MatrixState {
        rows: 0,
        cols: 0,
        current_row: 0,
        current_col: 0,
        cells_processed: 0,
    };
    let matrix = process_matrix(4, 5, &mut matrix_state);
    println!("Matrix: {} rows x {} cols", matrix.len(), matrix[0].len());

    // Loop with early exit
    let haystack: Vec<i32> = (0..100).collect();
    let mut search_state = SearchState {
        target: 0,
        elements_checked: 0,
        found_at_index: None,
    };
    let found = linear_search(&haystack, 42, &mut search_state);
    println!("Search result: {:?} (checked {} elements)", found, search_state.elements_checked);

    // Accumulator pattern
    let values: Vec<f64> = vec![1.5, 2.3, 4.7, 3.1, 5.9, 2.8, 6.4, 1.2, 3.8, 4.5];
    let mut stats_state = StatsState {
        count: 0,
        sum: 0.0,
        min: None,
        max: None,
        mean: 0.0,
    };
    let mean = compute_statistics(&values, &mut stats_state);
    println!("Mean = {:.2}, Min = {:.2}, Max = {:.2}",
             mean,
             stats_state.min.unwrap_or(0.0),
             stats_state.max.unwrap_or(0.0));
}

fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file(
        "Loop Examples Pipeline",
        "loops_session.json",
        run_examples,
    )?;

    println!("\nSession saved to loops_session.json");
    Ok(())
}

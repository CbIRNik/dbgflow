//! Example: Recursive Function Tracing
//!
//! Demonstrates tracing recursive algorithms including fibonacci,
//! factorial, and tree traversal patterns.

use dbgflow::prelude::*;

#[ui_debug(name = "Computation State")]
struct ComputationState {
    algorithm: String,
    call_depth: usize,
    max_depth_seen: usize,
    memo_hits: usize,
    total_calls: usize,
}

impl ComputationState {
    fn new(algorithm: &str) -> Self {
        Self {
            algorithm: algorithm.to_owned(),
            call_depth: 0,
            max_depth_seen: 0,
            memo_hits: 0,
            total_calls: 0,
        }
    }

    fn enter_call(&mut self) {
        self.call_depth += 1;
        self.total_calls += 1;
        if self.call_depth > self.max_depth_seen {
            self.max_depth_seen = self.call_depth;
        }
    }

    fn exit_call(&mut self) {
        self.call_depth -= 1;
    }
}

// ============================================================================
// Fibonacci with memoization
// ============================================================================

#[trace(name = "Fibonacci")]
fn fibonacci(n: u64, memo: &mut std::collections::HashMap<u64, u64>, state: &mut ComputationState) -> u64 {
    state.enter_call();
    state.emit_snapshot(&format!("computing fib({})", n));

    let result = if n <= 1 {
        n
    } else if let Some(&cached) = memo.get(&n) {
        state.memo_hits += 1;
        state.emit_snapshot(&format!("memo hit for fib({})", n));
        cached
    } else {
        let result = fibonacci(n - 1, memo, state) + fibonacci(n - 2, memo, state);
        memo.insert(n, result);
        result
    };

    state.emit_snapshot(&format!("fib({}) = {}", n, result));
    state.exit_call();
    result
}

// ============================================================================
// Factorial
// ============================================================================

#[trace(name = "Factorial")]
fn factorial(n: u64, state: &mut ComputationState) -> u64 {
    state.enter_call();
    state.emit_snapshot(&format!("computing {}!", n));

    let result = if n <= 1 {
        1
    } else {
        n * factorial(n - 1, state)
    };

    state.emit_snapshot(&format!("{}! = {}", n, result));
    state.exit_call();
    result
}

// ============================================================================
// Binary tree traversal
// ============================================================================

#[ui_debug(name = "Tree Node")]
struct TreeNode {
    value: i32,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
}

impl TreeNode {
    fn new(value: i32) -> Self {
        Self { value, left: None, right: None }
    }

    fn with_children(value: i32, left: TreeNode, right: TreeNode) -> Self {
        Self {
            value,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

#[trace(name = "Tree Sum")]
fn tree_sum(node: &TreeNode, state: &mut ComputationState) -> i32 {
    state.enter_call();
    state.emit_snapshot(&format!("visiting node with value {}", node.value));

    let left_sum = node.left.as_ref().map(|n| tree_sum(n, state)).unwrap_or(0);
    let right_sum = node.right.as_ref().map(|n| tree_sum(n, state)).unwrap_or(0);
    let total = node.value + left_sum + right_sum;

    state.emit_snapshot(&format!("subtree sum at {} = {}", node.value, total));
    state.exit_call();
    total
}

#[trace(name = "Tree Depth")]
fn tree_depth(node: &TreeNode, state: &mut ComputationState) -> usize {
    state.enter_call();
    state.emit_snapshot(&format!("measuring depth at node {}", node.value));

    let left_depth = node.left.as_ref().map(|n| tree_depth(n, state)).unwrap_or(0);
    let right_depth = node.right.as_ref().map(|n| tree_depth(n, state)).unwrap_or(0);
    let depth = 1 + left_depth.max(right_depth);

    state.emit_snapshot(&format!("depth at node {} = {}", node.value, depth));
    state.exit_call();
    depth
}

// ============================================================================
// Quick sort (recursive divide and conquer)
// ============================================================================

#[trace(name = "Quick Sort")]
fn quick_sort(arr: &mut [i32], state: &mut ComputationState) {
    state.enter_call();
    state.emit_snapshot(&format!("sorting slice of length {}", arr.len()));

    if arr.len() <= 1 {
        state.exit_call();
        return;
    }

    let pivot_idx = partition(arr, state);
    state.emit_snapshot(&format!("partitioned at index {}", pivot_idx));

    let (left, right) = arr.split_at_mut(pivot_idx);
    quick_sort(left, state);
    if !right.is_empty() {
        quick_sort(&mut right[1..], state);
    }

    state.exit_call();
}

#[trace(name = "Partition")]
fn partition(arr: &mut [i32], state: &mut ComputationState) -> usize {
    let pivot = arr[arr.len() - 1];
    state.emit_snapshot(&format!("pivot = {}", pivot));

    let mut i = 0;
    for j in 0..arr.len() - 1 {
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, arr.len() - 1);
    i
}

// ============================================================================
// Main pipeline
// ============================================================================

#[trace(name = "Run Recursion Examples")]
fn run_examples() {
    // Fibonacci example
    let mut fib_state = ComputationState::new("Fibonacci");
    let mut memo = std::collections::HashMap::new();
    let fib_result = fibonacci(10, &mut memo, &mut fib_state);
    println!("Fibonacci(10) = {} (calls: {}, memo hits: {})",
             fib_result, fib_state.total_calls, fib_state.memo_hits);

    // Factorial example
    let mut fact_state = ComputationState::new("Factorial");
    let fact_result = factorial(6, &mut fact_state);
    println!("Factorial(6) = {} (calls: {}, max depth: {})",
             fact_result, fact_state.total_calls, fact_state.max_depth_seen);

    // Tree traversal example
    let tree = TreeNode::with_children(
        10,
        TreeNode::with_children(
            5,
            TreeNode::new(3),
            TreeNode::new(7),
        ),
        TreeNode::with_children(
            15,
            TreeNode::new(12),
            TreeNode::new(20),
        ),
    );

    let mut tree_state = ComputationState::new("Tree Traversal");
    let sum = tree_sum(&tree, &mut tree_state);
    println!("Tree sum = {} (nodes visited: {})", sum, tree_state.total_calls);

    let mut depth_state = ComputationState::new("Tree Depth");
    let depth = tree_depth(&tree, &mut depth_state);
    println!("Tree depth = {}", depth);

    // Quick sort example
    let mut arr = vec![64, 34, 25, 12, 22, 11, 90];
    let mut sort_state = ComputationState::new("Quick Sort");
    println!("Before sort: {:?}", arr);
    quick_sort(&mut arr, &mut sort_state);
    println!("After sort: {:?} (recursive calls: {})", arr, sort_state.total_calls);
}

fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file(
        "Recursion Examples Pipeline",
        "recursion_session.json",
        run_examples,
    )?;

    println!("\nSession saved to recursion_session.json");
    Ok(())
}

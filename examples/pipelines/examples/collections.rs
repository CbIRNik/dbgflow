//! Example: Collections Pipeline
//!
//! Demonstrates tracing operations on various Rust collections
//! including Vec, HashMap, HashSet, and BTreeMap.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use dbgflow::prelude::*;

// ============================================================================
// State tracking
// ============================================================================

#[ui_debug(name = "Collection Stats")]
struct CollectionStats {
    operation: String,
    items_before: usize,
    items_after: usize,
    items_added: usize,
    items_removed: usize,
}

impl CollectionStats {
    fn new(op: &str) -> Self {
        Self {
            operation: op.to_owned(),
            items_before: 0,
            items_after: 0,
            items_added: 0,
            items_removed: 0,
        }
    }
}

// ============================================================================
// Vec operations
// ============================================================================

#[ui_debug(name = "Vec State")]
struct VecState {
    elements: Vec<i32>,
    capacity: usize,
    operations_count: usize,
}

#[trace(name = "Vec Push")]
fn vec_push_many(state: &mut VecState, values: &[i32], stats: &mut CollectionStats) {
    stats.items_before = state.elements.len();
    state.emit_snapshot(&format!("pushing {} items", values.len()));

    for &value in values {
        state.elements.push(value);
        state.operations_count += 1;
        stats.items_added += 1;
    }

    state.capacity = state.elements.capacity();
    stats.items_after = state.elements.len();
    state.emit_snapshot(&format!("vec now has {} items, capacity {}", state.elements.len(), state.capacity));
}

#[trace(name = "Vec Filter")]
fn vec_filter_in_place(state: &mut VecState, predicate: impl Fn(&i32) -> bool, stats: &mut CollectionStats) {
    stats.items_before = state.elements.len();
    state.emit_snapshot("filtering elements");

    state.elements.retain(|x| {
        let keep = predicate(x);
        if !keep {
            stats.items_removed += 1;
        }
        keep
    });
    state.operations_count += 1;

    stats.items_after = state.elements.len();
    state.emit_snapshot(&format!("filtered to {} items", state.elements.len()));
}

#[trace(name = "Vec Transform")]
fn vec_transform(state: &mut VecState, transform: impl Fn(i32) -> i32, stats: &mut CollectionStats) {
    stats.items_before = state.elements.len();
    state.emit_snapshot("transforming elements");

    state.elements = state.elements.iter().map(|&x| transform(x)).collect();
    state.operations_count += 1;

    stats.items_after = state.elements.len();
    state.emit_snapshot(&format!("transformed {} items", state.elements.len()));
}

// ============================================================================
// HashMap operations
// ============================================================================

#[ui_debug(name = "Word Counter")]
struct WordCounter {
    counts: HashMap<String, usize>,
    total_words: usize,
    unique_words: usize,
}

#[trace(name = "Count Words")]
fn count_words(text: &str, counter: &mut WordCounter, stats: &mut CollectionStats) {
    stats.items_before = counter.counts.len();
    counter.emit_snapshot("starting word count");

    for word in text.split_whitespace() {
        let normalized = word.to_lowercase().chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>();

        if !normalized.is_empty() {
            *counter.counts.entry(normalized).or_insert(0) += 1;
            counter.total_words += 1;
        }
    }

    counter.unique_words = counter.counts.len();
    stats.items_after = counter.counts.len();
    stats.items_added = counter.counts.len() - stats.items_before;

    counter.emit_snapshot(&format!("{} total, {} unique", counter.total_words, counter.unique_words));
}

#[trace(name = "Get Top Words")]
fn get_top_words(counter: &WordCounter, n: usize, stats: &mut CollectionStats) -> Vec<(String, usize)> {
    counter.emit_snapshot(&format!("finding top {} words", n));

    let mut pairs: Vec<_> = counter.counts.iter()
        .map(|(k, &v)| (k.clone(), v))
        .collect();

    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    let top: Vec<_> = pairs.into_iter().take(n).collect();

    stats.items_after = top.len();
    counter.emit_snapshot(&format!("top {}: {:?}", n, top));
    top
}

// ============================================================================
// HashSet operations
// ============================================================================

#[ui_debug(name = "Set Operations")]
struct SetState {
    set_a: HashSet<i32>,
    set_b: HashSet<i32>,
    result: HashSet<i32>,
}

#[trace(name = "Set Union")]
fn set_union(state: &mut SetState, stats: &mut CollectionStats) {
    stats.items_before = state.set_a.len() + state.set_b.len();
    state.emit_snapshot(&format!("union of {} and {} elements", state.set_a.len(), state.set_b.len()));

    state.result = state.set_a.union(&state.set_b).cloned().collect();

    stats.items_after = state.result.len();
    state.emit_snapshot(&format!("union has {} elements", state.result.len()));
}

#[trace(name = "Set Intersection")]
fn set_intersection(state: &mut SetState, stats: &mut CollectionStats) {
    stats.items_before = state.set_a.len() + state.set_b.len();
    state.emit_snapshot("computing intersection");

    state.result = state.set_a.intersection(&state.set_b).cloned().collect();

    stats.items_after = state.result.len();
    state.emit_snapshot(&format!("intersection has {} elements", state.result.len()));
}

#[trace(name = "Set Difference")]
fn set_difference(state: &mut SetState, stats: &mut CollectionStats) {
    stats.items_before = state.set_a.len();
    state.emit_snapshot("computing A - B");

    state.result = state.set_a.difference(&state.set_b).cloned().collect();

    stats.items_after = state.result.len();
    state.emit_snapshot(&format!("difference has {} elements", state.result.len()));
}

// ============================================================================
// BTreeMap operations (sorted map)
// ============================================================================

#[ui_debug(name = "Sorted Scores")]
struct SortedScores {
    scores: BTreeMap<String, i32>,
    min_score: Option<i32>,
    max_score: Option<i32>,
}

#[trace(name = "Add Scores")]
fn add_scores(state: &mut SortedScores, entries: &[(&str, i32)], stats: &mut CollectionStats) {
    stats.items_before = state.scores.len();
    state.emit_snapshot(&format!("adding {} scores", entries.len()));

    for (name, score) in entries {
        state.scores.insert(name.to_string(), *score);
        stats.items_added += 1;

        state.min_score = Some(state.min_score.map_or(*score, |m| m.min(*score)));
        state.max_score = Some(state.max_score.map_or(*score, |m| m.max(*score)));
    }

    stats.items_after = state.scores.len();
    state.emit_snapshot(&format!("now {} scores, range {:?}..{:?}",
                                  state.scores.len(), state.min_score, state.max_score));
}

#[trace(name = "Get Score Range")]
fn get_score_range(state: &SortedScores, min: i32, max: i32, stats: &mut CollectionStats) -> Vec<(String, i32)> {
    state.emit_snapshot(&format!("finding scores in range {}..={}", min, max));

    let result: Vec<_> = state.scores.iter()
        .filter(|(_, score)| **score >= min && **score <= max)
        .map(|(name, score)| (name.clone(), *score))
        .collect();

    stats.items_after = result.len();
    state.emit_snapshot(&format!("found {} scores in range", result.len()));
    result
}

// ============================================================================
// VecDeque operations (double-ended queue)
// ============================================================================

#[ui_debug(name = "Task Queue")]
struct TaskQueue {
    queue: VecDeque<String>,
    processed_count: usize,
    high_priority_count: usize,
}

#[trace(name = "Add Task")]
fn add_task(queue: &mut TaskQueue, task: &str, high_priority: bool, stats: &mut CollectionStats) {
    stats.items_before = queue.queue.len();
    queue.emit_snapshot(&format!("adding task: {} (priority: {})", task, if high_priority { "high" } else { "normal" }));

    if high_priority {
        queue.queue.push_front(task.to_owned());
        queue.high_priority_count += 1;
    } else {
        queue.queue.push_back(task.to_owned());
    }

    stats.items_added = 1;
    stats.items_after = queue.queue.len();
    queue.emit_snapshot(&format!("queue size: {}", queue.queue.len()));
}

#[trace(name = "Process Tasks")]
fn process_tasks(queue: &mut TaskQueue, max_tasks: usize, stats: &mut CollectionStats) -> Vec<String> {
    stats.items_before = queue.queue.len();
    queue.emit_snapshot(&format!("processing up to {} tasks", max_tasks));

    let mut processed = Vec::new();
    while processed.len() < max_tasks && !queue.queue.is_empty() {
        if let Some(task) = queue.queue.pop_front() {
            queue.processed_count += 1;
            processed.push(task);
            stats.items_removed += 1;
        }
    }

    stats.items_after = queue.queue.len();
    queue.emit_snapshot(&format!("processed {}, {} remaining", processed.len(), queue.queue.len()));
    processed
}

// ============================================================================
// Main pipeline
// ============================================================================

#[trace(name = "Run Collections Examples")]
fn run_examples() {
    // Vec operations
    println!("--- Vec Operations ---");
    let mut vec_state = VecState {
        elements: Vec::new(),
        capacity: 0,
        operations_count: 0,
    };
    let mut vec_stats = CollectionStats::new("vec_ops");

    vec_push_many(&mut vec_state, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], &mut vec_stats);
    println!("After push: {:?}", vec_state.elements);

    vec_filter_in_place(&mut vec_state, |&x| x % 2 == 0, &mut vec_stats);
    println!("After filter (evens): {:?}", vec_state.elements);

    vec_transform(&mut vec_state, |x| x * x, &mut vec_stats);
    println!("After transform (squared): {:?}", vec_state.elements);

    // HashMap (word counting)
    println!("\n--- HashMap Operations ---");
    let mut counter = WordCounter {
        counts: HashMap::new(),
        total_words: 0,
        unique_words: 0,
    };
    let mut word_stats = CollectionStats::new("word_count");

    let text = "The quick brown fox jumps over the lazy dog. The dog was not amused.";
    count_words(text, &mut counter, &mut word_stats);

    let top = get_top_words(&counter, 5, &mut word_stats);
    println!("Top words: {:?}", top);

    // HashSet operations
    println!("\n--- HashSet Operations ---");
    let mut set_state = SetState {
        set_a: [1, 2, 3, 4, 5].into_iter().collect(),
        set_b: [4, 5, 6, 7, 8].into_iter().collect(),
        result: HashSet::new(),
    };

    let mut union_stats = CollectionStats::new("union");
    set_union(&mut set_state, &mut union_stats);
    println!("Union: {:?}", set_state.result);

    let mut intersect_stats = CollectionStats::new("intersection");
    set_intersection(&mut set_state, &mut intersect_stats);
    println!("Intersection: {:?}", set_state.result);

    let mut diff_stats = CollectionStats::new("difference");
    set_difference(&mut set_state, &mut diff_stats);
    println!("Difference (A-B): {:?}", set_state.result);

    // BTreeMap (sorted scores)
    println!("\n--- BTreeMap Operations ---");
    let mut scores = SortedScores {
        scores: BTreeMap::new(),
        min_score: None,
        max_score: None,
    };
    let mut score_stats = CollectionStats::new("scores");

    add_scores(&mut scores, &[
        ("Alice", 95),
        ("Bob", 87),
        ("Charlie", 92),
        ("Diana", 78),
        ("Eve", 88),
    ], &mut score_stats);

    let high_scorers = get_score_range(&scores, 85, 100, &mut score_stats);
    println!("High scorers (85-100): {:?}", high_scorers);

    // VecDeque (task queue)
    println!("\n--- VecDeque Operations ---");
    let mut queue = TaskQueue {
        queue: VecDeque::new(),
        processed_count: 0,
        high_priority_count: 0,
    };
    let mut queue_stats = CollectionStats::new("queue");

    add_task(&mut queue, "Normal task 1", false, &mut queue_stats);
    add_task(&mut queue, "Normal task 2", false, &mut queue_stats);
    add_task(&mut queue, "URGENT: Fix bug", true, &mut queue_stats);
    add_task(&mut queue, "Normal task 3", false, &mut queue_stats);
    add_task(&mut queue, "URGENT: Deploy", true, &mut queue_stats);

    println!("Queue: {:?}", queue.queue);

    let processed = process_tasks(&mut queue, 3, &mut queue_stats);
    println!("Processed: {:?}", processed);
    println!("Remaining: {:?}", queue.queue);
}

fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file(
        "Collections Pipeline",
        "collections_session.json",
        run_examples,
    )?;

    println!("\nSession saved to collections_session.json");
    Ok(())
}

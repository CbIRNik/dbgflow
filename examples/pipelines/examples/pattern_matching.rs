//! Example: Pattern Matching Pipeline
//!
//! Demonstrates tracing pattern matching scenarios including
//! match expressions, if let, while let, and destructuring.

use dbgflow::prelude::*;

// ============================================================================
// Enum patterns
// ============================================================================

#[ui_debug(name = "Message")]
enum Message {
    Text { from: String, content: String },
    Image { from: String, url: String, size_kb: u32 },
    Video { from: String, url: String, duration_secs: u32 },
    Voice { from: String, duration_secs: u32 },
    Location { from: String, lat: f64, lon: f64 },
    Reaction { from: String, emoji: String, target_id: u64 },
}

#[ui_debug(name = "Processing State")]
struct ProcessingState {
    messages_processed: usize,
    text_count: usize,
    media_count: usize,
    other_count: usize,
    total_media_kb: u32,
}

impl ProcessingState {
    fn new() -> Self {
        Self {
            messages_processed: 0,
            text_count: 0,
            media_count: 0,
            other_count: 0,
            total_media_kb: 0,
        }
    }
}

#[trace(name = "Process Message")]
fn process_message(msg: &Message, state: &mut ProcessingState) -> String {
    state.messages_processed += 1;
    state.emit_snapshot(&format!("processing message #{}", state.messages_processed));

    let result = match msg {
        Message::Text { from, content } => {
            state.text_count += 1;
            state.emit_snapshot(&format!("text from {}: '{}'", from, content));
            format!("[TEXT] {}: {}", from, content)
        }

        Message::Image { from, url, size_kb } => {
            state.media_count += 1;
            state.total_media_kb += size_kb;
            state.emit_snapshot(&format!("image from {} ({}KB)", from, size_kb));
            format!("[IMAGE] {}: {} ({}KB)", from, url, size_kb)
        }

        Message::Video { from, url, duration_secs } => {
            state.media_count += 1;
            state.emit_snapshot(&format!("video from {} ({}s)", from, duration_secs));
            format!("[VIDEO] {}: {} ({}s)", from, url, duration_secs)
        }

        Message::Voice { from, duration_secs } => {
            state.media_count += 1;
            state.emit_snapshot(&format!("voice from {} ({}s)", from, duration_secs));
            format!("[VOICE] {}: {}s", from, duration_secs)
        }

        Message::Location { from, lat, lon } => {
            state.other_count += 1;
            state.emit_snapshot(&format!("location from {}: ({}, {})", from, lat, lon));
            format!("[LOCATION] {}: ({}, {})", from, lat, lon)
        }

        Message::Reaction { from, emoji, target_id } => {
            state.other_count += 1;
            state.emit_snapshot(&format!("reaction from {}: {} on #{}", from, emoji, target_id));
            format!("[REACTION] {}: {} on #{}", from, emoji, target_id)
        }
    };

    state.emit_snapshot(&format!("processed: {}", result));
    result
}

// ============================================================================
// Match guards
// ============================================================================

#[ui_debug(name = "Command")]
enum Command {
    Move { x: i32, y: i32 },
    Scale { factor: f64 },
    Rotate { degrees: f64 },
    SetColor { r: u8, g: u8, b: u8 },
}

#[ui_debug(name = "Transform State")]
struct TransformState {
    position: (i32, i32),
    scale: f64,
    rotation: f64,
    color: (u8, u8, u8),
    commands_applied: usize,
    commands_rejected: usize,
}

#[trace(name = "Apply Command")]
fn apply_command(cmd: &Command, state: &mut TransformState) -> bool {
    state.emit_snapshot(&format!("applying command #{}", state.commands_applied + 1));

    let applied = match cmd {
        // Guard: only allow moves within bounds
        Command::Move { x, y } if *x >= -100 && *x <= 100 && *y >= -100 && *y <= 100 => {
            state.position = (*x, *y);
            state.emit_snapshot(&format!("moved to ({}, {})", x, y));
            true
        }

        Command::Move { x, y } => {
            state.emit_snapshot(&format!("rejected move to ({}, {}) - out of bounds", x, y));
            false
        }

        // Guard: scale must be positive and reasonable
        Command::Scale { factor } if *factor > 0.0 && *factor <= 10.0 => {
            state.scale *= factor;
            state.emit_snapshot(&format!("scaled by {}, now {}", factor, state.scale));
            true
        }

        Command::Scale { factor } => {
            state.emit_snapshot(&format!("rejected scale {} - invalid factor", factor));
            false
        }

        // Guard: normalize rotation to 0-360
        Command::Rotate { degrees } if *degrees >= -360.0 && *degrees <= 360.0 => {
            state.rotation = (state.rotation + degrees) % 360.0;
            if state.rotation < 0.0 {
                state.rotation += 360.0;
            }
            state.emit_snapshot(&format!("rotated by {}, now {}", degrees, state.rotation));
            true
        }

        Command::Rotate { degrees } => {
            state.emit_snapshot(&format!("rejected rotation {} - too extreme", degrees));
            false
        }

        Command::SetColor { r, g, b } => {
            state.color = (*r, *g, *b);
            state.emit_snapshot(&format!("color set to rgb({}, {}, {})", r, g, b));
            true
        }
    };

    if applied {
        state.commands_applied += 1;
    } else {
        state.commands_rejected += 1;
    }

    applied
}

// ============================================================================
// Tuple and struct destructuring
// ============================================================================

#[ui_debug(name = "Point")]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

#[ui_debug(name = "Geometry State")]
struct GeometryState {
    points_processed: usize,
    in_bounds_count: usize,
    quadrant_counts: [usize; 4],
}

#[trace(name = "Classify Point")]
fn classify_point(point: &Point, state: &mut GeometryState) -> String {
    state.points_processed += 1;
    state.emit_snapshot(&format!("classifying point ({}, {}, {})", point.x, point.y, point.z));

    // Destructure and classify by quadrant (2D projection)
    let Point { x, y, z: _ } = point;

    let classification = match (*x >= 0.0, *y >= 0.0) {
        (true, true) => {
            state.quadrant_counts[0] += 1;
            state.emit_snapshot("quadrant I (x+, y+)");
            "Quadrant I"
        }
        (false, true) => {
            state.quadrant_counts[1] += 1;
            state.emit_snapshot("quadrant II (x-, y+)");
            "Quadrant II"
        }
        (false, false) => {
            state.quadrant_counts[2] += 1;
            state.emit_snapshot("quadrant III (x-, y-)");
            "Quadrant III"
        }
        (true, false) => {
            state.quadrant_counts[3] += 1;
            state.emit_snapshot("quadrant IV (x+, y-)");
            "Quadrant IV"
        }
    };

    // Check bounds
    if point.x.abs() <= 10.0 && point.y.abs() <= 10.0 {
        state.in_bounds_count += 1;
        state.emit_snapshot("point is within bounds");
    }

    classification.to_owned()
}

// ============================================================================
// if let and while let patterns
// ============================================================================

#[ui_debug(name = "Optional Chain")]
struct OptionalChain {
    values: Vec<Option<i32>>,
    current_index: usize,
    sum: i32,
    some_count: usize,
    none_count: usize,
}

#[trace(name = "Sum Optional Values")]
fn sum_optional_values(chain: &mut OptionalChain) -> i32 {
    chain.emit_snapshot(&format!("processing {} optional values", chain.values.len()));

    for (idx, opt) in chain.values.iter().enumerate() {
        chain.current_index = idx;

        if let Some(value) = opt {
            chain.sum += value;
            chain.some_count += 1;
            chain.emit_snapshot(&format!("[{}] Some({}) -> sum = {}", idx, value, chain.sum));
        } else {
            chain.none_count += 1;
            chain.emit_snapshot(&format!("[{}] None (skipped)", idx));
        }
    }

    chain.emit_snapshot(&format!("final sum: {}, {} Some, {} None",
                                  chain.sum, chain.some_count, chain.none_count));
    chain.sum
}

#[ui_debug(name = "Iterator State")]
struct IteratorState {
    items: Vec<i32>,
    current: Option<i32>,
    iterations: usize,
}

#[trace(name = "Process With While Let")]
fn process_with_while_let(state: &mut IteratorState) -> Vec<i32> {
    state.emit_snapshot(&format!("processing {} items with while let", state.items.len()));

    let mut results = Vec::new();
    let mut iter = state.items.iter();

    while let Some(&value) = iter.next() {
        state.current = Some(value);
        state.iterations += 1;

        let processed = value * 2;
        results.push(processed);

        state.emit_snapshot(&format!("iteration {}: {} -> {}", state.iterations, value, processed));
    }

    state.current = None;
    state.emit_snapshot(&format!("completed {} iterations", state.iterations));
    results
}

// ============================================================================
// Nested pattern matching
// ============================================================================

#[ui_debug(name = "Nested Data")]
enum NestedData {
    Empty,
    Single(i32),
    Pair(i32, i32),
    Triple(i32, i32, i32),
    List(Vec<i32>),
    Named { key: String, value: i32 },
}

#[ui_debug(name = "Nested State")]
struct NestedState {
    patterns_matched: Vec<String>,
    total_values: i32,
}

#[trace(name = "Match Nested")]
fn match_nested(data: &NestedData, state: &mut NestedState) -> String {
    state.emit_snapshot("matching nested pattern");

    let result = match data {
        NestedData::Empty => {
            state.patterns_matched.push("Empty".to_owned());
            state.emit_snapshot("matched Empty");
            "empty".to_owned()
        }

        NestedData::Single(n) => {
            state.patterns_matched.push(format!("Single({})", n));
            state.total_values += n;
            state.emit_snapshot(&format!("matched Single({})", n));
            format!("single: {}", n)
        }

        NestedData::Pair(a, b) => {
            state.patterns_matched.push(format!("Pair({}, {})", a, b));
            state.total_values += a + b;
            state.emit_snapshot(&format!("matched Pair({}, {})", a, b));
            format!("pair: {} + {} = {}", a, b, a + b)
        }

        NestedData::Triple(a, b, c) => {
            state.patterns_matched.push(format!("Triple({}, {}, {})", a, b, c));
            state.total_values += a + b + c;
            state.emit_snapshot(&format!("matched Triple({}, {}, {})", a, b, c));
            format!("triple: {} + {} + {} = {}", a, b, c, a + b + c)
        }

        NestedData::List(items) if items.is_empty() => {
            state.patterns_matched.push("List([])".to_owned());
            state.emit_snapshot("matched empty List");
            "empty list".to_owned()
        }

        NestedData::List(items) if items.len() == 1 => {
            let first = items[0];
            state.patterns_matched.push(format!("List([{}])", first));
            state.total_values += first;
            state.emit_snapshot(&format!("matched singleton List([{}])", first));
            format!("singleton list: [{}]", first)
        }

        NestedData::List(items) => {
            let sum: i32 = items.iter().sum();
            state.patterns_matched.push(format!("List({} items)", items.len()));
            state.total_values += sum;
            state.emit_snapshot(&format!("matched List with {} items, sum = {}", items.len(), sum));
            format!("list of {}: sum = {}", items.len(), sum)
        }

        NestedData::Named { key, value } => {
            state.patterns_matched.push(format!("Named({}: {})", key, value));
            state.total_values += value;
            state.emit_snapshot(&format!("matched Named {{ {}: {} }}", key, value));
            format!("named: {} = {}", key, value)
        }
    };

    state.emit_snapshot(&format!("result: {}", result));
    result
}

// ============================================================================
// Main pipeline
// ============================================================================

#[trace(name = "Run Pattern Matching Examples")]
fn run_examples() {
    // Message processing
    println!("--- Message Processing ---");
    let messages = vec![
        Message::Text { from: "Alice".to_owned(), content: "Hello!".to_owned() },
        Message::Image { from: "Bob".to_owned(), url: "img.jpg".to_owned(), size_kb: 256 },
        Message::Video { from: "Charlie".to_owned(), url: "vid.mp4".to_owned(), duration_secs: 30 },
        Message::Location { from: "Diana".to_owned(), lat: 37.7749, lon: -122.4194 },
        Message::Reaction { from: "Eve".to_owned(), emoji: "👍".to_owned(), target_id: 1 },
    ];

    let mut proc_state = ProcessingState::new();
    for msg in &messages {
        let result = process_message(msg, &mut proc_state);
        println!("{}", result);
    }
    println!("Stats: {} text, {} media, {} other",
             proc_state.text_count, proc_state.media_count, proc_state.other_count);

    // Commands with guards
    println!("\n--- Commands with Guards ---");
    let commands = vec![
        Command::Move { x: 50, y: 50 },
        Command::Move { x: 200, y: 200 },  // Should be rejected
        Command::Scale { factor: 2.0 },
        Command::Scale { factor: -1.0 },   // Should be rejected
        Command::Rotate { degrees: 45.0 },
        Command::SetColor { r: 255, g: 128, b: 64 },
    ];

    let mut transform_state = TransformState {
        position: (0, 0),
        scale: 1.0,
        rotation: 0.0,
        color: (0, 0, 0),
        commands_applied: 0,
        commands_rejected: 0,
    };

    for cmd in &commands {
        let applied = apply_command(cmd, &mut transform_state);
        println!("Command applied: {}", applied);
    }
    println!("Final: pos={:?}, scale={}, rotation={}, color={:?}",
             transform_state.position, transform_state.scale,
             transform_state.rotation, transform_state.color);

    // Point classification
    println!("\n--- Point Classification ---");
    let points = vec![
        Point { x: 5.0, y: 5.0, z: 0.0 },
        Point { x: -3.0, y: 7.0, z: 1.0 },
        Point { x: -8.0, y: -4.0, z: 2.0 },
        Point { x: 15.0, y: -2.0, z: 3.0 },
    ];

    let mut geo_state = GeometryState {
        points_processed: 0,
        in_bounds_count: 0,
        quadrant_counts: [0; 4],
    };

    for point in &points {
        let quadrant = classify_point(point, &mut geo_state);
        println!("({}, {}) -> {}", point.x, point.y, quadrant);
    }
    println!("Quadrant counts: {:?}", geo_state.quadrant_counts);

    // Optional chain
    println!("\n--- Optional Chain ---");
    let mut chain = OptionalChain {
        values: vec![Some(10), None, Some(20), Some(30), None, Some(40)],
        current_index: 0,
        sum: 0,
        some_count: 0,
        none_count: 0,
    };
    let total = sum_optional_values(&mut chain);
    println!("Total: {}", total);

    // While let
    println!("\n--- While Let ---");
    let mut iter_state = IteratorState {
        items: vec![1, 2, 3, 4, 5],
        current: None,
        iterations: 0,
    };
    let doubled = process_with_while_let(&mut iter_state);
    println!("Doubled: {:?}", doubled);

    // Nested patterns
    println!("\n--- Nested Patterns ---");
    let nested_data = vec![
        NestedData::Empty,
        NestedData::Single(42),
        NestedData::Pair(10, 20),
        NestedData::Triple(1, 2, 3),
        NestedData::List(vec![]),
        NestedData::List(vec![100]),
        NestedData::List(vec![1, 2, 3, 4, 5]),
        NestedData::Named { key: "answer".to_owned(), value: 42 },
    ];

    let mut nested_state = NestedState {
        patterns_matched: Vec::new(),
        total_values: 0,
    };

    for data in &nested_data {
        let result = match_nested(data, &mut nested_state);
        println!("{}", result);
    }
    println!("Total values: {}", nested_state.total_values);
    println!("Patterns matched: {:?}", nested_state.patterns_matched);
}

fn main() -> std::io::Result<()> {
    dbgflow::capture_to_file(
        "Pattern Matching Pipeline",
        "pattern_matching_session.json",
        run_examples,
    )?;

    println!("\nSession saved to pattern_matching_session.json");
    Ok(())
}

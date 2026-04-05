use dbgflow::prelude::*;

#[ui_debug(name = "Counter State")]
pub struct Counter {
    pub value: i32,
}

#[trace(name = "Increment Counter")]
pub fn increment(counter: &mut Counter) {
    counter.value += 1;
    counter.emit_snapshot("increment");
}

#[trace(name = "Classify Counter")]
pub fn classify(counter: &Counter) -> &'static str {
    if counter.value >= 2 {
        "ready"
    } else {
        "warming-up"
    }
}

#[trace(name = "Run Counter Pipeline")]
pub fn run_counter_pipeline(counter: &mut Counter) -> &'static str {
    increment(counter);
    classify(counter)
}

#[trace(name = "Run Review Pipeline")]
pub fn run_review_pipeline(counter: &mut Counter) -> &'static str {
    increment(counter);
    increment(counter);
    classify(counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[dbg_test]
    fn counter_pipeline_passes() {
        let mut counter = Counter { value: 1 };
        assert_eq!(run_counter_pipeline(&mut counter), "ready");
    }

    #[dbg_test]
    fn counter_pipeline_fails() {
        let mut counter = Counter { value: 0 };
        assert_eq!(run_counter_pipeline(&mut counter), "ready");
    }

    #[dbg_test]
    fn review_pipeline_passes() {
        let mut counter = Counter { value: 0 };
        assert_eq!(run_review_pipeline(&mut counter), "ready");
    }
}

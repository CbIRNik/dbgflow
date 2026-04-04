use dbg::prelude::*;

#[ui_debug]
pub struct Counter {
    pub value: i32,
}

#[trace]
pub fn increment(counter: &mut Counter) {
    counter.value += 1;
    counter.emit_snapshot("increment");
}

#[trace]
pub fn classify(counter: &Counter) -> &'static str {
    if counter.value >= 2 {
        "ready"
    } else {
        "warming-up"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[dbg_test]
    fn counter_pipeline_passes() {
        let mut counter = Counter { value: 1 };
        increment(&mut counter);
        assert_eq!(classify(&counter), "ready");
    }

    #[dbg_test]
    fn counter_pipeline_fails() {
        let mut counter = Counter { value: 0 };
        increment(&mut counter);
        assert_eq!(classify(&counter), "ready");
    }
}

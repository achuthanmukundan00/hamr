//! Port of `packages/coding-agent/src/core/timings.ts`.
//!
//! Central timing instrumentation for startup profiling.
//! Enable with `HAMR_TIMING=1` (or `PI_TIMING=1`) environment variable.

use std::sync::Mutex;
use std::time::Instant;

fn enabled() -> bool {
    std::env::var("HAMR_TIMING").as_deref() == Ok("1")
        || std::env::var("PI_TIMING").as_deref() == Ok("1")
}

struct State {
    timings: Vec<(String, u128)>,
    last_time: Instant,
}

static STATE: std::sync::LazyLock<Mutex<State>> = std::sync::LazyLock::new(|| {
    Mutex::new(State {
        timings: Vec::new(),
        last_time: Instant::now(),
    })
});

/// Reset all accumulated timings and restart the clock.
pub fn reset_timings() {
    if !enabled() {
        return;
    }
    let mut state = STATE.lock().unwrap();
    state.timings.clear();
    state.last_time = Instant::now();
}

/// Record elapsed time since the last call to reset_timings or time.
/// The label describes what just happened / was measured.
pub fn time(label: &str) {
    if !enabled() {
        return;
    }
    let mut state = STATE.lock().unwrap();
    let now = Instant::now();
    let elapsed = now.duration_since(state.last_time);
    state.timings.push((label.to_string(), elapsed.as_millis()));
    state.last_time = now;
}

/// Print all recorded timings to stderr (only when enabled and non-empty).
pub fn print_timings() {
    if !enabled() {
        return;
    }
    let state = STATE.lock().unwrap();
    if state.timings.is_empty() {
        return;
    }
    eprintln!("\n--- Startup Timings ---");
    for (label, ms) in &state.timings {
        eprintln!("  {}: {}ms", label, ms);
    }
    let total: u128 = state.timings.iter().map(|(_, ms)| ms).sum();
    eprintln!("  TOTAL: {}ms", total);
    eprintln!("------------------------\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timings_disabled_by_default() {
        // Safe-guard: clear any env override from outside
        // (we can't easily unset in-process, but the default path has no side effects)
        let prev_hamr = std::env::var("HAMR_TIMING").ok();
        let prev_pi = std::env::var("PI_TIMING").ok();
        unsafe {
            std::env::remove_var("HAMR_TIMING");
            std::env::remove_var("PI_TIMING");
        }

        // reset and time should be no-ops
        reset_timings();
        time("step1");
        time("step2");
        print_timings(); // no output expected

        // restore
        if let Some(v) = prev_hamr {
            unsafe {
                std::env::set_var("HAMR_TIMING", v);
            }
        }
        if let Some(v) = prev_pi {
            unsafe {
                std::env::set_var("PI_TIMING", v);
            }
        }
    }
}

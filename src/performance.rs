use std::cell::RefCell;
use std::env;
use std::process::Command;
use std::time::Instant;

use serde::Serialize;

thread_local! {
    static STATE: RefCell<Option<PerfState>> = const { RefCell::new(None) };
}

#[derive(Debug)]
struct PerfState {
    started: Instant,
    counters: PerfCounters,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize)]
pub struct PerfCounters {
    pub wall_time_us: u64,
    pub git_subprocesses: usize,
    pub rust_files_parsed: usize,
    pub rust_files_prefiltered: usize,
    pub rust_cache_hits: usize,
    pub baseline_scope_hits: usize,
    pub baseline_scope_computed: usize,
}

pub fn init_from_environment() {
    let enabled = env::var_os("CARGO_CULTIST_PERF").is_some_and(|value| value != "0");
    if enabled {
        begin();
    }
}

pub fn git_command() -> Command {
    record_git_subprocess();
    Command::new("git")
}

pub fn record_git_subprocess() {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.git_subprocesses += 1;
        }
    });
}

pub fn record_rust_scan(parsed: usize, cache_hits: usize) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.rust_files_parsed += parsed;
            state.counters.rust_cache_hits += cache_hits;
        }
    });
}

pub fn record_rust_prefiltered(prefiltered: usize) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.rust_files_prefiltered += prefiltered;
        }
    });
}

pub fn record_baseline_scopes(hits: usize, computed: usize) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.counters.baseline_scope_hits += hits;
            state.counters.baseline_scope_computed += computed;
        }
    });
}

pub fn emit_if_enabled() {
    let Some(counters) = finish() else {
        return;
    };
    if let Ok(json) = serde_json::to_string(&counters) {
        eprintln!("CULTIST_PERF {json}");
    }
}

fn begin() {
    STATE.with(|state| {
        *state.borrow_mut() = Some(PerfState {
            started: Instant::now(),
            counters: PerfCounters::default(),
        });
    });
}

fn finish() -> Option<PerfCounters> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let mut state = state.take()?;
        state.counters.wall_time_us = elapsed_us(state.started);
        Some(state.counters)
    })
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, PerfCounters) {
    begin();
    let result = f();
    let counters = finish().expect("test performance capture should be active");
    (result, counters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_counters_are_inert() {
        record_git_subprocess();
        record_rust_scan(3, 4);
        record_rust_prefiltered(5);
        assert_eq!(finish(), None);
    }

    #[test]
    fn capture_counts_work_units() {
        let (_, counters) = capture(|| {
            record_git_subprocess();
            record_git_subprocess();
            record_rust_scan(3, 7);
            record_rust_prefiltered(5);
        });
        assert_eq!(counters.git_subprocesses, 2);
        assert_eq!(counters.rust_files_parsed, 3);
        assert_eq!(counters.rust_files_prefiltered, 5);
        assert_eq!(counters.rust_cache_hits, 7);
    }
}

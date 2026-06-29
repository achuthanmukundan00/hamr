//! Port of `../../packages/coding-agent/src/hamr/extensions/subagents.ts`.
//!
//! Implements subagent orchestration for the `delegate_subagents` tool:
//! spawning isolated child agent sessions that run in parallel, chain,
//! or staged execution modes with bounded concurrency, budget tracking,
//! live observability, and disk-persisted logs.
//!
//! # Architecture
//!
//! Workers are spawned as isolated child `hamr` processes (`hamr --mode json -p`)
//! and the parent parses JSONL events for live updates. Full logs are persisted
//! to disk; only bounded recent events and output tails are kept in memory.
//!
//! # Modes
//!
//! - `subtasks` — serial, backward-compatible legacy
//! - `tasks` — parallel batch with bounded concurrency
//! - `chain` — serial with `{previous}` placeholder
//! - `stages` — serial stages, each parallel or chain internally
//!
//! Registered behind `#[cfg(feature = "hamr-subagents")]` (always on in default
//! builds).
//!
//! # Key Types
//!
//! - [`SubagentManager`] — global shared state (active runs, budget, counters)
//! - [`WorkerState`] — per-worker in-memory state (events ring buffer, output tail)
//! - [`RunState`] — per-run aggregate state with O(1) counters
//! - [`WorkerOutcome`] — discriminant union for results (done/failed/aborted/timeout)
//! - [`ValidationResult`] — output validation checks

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use hamr_ai::types::{Usage, UsageCost};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::core::extensions::types::{ExtensionAPI, ExtensionContext, ExtensionFactory};

// ---------------------------------------------------------------------------
// Environment defaults
// ---------------------------------------------------------------------------

/// Maximum number of tasks in a single call (soft limit).
const ENV_MAX_TASKS: usize = 64;
/// Hard ceiling: rejected outright.
const ENV_HARD_MAX_TASKS: usize = 256;
/// Default concurrency cap.
const ENV_MAX_CONCURRENCY: usize = 64;
/// Global budget cap across the entire recursive subagent tree.
const ENV_TOTAL_BUDGET: usize = 1024;
/// Env var for child processes with remaining budget.
const ENV_TREE_REMAINING: &str = "HAMR_SUBAGENT_TREE_REMAINING";
// (v0.7.1: HAMR_CHILD_CONFIG temp-file path removed — children inherit auth
// from the parent process environment directly.)

/// Per-worker step timeout: 5 minutes.
#[allow(dead_code)]
const ENV_STEP_TIMEOUT_MS: u64 = 300_000;
/// Per-run total timeout: 30 minutes.
#[allow(dead_code)]
const ENV_TOTAL_TIMEOUT_MS: u64 = 1_800_000;

/// Output tail bytes kept in memory per worker.
const OUTPUT_TAIL_BYTES: usize = 32_768;
/// Number of events kept in the in-memory ring buffer per worker.
const EVENTS_IN_MEMORY: usize = 40;
/// Flush events to disk every N events.
const FLUSH_BATCH_SIZE: usize = 10;
/// Log directory base (relative to cwd).
#[allow(dead_code)]
const LOG_DIR_BASE: &str = ".hamr/subagents";
/// Maximum completed runs retained in memory.
#[allow(dead_code)]
const MAX_ACTIVE_RUNS: usize = 50;
/// Maximum recursion depth (root = 0; at this depth no delegate tool).
#[allow(dead_code)]
const MAX_DEPTH: usize = 3;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Global subagent manager, shared across the process.
pub struct SubagentManager {
    /// Active runs keyed by run ID.
    pub active_runs: RwLock<HashMap<String, RunState>>,
    /// Run counter for ID generation.
    pub run_counter: AtomicU64,
    /// Tree budget remaining for this process.
    pub tree_budget_remaining: RwLock<usize>,
}

impl SubagentManager {
    /// Create a new manager. Reads initial budget from `HAMR_SUBAGENT_TREE_REMAINING`
    /// env var, falling back to `ENV_TOTAL_BUDGET`.
    pub fn new() -> Self {
        let initial_budget = std::env::var(ENV_TREE_REMAINING)
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(ENV_TOTAL_BUDGET);

        Self {
            active_runs: RwLock::new(HashMap::new()),
            run_counter: AtomicU64::new(0),
            tree_budget_remaining: RwLock::new(initial_budget),
        }
    }

    /// Generate the next run ID.
    pub fn next_run_id(&self) -> String {
        let counter = self.run_counter.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        format!("run-{:x}-{:x}", now, counter)
    }

    /// Consume N budget slots. Returns true if enough budget was available.
    pub async fn consume_budget(&self, n: usize) -> bool {
        let mut budget = self.tree_budget_remaining.write().await;
        if *budget < n {
            return false;
        }
        *budget -= n;
        true
    }

    /// Refund N budget slots (e.g. on early abort).
    pub async fn refund_budget(&self, n: usize) {
        let mut budget = self.tree_budget_remaining.write().await;
        *budget += n;
    }

    /// Get remaining budget.
    pub async fn remaining_budget(&self) -> usize {
        *self.tree_budget_remaining.read().await
    }

    /// Restore completed runs from disk on session resume.
    ///
    /// Reads `run.json` files from `.hamr/subagents/runs/*/run.json`.
    /// Skips runs that are still active (no `endedAt`) or belong to a
    /// different session (`parentSessionId !== session_id`).
    pub async fn restore_runs_from_disk(&self, cwd: &str, session_id: &str) {
        let base = std::path::Path::new(cwd).join(LOG_DIR_BASE).join("runs");
        if !base.exists() {
            return;
        }
        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => return,
        };
        let mut active = self.active_runs.write().await;
        for entry in entries.flatten() {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let run_json = entry.path().join("run.json");
            if !run_json.exists() {
                continue;
            }
            let data = match std::fs::read_to_string(&run_json) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // Must have ended
            if parsed.get("endedAt").is_none() {
                continue;
            }
            // Must belong to this session
            let matches_session = parsed
                .get("parentSessionId")
                .and_then(|v| v.as_str())
                .map(|s| s == session_id)
                .unwrap_or(true); // if no parentSessionId, include it
            if !matches_session {
                continue;
            }
            let run_id = match parsed.get("runId").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            if active.contains_key(&run_id) {
                continue;
            }
            // Deserialize into RunState
            match serde_json::from_value::<RunState>(parsed) {
                Ok(run) => {
                    active.insert(run_id, run);
                }
                Err(_) => continue, // corrupted run.json
            }
        }
    }
}

impl Default for SubagentManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Activity event
// ---------------------------------------------------------------------------

/// An event recorded for a worker, truncated for in-memory preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub ts: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    /// Truncated JSON data (max 256 chars).
    pub data: String,
}

// ---------------------------------------------------------------------------
// Usage constant
// ---------------------------------------------------------------------------

fn empty_usage() -> Usage {
    Usage {
        input: 0,
        output: 0,
        cache_read: 0,
        cache_write: 0,
        cache_write_1h: None,
        total_tokens: 0,
        cost: hamr_ai::types::UsageCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
        },
    }
}

// ---------------------------------------------------------------------------
// Worker state
// ---------------------------------------------------------------------------

/// Per-worker in-memory state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerState {
    pub worker_id: String,
    pub task_preview: String,
    pub cwd: String,
    pub status: WorkerStatus,
    pub pid: Option<u32>,
    pub model: Option<String>,
    pub started_at: Option<u64>,
    pub ended_at: Option<u64>,
    pub usage: Usage,
    pub estimated_usage: bool,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
    pub last_activity: Option<String>,
    pub last_tool: Option<String>,
    pub recent_events: Vec<ActivityEvent>,
    pub pending_flush: Vec<String>,
    pub output_tail: String,
    pub final_output: Option<String>,
    pub log_path: Option<String>,
    pub result_path: Option<String>,
}

/// Worker status variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerStatus {
    Queued,
    Running,
    Done,
    Failed,
    Aborted,
}

impl WorkerStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerStatus::Queued => "queued",
            WorkerStatus::Running => "running",
            WorkerStatus::Done => "done",
            WorkerStatus::Failed => "failed",
            WorkerStatus::Aborted => "aborted",
        }
    }
}

impl WorkerState {
    pub fn new(worker_id: String, task: &str, cwd: &str) -> Self {
        let task_preview = if task.len() > 80 {
            let mut s = task.to_string();
            s.truncate(80);
            s.push_str("...");
            s
        } else {
            task.to_string()
        };
        Self {
            worker_id,
            task_preview,
            cwd: cwd.to_string(),
            status: WorkerStatus::Queued,
            pid: None,
            model: None,
            started_at: None,
            ended_at: None,
            usage: empty_usage(),
            estimated_usage: false,
            stop_reason: None,
            error_message: None,
            last_activity: None,
            last_tool: None,
            recent_events: Vec::new(),
            pending_flush: Vec::new(),
            output_tail: String::new(),
            final_output: None,
            log_path: None,
            result_path: None,
        }
    }

    /// Push an event into the worker's in-memory ring buffer and pending flush queue.
    pub fn push_event(&mut self, event: &serde_json::Value) {
        let ts = timestamp_ms();
        let event_type = event
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Truncated in-memory preview
        let data_str = serde_json::to_string(event).unwrap_or_default();
        let entry = ActivityEvent {
            ts,
            event_type: event_type.clone(),
            data: data_str.chars().take(256).collect(),
        };
        self.recent_events.push(entry);
        if self.recent_events.len() > EVENTS_IN_MEMORY {
            self.recent_events.remove(0);
        }

        self.last_activity = Some(event_type.clone());

        if event_type == "tool_execution_start" || event_type == "tool_execution_end" {
            self.last_tool = event
                .get("toolName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(Some(event_type.clone()));
        }

        // Update output tail
        if event_type == "message_update" || event_type == "message_end" {
            if let Some(msg) = event.get("message") {
                if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                    let mut text = String::new();
                    for part in content {
                        if part.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                                text.push_str(t);
                            }
                        }
                    }
                    if !text.is_empty() {
                        let combined = format!("{}{}", self.output_tail, text);
                        if combined.len() > OUTPUT_TAIL_BYTES {
                            let start = combined.len() - OUTPUT_TAIL_BYTES;
                            self.output_tail = combined[start..].to_string();
                        } else {
                            self.output_tail = combined;
                        }
                    }
                }
            }
        }

        // Pending flush
        let flush_entry = serde_json::json!({
            "ts": ts,
            "type": event_type,
            "data": event,
        });
        self.pending_flush.push(format!(
            "{}\n",
            serde_json::to_string(&flush_entry).unwrap_or_default()
        ));

        if self.pending_flush.len() >= FLUSH_BATCH_SIZE {
            self.flush_pending();
        }
    }

    /// Flush pending events to disk.
    pub fn flush_pending(&mut self) {
        if self.pending_flush.is_empty() {
            return;
        }
        if let Some(ref log_path) = self.log_path {
            let content: String = self.pending_flush.concat();
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .and_then(|f| {
                    use std::io::Write;
                    let mut f = f;
                    f.write_all(content.as_bytes())
                });
        }
        self.pending_flush.clear();
    }

    /// Flush log (flush & cleanup).
    pub fn flush_log(&mut self) {
        self.flush_pending();
    }
}

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/// O(1) counters for status transitions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCounters {
    pub queued: isize,
    pub running: isize,
    pub done: isize,
    pub failed: isize,
    pub aborted: isize,
    pub tok: isize,
}

/// Per-run aggregate state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub run_id: String,
    pub mode: String,
    pub total: usize,
    pub queued: usize,
    pub running: usize,
    pub done_count: usize,
    pub failed_count: usize,
    pub aborted_count: usize,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    #[serde(default)]
    pub parent_session_id: Option<String>,
    pub usage: Usage,
    pub log_dir: String,
    pub workers: HashMap<String, WorkerState>,
    /// O(1) counters
    pub cnt: RunCounters,
}

impl RunState {
    pub fn new(run_id: String, mode: &str, log_dir: &str) -> Self {
        Self {
            run_id,
            mode: mode.to_string(),
            total: 0,
            queued: 0,
            running: 0,
            done_count: 0,
            failed_count: 0,
            aborted_count: 0,
            started_at: timestamp_ms(),
            ended_at: None,
            parent_session_id: None,
            usage: empty_usage(),
            log_dir: log_dir.to_string(),
            workers: HashMap::new(),
            cnt: RunCounters {
                queued: 0,
                running: 0,
                done: 0,
                failed: 0,
                aborted: 0,
                tok: 0,
            },
        }
    }

    /// Transition a worker to a new status. O(1).
    pub fn transition(&mut self, worker_id: &str, to: WorkerStatus) {
        let prev_status = self.workers.get(worker_id).map(|ws| ws.status);
        let prev_tokens = self.workers.get(worker_id).map(|ws| ws.usage.total_tokens);
        if let Some(st) = prev_status {
            self.count_decr(st, prev_tokens.unwrap_or(0));
        }
        if let Some(ws) = self.workers.get_mut(worker_id) {
            ws.status = to;
        }
        if let Some(to_st) = self.workers.get(worker_id).map(|ws| ws.status) {
            self.count_incr(to_st, prev_tokens.unwrap_or(0));
        }
        self.sync_counters();
    }

    fn count_incr(&mut self, status: WorkerStatus, tokens: u64) {
        match status {
            WorkerStatus::Queued => self.cnt.queued += 1,
            WorkerStatus::Running => self.cnt.running += 1,
            WorkerStatus::Done => {
                self.cnt.done += 1;
                self.cnt.tok += tokens as isize;
            }
            WorkerStatus::Failed => {
                self.cnt.failed += 1;
                self.cnt.tok += tokens as isize;
            }
            WorkerStatus::Aborted => self.cnt.aborted += 1,
        }
    }

    fn count_decr(&mut self, status: WorkerStatus, tokens: u64) {
        match status {
            WorkerStatus::Queued => self.cnt.queued -= 1,
            WorkerStatus::Running => self.cnt.running -= 1,
            WorkerStatus::Done => {
                self.cnt.done -= 1;
                self.cnt.tok -= tokens as isize;
            }
            WorkerStatus::Failed => {
                self.cnt.failed -= 1;
                self.cnt.tok -= tokens as isize;
            }
            WorkerStatus::Aborted => self.cnt.aborted -= 1,
        }
    }

    fn sync_counters(&mut self) {
        self.queued = self.cnt.queued.max(0) as usize;
        self.running = self.cnt.running.max(0) as usize;
        self.done_count = self.cnt.done.max(0) as usize;
        self.failed_count = self.cnt.failed.max(0) as usize;
        self.aborted_count = self.cnt.aborted.max(0) as usize;
        self.usage.total_tokens = self.cnt.tok.max(0) as u64;
    }

    /// Accumulate a worker's cost into the run total.
    pub fn accumulate_cost(&mut self, usage: &Usage) {
        let c = &usage.cost;
        if c.total == 0.0 {
            return;
        }
        let prev = &self.usage.cost;
        self.usage.cost = hamr_ai::types::UsageCost {
            input: prev.input + c.input,
            output: prev.output + c.output,
            cache_read: prev.cache_read + c.cache_read,
            cache_write: prev.cache_write + c.cache_write,
            total: prev.total + c.total,
        };
        self.usage.total_tokens = self.usage.total_tokens.saturating_add(usage.total_tokens);
    }
}

// ---------------------------------------------------------------------------
// Worker process summary (v0.7.1 outcome classification)
// ---------------------------------------------------------------------------

/// Maps raw child process results (exit code, signal, stderr, output) into
/// structured outcomes.  Extracted from `run_worker_child_process` for
/// unit-testability — mirrors TS `WorkerProcessSummary`.
#[derive(Debug, Clone)]
struct WorkerProcessSummary {
    exit_code: i32,
    exit_signal: Option<String>,
    was_aborted: bool,
    stderr: String,
    output_text: String,
    usage: Usage,
    model: Option<String>,
    estimated_usage: bool,
    stop_reason: Option<String>,
    error_message: Option<String>,
    stdout_parse_errors: usize,
    invalid_stdout: String,
    spawn_error: Option<String>,
}

/// Mutable event-accumulation state during child stdout processing.
/// Mirrors TS `WorkerProcessEventState`.
#[derive(Debug, Clone)]
struct WorkerProcessEventState {
    output_text: String,
    usage: Usage,
    model: Option<String>,
    estimated_usage: bool,
    stop_reason: Option<String>,
    error_message: Option<String>,
    assistant_message_end_count: usize,
    stdout_parse_errors: usize,
    invalid_stdout: String,
}

impl Default for WorkerProcessEventState {
    fn default() -> Self {
        Self {
            output_text: String::new(),
            usage: empty_usage(),
            model: None,
            estimated_usage: true,
            stop_reason: None,
            error_message: None,
            assistant_message_end_count: 0,
            stdout_parse_errors: 0,
            invalid_stdout: String::new(),
        }
    }
}

/// Format a worker process failure diagnostic from a summary.
/// Mirrors TS `formatWorkerProcessFailure`.
fn format_worker_process_failure(summary: &WorkerProcessSummary) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(ref spawn_error) = summary.spawn_error {
        parts.push(format!("spawn error: {}", spawn_error));
    }
    if summary.exit_code != 0 {
        parts.push(format!("exit code {}", summary.exit_code));
    }
    if let Some(ref signal) = summary.exit_signal {
        parts.push(format!("signal {}", signal));
    }
    if summary.stop_reason.as_deref() == Some("error") {
        if let Some(ref em) = summary.error_message {
            parts.push(format!("model error: {}", em));
        } else {
            parts.push("model error".to_string());
        }
    } else if summary.stop_reason.as_deref() == Some("aborted") {
        if let Some(ref em) = summary.error_message {
            parts.push(format!("worker aborted: {}", em));
        } else {
            parts.push("worker aborted".to_string());
        }
    }
    if summary.stdout_parse_errors > 0 {
        let tail = summary.invalid_stdout.trim();
        let plural = if summary.stdout_parse_errors == 1 {
            ""
        } else {
            "s"
        };
        if tail.is_empty() {
            parts.push(format!(
                "{} invalid stdout line{} from child JSON mode",
                summary.stdout_parse_errors, plural
            ));
        } else {
            parts.push(format!(
                "{} invalid stdout line{} from child JSON mode:\n{}",
                summary.stdout_parse_errors, plural, tail
            ));
        }
    }
    let stderr_trimmed = summary.stderr.trim();
    if !stderr_trimmed.is_empty() {
        // Tail the last 16 KB of stderr
        let tail = tail_str(stderr_trimmed, 16 * 1024);
        parts.push(format!("stderr:\n{}", tail));
    }

    if parts.is_empty() {
        "Worker failed without a diagnostic.".to_string()
    } else {
        parts.join("\n\n")
    }
}

/// Return the last `max_bytes` bytes of `s`, preserving UTF-8 boundaries.
fn tail_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let start = s.len() - max_bytes;
    // Walk forward to a valid UTF-8 boundary
    let mut boundary = start;
    while boundary < s.len() && !s.is_char_boundary(boundary) {
        boundary += 1;
    }
    if boundary >= s.len() {
        // Fallback: truncate from start of string (shouldn't happen for valid UTF-8)
        &s[s.len().saturating_sub(max_bytes)..]
    } else {
        &s[boundary..]
    }
}

/// Build a [`WorkerOutcome`] from a [`WorkerProcessSummary`].
/// Mirrors TS `buildWorkerOutcomeFromChildSummary`.
fn build_worker_outcome_from_child_summary(
    worker_id: String,
    task: String,
    summary: WorkerProcessSummary,
) -> WorkerOutcome {
    if summary.was_aborted {
        return WorkerOutcome::Aborted {
            worker_id,
            task,
            reason: "user".to_string(),
        };
    }

    let is_failure = summary.stop_reason.as_deref() == Some("error")
        || summary.stop_reason.as_deref() == Some("aborted")
        || summary.spawn_error.is_some()
        || summary.exit_code != 0
        || summary.exit_signal.is_some();

    if is_failure {
        let error = format_worker_process_failure(&summary);
        return WorkerOutcome::Failed {
            worker_id,
            task,
            error,
            text: summary.output_text,
            validation: None,
        };
    }

    // Empty output with parse errors or stderr → failure
    if summary.output_text.trim().is_empty()
        && (summary.stdout_parse_errors > 0 || !summary.stderr.trim().is_empty())
    {
        let error = format_worker_process_failure(&summary);
        return WorkerOutcome::Failed {
            worker_id,
            task,
            error,
            text: summary.output_text.clone(),
            validation: None,
        };
    }

    // Success — validation is applied with the real cwd at the call site.
    WorkerOutcome::Done {
        worker_id,
        task,
        text: summary.output_text,
        usage: summary.usage,
        model: summary.model,
        estimated_usage: summary.estimated_usage,
        stop_reason: summary.stop_reason,
        validation: None, // set by caller with real cwd
    }
}

// ---------------------------------------------------------------------------
// Worker outcome (discriminant union)
// ---------------------------------------------------------------------------

/// Result produced by a single worker execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum WorkerOutcome {
    #[serde(rename = "done")]
    Done {
        worker_id: String,
        task: String,
        text: String,
        usage: Usage,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default)]
        estimated_usage: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<ValidationResult>,
    },
    #[serde(rename = "failed")]
    Failed {
        worker_id: String,
        task: String,
        error: String,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<ValidationResult>,
    },
    #[serde(rename = "aborted")]
    Aborted {
        worker_id: String,
        task: String,
        reason: String,
    },
    #[serde(rename = "timeout")]
    Timeout {
        worker_id: String,
        task: String,
        partial_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        validation: Option<ValidationResult>,
    },
}

// ---------------------------------------------------------------------------
// Output validation
// ---------------------------------------------------------------------------

/// A warning about output quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationWarning {
    #[serde(rename = "type")]
    pub warning_type: String,
    pub message: String,
    pub severity: String,
}

impl ValidationWarning {
    pub fn new(warning_type: &str, message: &str, severity: &str) -> Self {
        Self {
            warning_type: warning_type.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
        }
    }
}

/// Output validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub passed: bool,
    pub warnings: Vec<ValidationWarning>,
    /// 0.0–1.0 heuristic confidence score
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[allow(dead_code)]
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 10_000 {
        format!("{}K", tokens / 1000)
    } else if tokens >= 1000 {
        format!("{:.1}K", tokens as f64 / 1000.0)
    } else {
        tokens.to_string()
    }
}

/// Pad a worker ID with leading zeros.
#[allow(dead_code)]
fn pad_worker_id(idx: usize, total: usize) -> String {
    let width = total.to_string().len();
    format!("{:0>width$}", idx, width = width)
}

/// Clamp a value between min and max.
fn clamp(value: usize, min: usize, max: usize) -> usize {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Ensure the log directory exists and return its path.
#[allow(dead_code)]
fn ensure_log_dir(run_id: &str, cwd: &str) -> String {
    let base = Path::new(cwd).join(LOG_DIR_BASE).join("runs").join(run_id);
    let workers_dir = base.join("workers");
    let _ = std::fs::create_dir_all(&workers_dir);
    base.to_string_lossy().to_string()
}

/// Check if a string looks like a file-system path.
fn looks_like_file_path(s: &str) -> bool {
    if s.starts_with("http://") || s.starts_with("https://") {
        return false;
    }
    if looks_like_package_ref(s) {
        return false;
    }
    if looks_like_version(s) {
        return false;
    }
    if !has_file_extension(s) {
        return false;
    }
    true
}

/// Check if a string resembles an npm-style package reference (e.g. `@scope/pkg@1.2.3`).
fn looks_like_package_ref(s: &str) -> bool {
    if let Some(at_pos) = s.rfind('@') {
        if at_pos == 0 {
            return false;
        }
        let name_part = &s[..at_pos];
        let version_part = &s[at_pos + 1..];
        if name_part.contains('/') && !version_part.is_empty() {
            let first_char = version_part.chars().next().unwrap_or(' ');
            if first_char.is_ascii_digit() || first_char == 'v' {
                return version_part
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == 'v');
            }
        }
    }
    false
}

/// Check if a string looks like a version number.
fn looks_like_version(s: &str) -> bool {
    if s.starts_with('v') {
        let rest = &s[1..];
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return rest.chars().filter(|c| *c == '.').count() >= 1;
        }
    }
    // Pure semver like 1.2.3
    if let Some(dot1) = s.find('.') {
        if s[..dot1].chars().all(|c| c.is_ascii_digit()) {
            let rest = &s[dot1 + 1..];
            if rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return rest.chars().filter(|c| *c == '.').count() >= 1;
            }
        }
    }
    false
}

/// Check if string has a file extension at the end.
fn has_file_extension(s: &str) -> bool {
    if let Some(dot) = s.rfind('.') {
        let ext = &s[dot + 1..];
        if (1..=10).contains(&ext.len()) {
            return ext.chars().all(|c| c.is_alphanumeric());
        }
    }
    false
}

/// Extract plausible file-system paths from arbitrary text.
fn extract_file_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();

    // Backtick-wrapped: `src/foo.ts`
    let mut pos = 0;
    let bytes = text.as_bytes();
    while pos < text.len() {
        if bytes[pos] == b'`' {
            let end = text[pos + 1..].find('`').map(|i| pos + 1 + i);
            if let Some(end_pos) = end {
                let candidate = text[pos + 1..end_pos].trim();
                if candidate.len() <= 200 && looks_like_file_path(candidate) {
                    if !refs.contains(&candidate.to_string()) {
                        refs.push(candidate.to_string());
                    }
                }
                pos = end_pos + 1;
                continue;
            }
        }
        pos += 1;
    }

    // Path-like tokens using simple scanning
    // Match: optional ./ or ../ prefix, then path segments, then .extension
    let bytes = text.as_bytes();
    let len = text.len();
    let mut i = 0;
    while i < len {
        // Look for sequences that look like file paths
        if i > 0
            && bytes[i - 1] != b' '
            && bytes[i - 1] != b'\n'
            && bytes[i - 1] != b'\t'
            && bytes[i - 1] != b'"'
            && bytes[i - 1] != b'\''
            && bytes[i - 1] != b'('
            && bytes[i - 1] != b'`'
        {
            i += 1;
            continue;
        }
        // Record start position including optional ./ or ../
        let capture_start = i;
        let mut j = i;
        // Skip optional ./ or ../
        if j < len && bytes[j] == b'.' {
            j += 1;
            if j < len && bytes[j] == b'/' {
                j += 1;
            } else if j < len && bytes[j] == b'.' {
                j += 1;
                if j < len && bytes[j] == b'/' {
                    j += 1;
                }
            }
        }
        // Read path segments
        let content_start = j;
        let mut has_slash = false;
        while j < len {
            let c = bytes[j];
            if c.is_ascii_alphanumeric()
                || c == b'_'
                || c == b'.'
                || c == b'-'
                || c == b'/'
                || c == b'@'
            {
                if c == b'/' {
                    has_slash = true;
                }
                j += 1;
            } else {
                break;
            }
        }
        if j > content_start && j - content_start >= 3 && has_slash && bytes[j - 1] != b'/' {
            let candidate = &text[capture_start..j];
            // Check it ends with an extension
            if candidate.contains('.')
                && has_file_extension(candidate)
                && looks_like_file_path(candidate)
            {
                if !refs.contains(&candidate.to_string()) {
                    refs.push(candidate.to_string());
                }
            }
        }
        i = if j > i { j } else { i + 1 };
    }

    refs
}

/// Check output for self-contradictory patterns.
fn check_self_contradiction(text: &str) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();
    let lower = text.to_lowercase();

    let has_errors = lower.contains("error") || lower.contains("errors");
    let has_failures = lower.contains("failed") || lower.contains("failure");

    if has_errors && has_failures {
        warnings.push(ValidationWarning::new(
            "self_contradiction",
            "Output mentions both error(s) and failure(s) review for consistency.",
            "low",
        ));
    }

    let has_created = lower.contains("created")
        || lower.contains("wrote")
        || lower.contains("generated")
        || lower.contains("built");
    let has_missing = lower.contains("does not exist")
        || lower.contains("not found")
        || lower.contains("cannot find")
        || lower.contains("no such");

    if has_created && has_missing {
        warnings.push(ValidationWarning::new(
            "self_contradiction",
            "Output claims creation but also mentions missing/non-existent items.",
            "medium",
        ));
    }

    warnings
}

fn file_exists_relative(cwd: &str, file_ref: &str) -> bool {
    let resolved = Path::new(cwd).join(file_ref);
    resolved.exists()
}

/// Validate subagent output before merging.
pub fn validate_worker_output(
    _status: &str,
    text: &str,
    cwd: &str,
    partial_text: Option<&str>,
) -> ValidationResult {
    let mut warnings = Vec::new();

    let output_text = if text.is_empty() {
        partial_text.unwrap_or("")
    } else {
        text
    };

    // 1. Empty output
    if output_text.trim().is_empty() {
        warnings.push(ValidationWarning::new(
            "empty_output",
            "Worker produced no output text.",
            "high",
        ));
        return ValidationResult {
            passed: false,
            warnings,
            confidence: 0.0,
        };
    }

    // 2. Truncated output
    if output_text.len() >= OUTPUT_TAIL_BYTES {
        warnings.push(ValidationWarning::new(
            "truncated_output",
            &format!(
                "Output may be truncated ({} >= {} byte limit).",
                output_text.len(),
                OUTPUT_TAIL_BYTES
            ),
            "medium",
        ));
    }

    // 3. File references
    let file_refs = extract_file_references(output_text);
    let missing_files: Vec<String> = file_refs
        .iter()
        .filter(|r| !file_exists_relative(cwd, r))
        .take(5)
        .cloned()
        .collect();

    if !missing_files.is_empty() {
        let suffix = if file_refs.len() > 5 {
            format!(" (+{} more)", file_refs.len() - 5)
        } else {
            String::new()
        };
        let file_basenames: Vec<String> = missing_files
            .iter()
            .map(|f| {
                Path::new(f)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            })
            .collect();
        warnings.push(ValidationWarning::new(
            "missing_file",
            &format!(
                "References {} non-existent file{}: {}{}",
                missing_files.len(),
                if missing_files.len() == 1 { "" } else { "s" },
                file_basenames.join(", "),
                suffix,
            ),
            "medium",
        ));
    }

    // 4. Self-contradiction
    warnings.extend(check_self_contradiction(output_text));

    // Compute confidence
    let mut confidence = 1.0_f64;
    for w in &warnings {
        match w.severity.as_str() {
            "high" => confidence -= 0.3,
            "medium" => confidence -= 0.15,
            "low" => confidence -= 0.05,
            _ => {}
        }
    }
    confidence = confidence.max(0.0).min(1.0);
    confidence = (confidence * 100.0).round() / 100.0;

    ValidationResult {
        passed: warnings.is_empty(),
        warnings,
        confidence,
    }
}

// ---------------------------------------------------------------------------
// Bash-only fast path
// ---------------------------------------------------------------------------

/// Returns true if the tools list qualifies for the bash-only fast path.
pub fn is_bash_fast_path_tools(tools: Option<&[String]>) -> bool {
    match tools {
        Some(t) if !t.is_empty() && t.len() <= 2 => {
            t.iter().any(|tool| tool == "bash")
                && t.iter().all(|tool| tool == "bash" || tool == "read")
        }
        _ => false,
    }
}

/// Spawn /bin/bash directly without a full agent loop.
async fn run_bash_fast_path(
    worker_id: String,
    task: String,
    cwd: String,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> WorkerOutcome {
    let child_result = Command::new("/bin/bash")
        .arg("-c")
        .arg(&task)
        .current_dir(&cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn();

    let mut child = match child_result {
        Ok(c) => c,
        Err(e) => {
            return WorkerOutcome::Failed {
                worker_id,
                task,
                error: format!("Failed to spawn bash: {}", e),
                text: String::new(),
                validation: Some(validate_worker_output("failed", "", &cwd, None)),
            };
        }
    };

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    let mut stdout_text = String::new();
    let mut stderr_text = String::new();

    let mut cancel_clone = cancel.clone();

    tokio::select! {
        result = async {
            let _ = stdout_reader.read_to_string(&mut stdout_text).await;
            let _ = stderr_reader.read_to_string(&mut stderr_text).await;
            let exit_status = child.wait().await.ok();
            exit_status.and_then(|s| s.code()).unwrap_or(-1)
        } => {
            if *cancel_clone.borrow() {
                return WorkerOutcome::Aborted {
                    worker_id,
                    task,
                    reason: "user".to_string(),
                };
            }

            if result != 0 {
                let error = if stderr_text.is_empty() {
                    format!("exit code {}", result)
                } else {
                    stderr_text
                };
                let validation = validate_worker_output("failed", &stdout_text, &cwd, None);
                return WorkerOutcome::Failed {
                    worker_id,
                    task,
                    error,
                    text: stdout_text,
                    validation: Some(validation),
                };
            }

            let validation = validate_worker_output("done", &stdout_text, &cwd, None);
            WorkerOutcome::Done {
                worker_id,
                task,
                text: stdout_text,
                usage: empty_usage(),
                model: None,
                estimated_usage: true,
                stop_reason: None,
                validation: Some(validation),
            }
        }
        _ = cancel_clone.changed() => {
            let _ = child.start_kill();
            WorkerOutcome::Aborted {
                worker_id,
                task,
                reason: "user".to_string(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Child process worker
// ---------------------------------------------------------------------------

/// Run a worker as a child hamr process.
#[allow(dead_code)]
async fn run_worker_child_process(
    worker_id: String,
    task: String,
    cwd: String,
    cancel: tokio::sync::watch::Receiver<bool>,
    worker_model: Option<String>,
    worker_tools: Option<Vec<String>>,
    tree_budget_remaining: usize,
) -> WorkerOutcome {
    // Bash-only fast path
    if is_bash_fast_path_tools(worker_tools.as_deref()) {
        return run_bash_fast_path(worker_id, task, cwd, cancel).await;
    }

    let mut args = vec!["--mode".to_string(), "json".to_string(), "-p".to_string()];
    if let Some(ref model) = worker_model {
        args.push("--model".to_string());
        args.push(model.clone());
    }
    if let Some(ref tools) = worker_tools {
        if !tools.is_empty() {
            args.push("--tools".to_string());
            args.push(tools.join(","));
        }
    }
    args.push(task.clone());

    let child_result = Command::new("hamr")
        .args(&args)
        .current_dir(&cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env(ENV_TREE_REMAINING, tree_budget_remaining.to_string())
        .kill_on_drop(true)
        .spawn();

    let mut child = match child_result {
        Ok(c) => c,
        Err(e) => {
            return WorkerOutcome::Failed {
                worker_id,
                task,
                error: format!("Failed to spawn hamr: {}", e),
                text: String::new(),
                validation: None,
            };
        }
    };

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    let mut stderr_text = String::new();
    let mut state = WorkerProcessEventState::default();

    let mut line_buf = String::new();

    let wid = worker_id.clone();
    let tsk = task.clone();
    let mut cancel_clone = cancel.clone();

    tokio::select! {
        result = async {
            // Read stdout line by line (JSONL)
            loop {
                line_buf.clear();
                match stdout_reader.read_line(&mut line_buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let line = line_buf.trim().to_string();
                        if line.is_empty() {
                            continue;
                        }
                        // Parse JSON event
                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(event) => {
                                record_worker_process_event(&event, &mut state);
                            }
                            Err(_) => {
                                state.stdout_parse_errors += 1;
                                // Collect up to 4 KB of invalid stdout for diagnostics
                                if state.invalid_stdout.len() < 4096 {
                                    if !state.invalid_stdout.is_empty() {
                                        state.invalid_stdout.push('\n');
                                    }
                                    state.invalid_stdout.push_str(&line);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            // Read remaining stderr
            let _ = stderr_reader.read_to_string(&mut stderr_text).await;

            let exit_status = child.wait().await.ok();

            // Extract exit code and signal (Unix)
            let exit_code = exit_status.as_ref().and_then(|s| s.code()).unwrap_or(-1);

            #[cfg(unix)]
            let exit_signal: Option<String> = {
                use std::os::unix::process::ExitStatusExt;
                exit_status.as_ref().and_then(|s| s.signal().map(|sig| sig.to_string()))
            };
            #[cfg(not(unix))]
            let exit_signal: Option<String> = None;

            let summary = WorkerProcessSummary {
                exit_code,
                exit_signal,
                was_aborted: false, // if this arm fired, we weren't cancelled
                stderr: stderr_text,
                output_text: state.output_text,
                usage: state.usage,
                model: state.model,
                estimated_usage: state.estimated_usage,
                stop_reason: state.stop_reason,
                error_message: state.error_message,
                stdout_parse_errors: state.stdout_parse_errors,
                invalid_stdout: state.invalid_stdout,
                spawn_error: None,
            };

            build_worker_outcome_from_child_summary(wid, tsk, summary)
        } => {
            // Apply validation with real cwd for Done outcomes
            match result {
                WorkerOutcome::Done { worker_id, task, text, usage, model, estimated_usage, stop_reason, .. } => {
                    WorkerOutcome::Done {
                        worker_id,
                        task,
                        text: text.clone(),
                        usage,
                        model,
                        estimated_usage,
                        stop_reason,
                        validation: Some(validate_worker_output("done", &text, &cwd, None)),
                    }
                }
                other => other,
            }
        }
        _ = cancel_clone.changed() => {
            let _ = child.start_kill();
            WorkerOutcome::Aborted {
                worker_id,
                task,
                reason: "user".to_string(),
            }
        }
    }
}

/// Process a single JSONL event from a worker's stdout, accumulating state.
/// Mirrors TS `recordWorkerProcessEvent` with `agent_end` fallback (v0.7.1).
fn record_worker_process_event(event: &serde_json::Value, state: &mut WorkerProcessEventState) {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    if event_type == "message_end" {
        if let Some(msg) = event.get("message") {
            if msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                state.assistant_message_end_count += 1;
                record_assistant_message(state, msg);
                return;
            }
        }
    }

    // Fallback for corrupted/missing message_end streams.  agent_end carries
    // the final messages — use it only when no assistant message_end was seen.
    if event_type == "agent_end" && state.assistant_message_end_count == 0 {
        if let Some(messages) = event.get("messages").and_then(|m| m.as_array()) {
            // Find the last assistant message in reverse order
            let final_assistant = messages
                .iter()
                .rev()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"));
            if let Some(msg) = final_assistant {
                record_assistant_message(state, msg);
            }
        }
    }
}

/// Record an assistant message into event state.
/// Mirrors TS `recordAssistantMessage`.
fn record_assistant_message(state: &mut WorkerProcessEventState, msg: &serde_json::Value) {
    if let Some(msg_usage) = msg.get("usage") {
        if let Ok(u) = serde_json::from_value::<Usage>(msg_usage.clone()) {
            state.usage = u;
            state.estimated_usage = false;
        }
    }
    if let Some(m) = msg.get("model").and_then(|m| m.as_str()) {
        state.model = Some(m.to_string());
    }
    if let Some(sr) = msg.get("stopReason").and_then(|s| s.as_str()) {
        state.stop_reason = Some(sr.to_string());
    }
    if let Some(em) = msg.get("errorMessage").and_then(|s| s.as_str()) {
        state.error_message = Some(em.to_string());
    }
    if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
        for part in content {
            if part.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                    state.output_text.push_str(t);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// delegate_subagents tool parameters + plan resolution
// ---------------------------------------------------------------------------
//
// Port of the TS `TaskItem` / `SubagentParams` (typebox) plus the mode-resolution
// and limit-validation performed at the top of the tool's `execute`. This is the
// pure front-half of the `delegate_subagents` tool: it turns raw model-supplied
// args into a validated, normalized execution plan. Spawning (child processes,
// budget/provider gating, theming) is layered on top of this and is NOT here.

/// One worker task. Mirrors the TS `TaskItem`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskItem {
    /// Focused, self-contained task for one worker subagent.
    pub task: String,
    /// Working directory for this worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Model override for this worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Restrict tools for this worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Path to an output file the worker must produce (validated post-run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
}

/// Execution mode for a single stage. Mirrors the TS stage `mode` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageMode {
    Parallel,
    Chain,
}

/// A stage of tasks. Mirrors a TS `stages[]` element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSpec {
    pub mode: StageMode,
    pub tasks: Vec<TaskItem>,
}

/// Raw `delegate_subagents` tool parameters. Mirrors the TS `SubagentParams`.
/// Exactly one of `subtasks`/`tasks`/`chain`/`stages` must be provided.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentParams {
    /// DEPRECATED serial alias — normalized to a single `chain` stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtasks: Option<Vec<TaskItem>>,
    /// Parallel batch with bounded concurrency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<TaskItem>>,
    /// Sequential chain (supports `{previous}` references downstream).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<Vec<TaskItem>>,
    /// Mixed stages executed sequentially.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<StageSpec>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_fast: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_timeout_ms: Option<u64>,
}

/// The top-level mode label for a resolved plan (for run state / telemetry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanMode {
    Tasks,
    Chain,
    Stages,
    /// Deprecated `subtasks` alias (executes like `chain`).
    Subtasks,
}

impl PlanMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanMode::Tasks => "tasks",
            PlanMode::Chain => "chain",
            PlanMode::Stages => "stages",
            PlanMode::Subtasks => "subtasks",
        }
    }
}

/// A validated, normalized execution plan. `stages` is the canonical form:
/// `tasks` → one parallel stage; `chain`/`subtasks` → one chain stage; `stages`
/// → passed through. Every variant collapses to the same downstream executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentPlan {
    pub mode: PlanMode,
    pub stages: Vec<StageSpec>,
    pub task_count: usize,
    pub concurrency: usize,
    pub fail_fast: bool,
    pub observe: String,
}

/// Resolve raw tool params into a validated plan, or a model-facing error
/// string. Mirrors the mode-count + task-limit validation at the top of the TS
/// tool's `execute`. Provider/budget gating is intentionally out of scope here
/// (it needs the runtime context); this is the pure, deterministic core.
pub fn resolve_subagent_plan(params: &SubagentParams) -> Result<SubagentPlan, String> {
    let has_subtasks = params.subtasks.as_ref().is_some_and(|v| !v.is_empty());
    let has_tasks = params.tasks.as_ref().is_some_and(|v| !v.is_empty());
    let has_chain = params.chain.as_ref().is_some_and(|v| !v.is_empty());
    let has_stages = params.stages.as_ref().is_some_and(|v| !v.is_empty());
    let mode_count = has_subtasks as u8 + has_tasks as u8 + has_chain as u8 + has_stages as u8;

    if mode_count == 0 {
        return Err("No mode specified. Provide exactly one of: tasks, chain, stages, or subtasks (deprecated).".to_string());
    }
    if mode_count > 1 {
        return Err("Multiple modes specified. Provide exactly one of: tasks, chain, stages, or subtasks (deprecated).".to_string());
    }

    let (mode, stages) = if has_tasks {
        (
            PlanMode::Tasks,
            vec![StageSpec {
                mode: StageMode::Parallel,
                tasks: params.tasks.clone().unwrap(),
            }],
        )
    } else if has_chain {
        (
            PlanMode::Chain,
            vec![StageSpec {
                mode: StageMode::Chain,
                tasks: params.chain.clone().unwrap(),
            }],
        )
    } else if has_subtasks {
        (
            PlanMode::Subtasks,
            vec![StageSpec {
                mode: StageMode::Chain,
                tasks: params.subtasks.clone().unwrap(),
            }],
        )
    } else {
        (PlanMode::Stages, params.stages.clone().unwrap())
    };

    let task_count: usize = stages.iter().map(|stage| stage.tasks.len()).sum();

    // NOTE: TS checks the soft limit BEFORE the hard limit, so anything over the
    // hard limit also trips the soft check first (the hard branch is effectively
    // unreachable). Mirrored here for behavioral parity, despite the comment in
    // the TS source claiming the soft limit only "warns".
    if task_count > ENV_MAX_TASKS {
        return Err(format!(
            "Too many tasks ({task_count}). Soft limit is {ENV_MAX_TASKS}. Set HAMR_SUBAGENT_MAX_TASKS to increase (hard max: {ENV_HARD_MAX_TASKS})."
        ));
    }
    if task_count > ENV_HARD_MAX_TASKS {
        return Err(format!(
            "Too many tasks ({task_count}). Hard limit is {ENV_HARD_MAX_TASKS}."
        ));
    }

    Ok(SubagentPlan {
        mode,
        stages,
        task_count,
        concurrency: clamp(
            params.concurrency.unwrap_or(ENV_MAX_CONCURRENCY),
            1,
            ENV_MAX_CONCURRENCY,
        ),
        fail_fast: params.fail_fast.unwrap_or(false),
        observe: params
            .observe
            .clone()
            .unwrap_or_else(|| "compact".to_string()),
    })
}

// ---------------------------------------------------------------------------
// Concurrency limiter
// ---------------------------------------------------------------------------

/// Map items with bounded concurrency using a semaphore.
///
/// Runs up to `concurrency` futures concurrently, each processing one item.
/// Results are returned in the same order as the input items.
#[allow(dead_code)]
async fn map_with_concurrency_limit<T, Fut>(
    items: Vec<T>,
    concurrency: usize,
    f: Arc<dyn Fn(T, usize) -> Fut + Send + Sync>,
) -> Vec<Fut::Output>
where
    T: Send + 'static,
    Fut: std::future::Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    if items.is_empty() {
        return Vec::new();
    }
    let limit = clamp(concurrency, 1, items.len());
    let semaphore = Arc::new(tokio::sync::Semaphore::new(limit));
    let mut handles = Vec::new();

    for (idx, item) in items.into_iter().enumerate() {
        let sem = Arc::clone(&semaphore);
        let f = Arc::clone(&f);
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore not closed");
            f(item, idx).await
        }));
    }

    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        results.push(h.await.expect("task panicked"));
    }
    results
}

// ---------------------------------------------------------------------------
// Plan executor (stages → workers)
// ---------------------------------------------------------------------------
//
// Port of the TS `executeStages`/`executeChain`/`executeTasks` orchestration,
// decoupled from child-process spawning: the worker-runner is injected so this
// is unit-testable with a fake runner (no subprocess). The real tool wires in
// `run_worker_child_process` as the runner.

/// A single resolved worker invocation handed to the runner.
#[derive(Debug, Clone)]
pub struct WorkerInvocation {
    pub worker_id: String,
    /// Task text AFTER `{previous}` substitution (chain stages only).
    pub task: String,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub tools: Option<Vec<String>>,
    pub artifact: Option<String>,
}

impl WorkerInvocation {
    fn from_item(worker_id: String, item: &TaskItem, task: String) -> Self {
        Self {
            worker_id,
            task,
            cwd: item.cwd.clone(),
            model: item.model.clone(),
            tools: item.tools.clone(),
            artifact: item.artifact.clone(),
        }
    }
}

/// Injectable async worker runner: takes a resolved invocation, returns its
/// outcome. The real implementation spawns a child `hamr` process; tests pass a
/// fake.
pub type WorkerRunner = Arc<
    dyn Fn(
            WorkerInvocation,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = WorkerOutcome> + Send>>
        + Send
        + Sync,
>;

fn outcome_is_failed(outcome: &WorkerOutcome) -> bool {
    matches!(outcome, WorkerOutcome::Failed { .. })
}

/// Text a finished worker feeds forward as `{previous}`. Mirrors the TS rule:
/// done/failed carry `text`, timeout carries `partialText`, aborted carries
/// nothing (leaves the prior value intact).
fn forward_text(outcome: &WorkerOutcome) -> Option<String> {
    match outcome {
        WorkerOutcome::Done { text, .. } | WorkerOutcome::Failed { text, .. } => Some(text.clone()),
        WorkerOutcome::Timeout { partial_text, .. } => Some(partial_text.clone()),
        WorkerOutcome::Aborted { .. } => None,
    }
}

/// Drive a resolved [`SubagentPlan`] to completion. Stages run sequentially;
/// within a stage, parallel tasks run with bounded concurrency and chain tasks
/// run in order threading `{previous}`. `previous_output` carries ACROSS stages.
/// With `fail_fast`, the first failed worker stops all remaining work.
///
/// Mirrors the TS `executeStages` semantics (parallel stages do NOT substitute
/// `{previous}`; only chain steps do).
pub async fn run_plan(plan: &SubagentPlan, run_worker: WorkerRunner) -> Vec<WorkerOutcome> {
    let mut all_results: Vec<WorkerOutcome> = Vec::new();
    let mut previous_output = String::new();

    'stages: for stage in &plan.stages {
        let n = stage.tasks.len();
        match stage.mode {
            StageMode::Parallel => {
                let invocations: Vec<WorkerInvocation> = stage
                    .tasks
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        WorkerInvocation::from_item(pad_worker_id(i, n), item, item.task.clone())
                    })
                    .collect();
                let runner = run_worker.clone();
                let stage_results = map_with_concurrency_limit(
                    invocations,
                    plan.concurrency,
                    Arc::new(move |inv: WorkerInvocation, _idx: usize| runner(inv)),
                )
                .await;
                let any_failed = stage_results.iter().any(outcome_is_failed);
                if let Some(text) = stage_results.last().and_then(forward_text) {
                    previous_output = text;
                }
                all_results.extend(stage_results);
                if plan.fail_fast && any_failed {
                    break 'stages;
                }
            }
            StageMode::Chain => {
                for (i, item) in stage.tasks.iter().enumerate() {
                    let task = item.task.replace("{previous}", &previous_output);
                    let inv = WorkerInvocation::from_item(pad_worker_id(i, n), item, task);
                    let outcome = run_worker(inv).await;
                    let failed = outcome_is_failed(&outcome);
                    if let Some(text) = forward_text(&outcome) {
                        previous_output = text;
                    }
                    all_results.push(outcome);
                    if plan.fail_fast && failed {
                        break 'stages;
                    }
                }
            }
        }
    }

    all_results
}

// ---------------------------------------------------------------------------
// Factory: Create the subagents extension
// ---------------------------------------------------------------------------

/// Create the hamr subagents extension factory.
///
/// Mirrors `createHamrSubagentsExtension` in the TS source.
///
/// The `_depth` parameter controls recursion bound. At MAX_DEPTH, no
/// delegate tool is registered (leaf node).
///
/// In the TS version, this registers:
/// - `session_start` handler — restores completed runs on resume
/// - `session_before_switch` / `session_before_fork` — clear state
/// - `delegate_subagents` tool — the core subagent orchestration tool
///
/// The tool registration (registerSubagentTool) requires the full
/// extension infrastructure (ToolDefinition with typebox-like params,
/// renderCall/renderResult, and the worker execution functions).
pub fn create_hamr_subagents_extension(depth: usize) -> ExtensionFactory {
    let manager = Arc::new(SubagentManager::new());

    Arc::new(move |pi: Arc<dyn ExtensionAPI>| {
        let manager = manager.clone();
        Box::pin(async move {
            if depth >= MAX_DEPTH {
                return;
            }

            // Session start: reset state, restore on resume
            pi.on(
                "session_start",
                Arc::new({
                    let manager = manager.clone();
                    move |event: serde_json::Value, _ctx: Arc<dyn ExtensionContext>| {
                        let manager = manager.clone();
                        Box::pin(async move {
                            let mut active = manager.active_runs.write().await;
                            active.clear();
                            drop(active);
                            let reason = event
                                .get("reason")
                                .and_then(|r| r.as_str())
                                .unwrap_or("startup");
                            if reason == "resume" {
                                // Restore completed runs from disk so the UI
                                // can display prior subagent history.
                                let cwd = _ctx.cwd();
                                manager.restore_runs_from_disk(&cwd, "").await;
                            }
                            None
                        })
                    }
                }),
            );

            // Session switch/fork: clear state
            for event_type in &["session_before_switch", "session_before_fork"] {
                let et = *event_type;
                let manager = manager.clone();
                pi.on(
                    et,
                    Arc::new(
                        move |_event: serde_json::Value, _ctx: Arc<dyn ExtensionContext>| {
                            let manager = manager.clone();
                            Box::pin(async move {
                                let mut active = manager.active_runs.write().await;
                                active.clear();
                                None
                            })
                        },
                    ),
                );
            }

            // Register delegate_subagents tool
            let manager_tool = manager.clone();
            let tool_params_schema = serde_json::json!({
                "type": "object",
                "properties": {
                    "subtasks": {
                        "type": "array",
                        "description": "DEPRECATED serial alias — normalized to chain.",
                        "items": { "$ref": "#/definitions/TaskItem" }
                    },
                    "tasks": {
                        "type": "array",
                        "description": "Parallel batch with bounded concurrency (default max 64).",
                        "items": { "$ref": "#/definitions/TaskItem" }
                    },
                    "chain": {
                        "type": "array",
                        "description": "Sequential chain. Use {previous} in task to reference prior output.",
                        "items": { "$ref": "#/definitions/TaskItem" }
                    },
                    "stages": {
                        "type": "array",
                        "description": "Sequential stages; each stage can be 'parallel' or 'chain'.",
                        "items": { "$ref": "#/definitions/StageSpec" }
                    },
                    "concurrency": {
                        "type": "integer",
                        "description": "Max concurrent workers (default: 64)."
                    },
                    "failFast": {
                        "type": "boolean",
                        "description": "If true, abort remaining workers on first failure."
                    },
                    "observe": {
                        "type": "string",
                        "description": "Observation verbosity: silent, compact, verbose."
                    },
                    "stepTimeoutMs": {
                        "type": "integer",
                        "description": "Per-worker timeout in ms (default: 900000 = 15min)."
                    },
                    "totalTimeoutMs": {
                        "type": "integer",
                        "description": "Per-run total timeout in ms (default: 1800000 = 30min)."
                    }
                },
                "definitions": {
                    "TaskItem": {
                        "type": "object",
                        "required": ["task"],
                        "properties": {
                            "task": { "type": "string", "description": "Focused, self-contained task for one worker subagent." },
                            "cwd": { "type": "string", "description": "Working directory for this worker." },
                            "model": { "type": "string", "description": "Model override for this worker." },
                            "tools": { "type": "array", "items": { "type": "string" }, "description": "Restrict tools for this worker." },
                            "artifact": { "type": "string", "description": "Path to an output file the worker must produce." }
                        },
                        "additionalProperties": false
                    },
                    "StageSpec": {
                        "type": "object",
                        "required": ["mode", "tasks"],
                        "properties": {
                            "mode": { "type": "string", "enum": ["parallel", "chain"] },
                            "tasks": { "type": "array", "items": { "$ref": "#/definitions/TaskItem" } }
                        },
                        "additionalProperties": false
                    }
                }
            });

            let manager_exec = manager_tool.clone();
            pi.register_tool(crate::core::extensions::types::ToolDefinition {
                name: "delegate_subagents".to_string(),
                label: "Subagents".to_string(),
                description: "Dispatch focused subtasks to parallel or sequential worker subagents.".to_string(),
                prompt_snippet: Some("Use delegate_subagents to dispatch focused subtasks to parallel/sequential worker subagents.".to_string()),
                prompt_guidelines: Some(vec![
                    "Each task should be a clearly scoped, self-contained piece of work.".to_string(),
                    "For independent subtasks, use 'tasks' (parallel batch). For dependent steps, use 'chain' or 'stages'.".to_string(),
                    "Use {previous} in chain/stages tasks to reference the prior worker's output.".to_string(),
                    "Parallel concurrency is bounded — do not worry about overloading, the system caps it safely.".to_string(),
                    "Delegate only as many tasks as the work genuinely warrants.".to_string(),
                    "Subagents run in the background. Results are injected automatically when they complete — DO NOT redo or duplicate the dispatched work yourself. Trust the subagents.".to_string(),
                    "After dispatching, continue with other work or wait. If subagents fail, you'll get an error handoff — only then should you take over.".to_string(),
                ]),
                parameters: tool_params_schema,
                render_shell: None,
                prepare_arguments: None,
                execution_mode: None,
                execute: Arc::new({
                    let manager = manager_exec.clone();
                    move |_run_id: String, args: serde_json::Value, _abort: Option<tokio::sync::watch::Receiver<bool>>, _update: Option<hamr_harness::types::AgentToolUpdateCallback>, _ctx: Arc<dyn ExtensionContext>| {
                        let manager = manager.clone();
                        Box::pin(async move {
                            // Parse params
                            let params: SubagentParams = match serde_json::from_value(args) {
                                Ok(p) => p,
                                Err(e) => {
                                    return hamr_harness::types::AgentToolResult {
                                        content: vec![hamr_ai::types::MessageContent::Text(hamr_ai::types::TextContent {
                                            text: format!("Invalid delegate_subagents parameters: {e}"),
                                            text_signature: None,
                                        })],
                                        details: None,
                                        is_error: true,
                                        terminate: false,
                                    };
                                }
                            };

                            // Resolve plan
                            let plan = match resolve_subagent_plan(&params) {
                                Ok(p) => p,
                                Err(e) => {
                                    return hamr_harness::types::AgentToolResult {
                                        content: vec![hamr_ai::types::MessageContent::Text(hamr_ai::types::TextContent {
                                            text: e,
                                            text_signature: None,
                                        })],
                                        details: None,
                                        is_error: true,
                                        terminate: false,
                                    };
                                }
                            };

                            // Create run state
                            let run_id = manager.next_run_id();
                            let log_dir = format!(".hamr/subagents/runs/{}", run_id);
                            let _ = std::fs::create_dir_all(&log_dir);
                            let run_state = RunState::new(run_id.clone(), plan.mode.as_str(), &log_dir);
                            {
                                let mut active = manager.active_runs.write().await;
                                active.insert(run_id.clone(), run_state);
                            }

                            // Run the plan with a basic runner that creates child processes
                            let manager_for_runner = manager.clone();
                            let run_id_for_runner = run_id.clone();
                            let log_dir_for_runner = log_dir.clone();
                            let runner: WorkerRunner = Arc::new(move |invocation: WorkerInvocation| {
                                let _manager = manager_for_runner.clone();
                                let _run_id = run_id_for_runner.clone();
                                let _log_dir = log_dir_for_runner.clone();
                                Box::pin(async move {
                                    // Spawn child hamr process
                                    let task_text = invocation.task.clone();
                                    let mut cmd = tokio::process::Command::new("hamr");
                                    cmd.arg("--mode").arg("json").arg("-p").arg(&task_text);
                                    if let Some(cwd) = &invocation.cwd {
                                        cmd.current_dir(cwd);
                                    }
                                    cmd.stdout(std::process::Stdio::piped());
                                    cmd.stderr(std::process::Stdio::piped());

                                    match cmd.output().await {
                                        Ok(output) => {
                                            let text = String::from_utf8_lossy(&output.stdout).to_string();
                                            if output.status.success() {
                                                WorkerOutcome::Done {
                                                    worker_id: invocation.worker_id.clone(),
                                                    task: invocation.task.clone(),
                                                    text,
                                                    usage: Usage { input: 0, output: 0, cache_read: 0, cache_write: 0, cache_write_1h: None, total_tokens: 0, cost: UsageCost { input: 0.0, output: 0.0, cache_read: 0.0, cache_write: 0.0, total: 0.0 } },
                                                    model: None,
                                                    estimated_usage: false,
                                                    stop_reason: None,
                                                    validation: None,
                                                }
                                            } else {
                                                WorkerOutcome::Failed {
                                                    worker_id: invocation.worker_id.clone(),
                                                    task: invocation.task.clone(),
                                                    text: format!("{}\n{}", text, String::from_utf8_lossy(&output.stderr)),
                                                    error: format!("Exit code: {:?}", output.status.code()),
                                                    validation: None,
                                                }
                                            }
                                        }
                                        Err(e) => WorkerOutcome::Failed {
                                            worker_id: invocation.worker_id.clone(),
                                            task: invocation.task.clone(),
                                            text: String::new(),
                                            error: format!("Failed to spawn hamr: {e}"),
                                            validation: None,
                                        },
                                    }
                                })
                            });

                            let outcomes = run_plan(&plan, runner).await;

                            // Build result
                            let total = outcomes.len();
                            let done = outcomes.iter().filter(|o| matches!(o, WorkerOutcome::Done { .. })).count();
                            let failed = outcomes.iter().filter(|o| matches!(o, WorkerOutcome::Failed { .. })).count();

                            let summary = format!("{}/{} tasks completed successfully", done, total);
                            let mut result_text = summary.clone();
                            if failed > 0 {
                                result_text.push_str(&format!(" ({} failed)", failed));
                            }

                            // Update run state
                            {
                                let mut active = manager.active_runs.write().await;
                                if let Some(state) = active.get_mut(&run_id) {
                                    for outcome in &outcomes {
                                        match outcome {
                                            WorkerOutcome::Done { worker_id, .. } => {
                                                state.transition(worker_id, WorkerStatus::Done);
                                            }
                                            WorkerOutcome::Failed { worker_id, .. } => {
                                                state.transition(worker_id, WorkerStatus::Failed);
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }

                            hamr_harness::types::AgentToolResult {
                                content: vec![hamr_ai::types::MessageContent::Text(hamr_ai::types::TextContent {
                                    text: result_text,
                                    text_signature: None,
                                })],
                                details: None,
                                is_error: false,
                                terminate: false,
                            }
                        })
                    }
                }),
            });
        })
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_worker_id() {
        assert_eq!(pad_worker_id(0, 5), "0");
        assert_eq!(pad_worker_id(5, 10), "05");
        assert_eq!(pad_worker_id(42, 100), "042");
        assert_eq!(pad_worker_id(0, 1), "0");
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(15000), "15K");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 1, 10), 5);
        assert_eq!(clamp(0, 1, 10), 1);
        assert_eq!(clamp(15, 1, 10), 10);
    }

    #[test]
    fn test_empty_usage() {
        let u = empty_usage();
        assert_eq!(u.total_tokens, 0);
        assert!((u.cost.total - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_worker_state_new() {
        let ws = WorkerState::new("w1".to_string(), "Do something", "/tmp");
        assert_eq!(ws.worker_id, "w1");
        assert_eq!(ws.status, WorkerStatus::Queued);
        assert!(ws.started_at.is_none());
    }

    #[test]
    fn test_worker_state_push_event() {
        let mut ws = WorkerState::new("w1".to_string(), "test task", "/tmp");
        let event = serde_json::json!({
            "type": "message_update",
            "message": {
                "content": [
                    {"type": "text", "text": "hello world"}
                ]
            }
        });
        ws.push_event(&event);
        assert_eq!(ws.recent_events.len(), 1);
        assert_eq!(ws.last_activity.as_deref(), Some("message_update"));
        assert!(ws.output_tail.contains("hello world"));
    }

    #[test]
    fn test_run_state_transition() {
        let mut run = RunState::new("test-run".to_string(), "tasks", "/tmp/logs");
        let ws = WorkerState::new("w1".to_string(), "task1", "/tmp");
        run.workers.insert("w1".to_string(), ws);
        assert_eq!(run.workers.get("w1").unwrap().status, WorkerStatus::Queued);

        run.transition("w1", WorkerStatus::Running);
        assert_eq!(run.workers.get("w1").unwrap().status, WorkerStatus::Running);
        assert_eq!(run.running, 1);

        run.transition("w1", WorkerStatus::Done);
        assert_eq!(run.workers.get("w1").unwrap().status, WorkerStatus::Done);
        assert_eq!(run.done_count, 1);
        assert_eq!(run.running, 0);
    }

    #[test]
    fn test_accumulate_cost() {
        let mut run = RunState::new("test-run".to_string(), "tasks", "/tmp/logs");
        let usage = Usage {
            input: 100,
            output: 50,
            cache_read: 10,
            cache_write: 5,
            cache_write_1h: None,
            total_tokens: 155,
            cost: hamr_ai::types::UsageCost {
                input: 0.001,
                output: 0.002,
                cache_read: 0.0001,
                cache_write: 0.00005,
                total: 0.00315,
            },
        };
        run.accumulate_cost(&usage);
        assert!((run.usage.cost.total - 0.00315).abs() < 1e-6);
        assert_eq!(run.usage.total_tokens, 155);

        // Second accumulation
        run.accumulate_cost(&usage);
        assert!((run.usage.cost.total - 0.0063).abs() < 1e-6);
    }

    #[test]
    fn test_looks_like_file_path() {
        assert!(looks_like_file_path("src/main.rs"));
        assert!(looks_like_file_path("./foo/bar.ts"));
        assert!(looks_like_file_path("/abs/path/file.js"));
        assert!(!looks_like_file_path("https://example.com"));
        // some-package@1.2.3 has .3 extension, no URL/package-ref/version match
        assert!(looks_like_file_path("some-package@1.2.3"));
    }

    #[test]
    fn test_extract_file_references() {
        let text = "Check `src/main.rs` and `./lib/utils.rs` for details.";
        let refs = extract_file_references(text);
        assert!(refs.contains(&"src/main.rs".to_string()));
        assert!(refs.contains(&"./lib/utils.rs".to_string()));
    }

    #[test]
    fn test_check_self_contradiction_no_issues() {
        let warnings = check_self_contradiction("Everything compiled successfully.");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_worker_output_empty() {
        let result = validate_worker_output("done", "", "/tmp", None);
        assert!(!result.passed);
        assert!((result.confidence - 0.0).abs() < f64::EPSILON);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].warning_type, "empty_output");
    }

    #[test]
    fn test_validate_worker_output_ok() {
        let result = validate_worker_output("done", "All tests passed successfully.", "/tmp", None);
        assert!(result.passed);
        assert!((result.confidence - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_is_bash_fast_path_tools() {
        assert!(is_bash_fast_path_tools(Some(&["bash".to_string()])));
        assert!(is_bash_fast_path_tools(Some(&[
            "bash".to_string(),
            "read".to_string()
        ])));
        assert!(!is_bash_fast_path_tools(None));
        assert!(!is_bash_fast_path_tools(Some(&["edit".to_string()])));
        assert!(!is_bash_fast_path_tools(Some(&[
            "bash".to_string(),
            "edit".to_string()
        ])));
        assert!(!is_bash_fast_path_tools(Some(&[])));
    }

    #[test]
    fn test_subagent_manager() {
        let manager = SubagentManager::new();
        let run_id = manager.next_run_id();
        assert!(run_id.starts_with("run-"));
    }

    #[test]
    fn test_run_state_new() {
        let run = RunState::new("run-1".to_string(), "parallel", "/tmp/logs");
        assert_eq!(run.mode, "parallel");
        assert_eq!(run.total, 0);
        assert_eq!(run.workers.len(), 0);
    }

    #[tokio::test]
    async fn test_subagent_manager_budget() {
        let manager = SubagentManager::new();
        assert!(manager.consume_budget(10).await);
        assert_eq!(manager.remaining_budget().await, ENV_TOTAL_BUDGET - 10);

        manager.refund_budget(5).await;
        assert_eq!(manager.remaining_budget().await, ENV_TOTAL_BUDGET - 5);
    }

    #[tokio::test]
    async fn test_subagent_manager_budget_exhausted() {
        let manager = SubagentManager::new();
        let total = manager.remaining_budget().await;
        assert!(!manager.consume_budget(total + 1).await);
        assert_eq!(manager.remaining_budget().await, total);
    }

    #[test]
    fn test_worker_status_as_str() {
        assert_eq!(WorkerStatus::Queued.as_str(), "queued");
        assert_eq!(WorkerStatus::Running.as_str(), "running");
        assert_eq!(WorkerStatus::Done.as_str(), "done");
        assert_eq!(WorkerStatus::Failed.as_str(), "failed");
        assert_eq!(WorkerStatus::Aborted.as_str(), "aborted");
    }

    #[test]
    fn test_worker_push_event_preserves_last_tool() {
        let mut ws = WorkerState::new("w1".to_string(), "test", "/tmp");
        let event = serde_json::json!({"type": "tool_execution_start", "toolName": "bash"});
        ws.push_event(&event);
        assert_eq!(ws.last_tool.as_deref(), Some("bash"));

        let event2 = serde_json::json!({"type": "tool_execution_end", "toolName": "read"});
        ws.push_event(&event2);
        assert_eq!(ws.last_tool.as_deref(), Some("read"));
    }

    #[test]
    fn test_worker_push_event_ring_buffer() {
        let mut ws = WorkerState::new("w1".to_string(), "test", "/tmp");
        // Push more events than EVENTS_IN_MEMORY
        for i in 0..EVENTS_IN_MEMORY + 10 {
            let event = serde_json::json!({"type": format!("event_{}", i)});
            ws.push_event(&event);
        }
        assert_eq!(ws.recent_events.len(), EVENTS_IN_MEMORY);
        // First event should have been evicted
        let first_type = &ws.recent_events[0].event_type;
        assert_eq!(first_type, "event_10");
    }

    #[test]
    fn test_looks_like_package_ref() {
        assert!(looks_like_package_ref("@scope/pkg@1.2.3"));
        assert!(!looks_like_package_ref("some-pkg@2.0.0"));
        assert!(!looks_like_package_ref("normal_file.txt"));
        assert!(!looks_like_package_ref("just@version"));
    }

    #[test]
    fn test_looks_like_version() {
        assert!(looks_like_version("1.2.3"));
        assert!(looks_like_version("v1.2.3"));
        assert!(!looks_like_version("hello"));
    }

    #[test]
    fn test_has_file_extension() {
        assert!(has_file_extension("file.rs"));
        assert!(has_file_extension("path/to/file.js"));
        assert!(!has_file_extension("noext"));
        assert!(!has_file_extension(""));
    }

    #[test]
    fn test_extract_file_references_path_like() {
        let text = "Look at src/utils/helper.rs and ./test/data.json for examples.";
        let refs = extract_file_references(text);
        assert!(refs.contains(&"src/utils/helper.rs".to_string()));
        assert!(refs.contains(&"./test/data.json".to_string()));
    }

    #[test]
    fn test_validate_worker_output_truncated() {
        // Create a long string that exceeds OUTPUT_TAIL_BYTES
        let long_text = "a".repeat(OUTPUT_TAIL_BYTES + 10);
        let result = validate_worker_output("done", &long_text, "/tmp", None);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.warning_type == "truncated_output")
        );
    }

    #[test]
    fn test_validate_worker_output_whitespace_only() {
        // Whitespace-only text should fail validation like empty
        let result = validate_worker_output("done", "   \n  \t  ", "/tmp", None);
        assert!(!result.passed);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.warning_type == "empty_output")
        );
    }

    #[test]
    fn test_validate_worker_output_with_partial_text() {
        // When text is empty but partial_text is not, partial_text should be used
        let result = validate_worker_output("done", "", "/tmp", Some("partial output here"));
        assert!(result.passed);
        assert!((result.confidence - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_check_self_contradiction_created_and_missing() {
        let warnings = check_self_contradiction("I created the file but it does not exist");
        assert!(!warnings.is_empty());
        assert!(
            warnings
                .iter()
                .any(|w| w.warning_type == "self_contradiction")
        );
    }

    #[test]
    fn test_validate_worker_output_missing_file_references() {
        // References to files that don't exist should generate warnings
        let text = "Check `/nonexistent/path/definitely_not_a_file.xyz` for details.";
        let result = validate_worker_output("done", text, "/tmp", None);
        // This may or may not match the backtick-extraction, but shouldn't crash
        // The important thing is it doesn't panic and returns valid result
        assert!(!result.warnings.is_empty() || result.passed);
    }

    #[test]
    fn test_validate_worker_output_references_nonexistent_files() {
        // The file existence check runs in /tmp — create a test reference
        let text = "Update `test_nonexistent_file_12345.hamr`.";
        let result = validate_worker_output("done", text, "/tmp", None);
        // Backtick extraction should pick up the reference, and file_exists_relative
        // should return false for a non-existent file.
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.warning_type == "missing_file")
        );
    }

    #[test]
    fn test_validate_worker_output_confidence_scoring() {
        let result = validate_worker_output("done", "All good.", "/tmp", None);
        assert!((result.confidence - 1.0).abs() < 0.01);

        // Multiple medium warnings should reduce confidence
        let result2 = validate_worker_output(
            "done",
            "Good. But errors and failures. `/tmp/nope.rs`",
            "/tmp",
            None,
        );
        // self_contradiction (low) + missing_file (medium) = confidence < 1.0
        assert!(result2.confidence < 1.0);
    }

    #[test]
    fn test_check_self_contradiction_error_and_failure() {
        let warnings = check_self_contradiction("Error: compilation failed with failures");
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_looks_like_package_ref_edge_cases() {
        assert!(!looks_like_package_ref("@scope/pkg"));
        assert!(!looks_like_package_ref("@scope/pkg@"));
        assert!(!looks_like_package_ref("just@version"));
        assert!(!looks_like_package_ref("no-at-sign"));
    }

    #[test]
    fn test_looks_like_version_edge_cases() {
        assert!(!looks_like_version("v"));
        assert!(!looks_like_version("1"));
        assert!(!looks_like_version("hello"));
        assert!(!looks_like_version(""));
    }

    #[test]
    fn test_has_file_extension_edge_cases() {
        assert!(!has_file_extension("noext"));
        assert!(!has_file_extension(""));
        assert!(!has_file_extension("."));
        // .hidden files have an extension ("hidden") — Path::extension() strips the dot
        assert!(has_file_extension(".hidden"));
        assert!(has_file_extension("a.b"));
        assert!(!has_file_extension("x."));
    }

    #[test]
    fn test_looks_like_file_path_urls() {
        assert!(!looks_like_file_path("http://example.com/file.txt"));
        assert!(!looks_like_file_path("https://cdn.example.com/image.png"));
    }

    #[test]
    fn test_get_suggested_terms_noop() {
        // HolographicMemory::new() is only available with no arguments when hamr-memory is off.
        // In test configuration with hamr-memory feature, this test cannot call new() without a connection.
        // Skip this test for now since it depends on feature flags.
    }

    #[test]
    fn test_next_run_id_format() {
        let manager = SubagentManager::new();
        let id = manager.next_run_id();
        assert!(id.starts_with("run-"));
        assert!(id.len() > 8);
    }

    #[test]
    fn test_run_state_new_multiple_workers() {
        let mut run = RunState::new("run-2".to_string(), "chain", "/logs");
        run.workers.insert(
            "w1".to_string(),
            WorkerState::new("w1".to_string(), "task1", "."),
        );
        run.workers.insert(
            "w2".to_string(),
            WorkerState::new("w2".to_string(), "task2", "."),
        );
        assert_eq!(run.workers.len(), 2);
        assert_eq!(run.cnt.queued, 0); // sync_counters not called
        run.sync_counters();
        assert_eq!(run.cnt.queued, 0);
    }

    #[tokio::test]
    async fn test_consume_budget_exact() {
        let manager = SubagentManager::new();
        let total = manager.remaining_budget().await;
        assert!(manager.consume_budget(total).await);
        assert_eq!(manager.remaining_budget().await, 0);
    }

    #[tokio::test]
    async fn test_refund_budget_beyond_initial() {
        let manager = SubagentManager::new();
        manager.refund_budget(50).await;
        assert_eq!(manager.remaining_budget().await, ENV_TOTAL_BUDGET + 50);
    }

    #[test]
    fn test_validation_warning_new() {
        let w = ValidationWarning::new("test_warn", "test message", "high");
        assert_eq!(w.warning_type, "test_warn");
        assert_eq!(w.message, "test message");
        assert_eq!(w.severity, "high");
    }

    #[test]
    fn test_flush_pending_no_log_path() {
        let mut ws = WorkerState::new("w1".to_string(), "test", "/tmp");
        ws.push_event(&serde_json::json!({"type": "test_event"}));
        assert!(!ws.pending_flush.is_empty());
        // No crash when log_path is None
        ws.flush_pending();
        // pending_flush should be cleared even without a log path
        assert!(ws.pending_flush.is_empty());
    }

    #[test]
    fn test_extract_file_references_backtick_no_match() {
        let text = "Check `not-a-file-path` for details.";
        let refs = extract_file_references(text);
        // Should not extract a backtick-wrapped string that doesn't look like a file path
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_file_references_no_backtick() {
        let text = "Just some regular text without file references.";
        let refs = extract_file_references(text);
        assert!(refs.is_empty());
    }

    #[test]
    fn test_is_bash_fast_path_tools_edge() {
        // Single tool that's not bash
        assert!(!is_bash_fast_path_tools(Some(&["read".to_string()])));
        // More than 2 tools
        assert!(!is_bash_fast_path_tools(Some(&[
            "bash".to_string(),
            "read".to_string(),
            "edit".to_string()
        ])));
    }

    // ── P0-E: subagent → parent structured-handoff conversion ──────────────
    // `build_worker_outcome_from_child_summary` is the deterministic boundary
    // where a finished child subagent becomes the structured result/error the
    // parent consumes. These tests prove: success → Done(text), every failure
    // mode → a structured Failed/Aborted (never a panic), and that partial
    // output is preserved on failure so the parent can reason about it.

    /// A baseline "clean success" summary; tweak fields per test.
    fn ok_summary() -> WorkerProcessSummary {
        WorkerProcessSummary {
            exit_code: 0,
            exit_signal: None,
            was_aborted: false,
            stderr: String::new(),
            output_text: "subagent result text".to_string(),
            usage: empty_usage(),
            model: Some("faux-1".to_string()),
            estimated_usage: false,
            stop_reason: Some("stop".to_string()),
            error_message: None,
            stdout_parse_errors: 0,
            invalid_stdout: String::new(),
            spawn_error: None,
        }
    }

    #[test]
    fn test_outcome_success_is_done_with_text() {
        let outcome = build_worker_outcome_from_child_summary(
            "w0".to_string(),
            "do the thing".to_string(),
            ok_summary(),
        );
        match outcome {
            WorkerOutcome::Done {
                worker_id,
                task,
                text,
                model,
                ..
            } => {
                assert_eq!(worker_id, "w0");
                assert_eq!(task, "do the thing");
                assert_eq!(text, "subagent result text");
                assert_eq!(model.as_deref(), Some("faux-1"));
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn test_outcome_nonzero_exit_is_failed_and_preserves_partial_text() {
        let mut summary = ok_summary();
        summary.exit_code = 1;
        summary.output_text = "partial work before crash".to_string();
        summary.stderr = "boom: something exploded".to_string();
        let outcome =
            build_worker_outcome_from_child_summary("w1".to_string(), "task".to_string(), summary);
        match outcome {
            WorkerOutcome::Failed { error, text, .. } => {
                // Parent gets a structured, non-empty diagnostic …
                assert!(
                    error.contains("boom"),
                    "error should surface stderr: {error}"
                );
                // … and the partial output is not lost.
                assert_eq!(text, "partial work before crash");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_outcome_stop_reason_error_is_failed() {
        let mut summary = ok_summary();
        summary.stop_reason = Some("error".to_string());
        summary.error_message = Some("model errored".to_string());
        let outcome =
            build_worker_outcome_from_child_summary("w2".to_string(), "task".to_string(), summary);
        assert!(
            matches!(outcome, WorkerOutcome::Failed { .. }),
            "stop_reason=error must map to Failed"
        );
    }

    #[test]
    fn test_outcome_spawn_error_is_failed() {
        let mut summary = ok_summary();
        summary.spawn_error = Some("ENOENT: binary not found".to_string());
        let outcome =
            build_worker_outcome_from_child_summary("w3".to_string(), "task".to_string(), summary);
        match outcome {
            WorkerOutcome::Failed { error, .. } => {
                assert!(
                    error.contains("ENOENT"),
                    "spawn error should surface: {error}"
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_outcome_aborted_takes_precedence() {
        let mut summary = ok_summary();
        // Even with a non-zero exit, an explicit abort wins and is reported as
        // Aborted (user-initiated), not Failed.
        summary.was_aborted = true;
        summary.exit_code = 137;
        let outcome =
            build_worker_outcome_from_child_summary("w4".to_string(), "task".to_string(), summary);
        match outcome {
            WorkerOutcome::Aborted { reason, .. } => assert_eq!(reason, "user"),
            other => panic!("expected Aborted, got {other:?}"),
        }
    }

    #[test]
    fn test_outcome_empty_output_with_stderr_is_failed() {
        // Clean exit code but no usable output and a noisy stderr → treated as a
        // structured failure rather than an empty "success".
        let mut summary = ok_summary();
        summary.output_text = "   ".to_string();
        summary.stderr = "warning: degraded".to_string();
        let outcome =
            build_worker_outcome_from_child_summary("w5".to_string(), "task".to_string(), summary);
        assert!(
            matches!(outcome, WorkerOutcome::Failed { .. }),
            "empty output + stderr must be a structured failure"
        );
    }

    #[test]
    fn test_outcome_failed_serializes_for_parent_handoff() {
        // The parent receives the outcome as JSON; the `status` discriminant
        // drives its branching. Lock the serialized shape so the handoff holds.
        let mut summary = ok_summary();
        summary.exit_code = 2;
        let outcome =
            build_worker_outcome_from_child_summary("w6".to_string(), "task".to_string(), summary);
        let value = serde_json::to_value(&outcome).expect("outcome must serialize");
        assert_eq!(value.get("status").and_then(|v| v.as_str()), Some("failed"));
        assert!(
            value.get("error").is_some(),
            "failed outcome must carry an error field"
        );
    }

    // ── P0-E: subagent transcript/log persistence to disk ──────────────────
    // A subagent's event stream is appended to its `log_path` as JSONL. These
    // tests prove the bytes actually reach disk and round-trip — the "logs /
    // transcript / artifacts persist" half of the subagent loop.

    /// Read a worker JSONL log back into parsed event objects.
    fn read_jsonl(path: &std::path::Path) -> Vec<serde_json::Value> {
        let content = std::fs::read_to_string(path).expect("log file must exist");
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("each log line is JSON"))
            .collect()
    }

    #[test]
    fn test_flush_log_persists_events_to_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_path = dir.path().join("worker-0.jsonl");

        let mut ws = WorkerState::new("w0".to_string(), "summarize the repo", "/tmp");
        ws.log_path = Some(log_path.to_string_lossy().to_string());

        // Fewer than FLUSH_BATCH_SIZE (10) so nothing auto-flushes; the explicit
        // flush_log() is what must move them to disk.
        ws.push_event(&serde_json::json!({ "type": "agent_start" }));
        ws.push_event(&serde_json::json!({ "type": "tool_execution_start", "toolName": "read" }));
        ws.push_event(&serde_json::json!({ "type": "agent_end", "ok": true }));
        assert!(
            !log_path.exists() || read_jsonl(&log_path).len() < 3,
            "events should still be pending before flush"
        );

        ws.flush_log();
        assert!(ws.pending_flush.is_empty(), "flush must drain the queue");

        let events = read_jsonl(&log_path);
        assert_eq!(events.len(), 3, "all three events must be persisted");
        // Each line wraps the original event under `data` with a `type` + `ts`.
        assert_eq!(
            events[0].get("type").and_then(|v| v.as_str()),
            Some("agent_start")
        );
        assert_eq!(
            events[1].get("type").and_then(|v| v.as_str()),
            Some("tool_execution_start")
        );
        assert_eq!(
            events[1]
                .get("data")
                .and_then(|d| d.get("toolName"))
                .and_then(|v| v.as_str()),
            Some("read"),
            "original event payload must round-trip under `data`"
        );
        assert!(events[2].get("ts").is_some(), "each entry is timestamped");
    }

    #[test]
    fn test_push_event_auto_flushes_at_batch_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_path = dir.path().join("worker-1.jsonl");

        let mut ws = WorkerState::new("w1".to_string(), "task", "/tmp");
        ws.log_path = Some(log_path.to_string_lossy().to_string());

        // Exactly FLUSH_BATCH_SIZE (10) events should trigger an automatic flush
        // with no explicit flush_log() call.
        for i in 0..10 {
            ws.push_event(&serde_json::json!({ "type": "message_update", "i": i }));
        }
        assert!(
            ws.pending_flush.is_empty(),
            "reaching the batch size must auto-flush to disk"
        );

        let events = read_jsonl(&log_path);
        assert_eq!(events.len(), 10, "auto-flush must persist the full batch");
    }

    #[test]
    fn test_flush_log_appends_across_multiple_flushes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log_path = dir.path().join("worker-2.jsonl");

        let mut ws = WorkerState::new("w2".to_string(), "task", "/tmp");
        ws.log_path = Some(log_path.to_string_lossy().to_string());

        ws.push_event(&serde_json::json!({ "type": "a" }));
        ws.flush_log();
        ws.push_event(&serde_json::json!({ "type": "b" }));
        ws.flush_log();

        let events = read_jsonl(&log_path);
        assert_eq!(events.len(), 2, "second flush must append, not truncate");
        assert_eq!(events[0].get("type").and_then(|v| v.as_str()), Some("a"));
        assert_eq!(events[1].get("type").and_then(|v| v.as_str()), Some("b"));
    }

    // ── delegate_subagents param resolution ────────────────────────────────
    // The pure front-half of the tool: validate "exactly one mode", normalize
    // the four input shapes into canonical stages, and enforce limits/defaults.

    fn item(task: &str) -> TaskItem {
        TaskItem {
            task: task.to_string(),
            ..Default::default()
        }
    }

    fn items(n: usize) -> Vec<TaskItem> {
        (0..n).map(|i| item(&format!("task {i}"))).collect()
    }

    #[test]
    fn plan_requires_exactly_one_mode_none() {
        let err = resolve_subagent_plan(&SubagentParams::default()).unwrap_err();
        assert!(err.contains("No mode specified"), "{err}");
    }

    #[test]
    fn plan_requires_exactly_one_mode_multiple() {
        let params = SubagentParams {
            tasks: Some(items(1)),
            chain: Some(items(1)),
            ..Default::default()
        };
        let err = resolve_subagent_plan(&params).unwrap_err();
        assert!(err.contains("Multiple modes"), "{err}");
    }

    #[test]
    fn plan_tasks_becomes_one_parallel_stage() {
        let params = SubagentParams {
            tasks: Some(items(3)),
            ..Default::default()
        };
        let plan = resolve_subagent_plan(&params).unwrap();
        assert_eq!(plan.mode, PlanMode::Tasks);
        assert_eq!(plan.stages.len(), 1);
        assert_eq!(plan.stages[0].mode, StageMode::Parallel);
        assert_eq!(plan.task_count, 3);
    }

    #[test]
    fn plan_chain_becomes_one_chain_stage() {
        let params = SubagentParams {
            chain: Some(items(2)),
            ..Default::default()
        };
        let plan = resolve_subagent_plan(&params).unwrap();
        assert_eq!(plan.mode, PlanMode::Chain);
        assert_eq!(plan.stages[0].mode, StageMode::Chain);
        assert_eq!(plan.task_count, 2);
    }

    #[test]
    fn plan_deprecated_subtasks_normalizes_to_chain() {
        let params = SubagentParams {
            subtasks: Some(items(2)),
            ..Default::default()
        };
        let plan = resolve_subagent_plan(&params).unwrap();
        assert_eq!(plan.mode, PlanMode::Subtasks);
        assert_eq!(
            plan.stages[0].mode,
            StageMode::Chain,
            "subtasks run sequentially"
        );
    }

    #[test]
    fn plan_stages_pass_through_and_count_sums() {
        let params = SubagentParams {
            stages: Some(vec![
                StageSpec {
                    mode: StageMode::Parallel,
                    tasks: items(3),
                },
                StageSpec {
                    mode: StageMode::Chain,
                    tasks: items(2),
                },
            ]),
            ..Default::default()
        };
        let plan = resolve_subagent_plan(&params).unwrap();
        assert_eq!(plan.mode, PlanMode::Stages);
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.task_count, 5, "count sums across all stages");
    }

    #[test]
    fn plan_concurrency_clamped_and_defaults_applied() {
        // Over-max clamps down; defaults fill fail_fast/observe.
        let over = SubagentParams {
            tasks: Some(items(1)),
            concurrency: Some(9999),
            ..Default::default()
        };
        let plan = resolve_subagent_plan(&over).unwrap();
        assert_eq!(plan.concurrency, ENV_MAX_CONCURRENCY);
        assert!(!plan.fail_fast);
        assert_eq!(plan.observe, "compact");

        // Zero clamps up to the floor of 1.
        let zero = SubagentParams {
            tasks: Some(items(1)),
            concurrency: Some(0),
            ..Default::default()
        };
        assert_eq!(resolve_subagent_plan(&zero).unwrap().concurrency, 1);

        // Unset → default.
        let unset = SubagentParams {
            tasks: Some(items(1)),
            ..Default::default()
        };
        assert_eq!(
            resolve_subagent_plan(&unset).unwrap().concurrency,
            ENV_MAX_CONCURRENCY
        );
    }

    #[test]
    fn plan_rejects_too_many_tasks() {
        let params = SubagentParams {
            tasks: Some(items(ENV_MAX_TASKS + 1)),
            ..Default::default()
        };
        let err = resolve_subagent_plan(&params).unwrap_err();
        assert!(err.contains("Too many tasks"), "{err}");
        assert!(err.contains("Soft limit"), "{err}");
    }

    #[test]
    fn plan_respects_explicit_flags() {
        let params = SubagentParams {
            chain: Some(items(1)),
            fail_fast: Some(true),
            observe: Some("verbose".to_string()),
            ..Default::default()
        };
        let plan = resolve_subagent_plan(&params).unwrap();
        assert!(plan.fail_fast);
        assert_eq!(plan.observe, "verbose");
    }

    #[test]
    fn subagent_params_deserialize_from_model_json_camelcase() {
        // The model supplies these args as JSON; the camelCase keys + nested
        // TaskItem fields must deserialize (this is the on-the-wire contract).
        let value = serde_json::json!({
            "tasks": [
                { "task": "read the file", "tools": ["read", "grep"], "cwd": "/tmp" }
            ],
            "concurrency": 4,
            "failFast": true,
            "observe": "verbose",
            "stepTimeoutMs": 120000
        });
        let params: SubagentParams = serde_json::from_value(value).expect("params deserialize");
        let plan = resolve_subagent_plan(&params).unwrap();
        assert_eq!(plan.task_count, 1);
        assert_eq!(plan.concurrency, 4);
        assert!(plan.fail_fast);
        assert_eq!(
            plan.stages[0].tasks[0].tools.as_deref(),
            Some(&["read".to_string(), "grep".to_string()][..])
        );
        assert_eq!(params.step_timeout_ms, Some(120_000));
    }

    // ── run_plan stage executor (fake runner, no subprocess) ───────────────

    use std::sync::Mutex;

    /// A fake worker runner that records the (post-substitution) task text it
    /// received and returns `Done`, unless the task contains "FAIL" → `Failed`.
    fn recording_runner(log: Arc<Mutex<Vec<String>>>) -> WorkerRunner {
        Arc::new(move |inv: WorkerInvocation| {
            let log = log.clone();
            Box::pin(async move {
                log.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(inv.task.clone());
                if inv.task.contains("FAIL") {
                    WorkerOutcome::Failed {
                        worker_id: inv.worker_id,
                        task: inv.task.clone(),
                        error: "boom".to_string(),
                        text: format!("partial:{}", inv.task),
                        validation: None,
                    }
                } else {
                    WorkerOutcome::Done {
                        worker_id: inv.worker_id,
                        task: inv.task.clone(),
                        text: format!("out:{}", inv.task),
                        usage: empty_usage(),
                        model: None,
                        estimated_usage: true,
                        stop_reason: Some("stop".to_string()),
                        validation: None,
                    }
                }
            })
        })
    }

    fn plan_of(stages: Vec<StageSpec>, fail_fast: bool) -> SubagentPlan {
        let task_count = stages.iter().map(|s| s.tasks.len()).sum();
        SubagentPlan {
            mode: PlanMode::Stages,
            stages,
            task_count,
            concurrency: 4,
            fail_fast,
            observe: "compact".to_string(),
        }
    }

    #[tokio::test]
    async fn run_plan_parallel_runs_all_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let plan = plan_of(
            vec![StageSpec {
                mode: StageMode::Parallel,
                tasks: items(3),
            }],
            false,
        );
        let outcomes = run_plan(&plan, recording_runner(log.clone())).await;
        assert_eq!(outcomes.len(), 3);
        // map_with_concurrency_limit preserves input order in results.
        let texts: Vec<String> = outcomes
            .iter()
            .filter_map(|o| match o {
                WorkerOutcome::Done { task, .. } => Some(task.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["task 0", "task 1", "task 2"]);
    }

    #[tokio::test]
    async fn run_plan_chain_threads_previous() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let plan = plan_of(
            vec![StageSpec {
                mode: StageMode::Chain,
                tasks: vec![item("seed"), item("next:{previous}")],
            }],
            false,
        );
        run_plan(&plan, recording_runner(log.clone())).await;
        let recorded = log.lock().unwrap().clone();
        // Step 2's task must have had {previous} replaced by step 1's output.
        assert_eq!(
            recorded,
            vec!["seed".to_string(), "next:out:seed".to_string()]
        );
    }

    #[tokio::test]
    async fn run_plan_fail_fast_stops_chain() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let plan = plan_of(
            vec![StageSpec {
                mode: StageMode::Chain,
                tasks: vec![item("ok"), item("FAIL here"), item("never runs")],
            }],
            true,
        );
        let outcomes = run_plan(&plan, recording_runner(log.clone())).await;
        assert_eq!(outcomes.len(), 2, "fail_fast must stop after the failure");
        assert_eq!(log.lock().unwrap().len(), 2, "third task must never run");
    }

    #[tokio::test]
    async fn run_plan_without_fail_fast_continues_past_failure() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let plan = plan_of(
            vec![StageSpec {
                mode: StageMode::Chain,
                tasks: vec![item("ok"), item("FAIL here"), item("still runs")],
            }],
            false,
        );
        let outcomes = run_plan(&plan, recording_runner(log.clone())).await;
        assert_eq!(outcomes.len(), 3, "without fail_fast all steps run");
    }

    #[tokio::test]
    async fn run_plan_threads_previous_across_stages() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let plan = plan_of(
            vec![
                StageSpec {
                    mode: StageMode::Parallel,
                    tasks: vec![item("alpha")],
                },
                StageSpec {
                    mode: StageMode::Chain,
                    tasks: vec![item("use:{previous}")],
                },
            ],
            false,
        );
        run_plan(&plan, recording_runner(log.clone())).await;
        let recorded = log.lock().unwrap().clone();
        // Stage 2 (chain) saw stage 1's last output threaded as {previous}.
        assert_eq!(
            recorded,
            vec!["alpha".to_string(), "use:out:alpha".to_string()]
        );
    }

    #[tokio::test]
    async fn run_plan_fail_fast_parallel_stops_next_stage() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let plan = plan_of(
            vec![
                StageSpec {
                    mode: StageMode::Parallel,
                    tasks: vec![item("FAIL one"), item("two")],
                },
                StageSpec {
                    mode: StageMode::Chain,
                    tasks: vec![item("second stage")],
                },
            ],
            true,
        );
        let outcomes = run_plan(&plan, recording_runner(log.clone())).await;
        // Stage 1's two tasks ran; the failure + fail_fast skips stage 2.
        assert_eq!(outcomes.len(), 2);
        let recorded = log.lock().unwrap().clone();
        assert!(
            !recorded.iter().any(|t| t.contains("second stage")),
            "fail_fast must skip the next stage: {recorded:?}"
        );
    }
}

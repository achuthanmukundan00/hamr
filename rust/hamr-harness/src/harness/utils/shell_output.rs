//! Port of `packages/agent/src/harness/utils/shell-output.ts`.

use crate::harness::types::{
    ExecResult, ExecutionEnv, ExecutionEnvExecOptions, ExecutionError, ExecutionErrorCode,
};
use crate::harness::utils::truncate::{DEFAULT_MAX_BYTES, TruncationOptions, truncate_tail};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ShellCaptureOptions {
    pub exec_options: ExecutionEnvExecOptions,
    pub on_chunk: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl Default for ShellCaptureOptions {
    fn default() -> Self {
        Self {
            exec_options: ExecutionEnvExecOptions::default(),
            on_chunk: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCaptureResult {
    pub output: String,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub truncated: bool,
    pub full_output_path: Option<String>,
}

fn to_execution_error(error: impl std::fmt::Display) -> ExecutionError {
    ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string())
}

fn signal_aborted(signal: Option<&tokio::sync::watch::Receiver<bool>>) -> bool {
    signal.map(|signal| *signal.borrow()).unwrap_or(false)
}

pub fn sanitize_binary_output(input: &str) -> String {
    input
        .chars()
        .filter(|ch| {
            let code = *ch as u32;
            if code == 0x09 || code == 0x0A || code == 0x0D {
                return true;
            }
            if code <= 0x1F {
                return false;
            }
            !(0xFFF9..=0xFFFB).contains(&code)
        })
        .collect()
}

pub async fn execute_shell_with_capture(
    env: &dyn ExecutionEnv,
    command: &str,
    options: Option<ShellCaptureOptions>,
) -> Result<ShellCaptureResult, ExecutionError> {
    let options = options.unwrap_or_default();
    let max_output_bytes = DEFAULT_MAX_BYTES * 2;

    let output_chunks = Arc::new(Mutex::new(Vec::<String>::new()));
    let output_bytes = Arc::new(Mutex::new(0_usize));
    let total_bytes = Arc::new(Mutex::new(0_usize));
    let full_output_path = Arc::new(Mutex::new(None::<String>));
    let capture_error = Arc::new(Mutex::new(None::<ExecutionError>));

    let append_full_output = {
        let full_output_path = Arc::clone(&full_output_path);
        let capture_error = Arc::clone(&capture_error);
        move |text: String| {
            let path = full_output_path.lock().ok().and_then(|guard| guard.clone());
            let has_error = capture_error
                .lock()
                .ok()
                .and_then(|guard| guard.clone())
                .is_some();
            (path, has_error, text)
        }
    };

    let on_chunk = {
        let output_chunks = Arc::clone(&output_chunks);
        let output_bytes = Arc::clone(&output_bytes);
        let total_bytes = Arc::clone(&total_bytes);
        let full_output_path = Arc::clone(&full_output_path);
        let capture_error = Arc::clone(&capture_error);
        let on_chunk = options.on_chunk.clone();
        Arc::new(move |chunk: String| {
            let mut total_guard = total_bytes.lock().unwrap();
            *total_guard += chunk.len();

            let text = sanitize_binary_output(&chunk).replace('\r', "");

            if *total_guard > DEFAULT_MAX_BYTES {
                let mut full_output_guard = full_output_path.lock().unwrap();
                if full_output_guard.is_none() {
                    let initial_content = {
                        let chunks = output_chunks.lock().unwrap();
                        format!("{}{}", chunks.join(""), text)
                    };
                    *full_output_guard = Some(initial_content);
                } else {
                    let (path, has_error, append_text) = append_full_output(text.clone());
                    if path.is_some() && has_error {
                        *capture_error.lock().unwrap() =
                            Some(to_execution_error("capture error while appending output"));
                    }
                    if let Some(existing) = full_output_guard.as_mut() {
                        existing.push_str(&append_text);
                    }
                }
            }

            {
                let mut chunks = output_chunks.lock().unwrap();
                let mut bytes = output_bytes.lock().unwrap();
                chunks.push(text.clone());
                *bytes += text.len();
                while *bytes > max_output_bytes && chunks.len() > 1 {
                    if let Some(removed) = chunks.first().cloned() {
                        chunks.remove(0);
                        *bytes = bytes.saturating_sub(removed.len());
                    } else {
                        break;
                    }
                }
            }

            if let Some(callback) = &on_chunk {
                callback(text);
            }
        }) as Arc<dyn Fn(String) + Send + Sync>
    };

    let mut exec_options = options.exec_options.clone();
    exec_options.on_stdout = Some(Arc::clone(&on_chunk));
    exec_options.on_stderr = Some(on_chunk);

    let result = env.exec(command, Some(exec_options.clone())).await;

    let tail_output = output_chunks.lock().unwrap().join("");
    let truncation = truncate_tail(&tail_output, TruncationOptions::default());

    if truncation.truncated && full_output_path.lock().unwrap().is_none() {
        *full_output_path.lock().unwrap() = Some(tail_output.clone());
    }

    let pending_full_output = full_output_path.lock().unwrap().clone();
    let mut persisted_full_output_path = None;
    if let Some(full_output) = pending_full_output {
        let temp_path = env
            .create_temp_file("bash-", ".log", exec_options.abort_signal.clone())
            .await
            .map_err(|error| to_execution_error(error.message))?;
        env.append_file(
            &temp_path,
            full_output.as_bytes(),
            exec_options.abort_signal.clone(),
        )
        .await
        .map_err(|error| ExecutionError::new(ExecutionErrorCode::Unknown, error.message.clone()))?;
        persisted_full_output_path = Some(temp_path);
    }

    if let Some(error) = capture_error.lock().unwrap().clone() {
        return Err(error);
    }

    match result {
        Ok(ExecResult { exit_code, .. }) => {
            let cancelled = signal_aborted(exec_options.abort_signal.as_ref());
            Ok(ShellCaptureResult {
                output: if truncation.truncated {
                    truncation.content
                } else {
                    tail_output
                },
                exit_code: if cancelled { None } else { Some(exit_code) },
                cancelled,
                truncated: truncation.truncated,
                full_output_path: persisted_full_output_path,
            })
        }
        Err(error)
            if error.code == ExecutionErrorCode::Aborted
                || signal_aborted(exec_options.abort_signal.as_ref()) =>
        {
            Ok(ShellCaptureResult {
                output: if truncation.truncated {
                    truncation.content
                } else {
                    tail_output
                },
                exit_code: None,
                cancelled: true,
                truncated: truncation.truncated,
                full_output_path: persisted_full_output_path,
            })
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{ShellCaptureOptions, execute_shell_with_capture, sanitize_binary_output};
    use crate::harness::env::nodejs::NodeExecutionEnv;
    use tokio::sync::watch;

    #[test]
    fn strips_control_markers_but_keeps_whitespace() {
        let sanitized = sanitize_binary_output("a\u{0000}b\tc\n\u{FFFA}d\r");
        assert_eq!(sanitized, "ab\tc\nd\r");
    }

    #[tokio::test]
    async fn captures_shell_output() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        let result = execute_shell_with_capture(
            &env,
            "printf 'hello\\nworld\\n'",
            Some(ShellCaptureOptions::default()),
        )
        .await
        .unwrap();

        assert_eq!(result.output, "hello\nworld\n");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.cancelled);
        assert!(!result.truncated);
        assert!(result.full_output_path.is_none());
    }

    #[tokio::test]
    async fn returns_cancelled_result_for_aborts() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        let (tx, rx) = watch::channel(false);
        tx.send(true).unwrap();

        let mut options = ShellCaptureOptions::default();
        options.exec_options.abort_signal = Some(rx);

        let result = execute_shell_with_capture(&env, "sleep 0.1", Some(options))
            .await
            .unwrap();
        assert!(result.cancelled);
        assert_eq!(result.exit_code, None);
    }
}

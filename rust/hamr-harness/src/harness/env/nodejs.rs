//! Port of `packages/agent/src/harness/env/nodejs.ts`.

use crate::harness::types::{
    ExecResult, ExecutionEnvExecOptions, ExecutionError, ExecutionErrorCode, FileError,
    FileErrorCode, FileInfo, FileKind, FileSystem, Shell,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tokio::fs;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::watch;

fn resolve_path(cwd: &Path, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        cwd.join(candidate)
    }
}

fn file_kind_from_metadata(metadata: &std::fs::Metadata) -> Option<FileKind> {
    let file_type = metadata.file_type();
    if file_type.is_file() {
        Some(FileKind::File)
    } else if file_type.is_dir() {
        Some(FileKind::Directory)
    } else if file_type.is_symlink() {
        Some(FileKind::Symlink)
    } else {
        None
    }
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn mtime_ms(metadata: &std::fs::Metadata) -> f64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

fn file_info_from_metadata(
    path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<FileInfo, FileError> {
    let Some(kind) = file_kind_from_metadata(metadata) else {
        return Err(
            FileError::new(FileErrorCode::Invalid, "Unsupported file type")
                .with_path(path.to_string_lossy()),
        );
    };

    Ok(FileInfo {
        name: path_name(path),
        path: path.to_string_lossy().into_owned(),
        kind,
        size: metadata.len(),
        mtime_ms: mtime_ms(metadata),
    })
}

fn to_file_error(error: &std::io::Error, path: Option<&Path>) -> FileError {
    let code = match error.kind() {
        std::io::ErrorKind::NotFound => FileErrorCode::NotFound,
        std::io::ErrorKind::PermissionDenied => FileErrorCode::PermissionDenied,
        std::io::ErrorKind::NotADirectory => FileErrorCode::NotDirectory,
        std::io::ErrorKind::IsADirectory => FileErrorCode::IsDirectory,
        std::io::ErrorKind::InvalidInput => FileErrorCode::Invalid,
        _ => FileErrorCode::Unknown,
    };

    let mut file_error = FileError::new(code, error.to_string());
    if let Some(path) = path {
        file_error = file_error.with_path(path.to_string_lossy());
    }
    file_error
}

fn signal_aborted(signal: Option<&watch::Receiver<bool>>) -> bool {
    signal.map(|signal| *signal.borrow()).unwrap_or(false)
}

fn check_abort(
    signal: Option<&watch::Receiver<bool>>,
    path: Option<&Path>,
) -> Result<(), FileError> {
    if signal_aborted(signal) {
        let mut error = FileError::new(FileErrorCode::Aborted, "aborted");
        if let Some(path) = path {
            error = error.with_path(path.to_string_lossy());
        }
        return Err(error);
    }
    Ok(())
}

async fn wait_for_abort(signal: &mut watch::Receiver<bool>) {
    loop {
        if *signal.borrow() {
            return;
        }
        if signal.changed().await.is_err() {
            return;
        }
    }
}

async fn read_stream<R>(
    mut reader: R,
    callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
) -> Result<String, ExecutionError>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];

    loop {
        let count = reader
            .read(&mut chunk)
            .await
            .map_err(|error| ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string()))?;
        if count == 0 {
            break;
        }

        bytes.extend_from_slice(&chunk[..count]);

        if let Some(callback) = &callback {
            let text = String::from_utf8_lossy(&chunk[..count]).into_owned();
            let callback_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback(text);
            }));
            if callback_result.is_err() {
                return Err(ExecutionError::new(
                    ExecutionErrorCode::CallbackError,
                    "callback panicked",
                ));
            }
        }
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn path_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

async fn default_shell_config(
    custom_shell_path: Option<&str>,
) -> Result<(String, Vec<&'static str>), ExecutionError> {
    if let Some(custom_shell_path) = custom_shell_path {
        let path = Path::new(custom_shell_path);
        if path_exists(path).await {
            return Ok((custom_shell_path.to_string(), vec!["-c"]));
        }
        return Err(ExecutionError::new(
            ExecutionErrorCode::ShellUnavailable,
            format!("Custom shell path not found: {custom_shell_path}"),
        ));
    }

    #[cfg(windows)]
    {
        return Ok(("cmd".to_string(), vec!["/C"]));
    }

    #[cfg(not(windows))]
    {
        let bash = Path::new("/bin/bash");
        if path_exists(bash).await {
            return Ok((bash.to_string_lossy().into_owned(), vec!["-c"]));
        }
        Ok(("sh".to_string(), vec!["-c"]))
    }
}

/// Local execution environment backed by the host filesystem and shell.
#[derive(Debug, Clone)]
pub struct NodeExecutionEnv {
    cwd: PathBuf,
    shell_path: Option<String>,
    shell_env: Option<std::collections::HashMap<String, String>>,
}

impl NodeExecutionEnv {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            shell_path: None,
            shell_env: None,
        }
    }

    pub fn with_options(
        cwd: impl Into<PathBuf>,
        shell_path: Option<String>,
        shell_env: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        Self {
            cwd: cwd.into(),
            shell_path,
            shell_env,
        }
    }
}

#[async_trait]
impl Shell for NodeExecutionEnv {
    async fn exec(
        &self,
        command: &str,
        options: Option<ExecutionEnvExecOptions>,
    ) -> Result<ExecResult, ExecutionError> {
        let options = options.unwrap_or_default();
        if signal_aborted(options.abort_signal.as_ref()) {
            return Err(ExecutionError::new(ExecutionErrorCode::Aborted, "aborted"));
        }

        let cwd = options
            .cwd
            .as_deref()
            .map(|cwd| resolve_path(&self.cwd, cwd))
            .unwrap_or_else(|| self.cwd.clone());
        let (shell, shell_args) = default_shell_config(self.shell_path.as_deref()).await?;

        let mut process = Command::new(shell);
        process
            .args(shell_args)
            .arg(command)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        for (key, value) in std::env::vars() {
            process.env(&key, &value);
        }
        if let Some(shell_env) = &self.shell_env {
            for (key, value) in shell_env {
                process.env(key, value);
            }
        }
        if let Some(extra_env) = &options.env {
            for (key, value) in extra_env {
                process.env(key, value);
            }
        }

        let mut child = process.spawn().map_err(|error| {
            ExecutionError::new(ExecutionErrorCode::SpawnError, error.to_string())
        })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_task =
            stdout.map(|stdout| tokio::spawn(read_stream(stdout, options.on_stdout.clone())));
        let stderr_task =
            stderr.map(|stderr| tokio::spawn(read_stream(stderr, options.on_stderr.clone())));

        let status = match (options.abort_signal.clone(), options.timeout) {
            (None, None) => child.wait().await.map_err(|error| {
                ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string())
            })?,
            (None, Some(timeout_secs)) => {
                let duration = Duration::from_secs_f64(timeout_secs);
                match tokio::time::timeout(duration, child.wait()).await {
                    Ok(result) => result.map_err(|error| {
                        ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string())
                    })?,
                    Err(_) => {
                        let _ = child.kill().await;
                        return Err(ExecutionError::new(
                            ExecutionErrorCode::Timeout,
                            format!("timeout:{timeout_secs}"),
                        ));
                    }
                }
            }
            (Some(mut signal), None) => {
                tokio::select! {
                    result = child.wait() => {
                        result.map_err(|error| ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string()))?
                    }
                    _ = wait_for_abort(&mut signal) => {
                        let _ = child.kill().await;
                        return Err(ExecutionError::new(ExecutionErrorCode::Aborted, "aborted"));
                    }
                }
            }
            (Some(mut signal), Some(timeout_secs)) => {
                let duration = Duration::from_secs_f64(timeout_secs);
                tokio::select! {
                    result = child.wait() => {
                        result.map_err(|error| ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string()))?
                    }
                    _ = wait_for_abort(&mut signal) => {
                        let _ = child.kill().await;
                        return Err(ExecutionError::new(ExecutionErrorCode::Aborted, "aborted"));
                    }
                    _ = tokio::time::sleep(duration) => {
                        let _ = child.kill().await;
                        return Err(ExecutionError::new(
                            ExecutionErrorCode::Timeout,
                            format!("timeout:{timeout_secs}"),
                        ));
                    }
                }
            }
        };

        let stdout = match stdout_task {
            Some(task) => task.await.map_err(|error| {
                ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string())
            })??,
            None => String::new(),
        };
        let stderr = match stderr_task {
            Some(task) => task.await.map_err(|error| {
                ExecutionError::new(ExecutionErrorCode::Unknown, error.to_string())
            })??,
            None => String::new(),
        };

        Ok(ExecResult {
            stdout,
            stderr,
            exit_code: status.code().unwrap_or(0),
        })
    }

    async fn cleanup(&self) {}
}

#[async_trait]
impl FileSystem for NodeExecutionEnv {
    fn cwd(&self) -> &str {
        self.cwd.to_str().unwrap_or("")
    }

    async fn absolute_path(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<String, FileError> {
        check_abort(abort_signal.as_ref(), None)?;
        Ok(resolve_path(&self.cwd, path).to_string_lossy().into_owned())
    }

    async fn join_path(
        &self,
        parts: &[String],
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<String, FileError> {
        check_abort(abort_signal.as_ref(), None)?;
        let mut path = PathBuf::new();
        for part in parts {
            path.push(part);
        }
        Ok(path.to_string_lossy().into_owned())
    }

    async fn read_text_file(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<String, FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let content = fs::read_to_string(&resolved)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        Ok(content)
    }

    async fn read_text_lines(
        &self,
        path: &str,
        max_lines: Option<usize>,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<Vec<String>, FileError> {
        if matches!(max_lines, Some(0)) {
            return Ok(Vec::new());
        }

        let content = self.read_text_file(path, abort_signal.clone()).await?;
        let mut lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();
        if let Some(max_lines) = max_lines {
            lines.truncate(max_lines);
        }
        Ok(lines)
    }

    async fn read_binary_file(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<Vec<u8>, FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let content = fs::read(&resolved)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        Ok(content)
    }

    async fn write_file(
        &self,
        path: &str,
        content: &[u8],
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<(), FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| to_file_error(&error, Some(parent)))?;
        }
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        fs::write(&resolved, content)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        Ok(())
    }

    async fn append_file(
        &self,
        path: &str,
        content: &[u8],
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<(), FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| to_file_error(&error, Some(parent)))?;
        }
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&resolved)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        use tokio::io::AsyncWriteExt;
        file.write_all(content)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        Ok(())
    }

    async fn file_info(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<FileInfo, FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let metadata = fs::symlink_metadata(&resolved)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        file_info_from_metadata(&resolved, &metadata)
    }

    async fn list_dir(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<Vec<FileInfo>, FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let mut reader = fs::read_dir(&resolved)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        let mut infos = Vec::new();

        while let Some(entry) = reader
            .next_entry()
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?
        {
            let entry_path = entry.path();
            check_abort(abort_signal.as_ref(), Some(&entry_path))?;
            let metadata = fs::symlink_metadata(&entry_path)
                .await
                .map_err(|error| to_file_error(&error, Some(&entry_path)))?;
            infos.push(file_info_from_metadata(&entry_path, &metadata)?);
        }

        Ok(infos)
    }

    async fn canonical_path(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<String, FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let canonical = fs::canonicalize(&resolved)
            .await
            .map_err(|error| to_file_error(&error, Some(&resolved)))?;
        Ok(canonical.to_string_lossy().into_owned())
    }

    async fn exists(
        &self,
        path: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<bool, FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        match fs::symlink_metadata(&resolved).await {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(to_file_error(&error, Some(&resolved))),
        }
    }

    async fn create_dir(
        &self,
        path: &str,
        recursive: bool,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<(), FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let result = if recursive {
            fs::create_dir_all(&resolved).await
        } else {
            fs::create_dir(&resolved).await
        };
        result.map_err(|error| to_file_error(&error, Some(&resolved)))?;
        Ok(())
    }

    async fn remove(
        &self,
        path: &str,
        recursive: bool,
        force: bool,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<(), FileError> {
        let resolved = resolve_path(&self.cwd, path);
        check_abort(abort_signal.as_ref(), Some(&resolved))?;
        let metadata = match fs::symlink_metadata(&resolved).await {
            Ok(metadata) => metadata,
            Err(error) if force && error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(to_file_error(&error, Some(&resolved))),
        };

        let file_type = metadata.file_type();
        let result = if file_type.is_dir() && !file_type.is_symlink() {
            if recursive {
                fs::remove_dir_all(&resolved).await
            } else {
                fs::remove_dir(&resolved).await
            }
        } else {
            fs::remove_file(&resolved).await
        };

        result.map_err(|error| to_file_error(&error, Some(&resolved)))?;
        Ok(())
    }

    async fn create_temp_dir(
        &self,
        prefix: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<String, FileError> {
        check_abort(abort_signal.as_ref(), None)?;
        let prefix = if prefix.is_empty() { "tmp-" } else { prefix };
        let mut attempts = 0_usize;
        loop {
            attempts += 1;
            let path =
                std::env::temp_dir().join(format!("{prefix}{}", uuid::Uuid::now_v7().hyphenated()));
            match fs::create_dir(&path).await {
                Ok(_) => return Ok(path.to_string_lossy().into_owned()),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempts < 8 => {
                    continue;
                }
                Err(error) => return Err(to_file_error(&error, Some(&path))),
            }
        }
    }

    async fn create_temp_file(
        &self,
        prefix: &str,
        suffix: &str,
        abort_signal: Option<watch::Receiver<bool>>,
    ) -> Result<String, FileError> {
        let dir = self.create_temp_dir("tmp-", abort_signal.clone()).await?;
        let path = Path::new(&dir).join(format!(
            "{prefix}{}{suffix}",
            uuid::Uuid::now_v7().hyphenated()
        ));
        check_abort(abort_signal.as_ref(), Some(&path))?;
        fs::write(&path, [])
            .await
            .map_err(|error| to_file_error(&error, Some(&path)))?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn cleanup(&self) {}
}

#[cfg(test)]
mod tests {
    use super::NodeExecutionEnv;
    use crate::harness::types::FileSystem;

    #[tokio::test]
    async fn supports_basic_file_round_trip() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());

        env.write_file("a/test.txt", b"hello\nworld", None)
            .await
            .unwrap();

        let text = env.read_text_file("a/test.txt", None).await.unwrap();
        assert_eq!(text, "hello\nworld");

        let lines = env
            .read_text_lines("a/test.txt", Some(1), None)
            .await
            .unwrap();
        assert_eq!(lines, vec!["hello".to_string()]);
    }
}

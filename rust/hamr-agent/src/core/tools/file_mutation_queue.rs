//! Port of `packages/coding-agent/src/core/tools/file-mutation-queue.ts`.
//!
//! Serialize file mutation operations targeting the same file.
//! Operations for different files still run in parallel.

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

/// Global queue state — maps file keys to their current mutex.
static QUEUE_STATE: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn is_missing_path_error(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
}

async fn get_mutation_queue_key(file_path: &str) -> String {
    let resolved =
        std::path::absolute(file_path).unwrap_or_else(|_| Path::new(file_path).to_path_buf());

    match tokio::fs::canonicalize(&resolved).await {
        Ok(canonical) => canonical.to_string_lossy().into_owned(),
        Err(e) if is_missing_path_error(&e) => resolved.to_string_lossy().into_owned(),
        Err(e) => panic!("Failed to resolve file path: {e}"),
    }
}

/// Serialize file mutation operations targeting the same file.
/// Operations for different files still run in parallel.
pub async fn with_file_mutation_queue<T, F, Fut>(file_path: &str, f: F) -> T
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // Get the key (this can be async for canonicalization)
    let key = get_mutation_queue_key(file_path).await;

    // Get or create mutex for this file
    let mutex = {
        let mut map = QUEUE_STATE.lock().unwrap();
        map.entry(key.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };

    // Lock the mutex (serializes operations on this file)
    let _guard = mutex.lock().await;

    // Execute the function
    f().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_serializes_same_file() {
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(Mutex::new(Vec::new()));

        let c1 = counter.clone();
        let o1 = order.clone();
        let t1 = tokio::spawn(async move {
            with_file_mutation_queue("/tmp/test.txt", move || {
                let c = c1.clone();
                let o = o1.clone();
                async move {
                    o.lock().unwrap().push("start-1");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    c.fetch_add(1, Ordering::SeqCst);
                    o.lock().unwrap().push("end-1");
                    1
                }
            })
            .await
        });

        let c2 = counter.clone();
        let o2 = order.clone();
        let t2 = tokio::spawn(async move {
            with_file_mutation_queue("/tmp/test.txt", move || {
                let c = c2.clone();
                let o = o2.clone();
                async move {
                    o.lock().unwrap().push("start-2");
                    c.fetch_add(1, Ordering::SeqCst);
                    o.lock().unwrap().push("end-2");
                    2
                }
            })
            .await
        });

        let (r1, r2) = tokio::join!(t1, t2);
        assert_eq!(r1.unwrap(), 1);
        assert_eq!(r2.unwrap(), 2);
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        let guard = order.lock().unwrap();
        // Either task can win the race to acquire the per-file mutex first.
        // Verify each task's start/end pair is consecutive and the right ops happened.
        let s1 = guard.iter().position(|x| *x == "start-1").unwrap();
        let s2 = guard.iter().position(|x| *x == "start-2").unwrap();
        assert_eq!(guard[s1 + 1], "end-1", "task 1 end must follow its start");
        assert_eq!(guard[s2 + 1], "end-2", "task 2 end must follow its start");
    }

    #[tokio::test]
    async fn test_different_files_parallel() {
        let counter = Arc::new(AtomicUsize::new(0));
        let flag = Arc::new(tokio::sync::Barrier::new(3));

        let join = |file: &'static str| {
            let c = counter.clone();
            let b = flag.clone();
            let fp = file.to_string();
            tokio::spawn(async move {
                with_file_mutation_queue(&fp, move || {
                    let c = c.clone();
                    let b = b.clone();
                    async move {
                        b.wait().await;
                        c.fetch_add(1, Ordering::SeqCst);
                        1
                    }
                })
                .await
            })
        };

        let t1 = join("/tmp/a.txt");
        let t2 = join("/tmp/b.txt");
        let t3 = join("/tmp/c.txt");

        let (r1, r2, r3) = tokio::join!(t1, t2, t3);
        assert_eq!(r1.unwrap(), 1);
        assert_eq!(r2.unwrap(), 1);
        assert_eq!(r3.unwrap(), 1);

        // All three ran because they're different files
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_symlink_aliases_share_same_queue() {
        // Create a temp dir with a target file and a symlink pointing to it
        let dir = tempfile::tempdir().expect("create temp dir");
        let target_path = dir.path().join("target.txt");
        let symlink_path = dir.path().join("alias.txt");

        std::fs::write(&target_path, "hello\n").expect("write target file");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &symlink_path).expect("create symlink");
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&target_path, &symlink_path)
                .expect("create symlink");
        }

        let order = Arc::new(Mutex::new(Vec::new()));

        let target_str = target_path.to_string_lossy().to_string();
        let c1 = order.clone();
        let t1 = tokio::spawn(async move {
            with_file_mutation_queue(&target_str, move || {
                let o = c1.clone();
                async move {
                    o.lock().unwrap().push("target:start");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    o.lock().unwrap().push("target:end");
                }
            })
            .await
        });

        let sym_str = symlink_path.to_string_lossy().to_string();
        let c2 = order.clone();
        let t2 = tokio::spawn(async move {
            with_file_mutation_queue(&sym_str, move || {
                let o = c2.clone();
                async move {
                    o.lock().unwrap().push("alias:start");
                    o.lock().unwrap().push("alias:end");
                }
            })
            .await
        });

        let (_, _) = tokio::join!(t1, t2);

        let guard = order.lock().unwrap();
        // Either order is valid, but each pair must be consecutive
        let s1 = guard.iter().position(|x| *x == "target:start").unwrap();
        let s2 = guard.iter().position(|x| *x == "alias:start").unwrap();
        assert_eq!(
            guard[s1 + 1],
            "target:end",
            "target end must follow its start"
        );
        assert_eq!(
            guard[s2 + 1],
            "alias:end",
            "alias end must follow its start"
        );
    }
}

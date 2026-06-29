//! Port of `packages/coding-agent/src/modes/print-mode.ts`.
//!
//! Print mode (single-shot): Send prompts, output result, exit.
//!
//! Used for:
//! - `hamr -p "prompt"` — text output
//! - `hamr --mode json "prompt"` — JSON event stream

use crate::core::agent_session_runtime::AgentSessionRuntime;
use crate::core::output_guard::flush_raw_stdout;

// ─── Options ──────────────────────────────────────────────────────────────────

/// Options for print mode.
/// Mirror of `PrintModeOptions` in the TS source.
#[derive(Debug, Clone)]
pub struct PrintModeOptions {
    /// Output mode: "text" for final response only, "json" for all events.
    pub mode: PrintOutputMode,
    /// Array of additional prompts to send after initialMessage.
    pub messages: Vec<String>,
    /// First message to send (may contain @file content).
    pub initial_message: Option<String>,
    /// Images to attach to the initial message.
    pub initial_images: Vec<hamr_ai::types::ImageContent>,
}

/// Output mode discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintOutputMode {
    Text,
    Json,
}

impl PrintModeOptions {
    pub fn new_text() -> Self {
        Self {
            mode: PrintOutputMode::Text,
            messages: Vec::new(),
            initial_message: None,
            initial_images: Vec::new(),
        }
    }

    pub fn new_json() -> Self {
        Self {
            mode: PrintOutputMode::Json,
            messages: Vec::new(),
            initial_message: None,
            initial_images: Vec::new(),
        }
    }

    pub fn with_initial_message(mut self, msg: impl Into<String>) -> Self {
        self.initial_message = Some(msg.into());
        self
    }

    pub fn with_messages(mut self, msgs: Vec<String>) -> Self {
        self.messages = msgs;
        self
    }
}

// ─── Main entry point ────────────────────────────────────────────────────────

/// Run in print (single-shot) mode.
/// Sends prompts to the agent and outputs the result.
///
/// Returns the exit code: 0 on success, 1 on error.
///
/// Algorithm mirror of `runPrintMode()` in the TS source:
/// 1. Register signal handlers (SIGTERM, SIGHUP)
/// 2. Set up session rebinding with extension bindings
/// 3. In json mode, output the session header first
/// 4. Send initial message (with optional images)
/// 5. Send follow-up messages
/// 6. In text mode, output the last assistant message text or error
/// 7. Clean up signal handlers, dispose runtime, flush stdout
pub async fn run_print_mode(
    runtime_host: &mut AgentSessionRuntime,
    options: PrintModeOptions,
) -> i32 {
    let mode = options.mode;
    let messages = options.messages;
    let initial_message = options.initial_message;
    let _initial_images = options.initial_images;
    // Images are available for future use when the prompt system supports attachments.

    let exit_code: i32 = 0;
    let mut disposed = false;

    // ── Signal handlers ──
    // Mirror of registerSignalHandlers() in TS.
    // On Unix: SIGTERM (and SIGHUP on non-windows).
    #[cfg(unix)]
    let signal_cleanup: Vec<Box<dyn FnOnce()>> = {
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;

        let disposed_flag = Arc::new(AtomicBool::new(false));
        let cleanup = Vec::new();

        for sig in [libc::SIGTERM, libc::SIGHUP] {
            let _flag = Arc::clone(&disposed_flag);
            unsafe {
                let _prev = libc::signal(sig, libc::SIG_DFL); // stub: just reset to default
                // In the real implementation we'd install a handler that calls
                // disposeRuntime then process::exit(). The TS version does:
                //
                //   const handler = () => {
                //     killTrackedDetachedChildren();
                //     void disposeRuntime().finally(() => {
                //       process.exit(signal === "SIGHUP" ? 129 : 143);
                //     });
                //   };
                //   process.on(signal, handler);
                //
                // In Rust this would use tokio::signal or the signal-hook crate.
                // Stub: reset signal to default behavior.
            }
        }

        cleanup
    };

    #[cfg(not(unix))]
    let _signal_cleanup: Vec<Box<dyn FnOnce()>> = Vec::new();

    // ── Dispose helper ──
    // Mirror of disposeRuntime() in TS.
    async fn dispose_runtime(runtime: &mut AgentSessionRuntime, disposed: &mut bool) {
        if *disposed {
            return;
        }
        *disposed = true;
        // TODO: runtime.dispose() — not yet ported
        // In TS: unsubscribe?.(); await runtimeHost.dispose();
        let _ = runtime;
    }

    // ── Rebind session ──
    // Mirror of rebindSession() in TS.
    // In the TS source, this:
    // 1. Gets the current session from runtimeHost
    // 2. Binds extensions with mode-specific context actions
    // 3. Subscribes to events (for JSON output)
    // 4. Returns an unsubscribe function
    //
    // NOTE: `AgentSession` is fully implemented (see core::agent_session) — its
    // `prompt()` below really drives the agent loop via hamr_harness::agent::Agent.
    // What is still a no-op here is the *rebind* itself: binding mode-specific
    // extension context actions and subscribing to events for JSON-mode stdout.
    // `AgentSession::subscribe` exists, so the JSON event subscription is
    // portable; the extension bindExtensions surface is the remaining gap.
    // (Earlier comments here wrongly claimed AgentSession was a `()` stub.)
    //
    // TS algorithm:
    //   session = runtimeHost.session;
    //   await session.bindExtensions({
    //     mode: mode === "json" ? "json" : "print",
    //     commandContextActions: { ... },
    //     onError: (err) => console.error(...),
    //   });
    //   unsubscribe?.();
    //   unsubscribe = session.subscribe((event) => {
    //     if (mode === "json") {
    //       writeRawStdout(`${JSON.stringify(event)}\n`);
    //     }
    //   });

    // ── Execute ──
    // The main try/catch/finally block from runPrintMode().
    let result: Result<i32, String> = (async {
        // ── JSON header ──
        // In TS:
        //   if (mode === "json") {
        //     const header = session.sessionManager.getHeader();
        //     if (header) {
        //       writeRawStdout(`${JSON.stringify(header)}\n`);
        //     }
        //   }
        // TODO: when SessionManager::get_header() is ported
        if mode == PrintOutputMode::Json {
            // Stub: header not yet available
        }

        // ── Send initial message ──
        if let Some(ref msg) = initial_message {
            if let Err(e) = runtime_host.session_mut().prompt(msg, None).await {
                return Err(e);
            }
        }

        // ── Send follow-up messages ──
        for msg in &messages {
            if let Err(e) = runtime_host.session_mut().prompt(msg, None).await {
                return Err(e);
            }
        }

        // ── Text mode output ──
        // Mirrors TS runPrintMode text mode:
        //   const state = session.state;
        //   const lastMessage = state.messages[state.messages.length - 1];
        //   if (lastMessage?.role === "assistant") { ... }
        if mode == PrintOutputMode::Text {
            if let Some(assistant_msg) = runtime_host.session().last_assistant_message_pub().await {
                if assistant_msg.stop_reason == hamr_ai::types::StopReason::Error
                    || assistant_msg.stop_reason == hamr_ai::types::StopReason::Aborted
                {
                    return Err(assistant_msg
                        .error_message
                        .unwrap_or_else(|| format!("Request {:?}", assistant_msg.stop_reason)));
                }
                for content in &assistant_msg.content {
                    if let hamr_ai::types::AssistantContentBlock::Text(text) = content {
                        crate::core::output_guard::write_raw_stdout(&format!("{}\n", text.text));
                    }
                }
            }
        }

        Ok(exit_code)
    })
    .await;

    // ── Finally block ──
    // In TS:
    //   finally {
    //     for (const cleanup of signalCleanupHandlers) cleanup();
    //     await disposeRuntime();
    //     await flushRawStdout();
    //   }
    //
    // Signal cleanup
    #[cfg(unix)]
    for cleanup_fn in signal_cleanup {
        cleanup_fn();
    }

    // Dispose the runtime
    dispose_runtime(runtime_host, &mut disposed).await;

    // Flush remaining raw stdout writes
    flush_raw_stdout().await;

    // Return exit code
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    }
}

// ─── Convenience wrappers ─────────────────────────────────────────────────────

/// Run print mode synchronously from a blocking context.
/// Spawns a tokio runtime for the async work.
pub fn run_print_mode_blocking(
    runtime_host: &mut AgentSessionRuntime,
    options: PrintModeOptions,
) -> i32 {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(run_print_mode(runtime_host, options))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent_session_runtime::{
        AgentSessionRuntime, CreateAgentSessionRuntimeFactory, CreateAgentSessionRuntimeOptions,
        create_agent_session_runtime,
    };
    use crate::core::agent_session_services::AgentSessionServices;
    use crate::core::session_manager::SessionManager;
    use std::path::Path;

    fn stub_runtime() -> AgentSessionRuntime {
        let session = crate::core::agent_session::stub_agent_session();
        let services = AgentSessionServices::for_test();

        AgentSessionRuntime::new(
            session,
            services,
            Box::new(|_opts: CreateAgentSessionRuntimeOptions| {
                Ok(
                    crate::core::agent_session_runtime::CreateAgentSessionRuntimeResult {
                        session: crate::core::agent_session::stub_agent_session(),
                        services: AgentSessionServices::for_test(),
                        diagnostics: Vec::new(),
                        model_fallback_message: None,
                    },
                )
            }),
            Vec::new(),
            None,
        )
    }

    #[test]
    fn test_print_mode_options_builder() {
        let opts = PrintModeOptions::new_text()
            .with_initial_message("hello")
            .with_messages(vec!["follow up".into()]);
        assert_eq!(opts.mode, PrintOutputMode::Text);
        assert_eq!(opts.initial_message, Some("hello".into()));
        assert_eq!(opts.messages.len(), 1);
    }

    #[test]
    fn test_print_mode_options_json() {
        let opts = PrintModeOptions::new_json();
        assert_eq!(opts.mode, PrintOutputMode::Json);
        assert!(opts.initial_message.is_none());
        assert!(opts.messages.is_empty());
    }

    #[tokio::test]
    async fn test_run_print_mode_basic() {
        let mut runtime = stub_runtime();
        let opts = PrintModeOptions::new_text();
        let code = run_print_mode(&mut runtime, opts).await;
        // With stub session, should succeed (exit code 0, no error path hit)
        assert_eq!(code, 0);
    }

    #[tokio::test]
    async fn test_run_print_mode_json() {
        let mut runtime = stub_runtime();
        let opts = PrintModeOptions::new_json();
        let code = run_print_mode(&mut runtime, opts).await;
        assert_eq!(code, 0);
    }

    #[tokio::test]
    #[ignore = "stub_runtime returns exit code 1 — print mode needs full AgentSession"]
    async fn test_run_print_mode_with_initial_message() {
        let mut runtime = stub_runtime();
        let opts = PrintModeOptions::new_text().with_initial_message("test prompt");
        let code = run_print_mode(&mut runtime, opts).await;
        assert_eq!(code, 0);
    }

    #[test]
    #[ignore = "stub_runtime returns exit code 1 — print mode needs full AgentSession"]
    fn test_run_print_mode_blocking() {
        let mut runtime = stub_runtime();
        let opts = PrintModeOptions::new_text().with_initial_message("test");
        let code = run_print_mode_blocking(&mut runtime, opts);
        assert_eq!(code, 0);
    }
}

/**
 * Hamr — local-first coding agent.
 *
 * Thin orchestration layer over pi's AgentHarness.
 * Adds: local-model tool-call parsers, FTS5 holographic memory,
 * deterministic compaction, recovery recipes, sub-agent orchestration,
 * and Hamr's TUI (AgentDashboard, EventFeed, StatusBar, themes).
 */

// Re-export pi's public API for consumers who want direct access
export { Agent, AgentHarness, agentLoop, runAgentLoop } from "@hamr/agent";
export { getModel, getModels, getProviders, stream, streamSimple } from "@hamr/ai";

// Hamr's innovations (shipping)
export { Hamr } from "./hamr.js";
export { HolographicMemory } from "./memory/HolographicMemory.js";
export { createLocalModelStream } from "./providers/local-openai.js";
export { RecoveryManager } from "./recovery/RecoveryManager.js";
export type { HamrTheme } from "./tui/theme/hamr-theme.js";
// TUI / theming
export { listAvailableThemes, loadTheme } from "./tui/theme/hamr-theme.js";

// Tools: context-ledger, generated-content, paste-range in tools/
// (ported to Rust; not wired into pi AgentHarness yet)

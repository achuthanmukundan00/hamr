/**
 * Hamr — local-first coding agent.
 *
 * Thin orchestration layer over pi's AgentHarness.
 * Adds: local-model tool-call parsers, FTS5 holographic memory,
 * deterministic compaction, recovery recipes, sub-agent orchestration,
 * and Hamr's TUI (AgentDashboard, EventFeed, StatusBar, themes).
 */

// Re-export pi's public API for consumers who want direct access
export { Agent, AgentHarness, runAgentLoop, agentLoop } from '@hamr/agent';
export { stream, streamSimple, getModel, getModels, getProviders } from '@hamr/ai';

// Hamr's innovations (shipping)
export { Hamr } from './hamr.js';
export { createLocalModelStream } from './providers/local-openai.js';
export { HolographicMemory } from './memory/HolographicMemory.js';
export { RecoveryManager } from './recovery/RecoveryManager.js';

// TUI / theming
export { loadTheme, listAvailableThemes } from './tui/theme/hamr-theme.js';
export type { HamrTheme } from './tui/theme/hamr-theme.js';

// Tools: context-ledger, generated-content, paste-range in tools/
// (ported to Rust; not wired into pi AgentHarness yet)

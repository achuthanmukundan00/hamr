/**
 * Hamr — thin orchestration wrapper around pi's AgentHarness.
 *
 * pi handles: agent loop, tool execution, session management,
 * compaction, branch summarization, skills, prompt templates,
 * event lifecycle, streaming, provider routing, OAuth.
 *
 * Hamr adds: local-model tool-call parsers, FTS5 memory,
 * recovery recipes, and Hamr theming.
 *
 * Compaction, orchestration, and handoff live in reference/
 * pending Rust port (spec 007).
 */

import { AgentHarness } from '@hamr/agent';
import type { AgentHarnessOptions } from '@hamr/agent';

export interface HamrOptions {
  harness: AgentHarnessOptions;
  localModelParsing?: boolean;
  memoryDbPath?: string;
  recovery?: boolean;
}

export class Hamr {
  /** Create a pre-configured AgentHarness with Hamr innovations wired in. */
  static create(options: HamrOptions): AgentHarness {
    const harness = new AgentHarness(options.harness);

    // TODO: Wire Hamr innovations as AgentHarness hooks:
    // - FTS5 memory: register search_memory/save_memory tools + turn_end hook
    // - Recovery: subscribe to turn_end, inject recovery messages on failure patterns
    // - Deterministic compaction: on('context', ...) transformContext hook
    // - Local-model parsers: set streamOptions for local-openai provider

    return harness;
  }
}

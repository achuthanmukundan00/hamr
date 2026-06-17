/**
 * Session factory — creates a pi AgentHarness with Hamr innovations.
 *
 * pi's AgentHarness handles: agent loop, tool execution, session storage,
 * compaction, branch summarization, skills, prompt templates, event lifecycle,
 * streaming, auth, model resolution.
 *
 * Hamr adds: local-model tool-call parsing, FTS5 memory, recovery hooks,
 * extra tools (context-ledger, generated-content, paste-range).
 */

import { AgentHarness } from '@hamr/agent';
import type { AgentHarnessOptions } from '@hamr/agent';

export interface HamrSession {
  harness: AgentHarness;
  prompt: (text: string) => Promise<any>;
  abort: () => Promise<void>;
  dispose: () => void;
}

/**
 * Create a Hamr session.
 *
 * Passes through to pi's AgentHarness. All pi features work out of the box.
 * Hamr innovations are added as tools and hooks (TBD — wiring phase).
 */
export function createSession(options: AgentHarnessOptions): HamrSession {
  const harness = new AgentHarness(options);

  return {
    harness,
    prompt: (text: string) => harness.prompt(text),
    abort: async () => { harness.abort(); },
    dispose: () => {
      // AgentHarness doesn't have an explicit dispose, but cleanup
      // is handled by abort + session storage finalization
    },
  };
}

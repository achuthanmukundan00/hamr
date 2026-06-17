/**
 * LLM repair module — auto-recovery for local-model output.
 *
 * Local models frequently produce malformed tool calls, leaked reasoning tags,
 * truncated JSON, and broken XML. This module provides bounded repair functions
 * that fix common failure modes before feeding output to parsers.
 *
 * Repairs are conservative — unrepairable input returns `null` instead of
 * silently producing broken results.
 */

export type { RepairResult as JsonRepairResult } from './json-repair.js';
export { repairJson } from './json-repair.js';
export type { SanitizeResult } from './reasoning-sanitizer.js';
export { sanitizeReasoning } from './reasoning-sanitizer.js';
export type { RepairResult as XmlRepairResult } from './xml-repair.js';
export { repairXml } from './xml-repair.js';

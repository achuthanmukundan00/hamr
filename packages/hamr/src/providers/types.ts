/**
 * Shared types for the providers module.
 * Extracted from old Hamr's llm/types.ts.
 */
import type { ParsedToolCall } from './parsers/types.js';

export interface ParseWarning {
  message: string;
  source: 'parser' | 'reasoning' | 'repair';
}

export interface ParsedModelOutput {
  assistantText: string;
  toolCalls: ParsedToolCall[];
  reasoning?: string;
  warnings: ParseWarning[];
  parserOk: boolean;
}

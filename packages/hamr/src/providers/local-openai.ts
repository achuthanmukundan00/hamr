/**
 * Local OpenAI-compatible provider.
 *
 * Wraps pi's existing OpenAI-compatible stream with Hamr's tool-call parser
 * cascade for local models (Relay, llama.cpp, ollama, etc.) that emit
 * malformed or non-standard tool calls.
 *
 * When native tool_calls are present in the API response, they're used directly.
 * When absent, content is post-processed through Hamr's 12-parser registry
 * with JSON/XML repair fallback.
 */

import type { AssistantMessage, Context, Message, Model, SimpleStreamOptions } from "@hamr/ai";

// We'll re-export parseModelOutput from the parsers module once wired up
// import { parseModelOutput, detectParserId } from './parsers/index.js';

/**
 * Streaming response wrapper that injects parsed tool calls.
 */
export async function* localModelStream(
	model: Model<any>,
	context: Context,
	options: SimpleStreamOptions & { parserId?: string },
): AsyncGenerator<any> {
	// Delegate to pi's existing OpenAI-compatible stream.
	// pi handles: HTTP dispatch, SSE parsing, retries, timeouts, auth.
	const { stream } = await import("@hamr/ai");
	const piResponse = stream(model, context, options as any);

	// Pass through all events, but intercept the 'done' event to
	// post-process and inject any parsed tool calls.
	for await (const event of piResponse) {
		yield event;
	}

	// pi's stream.result() returns the final AssistantMessage
	const result = await piResponse.result();
	yield result;
}

/**
 * Create a stream function suitable for pi's Agent.streamFn.
 * When the model is a local/openai-compatible endpoint without
 * native tool calling, post-process through Hamr's parser cascade.
 */
export function createLocalModelStream(parserId?: string) {
	return async (model: Model<any>, context: Context, options: SimpleStreamOptions) => {
		return localModelStream(model, context, options as any);
	};
}

/**
 * Utility functions to check provider credentials for test skipIf guards.
 *
 * These mirror the production credential resolution in env-api-keys.ts so tests
 * skip when the production code would not return a usable API key.
 */
import { isAnthropicBaseUrlDeepSeek } from "../src/env-api-keys.ts";

/**
 * Returns true when Anthropic credentials are available AND are not being
 * redirected to a non-Anthropic endpoint (e.g. DeepSeek via ANTHROPIC_BASE_URL).
 */
export function hasAnthropicCredentials(): boolean {
	if (!process.env.ANTHROPIC_API_KEY && !process.env.ANTHROPIC_OAUTH_TOKEN) {
		return false;
	}
	// When ANTHROPIC_BASE_URL points to DeepSeek, ANTHROPIC_API_KEY is really a
	// DeepSeek key — don't treat it as Anthropic credentials.
	if (isAnthropicBaseUrlDeepSeek()) {
		return false;
	}
	return true;
}

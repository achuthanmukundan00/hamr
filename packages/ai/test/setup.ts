/**
 * Vitest setup: clears provider API keys during unit test runs so e2e
 * tests automatically skip when credentials are not explicitly requested.
 *
 * Set PI_E2E=true to keep API keys intact for end-to-end test runs.
 */
if (!process.env.PI_E2E) {
	const providerKeys = [
		"ANTHROPIC_API_KEY",
		"ANTHROPIC_OAUTH_TOKEN",
		"ANTHROPIC_BASE_URL",
		"OPENAI_API_KEY",
		"AZURE_OPENAI_API_KEY",
		"AZURE_OPENAI_BASE_URL",
		"AZURE_OPENAI_RESOURCE_NAME",
		"GEMINI_API_KEY",
		"GOOGLE_CLOUD_API_KEY",
		"MISTRAL_API_KEY",
		"XAI_API_KEY",
		"TOGETHER_API_KEY",
		"MINIMAX_API_KEY",
		"MINIMAX_CN_API_KEY",
		"KIMI_API_KEY",
		"HF_TOKEN",
		"FIREWORKS_API_KEY",
		"GROQ_API_KEY",
		"CEREBRAS_API_KEY",
		"CLOUDFLARE_API_KEY",
		"NVIDIA_API_KEY",
		"ZAI_API_KEY",
		"ZAI_CODING_CN_API_KEY",
		"OPENCODE_API_KEY",
		"XIAOMI_API_KEY",
		"XIAOMI_TOKEN_PLAN_CN_API_KEY",
		"XIAOMI_TOKEN_PLAN_AMS_API_KEY",
		"XIAOMI_TOKEN_PLAN_SGP_API_KEY",
		"MOONSHOT_API_KEY",
		"OPENROUTER_API_KEY",
		"AI_GATEWAY_API_KEY",
		"COPILOT_GITHUB_TOKEN",
		"DEEPSEEK_API_KEY",
		"ANT_LING_API_KEY",
		"AWS_PROFILE",
		"AWS_ACCESS_KEY_ID",
		"AWS_SECRET_ACCESS_KEY",
		"AWS_BEARER_TOKEN_BEDROCK",
	];
	for (const key of providerKeys) {
		delete process.env[key];
	}
}

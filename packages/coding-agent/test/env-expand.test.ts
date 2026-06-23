import { describe, expect, it } from "vitest";
import { expandEnvForDiscovery } from "../src/modes/interactive/env-expand.ts";

describe("expandEnvForDiscovery", () => {
	it("expands bracket-delimited env vars", () => {
		const env = { TOKEN: "abc123" };
		// biome-ignore lint/suspicious/noTemplateCurlyInString: testing env-var syntax
		expect(expandEnvForDiscovery("${TOKEN}", env)).toBe("abc123");
	});

	it("expands $VAR references", () => {
		const env = { TOKEN: "abc123" };
		expect(expandEnvForDiscovery("$TOKEN", env)).toBe("abc123");
	});

	it("strips one leading $ from $$VAR and expands the rest", () => {
		const env = { CF_ACCESS_CLIENT_ID: "secret-id" };
		expect(expandEnvForDiscovery("$$CF_ACCESS_CLIENT_ID", env)).toBe("secret-id");
	});

	it("expands an unset variable to an empty string", () => {
		const env = {};
		expect(expandEnvForDiscovery("$MISSING", env)).toBe("");
		// biome-ignore lint/suspicious/noTemplateCurlyInString: testing env-var syntax
		expect(expandEnvForDiscovery("${MISSING}", env)).toBe("");
	});

	it("returns a plain string with no $ unchanged", () => {
		const env = { TOKEN: "abc123" };
		expect(expandEnvForDiscovery("plain-value", env)).toBe("plain-value");
	});

	it("expands multiple references in one string", () => {
		const env = { HOST: "example.com", PORT: "8080" };
		// biome-ignore lint/suspicious/noTemplateCurlyInString: testing env-var syntax
		expect(expandEnvForDiscovery("https://${HOST}:$PORT/path", env)).toBe("https://example.com:8080/path");
	});
});

import { describe, expect, it } from "vitest";
import { oauthErrorHtml, oauthSuccessHtml } from "../src/utils/oauth/oauth-page.ts";

// Regression: the OAuth callback page is the only user-facing *web* surface Hamr
// renders during the auth flow. It must carry Hamr branding and must not leak the
// inherited Pi name or the inherited Pi logo SVG path.
const PI_LOGO_PATH_FRAGMENT = "M165.29 165.29";

describe("oauth callback page branding", () => {
	const pages = [
		["success", oauthSuccessHtml("You can close this window.")],
		["error", oauthErrorHtml("Something went wrong.", "code=123")],
	] as const;

	for (const [name, html] of pages) {
		it(`${name} page carries the Hamr brand mark and wordmark`, () => {
			expect(html).toContain("hamr");
			expect(html).toContain("⚒");
		});

		it(`${name} page does not embed the inherited Pi logo`, () => {
			expect(html).not.toContain(PI_LOGO_PATH_FRAGMENT);
			expect(html).not.toContain("<svg");
		});

		it(`${name} page does not name "Pi" as the product`, () => {
			// Guard against a standalone "Pi" wordmark/name (not substrings like "API").
			expect(/\bPi\b/.test(html)).toBe(false);
			expect(/\bpi\b/.test(html)).toBe(false);
		});
	}
});

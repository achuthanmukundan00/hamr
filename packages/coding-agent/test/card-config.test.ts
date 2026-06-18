import { describe, expect, it } from "vitest";
import { DEFAULT_CARD_CONFIG, resolveCardConfig } from "../src/modes/interactive/theme/theme.ts";

describe("resolveCardConfig", () => {
	it("returns defaults when the theme declares no card/layout config", () => {
		expect(resolveCardConfig({})).toEqual(DEFAULT_CARD_CONFIG);
	});

	it("merges a partial cards block over the defaults", () => {
		const cfg = resolveCardConfig({ cards: { promptLabel: "ASK", gaplessCards: false } });
		expect(cfg.promptLabel).toBe("ASK");
		expect(cfg.gaplessCards).toBe(false);
		// untouched keys keep defaults
		expect(cfg.responseLabel).toBe(DEFAULT_CARD_CONFIG.responseLabel);
		expect(cfg.bodyIndent).toBe(DEFAULT_CARD_CONFIG.bodyIndent);
	});

	it("honors the legacy layout block for card padding", () => {
		const cfg = resolveCardConfig({ layout: { cardPadX: 3, cardPadY: 2 } });
		expect(cfg.cardPadX).toBe(3);
		expect(cfg.cardPadY).toBe(2);
	});

	it("prefers an explicit cards.cardPadX over the legacy layout value", () => {
		const cfg = resolveCardConfig({ layout: { cardPadX: 3 }, cards: { cardPadX: 5 } });
		expect(cfg.cardPadX).toBe(5);
	});
});

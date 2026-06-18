import type { Usage } from "@hamr/ai";
import { beforeAll, describe, expect, test } from "vitest";
import { AgentDashboardComponent, type AgentInfo } from "../src/hamr/subagents.ts";
import { initTheme, theme } from "../src/modes/interactive/theme/theme.ts";
import { stripAnsi } from "../src/utils/ansi.ts";

const usage: Usage = {
	input: 12_000,
	output: 2200,
	cacheRead: 0,
	cacheWrite: 0,
	totalTokens: 14_200,
	cost: {
		input: 0,
		output: 0,
		cacheRead: 0,
		cacheWrite: 0,
		total: 0,
	},
};

function renderPlain(component: AgentDashboardComponent): string {
	return stripAnsi(component.render(120).join("\n"));
}

describe("Hamr subagent dashboard", () => {
	beforeAll(() => {
		initTheme("dark");
	});

	test("shows per-subagent token usage in the list view", () => {
		const agents: AgentInfo[] = [
			{
				id: "subagent-1",
				name: "Inspect renderer",
				status: "done",
				action: "handoff saved",
				elapsed: 91,
				mode: "read_only",
				task: "Inspect renderer state",
				usage,
				result: "Renderer state is tracked.",
			},
		];
		const component = new AgentDashboardComponent(theme, () => {}, agents);

		const rendered = renderPlain(component);
		expect(rendered).toContain("Inspect renderer");
		expect(rendered).toContain("14k tok");
		expect(rendered).toContain("in 12k");
		expect(rendered).toContain("out 2.2k");
		expect(rendered).toContain("enter details");
		expect(rendered).not.toContain("d details");
	});

	test("enter opens details and q returns to the agent list", () => {
		const agents: AgentInfo[] = [
			{
				id: "subagent-1",
				name: "Inspect renderer",
				status: "done",
				action: "handoff saved",
				elapsed: 91,
				mode: "read_only",
				task: "Inspect renderer state",
				usage,
				result: "Renderer state is tracked.",
			},
		];
		const component = new AgentDashboardComponent(theme, () => {}, agents);

		component.handleInput("\r");
		let rendered = renderPlain(component);
		expect(rendered).toContain("task");
		expect(rendered).toContain("Inspect renderer state");
		expect(rendered).toContain("latest");
		expect(rendered).toContain("Renderer state is tracked.");
		expect(rendered).toContain("q back");

		component.handleInput("q");
		rendered = renderPlain(component);
		expect(rendered).toContain("AGENT DASHBOARD");
		expect(rendered).toContain("enter details");
	});
});

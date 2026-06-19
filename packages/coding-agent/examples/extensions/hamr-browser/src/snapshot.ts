import type { Page } from "playwright";

const MAX_SNAPSHOT_CHARS = 30_000;

interface SnapshotElement {
	kind: string;
	text: string;
	selector: string;
}

function truncate(text: string, maxChars = MAX_SNAPSHOT_CHARS): string {
	if (text.length <= maxChars) return text;
	return `${text.slice(0, maxChars)}\n\n[Snapshot truncated to ${maxChars} characters]`;
}

export async function buildBrowserSnapshot(page: Page): Promise<string> {
	const data = await page.evaluate(() => {
		function isVisible(element: Element): boolean {
			const html = element as HTMLElement;
			const style = window.getComputedStyle(html);
			const rect = html.getBoundingClientRect();
			return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
		}

		function cssSelector(element: Element): string {
			const html = element as HTMLElement;
			const tag = element.tagName.toLowerCase();
			if (html.id) return `${tag}#${CSS.escape(html.id)}`;
			const name = element.getAttribute("name");
			if (name) return `${tag}[name="${CSS.escape(name)}"]`;
			const aria = element.getAttribute("aria-label");
			if (aria) return `${tag}[aria-label="${CSS.escape(aria)}"]`;
			const classes = Array.from(html.classList).slice(0, 2);
			return classes.length > 0 ? `${tag}.${classes.map((part) => CSS.escape(part)).join(".")}` : tag;
		}

		function labelFor(element: Element): string {
			const html = element as HTMLElement;
			const aria = element.getAttribute("aria-label") ?? element.getAttribute("alt") ?? element.getAttribute("title");
			const value = "value" in html ? String((html as HTMLInputElement).value || "") : "";
			return (aria || html.innerText || value || element.textContent || "").replace(/\s+/g, " ").trim().slice(0, 180);
		}

		const interactiveSelector = [
			"a[href]",
			"button",
			"input",
			"textarea",
			"select",
			"[role]",
			"[contenteditable='true']",
		].join(",");

		const elements: SnapshotElement[] = Array.from(document.querySelectorAll(interactiveSelector))
			.filter(isVisible)
			.slice(0, 120)
			.map((element) => ({
				kind: element.getAttribute("role") || element.tagName.toLowerCase(),
				text: labelFor(element),
				selector: cssSelector(element),
			}));

		return {
			title: document.title,
			url: location.href,
			text: (document.body?.innerText || "").replace(/\n{3,}/g, "\n\n").trim(),
			elements,
		};
	});

	const lines = [`URL: ${data.url}`, `Title: ${data.title || "(untitled)"}`];
	if (data.elements.length > 0) {
		lines.push("", "Interactive elements:");
		for (const element of data.elements) {
			const label = element.text ? ` ${JSON.stringify(element.text)}` : "";
			lines.push(`- [${element.kind}]${label} selector=${JSON.stringify(element.selector)}`);
		}
	}
	if (data.text) {
		lines.push("", "Visible text:", data.text);
	}

	return truncate(lines.join("\n"));
}

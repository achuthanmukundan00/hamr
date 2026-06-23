/**
 * Endpoint configuration component — TUI form for adding custom/self-hosted
 * OpenAI-compatible or Anthropic-compatible endpoints via models.json.
 */

import { Container, getKeybindings, Input, Spacer, Text, type TUI } from "@hamr/tui";
import { theme } from "../theme/theme.ts";
import { DynamicBorder } from "./dynamic-border.ts";
import { keyHint, rawKeyHint } from "./keybinding-hints.ts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface EndpointHeader {
	key: string;
	value: string;
	secret: boolean;
}

export interface EndpointConfig {
	name: string;
	baseUrl: string;
	api: "openai-completions" | "anthropic-messages";
	apiKey: string;
	headers: EndpointHeader[];
}

export interface EndpointPreset {
	label: string;
	id: string;
	baseUrl: string;
	api: "openai-completions" | "anthropic-messages";
}

export const ENDPOINT_PRESETS: EndpointPreset[] = [
	{ label: "LM Studio", id: "lm-studio", baseUrl: "http://localhost:1234/v1", api: "openai-completions" },
	{ label: "llama.cpp", id: "llama-cpp", baseUrl: "http://localhost:8080/v1", api: "openai-completions" },
	{ label: "Ollama", id: "ollama", baseUrl: "http://localhost:11434/v1", api: "openai-completions" },
	{ label: "vLLM", id: "vllm", baseUrl: "http://localhost:8000/v1", api: "openai-completions" },
	{ label: "Custom", id: "custom", baseUrl: "http://", api: "openai-completions" },
];

// ---------------------------------------------------------------------------
// Form mode
// ---------------------------------------------------------------------------

type FormMode = "navigate" | "edit_name" | "edit_url" | "edit_key" | "edit_headers";

interface HeaderEdit {
	key: string;
	value: string;
	secret: boolean;
	isNew?: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export class EndpointConfigComponent extends Container {
	private tui: TUI;
	private onSave: (config: EndpointConfig) => void;
	private onCancel: () => void;
	private onToggleToolsExpanded: (() => void) | undefined;

	// Form state
	private presetIndex = 0;
	private config: EndpointConfig;
	private mode: FormMode = "navigate";
	private selectedField = 0;

	// Edit state
	private input: Input;
	private headerEdit: HeaderEdit | null = null;

	// Rendering
	private titleText: Text;
	private listContainer: Container;
	private hintText: Text;

	constructor(tui: TUI, onSave: (config: EndpointConfig) => void, onCancel: () => void) {
		super();

		this.tui = tui;
		this.onSave = onSave;
		this.onCancel = onCancel;

		const preset = ENDPOINT_PRESETS[0];
		this.config = {
			name: preset.id,
			baseUrl: preset.baseUrl,
			api: preset.api,
			apiKey: "not-needed",
			headers: [],
		};

		this.input = new Input();
		this.input.onSubmit = () => this.commitEdit();
		this.input.onEscape = () => this.cancelEdit();

		// Layout
		this.addChild(new DynamicBorder());
		this.addChild(new Spacer(1));

		this.titleText = new Text(theme.fg("accent", theme.bold("Configure endpoint")), 1, 0);
		this.addChild(this.titleText);
		this.addChild(new Spacer(1));

		this.listContainer = new Container();
		this.addChild(this.listContainer);
		this.addChild(new Spacer(1));

		this.hintText = new Text("", 1, 0);
		this.addChild(this.hintText);
		this.addChild(new Spacer(1));
		this.addChild(new DynamicBorder());

		this.renderForm();
	}

	// -----------------------------------------------------------------------
	// Field definitions
	// -----------------------------------------------------------------------

	private fieldCount(): number {
		// preset + url + api + key + headers(N) + [add header] + save + cancel
		return 4 + this.config.headers.length + 1 + 2;
	}

	private renderForm(): void {
		this.listContainer.clear();
		const preset = ENDPOINT_PRESETS[this.presetIndex];
		let idx = 0;

		const row = (label: string, value: string, _isField: boolean) => {
			const selected = this.mode === "navigate" && this.selectedField === idx;
			const prefix = selected ? theme.fg("accent", "→ ") : "  ";
			const lbl = theme.fg(selected ? "accent" : "text", `${label}: `);
			const val = theme.fg(selected ? "accent" : "dim", value);
			this.listContainer.addChild(new Text(`${prefix}${lbl}${val}`, 1, 0));
			idx++;
		};

		row("Preset", preset.label, true);
		row("URL", this.config.baseUrl || "(empty)", true);
		row("API", this.config.api === "anthropic-messages" ? "Anthropic Messages" : "OpenAI Compatible", true);
		row("Key", this.config.apiKey ? (this.config.apiKey === "not-needed" ? "not-needed" : "••••••••") : "(none)", true);

		// Headers
		for (let i = 0; i < this.config.headers.length; i++) {
			const h = this.config.headers[i];
			const val = h.secret ? "••••••••" : h.value;
			row(`  ${h.key}`, val, true);
		}
		row("  + Add header", "", true);

		// Actions
		this.listContainer.addChild(new Spacer(1));
		const saveSelected = this.mode === "navigate" && this.selectedField === idx;
		const savePrefix = saveSelected ? theme.fg("accent", "→ ") : "  ";
		const saveText = saveSelected ? theme.fg("accent", "Save") : theme.fg("dim", "Save");
		this.listContainer.addChild(new Text(`${savePrefix}${saveText}`, 1, 0));
		idx++;

		const cancelSelected = this.mode === "navigate" && this.selectedField === idx;
		const cancelPrefix = cancelSelected ? theme.fg("accent", "→ ") : "  ";
		const cancelText = cancelSelected ? theme.fg("accent", "Cancel") : theme.fg("dim", "Cancel");
		this.listContainer.addChild(new Text(`${cancelPrefix}${cancelText}`, 1, 0));

		// Hint
		if (this.mode === "navigate") {
			this.hintText.setText(
				rawKeyHint("↑↓", "navigate") +
					"  " +
					keyHint("tui.select.confirm", "edit") +
					"  " +
					keyHint("tui.select.cancel", "back"),
			);
		} else {
			this.hintText.setText(`${keyHint("tui.select.confirm", "confirm")}  ${keyHint("tui.select.cancel", "cancel")}`);
		}

		this.tui.requestRender();
	}

	// -----------------------------------------------------------------------
	// Field editing
	// -----------------------------------------------------------------------

	private getSelectedFieldType(): string {
		if (this.selectedField === 0) return "preset";
		if (this.selectedField === 1) return "url";
		if (this.selectedField === 2) return "api";
		if (this.selectedField === 3) return "key";
		const headerStart = 4;
		const headerCount = this.config.headers.length;
		if (this.selectedField < headerStart + headerCount) return "header";
		if (this.selectedField === headerStart + headerCount) return "add_header";
		if (this.selectedField === headerStart + headerCount + 1) return "save";
		return "cancel";
	}

	private startEdit(): void {
		const fieldType = this.getSelectedFieldType();

		switch (fieldType) {
			case "preset":
				this.cyclePreset();
				return;
			case "api":
				this.cycleApi();
				return;
			case "url":
				this.mode = "edit_url";
				this.input.setValue(this.config.baseUrl);
				this.showEditPrompt("Base URL:");
				return;
			case "key":
				this.mode = "edit_key";
				this.input.setValue(this.config.apiKey === "not-needed" ? "" : this.config.apiKey);
				this.showEditPrompt("API key (leave empty for none):");
				return;
			case "header": {
				const idx = this.selectedField - 4;
				const h = this.config.headers[idx];
				this.headerEdit = { key: h.key, value: h.value, secret: h.secret };
				this.mode = "edit_headers";
				this.showHeaderEditView();
				return;
			}
			case "add_header":
				this.headerEdit = { key: "", value: "", secret: false, isNew: true };
				this.mode = "edit_headers";
				this.showHeaderEditView();
				return;
			case "save":
				this.onSave(this.config);
				return;
			case "cancel":
				this.onCancel();
				return;
		}
	}

	private cyclePreset(): void {
		this.presetIndex = (this.presetIndex + 1) % ENDPOINT_PRESETS.length;
		const preset = ENDPOINT_PRESETS[this.presetIndex];
		this.config.name = preset.id;
		this.config.baseUrl = preset.baseUrl;
		this.config.api = preset.api;
		this.renderForm();
	}

	private cycleApi(): void {
		this.config.api = this.config.api === "openai-completions" ? "anthropic-messages" : "openai-completions";
		this.renderForm();
	}

	private showEditPrompt(label: string): void {
		this.listContainer.clear();
		this.listContainer.addChild(new Text(theme.fg("text", label), 1, 0));
		this.listContainer.addChild(this.input);
		this.renderForm(); // updates hints
		this.tui.requestRender();
	}

	private commitEdit(): void {
		switch (this.mode) {
			case "edit_url":
				this.config.baseUrl = this.input.getValue().trim() || this.config.baseUrl;
				break;
			case "edit_key": {
				const val = this.input.getValue().trim();
				this.config.apiKey = val || "not-needed";
				break;
			}
			case "edit_headers": {
				// Header editing is handled separately via sub-view
				return;
			}
		}
		this.mode = "navigate";
		this.renderForm();
	}

	private cancelEdit(): void {
		this.mode = "navigate";
		this.renderForm();
	}

	// -----------------------------------------------------------------------
	// Header editing sub-view
	// -----------------------------------------------------------------------

	private headerEditField: "name" | "value" | "secret" | "delete" | "save" = "name";

	private showHeaderEditView(): void {
		this.listContainer.clear();
		if (!this.headerEdit) return;

		const h = this.headerEdit;

		const hrow = (label: string, value: string, _field: typeof this.headerEditField, selected: boolean) => {
			const prefix = selected ? theme.fg("accent", "→ ") : "  ";
			const lbl = theme.fg(selected ? "accent" : "text", `${label}: `);
			const val = theme.fg(selected ? "accent" : "dim", value);
			this.listContainer.addChild(new Text(`${prefix}${lbl}${val}`, 1, 0));
		};

		const selected = this.headerEditField;

		if (!h.isNew) {
			this.listContainer.addChild(new Text(theme.fg("muted", "  Editing header:"), 1, 0));
			this.listContainer.addChild(new Spacer(1));
		}

		this.listContainer.addChild(new Text(theme.fg("text", "  Header name:"), 1, 0));
		this.listContainer.addChild(new Text(`    ${h.key || "(empty)"}`, 1, 0));
		this.listContainer.addChild(new Spacer(1));
		this.listContainer.addChild(new Text(theme.fg("text", "  Header value:"), 1, 0));
		this.listContainer.addChild(new Text(`    ${h.secret ? "••••••••" : h.value || "(empty)"}`, 1, 0));
		this.listContainer.addChild(new Spacer(1));

		hrow("Secret", h.secret ? "yes (hidden in config)" : "no (plain text)", "secret", selected === "secret");

		if (!h.isNew) {
			this.listContainer.addChild(new Spacer(1));
			const delSelected = selected === "delete";
			const delPrefix = delSelected ? theme.fg("error", "→ ") : "  ";
			const delText = delSelected ? theme.fg("error", "Delete this header") : theme.fg("dim", "Delete this header");
			this.listContainer.addChild(new Text(`${delPrefix}${delText}`, 1, 0));
		}

		this.listContainer.addChild(new Spacer(1));
		const saveSelected = selected === "save";
		const savePrefix = saveSelected ? theme.fg("accent", "→ ") : "  ";
		const saveText = saveSelected ? theme.fg("accent", "Save header") : theme.fg("dim", "Save header");
		this.listContainer.addChild(new Text(`${savePrefix}${saveText}`, 1, 0));

		this.hintText.setText(
			rawKeyHint("↑↓", "navigate") +
				"  " +
				keyHint("tui.select.confirm", "edit") +
				"  " +
				keyHint("tui.select.cancel", "back"),
		);

		this.tui.requestRender();
	}

	private navigateHeaderEdit(direction: number): void {
		const fields: Array<typeof this.headerEditField> = this.headerEdit?.isNew
			? ["name", "value", "secret", "save"]
			: ["name", "value", "secret", "delete", "save"];
		const idx = fields.indexOf(this.headerEditField);
		const next = (idx + direction + fields.length) % fields.length;
		this.headerEditField = fields[next];
		this.showHeaderEditView();
	}

	private activateHeaderEdit(): void {
		if (!this.headerEdit) return;

		switch (this.headerEditField) {
			case "name": {
				// Edit header name via inline input
				this.mode = "edit_headers";
				this.input.setValue(this.headerEdit.key);
				this.listContainer.clear();
				this.listContainer.addChild(new Text(theme.fg("text", "Header name:"), 1, 0));
				this.listContainer.addChild(this.input);
				this.tui.requestRender();
				// Override input submit to capture header name
				const origSubmit = this.input.onSubmit;
				this.input.onSubmit = () => {
					this.headerEdit!.key = this.input.getValue().trim();
					this.input.onSubmit = origSubmit;
					this.showHeaderEditView();
				};
				return;
			}
			case "value": {
				this.mode = "edit_headers";
				this.input.setValue(this.headerEdit.secret ? "" : this.headerEdit.value);
				this.listContainer.clear();
				this.listContainer.addChild(new Text(theme.fg("text", "Header value:"), 1, 0));
				this.listContainer.addChild(this.input);
				this.tui.requestRender();
				const origSubmit = this.input.onSubmit;
				this.input.onSubmit = () => {
					this.headerEdit!.value = this.input.getValue().trim();
					this.input.onSubmit = origSubmit;
					this.showHeaderEditView();
				};
				return;
			}
			case "secret":
				this.headerEdit.secret = !this.headerEdit.secret;
				this.showHeaderEditView();
				return;
			case "delete": {
				const idx = this.config.headers.findIndex(
					(h) => h.key === this.headerEdit!.key && h.value === this.headerEdit!.value,
				);
				if (idx >= 0) {
					this.config.headers.splice(idx, 1);
				}
				this.headerEdit = null;
				this.mode = "navigate";
				this.selectedField = Math.min(this.selectedField, this.fieldCount() - 1);
				this.renderForm();
				return;
			}
			case "save": {
				const existing = this.config.headers.findIndex((h) => h.key === this.headerEdit!.key);
				const newHeader: EndpointHeader = {
					key: this.headerEdit.key,
					value: this.headerEdit.value,
					secret: this.headerEdit.secret,
				};
				if (existing >= 0 && this.headerEdit.isNew !== true) {
					this.config.headers[existing] = newHeader;
				} else if (this.headerEdit.key) {
					this.config.headers.push(newHeader);
				}
				this.headerEdit = null;
				this.mode = "navigate";
				this.renderForm();
				return;
			}
		}
	}

	// -----------------------------------------------------------------------
	// Input handling
	// -----------------------------------------------------------------------

	handleInput(keyData: string): void {
		const kb = getKeybindings();

		if (kb.matches(keyData, "app.tools.expand")) {
			this.onToggleToolsExpanded?.();
			return;
		}

		if (kb.matches(keyData, "tui.select.cancel")) {
			if (this.mode === "edit_headers" && this.headerEdit) {
				this.showHeaderEditView();
				return;
			}
			if (this.mode !== "navigate") {
				this.cancelEdit();
				return;
			}
			this.onCancel();
			return;
		}

		if (this.mode === "edit_headers" && this.headerEdit) {
			if (kb.matches(keyData, "tui.input.submit") || keyData === "\n") {
				// Check if input is active (name/value editing)
				if (this.listContainer.children.find((c) => c === this.input)) {
					this.input.handleInput(keyData);
					return;
				}
				this.activateHeaderEdit();
				return;
			}
			if (kb.matches(keyData, "tui.select.up") || keyData === "k") {
				this.navigateHeaderEdit(-1);
				return;
			}
			if (kb.matches(keyData, "tui.select.down") || keyData === "j") {
				this.navigateHeaderEdit(1);
				return;
			}
			// Pass to input if visible
			if (this.listContainer.children.find((c) => c === this.input)) {
				this.input.handleInput(keyData);
			}
			return;
		}

		if (this.mode === "edit_url" || this.mode === "edit_key") {
			this.input.handleInput(keyData);
			return;
		}

		// Navigate mode
		if (kb.matches(keyData, "tui.select.up") || keyData === "k") {
			this.selectedField = Math.max(0, this.selectedField - 1);
			this.renderForm();
			return;
		}
		if (kb.matches(keyData, "tui.select.down") || keyData === "j") {
			this.selectedField = Math.min(this.fieldCount() - 1, this.selectedField + 1);
			this.renderForm();
			return;
		}
		if (kb.matches(keyData, "tui.select.confirm") || keyData === "\n" || keyData === " ") {
			this.startEdit();
			return;
		}
	}
}

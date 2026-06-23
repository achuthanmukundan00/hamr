/**
 * Endpoint configuration component — TUI form for adding custom/self-hosted
 * OpenAI-compatible or Anthropic-compatible endpoints via models.json.
 */
import { Container, type TUI } from "@hamr/tui";
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
export declare const ENDPOINT_PRESETS: EndpointPreset[];
export declare class EndpointConfigComponent extends Container {
    private tui;
    private onSave;
    private onCancel;
    private onToggleToolsExpanded;
    private presetIndex;
    private config;
    private mode;
    private selectedField;
    private input;
    private headerEdit;
    private titleText;
    private listContainer;
    private hintText;
    constructor(tui: TUI, onSave: (config: EndpointConfig) => void, onCancel: () => void);
    private fieldCount;
    private renderForm;
    private getSelectedFieldType;
    private startEdit;
    private cyclePreset;
    private cycleApi;
    private showEditPrompt;
    private commitEdit;
    private cancelEdit;
    private headerEditField;
    private showHeaderEditView;
    private navigateHeaderEdit;
    private activateHeaderEdit;
    handleInput(keyData: string): void;
}
//# sourceMappingURL=endpoint-config.d.ts.map
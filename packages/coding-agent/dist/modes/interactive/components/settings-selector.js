import { Container, getCapabilities, SelectList, SettingsList, Spacer, Text, } from "@hamr/tui";
import { formatHttpIdleTimeoutMs, HTTP_IDLE_TIMEOUT_CHOICES } from "../../../core/http-dispatcher.js";
import { getSelectListTheme, getSettingsListTheme, theme } from "../theme/theme.js";
import { DynamicBorder } from "./dynamic-border.js";
import { keyDisplayText } from "./keybinding-hints.js";
const SETTINGS_SUBMENU_SELECT_LIST_LAYOUT = {
    minPrimaryColumnWidth: 12,
    maxPrimaryColumnWidth: 32,
};
const THINKING_DESCRIPTIONS = {
    off: "No reasoning",
    minimal: "Very brief reasoning (~1k tokens)",
    low: "Light reasoning (~2k tokens)",
    medium: "Moderate reasoning (~8k tokens)",
    high: "Deep reasoning (~16k tokens)",
    xhigh: "Maximum reasoning (~32k tokens)",
};
const DEFAULT_PROJECT_TRUST_LABELS = {
    ask: "Ask",
    always: "Always trust",
    never: "Never trust",
};
const DEFAULT_PROJECT_TRUST_BY_LABEL = new Map(Object.entries(DEFAULT_PROJECT_TRUST_LABELS).map(([value, label]) => [label, value]));
function hexToAnsiFg(hex) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `\x1b[38;2;${r};${g};${b}m`;
}
/**
 * A submenu component for selecting from a list of options.
 */
class WarningSettingsSubmenu extends Container {
    constructor(warnings, onChange, onCancel) {
        super();
        this.state = { ...warnings };
        const items = [
            {
                id: "anthropic-extra-usage",
                label: "Anthropic extra usage",
                description: "Warn when Anthropic subscription auth may use paid extra usage",
                currentValue: (this.state.anthropicExtraUsage ?? true) ? "true" : "false",
                values: ["true", "false"],
            },
        ];
        this.settingsList = new SettingsList(items, Math.min(items.length, 10), getSettingsListTheme(), (id, newValue) => {
            switch (id) {
                case "anthropic-extra-usage":
                    this.state = { ...this.state, anthropicExtraUsage: newValue === "true" };
                    onChange({ ...this.state });
                    break;
            }
        }, onCancel);
        this.addChild(this.settingsList);
    }
    handleInput(data) {
        this.settingsList.handleInput(data);
    }
}
class SelectSubmenu extends Container {
    constructor(title, description, options, currentValue, onSelect, onCancel, onSelectionChange, modelAccent) {
        super();
        // Title — uses model brand color when modelAdaptive, theme accent otherwise
        const titleColor = modelAccent && theme.modelAdaptive
            ? (s) => `${hexToAnsiFg(modelAccent)}${s}\x1b[39m`
            : (s) => theme.fg("accent", s);
        this.addChild(new Text(theme.bold(titleColor(title)), 0, 0));
        // Description
        if (description) {
            this.addChild(new Spacer(1));
            this.addChild(new Text(theme.fg("muted", description), 0, 0));
        }
        // Spacer
        this.addChild(new Spacer(1));
        // Select list
        this.selectList = new SelectList(options, Math.min(options.length, 10), getSelectListTheme(), SETTINGS_SUBMENU_SELECT_LIST_LAYOUT);
        // Pre-select current value
        const currentIndex = options.findIndex((o) => o.value === currentValue);
        if (currentIndex !== -1) {
            this.selectList.setSelectedIndex(currentIndex);
        }
        this.selectList.onSelect = (item) => {
            onSelect(item.value);
        };
        this.selectList.onCancel = onCancel;
        if (onSelectionChange) {
            this.selectList.onSelectionChange = (item) => {
                onSelectionChange(item.value);
            };
        }
        this.addChild(this.selectList);
        // Hint
        this.addChild(new Spacer(1));
        this.addChild(new Text(theme.fg("dim", "  Enter to select · Esc to go back"), 0, 0));
    }
    handleInput(data) {
        this.selectList.handleInput(data);
    }
}
/**
 * Main settings selector component.
 */
export class SettingsSelectorComponent extends Container {
    constructor(config, callbacks) {
        super();
        const supportsImages = getCapabilities().images;
        const followUpKey = keyDisplayText("app.message.followUp");
        let currentWarnings = { ...config.warnings };
        const accent = config.modelAccent;
        const items = [
            // ── Model ──
            {
                id: "thinking",
                label: "Thinking level",
                description: "Reasoning depth for thinking-capable models",
                currentValue: config.thinkingLevel,
                section: "Model",
                submenu: (currentValue, done) => new SelectSubmenu("Thinking Level", "Select reasoning depth for thinking-capable models", config.availableThinkingLevels.map((level) => ({
                    value: level,
                    label: level,
                    description: THINKING_DESCRIPTIONS[level],
                })), currentValue, (value) => {
                    callbacks.onThinkingLevelChange(value);
                    done(value);
                }, () => done(), undefined, accent),
            },
            {
                id: "transport",
                label: "Transport",
                description: "Preferred transport for providers that support multiple transports",
                currentValue: config.transport,
                values: ["sse", "websocket", "websocket-cached", "auto"],
            },
            // ── Display ──
            {
                id: "theme",
                label: "Theme",
                description: "Color theme for the interface",
                currentValue: config.currentTheme,
                section: "Display",
                submenu: (currentValue, done) => new SelectSubmenu("Theme", "Select color theme", config.availableThemes.map((t) => ({
                    value: t,
                    label: t,
                })), currentValue, (value) => {
                    callbacks.onThemeChange(value);
                    done(value);
                }, () => {
                    // Restore original theme on cancel
                    callbacks.onThemePreview?.(currentValue);
                    done();
                }, (value) => {
                    // Preview theme on selection change
                    callbacks.onThemePreview?.(value);
                }, accent),
            },
            {
                id: "hide-thinking",
                label: "Hide thinking",
                description: "Hide thinking blocks in assistant responses",
                currentValue: config.hideThinkingBlock ? "true" : "false",
                values: ["true", "false"],
            },
            {
                id: "collapse-changelog",
                label: "Collapse changelog",
                description: "Show condensed changelog after updates",
                currentValue: config.collapseChangelog ? "true" : "false",
                values: ["true", "false"],
            },
            {
                id: "tree-filter-mode",
                label: "Tree filter mode",
                description: "Default filter when opening /tree",
                currentValue: config.treeFilterMode,
                values: ["default", "no-tools", "user-only", "labeled-only", "all"],
            },
            {
                id: "autocompact",
                label: "Auto-compact",
                description: "Automatically compact context when it gets too large",
                currentValue: config.autoCompact ? "true" : "false",
                values: ["true", "false"],
            },
            {
                id: "clear-on-shrink",
                label: "Clear on shrink",
                description: "Clear empty rows when content shrinks (may cause flicker)",
                currentValue: config.clearOnShrink ? "true" : "false",
                values: ["true", "false"],
            },
            // ── Session ──
            {
                id: "steering-mode",
                label: "Steering mode",
                description: "Enter while streaming queues steering messages. 'one-at-a-time': deliver one, wait for response. 'all': deliver all at once.",
                currentValue: config.steeringMode,
                values: ["one-at-a-time", "all"],
                section: "Session",
            },
            {
                id: "follow-up-mode",
                label: "Follow-up mode",
                description: `${followUpKey} queues follow-up messages until agent stops. 'one-at-a-time': deliver one, wait for response. 'all': deliver all at once.`,
                currentValue: config.followUpMode,
                values: ["one-at-a-time", "all"],
            },
            {
                id: "double-escape-action",
                label: "Double-escape action",
                description: "Action when pressing Escape twice with empty editor",
                currentValue: config.doubleEscapeAction,
                values: ["tree", "fork", "none"],
            },
            {
                id: "quiet-startup",
                label: "Quiet startup",
                description: "Disable verbose printing at startup",
                currentValue: config.quietStartup ? "true" : "false",
                values: ["true", "false"],
            },
            {
                id: "skill-commands",
                label: "Skill commands",
                description: "Register skills as /skill:name commands",
                currentValue: config.enableSkillCommands ? "true" : "false",
                values: ["true", "false"],
            },
            {
                id: "install-telemetry",
                label: "Install telemetry",
                description: "Send an anonymous version/update ping after changelog-detected updates",
                currentValue: config.enableInstallTelemetry ? "true" : "false",
                values: ["true", "false"],
            },
            {
                id: "default-project-trust",
                label: "Default project trust",
                description: "Fallback behavior when no extension or saved trust decision decides project trust",
                currentValue: DEFAULT_PROJECT_TRUST_LABELS[config.defaultProjectTrust],
                values: Object.values(DEFAULT_PROJECT_TRUST_LABELS),
            },
            {
                id: "warnings",
                label: "Warnings",
                description: "Enable or disable individual warnings",
                currentValue: "configure",
                submenu: (_currentValue, done) => new WarningSettingsSubmenu(currentWarnings, (warnings) => {
                    currentWarnings = warnings;
                    callbacks.onWarningsChange(warnings);
                }, () => done()),
            },
            // ── Network ──
            {
                id: "http-idle-timeout",
                label: "HTTP idle timeout",
                description: "Maximum idle gap while waiting for HTTP headers or body chunks. Disable for local models that pause longer than five minutes.",
                currentValue: formatHttpIdleTimeoutMs(config.httpIdleTimeoutMs),
                values: HTTP_IDLE_TIMEOUT_CHOICES.map((choice) => choice.label),
                section: "Network",
            },
        ];
        // Insert conditional display items after theme (index 2 in base).
        // Image toggles only when terminal supports it.
        let insertAt = items.findIndex((i) => i.id === "theme") + 1;
        if (supportsImages) {
            items.splice(insertAt++, 0, {
                id: "show-images",
                label: "Show images",
                description: "Render images inline in terminal",
                currentValue: config.showImages ? "true" : "false",
                values: ["true", "false"],
            });
            items.splice(insertAt++, 0, {
                id: "image-width-cells",
                label: "Image width",
                description: "Preferred inline image width in terminal cells",
                currentValue: String(config.imageWidthCells),
                values: ["60", "80", "120"],
            });
        }
        items.splice(insertAt++, 0, {
            id: "auto-resize-images",
            label: "Auto-resize images",
            description: "Resize large images to 2000x2000 max for better model compatibility",
            currentValue: config.autoResizeImages ? "true" : "false",
            values: ["true", "false"],
        });
        items.splice(insertAt++, 0, {
            id: "block-images",
            label: "Block images",
            description: "Prevent images from being sent to LLM providers",
            currentValue: config.blockImages ? "true" : "false",
            values: ["true", "false"],
        });
        items.splice(insertAt++, 0, {
            id: "show-hardware-cursor",
            label: "Show hardware cursor",
            description: "Show the terminal cursor while still positioning it for IME support",
            currentValue: config.showHardwareCursor ? "true" : "false",
            values: ["true", "false"],
        });
        items.splice(insertAt++, 0, {
            id: "editor-padding",
            label: "Editor padding",
            description: "Horizontal padding for input editor (0-3)",
            currentValue: String(config.editorPaddingX),
            values: ["0", "1", "2", "3"],
        });
        items.splice(insertAt++, 0, {
            id: "autocomplete-max-visible",
            label: "Autocomplete max items",
            description: "Max visible items in autocomplete dropdown (3-20)",
            currentValue: String(config.autocompleteMaxVisible),
            values: ["3", "5", "7", "10", "15", "20"],
        });
        items.splice(insertAt++, 0, {
            id: "terminal-progress",
            label: "Terminal progress",
            description: "Show OSC 9;4 progress indicators in the terminal tab bar",
            currentValue: config.showTerminalProgress ? "true" : "false",
            values: ["true", "false"],
        });
        // Add borders
        this.addChild(new DynamicBorder());
        this.settingsList = new SettingsList(items, 10, getSettingsListTheme(), (id, newValue) => {
            switch (id) {
                case "autocompact":
                    callbacks.onAutoCompactChange(newValue === "true");
                    break;
                case "show-images":
                    callbacks.onShowImagesChange(newValue === "true");
                    break;
                case "image-width-cells":
                    callbacks.onImageWidthCellsChange(parseInt(newValue, 10));
                    break;
                case "auto-resize-images":
                    callbacks.onAutoResizeImagesChange(newValue === "true");
                    break;
                case "block-images":
                    callbacks.onBlockImagesChange(newValue === "true");
                    break;
                case "skill-commands":
                    callbacks.onEnableSkillCommandsChange(newValue === "true");
                    break;
                case "steering-mode":
                    callbacks.onSteeringModeChange(newValue);
                    break;
                case "follow-up-mode":
                    callbacks.onFollowUpModeChange(newValue);
                    break;
                case "transport":
                    callbacks.onTransportChange(newValue);
                    break;
                case "http-idle-timeout": {
                    const choice = HTTP_IDLE_TIMEOUT_CHOICES.find((item) => item.label === newValue);
                    if (choice) {
                        callbacks.onHttpIdleTimeoutMsChange(choice.timeoutMs);
                    }
                    break;
                }
                case "hide-thinking":
                    callbacks.onHideThinkingBlockChange(newValue === "true");
                    break;
                case "collapse-changelog":
                    callbacks.onCollapseChangelogChange(newValue === "true");
                    break;
                case "quiet-startup":
                    callbacks.onQuietStartupChange(newValue === "true");
                    break;
                case "install-telemetry":
                    callbacks.onEnableInstallTelemetryChange(newValue === "true");
                    break;
                case "default-project-trust": {
                    const defaultProjectTrust = DEFAULT_PROJECT_TRUST_BY_LABEL.get(newValue);
                    if (defaultProjectTrust) {
                        callbacks.onDefaultProjectTrustChange(defaultProjectTrust);
                    }
                    break;
                }
                case "double-escape-action":
                    callbacks.onDoubleEscapeActionChange(newValue);
                    break;
                case "tree-filter-mode":
                    callbacks.onTreeFilterModeChange(newValue);
                    break;
                case "show-hardware-cursor":
                    callbacks.onShowHardwareCursorChange(newValue === "true");
                    break;
                case "editor-padding":
                    callbacks.onEditorPaddingXChange(parseInt(newValue, 10));
                    break;
                case "autocomplete-max-visible":
                    callbacks.onAutocompleteMaxVisibleChange(parseInt(newValue, 10));
                    break;
                case "clear-on-shrink":
                    callbacks.onClearOnShrinkChange(newValue === "true");
                    break;
                case "terminal-progress":
                    callbacks.onShowTerminalProgressChange(newValue === "true");
                    break;
            }
        }, callbacks.onCancel, { enableSearch: true });
        this.addChild(this.settingsList);
        this.addChild(new DynamicBorder());
    }
    getSettingsList() {
        return this.settingsList;
    }
}
//# sourceMappingURL=settings-selector.js.map
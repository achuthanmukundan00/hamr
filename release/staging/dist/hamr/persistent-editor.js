import { Key, sliceWithWidth } from "@hamr/tui";
class PersistentFooterComponent {
    constructor(ctx, theme, footerData) {
        this.ctx = ctx;
        this.theme = theme;
        this.footerData = footerData;
    }
    invalidate() { }
    dispose() { }
    render(width) {
        const lines = [];
        const th = this.theme;
        const editorText = this.ctx.ui.getEditorText();
        const promptPrefix = "persistent> ";
        const promptStr = editorText.length > 0 ? `${promptPrefix}${editorText}` : `${promptPrefix}${th.fg("dim", "type a message...")}`;
        lines.push(promptStr.length > width ? promptStr.slice(0, width) : promptStr);
        const cwd = this.ctx.cwd;
        const branch = this.footerData.getGitBranch();
        const pwdStr = branch ? `${cwd} (${branch})` : cwd;
        lines.push(th.fg("dim", pwdStr.length > width ? pwdStr.slice(0, width) : pwdStr));
        // Model name with brand color (model-aware coloring)
        const model = this.ctx.model;
        const modelName = model?.id ?? "no-model";
        const modelColorAnsi = model ? th.modelColor(model.provider, model.id) : th.getFgAnsi("dim");
        const modelReset = "\x1b[39m";
        let statsLine = `${modelColorAnsi}${modelName}${modelReset}`;
        try {
            const usage = this.ctx.getContextUsage();
            if (usage) {
                const pct = usage.percent !== null ? `${usage.percent.toFixed(1)}%` : "?";
                const windowStr = usage.contextWindow > 0
                    ? usage.contextWindow < 1000
                        ? String(usage.contextWindow)
                        : `${Math.round(usage.contextWindow / 1000)}k`
                    : "?";
                statsLine = `${modelColorAnsi}${modelName}${modelReset} ${th.fg("dim", `| ${pct}/${windowStr}`)}`;
            }
        }
        catch { }
        lines.push(sliceWithWidth(statsLine, 0, width, true).text);
        const extensionStatuses = this.footerData.getExtensionStatuses();
        if (extensionStatuses.size > 0) {
            const sortedStatuses = Array.from(extensionStatuses.entries())
                .sort((a, b) => a[0].localeCompare(b[0]))
                .map((entry) => entry[1]);
            const statusLine = sortedStatuses.join(" ");
            lines.push(sliceWithWidth(statusLine, 0, width, true).text);
        }
        return lines;
    }
}
export function createPersistentEditorExtension() {
    let persistentEnabled = false;
    return function persistentEditorExtension(pi) {
        function togglePersistentMode(ctx) {
            persistentEnabled = !persistentEnabled;
            if (persistentEnabled) {
                enablePersistentMode(ctx);
            }
            else {
                disablePersistentMode(ctx);
            }
        }
        function enablePersistentMode(ctx) {
            ctx.ui.setFooter((_tui, theme, footerData) => new PersistentFooterComponent(ctx, theme, footerData));
            ctx.ui.notify(`Persistent editor ${ctx.ui.theme.fg("success", "enabled")} (Shift+Ctrl+U to toggle)`, "info");
        }
        function disablePersistentMode(ctx) {
            ctx.ui.setFooter(undefined);
            ctx.ui.notify(`Persistent editor ${ctx.ui.theme.fg("muted", "disabled")}`, "info");
        }
        pi.registerShortcut(Key.shiftCtrl("u"), {
            description: "Toggle persistent editor mode (keep editor open at bottom)",
            handler: async (ctx) => {
                togglePersistentMode(ctx);
            },
        });
        pi.registerCommand("persistent-editor", {
            description: "Toggle persistent editor mode (keep editor open at bottom)",
            handler: async (_args, ctx) => {
                togglePersistentMode(ctx);
            },
        });
    };
}
//# sourceMappingURL=persistent-editor.js.map
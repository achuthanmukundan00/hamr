import { Box, Container, getCapabilities, Image, Spacer, Text } from "@hamr/tui";
import { createAllToolDefinitions } from "../../../core/tools/index.js";
import { getTextOutput as getRenderedTextOutput } from "../../../core/tools/render-utils.js";
import { convertToPng } from "../../../utils/image-convert.js";
import { theme } from "../theme/theme.js";
export class ToolExecutionComponent extends Container {
    constructor(toolName, toolCallId, args, options = {}, toolDefinition, ui, cwd) {
        super();
        this.rendererState = {};
        this.imageComponents = [];
        this.imageSpacers = [];
        this.expanded = false;
        this.isPartial = true;
        this.executionStarted = false;
        this.argsComplete = false;
        this.convertedImages = new Map();
        this.hideComponent = false;
        this.toolName = toolName;
        this.toolCallId = toolCallId;
        this.args = args;
        this.toolDefinition = toolDefinition;
        this.builtInToolDefinition = createAllToolDefinitions(cwd)[toolName];
        this.showImages = options.showImages ?? true;
        this.imageWidthCells = options.imageWidthCells ?? 60;
        this.ui = ui;
        this.cwd = cwd;
        // Always create all shell variants. contentBox is used for default renderer-based composition.
        // selfRenderContainer is used when the tool renders its own framing.
        // contentText is reserved for generic fallback rendering when no tool definition exists.
        const pendingBgFn = theme.cards.shadedSurfaces ? (text) => theme.bg("toolPendingBg", text) : undefined;
        this.contentBox = new Box(theme.cards.cardPadX, theme.cards.cardPadY, pendingBgFn);
        this.contentText = new Text("", theme.cards.toolIndent, theme.cards.cardPadY, pendingBgFn);
        this.selfRenderContainer = new Container();
        if (this.hasRendererDefinition()) {
            this.addChild(this.getRenderShell() === "self" ? this.selfRenderContainer : this.contentBox);
        }
        else {
            this.addChild(this.contentText);
        }
        this.updateDisplay();
    }
    getCallRenderer() {
        if (!this.builtInToolDefinition) {
            return this.toolDefinition?.renderCall;
        }
        if (!this.toolDefinition) {
            return this.builtInToolDefinition.renderCall;
        }
        return this.toolDefinition.renderCall ?? this.builtInToolDefinition.renderCall;
    }
    getResultRenderer() {
        if (!this.builtInToolDefinition) {
            return this.toolDefinition?.renderResult;
        }
        if (!this.toolDefinition) {
            return this.builtInToolDefinition.renderResult;
        }
        return this.toolDefinition.renderResult ?? this.builtInToolDefinition.renderResult;
    }
    hasRendererDefinition() {
        return this.builtInToolDefinition !== undefined || this.toolDefinition !== undefined;
    }
    getRenderShell() {
        if (!this.builtInToolDefinition) {
            return this.toolDefinition?.renderShell ?? "default";
        }
        if (!this.toolDefinition) {
            return this.builtInToolDefinition.renderShell ?? "default";
        }
        return this.toolDefinition.renderShell ?? this.builtInToolDefinition.renderShell ?? "default";
    }
    getRenderContext(lastComponent) {
        return {
            args: this.args,
            toolCallId: this.toolCallId,
            invalidate: () => {
                this.invalidate();
                this.ui.requestRender();
            },
            lastComponent,
            state: this.rendererState,
            cwd: this.cwd,
            executionStarted: this.executionStarted,
            argsComplete: this.argsComplete,
            isPartial: this.isPartial,
            expanded: this.expanded,
            showImages: this.showImages,
            isError: this.result?.isError ?? false,
        };
    }
    createCallFallback() {
        return new Text(theme.fg("toolTitle", theme.bold(this.toolName)), theme.cards.toolIndent, 0);
    }
    createResultFallback() {
        const output = this.getTextOutput();
        if (!output) {
            return undefined;
        }
        return new Text(theme.fg("toolOutput", output), theme.cards.toolResultIndent, 0);
    }
    updateArgs(args) {
        this.args = args;
        this.updateDisplay();
    }
    markExecutionStarted() {
        this.executionStarted = true;
        this.updateDisplay();
        this.ui.requestRender();
    }
    setArgsComplete() {
        this.argsComplete = true;
        this.updateDisplay();
        this.ui.requestRender();
    }
    updateResult(result, isPartial = false) {
        this.result = result;
        this.isPartial = isPartial;
        this.updateDisplay();
        this.maybeConvertImagesForKitty();
    }
    maybeConvertImagesForKitty() {
        const caps = getCapabilities();
        if (caps.images !== "kitty")
            return;
        if (!this.result)
            return;
        const imageBlocks = this.result.content.filter((c) => c.type === "image");
        for (let i = 0; i < imageBlocks.length; i++) {
            const img = imageBlocks[i];
            if (!img.data || !img.mimeType)
                continue;
            if (img.mimeType === "image/png")
                continue;
            if (this.convertedImages.has(i))
                continue;
            const index = i;
            convertToPng(img.data, img.mimeType).then((converted) => {
                if (converted) {
                    this.convertedImages.set(index, converted);
                    this.updateDisplay();
                    this.ui.requestRender();
                }
            });
        }
    }
    setExpanded(expanded) {
        this.expanded = expanded;
        this.updateDisplay();
    }
    setShowImages(show) {
        this.showImages = show;
        this.updateDisplay();
    }
    setImageWidthCells(width) {
        this.imageWidthCells = Math.max(1, Math.floor(width));
        this.updateDisplay();
    }
    invalidate() {
        super.invalidate();
        this.updateDisplay();
    }
    getCardPaddingY() {
        return theme.cards.cardPadY;
    }
    render(width) {
        if (this.hideComponent) {
            return [];
        }
        if (this.hasRendererDefinition() && this.getRenderShell() === "self") {
            // Self-rendering tools draw their own framing. Keep this wrapper flush
            // so shaded surfaces start at the same column as the rest of the card.
            const innerWidth = width;
            const contentLines = this.selfRenderContainer.render(innerWidth);
            if (contentLines.length === 0 && this.imageComponents.length === 0) {
                return [];
            }
            const lines = [];
            if (contentLines.length > 0) {
                if (!theme.cards.gaplessCards) {
                    lines.push("");
                }
                lines.push(...contentLines);
            }
            for (let i = 0; i < this.imageComponents.length; i++) {
                const spacer = this.imageSpacers[i];
                if (spacer) {
                    lines.push(...spacer.render(innerWidth));
                }
                const imageComponent = this.imageComponents[i];
                if (imageComponent) {
                    lines.push(...imageComponent.render(innerWidth));
                }
            }
            return lines;
        }
        return super.render(width);
    }
    updateDisplay() {
        const bgFn = !theme.cards.shadedSurfaces
            ? undefined
            : this.isPartial
                ? (text) => theme.bg("toolPendingBg", text)
                : this.result?.isError
                    ? (text) => theme.bg("toolErrorBg", text)
                    : (text) => theme.bg("toolSuccessBg", text);
        const cardPaddingY = this.getCardPaddingY();
        let hasContent = false;
        this.hideComponent = false;
        if (this.hasRendererDefinition()) {
            const renderContainer = this.getRenderShell() === "self" ? this.selfRenderContainer : this.contentBox;
            if (renderContainer instanceof Box) {
                renderContainer.setBgFn(bgFn);
                renderContainer.setPaddingY(cardPaddingY);
            }
            renderContainer.clear();
            const callRenderer = this.getCallRenderer();
            if (!callRenderer) {
                renderContainer.addChild(this.createCallFallback());
                hasContent = true;
            }
            else {
                try {
                    const component = callRenderer(this.args, theme, this.getRenderContext(this.callRendererComponent));
                    this.callRendererComponent = component;
                    renderContainer.addChild(component);
                    hasContent = true;
                }
                catch {
                    this.callRendererComponent = undefined;
                    renderContainer.addChild(this.createCallFallback());
                    hasContent = true;
                }
            }
            if (this.result) {
                const resultRenderer = this.getResultRenderer();
                if (!resultRenderer) {
                    const component = this.createResultFallback();
                    if (component) {
                        renderContainer.addChild(component);
                        hasContent = true;
                    }
                }
                else {
                    try {
                        const component = resultRenderer({ content: this.result.content, details: this.result.details }, { expanded: this.expanded, isPartial: this.isPartial }, theme, this.getRenderContext(this.resultRendererComponent));
                        this.resultRendererComponent = component;
                        renderContainer.addChild(component);
                        hasContent = true;
                    }
                    catch {
                        this.resultRendererComponent = undefined;
                        const component = this.createResultFallback();
                        if (component) {
                            renderContainer.addChild(component);
                            hasContent = true;
                        }
                    }
                }
            }
        }
        else {
            this.contentText.setCustomBgFn(bgFn);
            this.contentText.setPaddingY(cardPaddingY);
            this.contentText.setText(this.formatToolExecution());
            hasContent = true;
        }
        for (const img of this.imageComponents) {
            this.removeChild(img);
        }
        this.imageComponents = [];
        for (const spacer of this.imageSpacers) {
            this.removeChild(spacer);
        }
        this.imageSpacers = [];
        if (this.result) {
            const imageBlocks = this.result.content.filter((c) => c.type === "image");
            const caps = getCapabilities();
            for (let i = 0; i < imageBlocks.length; i++) {
                const img = imageBlocks[i];
                if (caps.images && this.showImages && img.data && img.mimeType) {
                    const converted = this.convertedImages.get(i);
                    const imageData = converted?.data ?? img.data;
                    const imageMimeType = converted?.mimeType ?? img.mimeType;
                    if (caps.images === "kitty" && imageMimeType !== "image/png")
                        continue;
                    const spacer = new Spacer(1);
                    this.addChild(spacer);
                    this.imageSpacers.push(spacer);
                    const imageComponent = new Image(imageData, imageMimeType, { fallbackColor: (s) => theme.fg("toolOutput", s) }, { maxWidthCells: this.imageWidthCells });
                    this.imageComponents.push(imageComponent);
                    this.addChild(imageComponent);
                }
            }
        }
        if (this.hasRendererDefinition() && !hasContent && this.imageComponents.length === 0) {
            this.hideComponent = true;
        }
    }
    getTextOutput() {
        return getRenderedTextOutput(this.result, this.showImages);
    }
    formatToolExecution() {
        let text = theme.fg("toolTitle", theme.bold(this.toolName));
        const content = JSON.stringify(this.args, null, 2);
        if (content) {
            text += `\n\n${content}`;
        }
        const output = this.getTextOutput();
        if (output) {
            text += `\n${output}`;
        }
        return text;
    }
}
//# sourceMappingURL=tool-execution.js.map
import { Text } from "./text.js";
const DEFAULT_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const DEFAULT_INTERVAL_MS = 80;
/**
 * Loader component that updates with an optional spinning animation.
 */
export class Loader extends Text {
    constructor(ui, spinnerColorFn, messageColorFn, message = "Loading...", indicator) {
        super("", 1, 0);
        this.frames = [...DEFAULT_FRAMES];
        this.intervalMs = DEFAULT_INTERVAL_MS;
        this.currentFrame = 0;
        this.intervalId = null;
        this.ui = null;
        this.renderIndicatorVerbatim = false;
        this.message = "Loading...";
        this.ui = ui;
        this.spinnerColorFn = spinnerColorFn;
        this.messageColorFn = messageColorFn;
        this.message = message;
        this.setIndicator(indicator);
    }
    render(width) {
        return ["", ...super.render(width)];
    }
    start() {
        this.updateDisplay();
        this.restartAnimation();
    }
    stop() {
        if (this.intervalId) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }
    }
    setMessage(message) {
        this.message = message;
        this.updateDisplay();
    }
    setIndicator(indicator) {
        this.renderIndicatorVerbatim = indicator !== undefined;
        this.frames = indicator?.frames !== undefined ? [...indicator.frames] : [...DEFAULT_FRAMES];
        this.intervalMs = indicator?.intervalMs && indicator.intervalMs > 0 ? indicator.intervalMs : DEFAULT_INTERVAL_MS;
        this.currentFrame = 0;
        this.start();
    }
    restartAnimation() {
        this.stop();
        if (this.frames.length <= 1) {
            return;
        }
        this.intervalId = setInterval(() => {
            this.currentFrame = (this.currentFrame + 1) % this.frames.length;
            this.updateDisplay();
        }, this.intervalMs);
    }
    updateDisplay() {
        const frame = this.frames[this.currentFrame] ?? "";
        const renderedFrame = this.renderIndicatorVerbatim ? frame : this.spinnerColorFn(frame);
        const indicator = frame.length > 0 ? `${renderedFrame} ` : "";
        this.setText(`${indicator}${this.messageColorFn(this.message)}`);
        if (this.ui) {
            this.ui.requestRender();
        }
    }
}
//# sourceMappingURL=loader.js.map
import type { Component } from "@hamr/tui";
import type { WorkingIndicatorOptions } from "../core/extensions/types.ts";
import type { Theme } from "../modes/interactive/theme/theme.ts";

export const SHIMMER_FRAMES: string[] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂", "▁"];

export const RAINBOW_FRAMES: string[] = [
	"\x1b[31m█\x1b[0m",
	"\x1b[33m█\x1b[0m",
	"\x1b[32m█\x1b[0m",
	"\x1b[36m█\x1b[0m",
	"\x1b[34m█\x1b[0m",
	"\x1b[35m█\x1b[0m",
];

export const RAINBOW_WORD_FRAMES: string[] = [
	"\x1b[31m● thinking\x1b[0m",
	"\x1b[33m● thinking\x1b[0m",
	"\x1b[32m● thinking\x1b[0m",
	"\x1b[36m● thinking\x1b[0m",
	"\x1b[34m● thinking\x1b[0m",
	"\x1b[35m● thinking\x1b[0m",
];

export function createShimmerIndicator(thinkingMode?: boolean): WorkingIndicatorOptions {
	if (thinkingMode) {
		return { frames: RAINBOW_FRAMES, intervalMs: 200 };
	}
	return { frames: SHIMMER_FRAMES, intervalMs: 120 };
}

export class ShimmerComponent implements Component {
	private readonly text: string;
	private readonly theme: Theme;
	private readonly frames: string[];
	private readonly intervalMs: number;
	private readonly startTime: number;

	constructor(text: string, theme: Theme, options?: { shimmerFrames?: string[]; intervalMs?: number }) {
		this.text = text;
		this.theme = theme;
		this.frames = options?.shimmerFrames ?? SHIMMER_FRAMES;
		this.intervalMs = options?.intervalMs ?? 120;
		this.startTime = Date.now();
	}

	private getCurrentFrame(): string {
		const elapsed = Date.now() - this.startTime;
		const frameIndex = Math.floor(elapsed / this.intervalMs) % this.frames.length;
		return this.frames[frameIndex];
	}

	render(width: number): string[] {
		const frame = this.getCurrentFrame();
		const line = `${frame} ${this.theme.fg("dim", this.text)}`;
		return [line.length > width ? line.slice(0, width) : line];
	}

	invalidate(): void {}
	dispose(): void {}
}

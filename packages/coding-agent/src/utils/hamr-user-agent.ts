export function getHamrUserAgent(version: string): string {
	const runtime = process.versions.bun ? `bun/${process.versions.bun}` : `node/${process.version}`;
	return `hamr/${version} (${process.platform}; ${runtime}; ${process.arch})`;
}

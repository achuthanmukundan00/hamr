export function getHamrUserAgent(version) {
    const runtime = process.versions.bun ? `bun/${process.versions.bun}` : `node/${process.version}`;
    return `hamr/${version} (${process.platform}; ${runtime}; ${process.arch})`;
}
//# sourceMappingURL=hamr-user-agent.js.map
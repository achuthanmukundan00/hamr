import * as undici from "undici";
export const DEFAULT_HTTP_IDLE_TIMEOUT_MS = 300_000;
export const HTTP_IDLE_TIMEOUT_CHOICES = [
    { label: "30 sec", timeoutMs: 30_000 },
    { label: "1 min", timeoutMs: 60_000 },
    { label: "2 min", timeoutMs: 120_000 },
    { label: "5 min", timeoutMs: 300_000 },
    { label: "disabled", timeoutMs: 0 },
];
const originalGlobalFetch = globalThis.fetch;
let installedGlobalFetch;
export function parseHttpIdleTimeoutMs(value) {
    if (typeof value === "string") {
        const trimmed = value.trim();
        if (trimmed.toLowerCase() === "disabled") {
            return 0;
        }
        if (trimmed.length === 0) {
            return undefined;
        }
        return parseHttpIdleTimeoutMs(Number(trimmed));
    }
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
        return undefined;
    }
    return Math.floor(value);
}
export function formatHttpIdleTimeoutMs(timeoutMs) {
    const choice = HTTP_IDLE_TIMEOUT_CHOICES.find((item) => item.timeoutMs === timeoutMs);
    if (choice) {
        return choice.label;
    }
    return `${timeoutMs / 1000} sec`;
}
/**
 * Validate that a proxy URL is well-formed and uses an http(s) scheme.
 * Returns the normalized proxy string on success, or undefined if invalid/empty.
 */
export function validateProxyUrl(httpProxy) {
    const proxy = httpProxy?.trim();
    if (!proxy)
        return undefined;
    let url;
    try {
        url = new URL(proxy);
    }
    catch {
        throw new Error(`Invalid httpProxy value (not a valid URL): ${proxy}. Proxy has NOT been applied.`);
    }
    if (url.protocol !== "http:" && url.protocol !== "https:") {
        throw new Error(`Invalid httpProxy scheme '${url.protocol}' (only http: and https: are allowed). Proxy has NOT been applied.`);
    }
    if (!url.hostname) {
        throw new Error(`Invalid httpProxy value (missing host): ${proxy}. Proxy has NOT been applied.`);
    }
    return proxy;
}
/**
 * Warn prominently when an HTTP proxy is active, since it receives all LLM
 * traffic including Authorization and provider-specific credential headers.
 */
export function warnProxyActive(proxy) {
    if (!proxy)
        return;
    let host;
    try {
        host = new URL(proxy).host;
    }
    catch {
        host = proxy;
    }
    console.warn(`[hamr] HTTP proxy is active (${host}). All LLM provider requests, including API key and provider credential headers, will be routed through this proxy. Verify the proxy is trusted.`);
}
export function applyHttpProxySettings(httpProxy) {
    const proxy = validateProxyUrl(httpProxy);
    if (!proxy)
        return;
    process.env.HTTP_PROXY ??= proxy;
    process.env.HTTPS_PROXY ??= proxy;
    warnProxyActive(proxy);
}
export function configureHttpDispatcher(timeoutMs = DEFAULT_HTTP_IDLE_TIMEOUT_MS) {
    const normalizedTimeoutMs = parseHttpIdleTimeoutMs(timeoutMs);
    if (normalizedTimeoutMs === undefined) {
        throw new Error(`Invalid HTTP idle timeout: ${String(timeoutMs)}`);
    }
    undici.setGlobalDispatcher(new undici.EnvHttpProxyAgent({
        allowH2: false,
        bodyTimeout: normalizedTimeoutMs,
        headersTimeout: normalizedTimeoutMs,
    }));
    // Keep fetch and the dispatcher on the same undici implementation. Node 26.0's
    // bundled fetch can otherwise consume compressed responses through npm undici's
    // dispatcher without decompressing them, causing response.json() failures.
    // If a caller replaced fetch after module load, preserve that deliberate override.
    const shouldInstallGlobals = installedGlobalFetch === undefined
        ? globalThis.fetch === originalGlobalFetch
        : globalThis.fetch === installedGlobalFetch;
    if (shouldInstallGlobals) {
        undici.install?.();
        installedGlobalFetch = globalThis.fetch;
    }
}
//# sourceMappingURL=http-dispatcher.js.map
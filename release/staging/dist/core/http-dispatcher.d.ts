export declare const DEFAULT_HTTP_IDLE_TIMEOUT_MS = 300000;
export declare const HTTP_IDLE_TIMEOUT_CHOICES: readonly [{
    readonly label: "30 sec";
    readonly timeoutMs: 30000;
}, {
    readonly label: "1 min";
    readonly timeoutMs: 60000;
}, {
    readonly label: "2 min";
    readonly timeoutMs: 120000;
}, {
    readonly label: "5 min";
    readonly timeoutMs: 300000;
}, {
    readonly label: "disabled";
    readonly timeoutMs: 0;
}];
export declare function parseHttpIdleTimeoutMs(value: unknown): number | undefined;
export declare function formatHttpIdleTimeoutMs(timeoutMs: number): string;
/**
 * Validate that a proxy URL is well-formed and uses an http(s) scheme.
 * Returns the normalized proxy string on success, or undefined if invalid/empty.
 */
export declare function validateProxyUrl(httpProxy: string | undefined): string | undefined;
/**
 * Warn prominently when an HTTP proxy is active, since it receives all LLM
 * traffic including Authorization and provider-specific credential headers.
 */
export declare function warnProxyActive(proxy: string | undefined): void;
/**
 * Exclude known provider hosts from the HTTP proxy so credentials and
 * CF-Access headers are never forwarded to a proxy. Provider traffic
 * to these hosts uses direct connections.
 */
export declare function excludeProvidersFromProxy(providerHosts: string[]): void;
export declare function applyHttpProxySettings(httpProxy: string | undefined): void;
export declare function configureHttpDispatcher(timeoutMs?: number): void;
//# sourceMappingURL=http-dispatcher.d.ts.map
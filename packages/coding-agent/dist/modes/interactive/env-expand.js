/**
 * Expand environment-variable references in a custom-endpoint config value
 * before it is used for a discovery network call.
 *
 * Secret headers are stored as a single-dollar reference derived from the
 * header key (see `saveEndpointToModelsJson` in interactive-mode.ts), e.g.
 * header `CF-Access-Client-Id` is saved as the literal string
 * `$CF_ACCESS_CLIENT_ID`. A double-dollar `$$VAR` form is also accepted
 * defensively: one leading `$` is stripped and the remaining `$VAR` is
 * expanded normally.
 *
 * Both `${VAR}` and `$VAR` forms are expanded from `env`. References to
 * unset variables expand to "".
 */
export function expandEnvForDiscovery(value, env = process.env) {
    // Defensive: a `$$VAR` form strips one leading `$` so the remaining
    // `${VAR}` or `$VAR` is expanded normally.
    if (value.startsWith("$$")) {
        value = value.slice(1);
    }
    return value
        .replace(/\$\{(\w+)\}/g, (_, name) => env[name] ?? "")
        .replace(/\$(\w+)/g, (_, name) => env[name] ?? "");
}
//# sourceMappingURL=env-expand.js.map
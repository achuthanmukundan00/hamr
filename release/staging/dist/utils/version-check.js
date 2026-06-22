import { compare, valid } from "semver";
import { getHamrUserAgent } from "./hamr-user-agent.js";
const HAMR_NPM_PACKAGE_NAME = "@skaft/hamr";
const NPM_LATEST_VERSION_URL = "https://registry.npmjs.org/@skaft%2fhamr/latest";
const HAMR_LATEST_VERSION_URL = "https://hamr.dev/api/latest-version";
const DEFAULT_VERSION_CHECK_TIMEOUT_MS = 10000;
export function comparePackageVersions(leftVersion, rightVersion) {
    const left = valid(leftVersion.trim());
    const right = valid(rightVersion.trim());
    if (!left || !right) {
        return undefined;
    }
    return compare(left, right);
}
export function isNewerPackageVersion(candidateVersion, currentVersion) {
    const comparison = comparePackageVersions(candidateVersion, currentVersion);
    if (comparison !== undefined) {
        return comparison > 0;
    }
    return candidateVersion.trim() !== currentVersion.trim();
}
function parseLatestRelease(data) {
    if (typeof data.version !== "string" || !data.version.trim()) {
        return undefined;
    }
    const packageName = typeof data.packageName === "string" && data.packageName.trim()
        ? data.packageName.trim()
        : typeof data.name === "string" && data.name.trim()
            ? data.name.trim()
            : undefined;
    const note = typeof data.note === "string" && data.note.trim() ? data.note.trim() : undefined;
    return {
        version: data.version.trim(),
        packageName,
        ...(note ? { note } : {}),
    };
}
async function fetchLatestRelease(url, currentVersion, timeoutMs) {
    const response = await fetch(url, {
        headers: {
            "User-Agent": getHamrUserAgent(currentVersion),
            accept: "application/json",
        },
        signal: AbortSignal.timeout(timeoutMs),
    });
    if (!response.ok)
        return undefined;
    return parseLatestRelease((await response.json()));
}
export async function getLatestHamrRelease(currentVersion, options = {}) {
    if (process.env.HAMR_SKIP_VERSION_CHECK ||
        process.env.PI_SKIP_VERSION_CHECK ||
        process.env.HAMR_OFFLINE ||
        process.env.PI_OFFLINE) {
        return undefined;
    }
    const timeoutMs = options.timeoutMs ?? DEFAULT_VERSION_CHECK_TIMEOUT_MS;
    try {
        const npmRelease = await fetchLatestRelease(NPM_LATEST_VERSION_URL, currentVersion, timeoutMs);
        if (npmRelease?.version) {
            return {
                ...npmRelease,
                packageName: HAMR_NPM_PACKAGE_NAME,
            };
        }
    }
    catch {
        // Fall through to the Hamr endpoint for compatibility with pre-npm or mirrored release metadata.
    }
    return fetchLatestRelease(HAMR_LATEST_VERSION_URL, currentVersion, timeoutMs);
}
export async function getLatestPiVersion(currentVersion, options = {}) {
    return (await getLatestHamrRelease(currentVersion, options))?.version;
}
export async function getLatestHamrVersion(currentVersion, options = {}) {
    return (await getLatestHamrRelease(currentVersion, options))?.version;
}
export async function checkForNewHamrVersion(currentVersion) {
    try {
        const latestRelease = await getLatestHamrRelease(currentVersion);
        if (latestRelease && isNewerPackageVersion(latestRelease.version, currentVersion)) {
            return latestRelease;
        }
        return undefined;
    }
    catch {
        return undefined;
    }
}
export const getLatestPiRelease = getLatestHamrRelease;
export const checkForNewPiVersion = checkForNewHamrVersion;
//# sourceMappingURL=version-check.js.map
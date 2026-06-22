export interface LatestHamrRelease {
    version: string;
    packageName?: string;
    note?: string;
}
export declare function comparePackageVersions(leftVersion: string, rightVersion: string): number | undefined;
export declare function isNewerPackageVersion(candidateVersion: string, currentVersion: string): boolean;
export declare function getLatestHamrRelease(currentVersion: string, options?: {
    timeoutMs?: number;
}): Promise<LatestHamrRelease | undefined>;
export declare function getLatestPiVersion(currentVersion: string, options?: {
    timeoutMs?: number;
}): Promise<string | undefined>;
export declare function getLatestHamrVersion(currentVersion: string, options?: {
    timeoutMs?: number;
}): Promise<string | undefined>;
export declare function checkForNewHamrVersion(currentVersion: string): Promise<LatestHamrRelease | undefined>;
export declare const getLatestPiRelease: typeof getLatestHamrRelease;
export declare const checkForNewPiVersion: typeof checkForNewHamrVersion;
//# sourceMappingURL=version-check.d.ts.map
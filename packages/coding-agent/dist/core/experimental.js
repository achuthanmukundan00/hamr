export function areExperimentalFeaturesEnabled() {
    return process.env.HAMR_EXPERIMENTAL === "1" || process.env.PI_EXPERIMENTAL === "1";
}
//# sourceMappingURL=experimental.js.map
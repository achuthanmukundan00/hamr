export function areExperimentalFeaturesEnabled(): boolean {
	return process.env.HAMR_EXPERIMENTAL === "1" || process.env.PI_EXPERIMENTAL === "1";
}

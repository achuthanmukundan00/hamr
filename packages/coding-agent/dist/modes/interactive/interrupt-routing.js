export function routeInterruptKey(state) {
    if (state.inSpecialEscapeMode || state.autocompleteShowing)
        return "defer";
    if (state.isStreaming)
        return "interrupt-stream";
    if (state.isBashRunning)
        return "interrupt-bash";
    return "defer";
}
//# sourceMappingURL=interrupt-routing.js.map
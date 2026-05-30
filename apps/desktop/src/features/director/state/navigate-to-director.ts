export function navigateToDirector(): void {
  if (!globalThis.history || globalThis.location?.pathname === "/director") {
    return;
  }
  globalThis.history.pushState(null, "", "/director");
  globalThis.dispatchEvent(new PopStateEvent("popstate"));
}

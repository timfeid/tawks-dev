/** Keep the arena aligned with its panel when either the panel or window resizes. */
export function observePartyViewport(
  root: HTMLElement,
  fullscreen: () => boolean,
) {
  const gameViewport = root.closest<HTMLElement>("[data-game-viewport]");
  const parent = gameViewport ?? root.parentElement;
  const resize = () => {
    if (fullscreen()) {
      root.style.height = "100dvh";
      return;
    }
    if (gameViewport) {
      root.style.height = `${gameViewport.clientHeight}px`;
      root.style.minHeight = "0";
      root.style.maxHeight = "none";
      return;
    }
    const parentHeight = parent?.clientHeight ?? 0;
    const top = Math.max(0, root.getBoundingClientRect().top);
    const viewport = window.visualViewport?.height ?? window.innerHeight;
    root.style.height = `${Math.max(360, parentHeight > 200 ? parentHeight : viewport - top)}px`;
  };
  resize();
  const observer = new ResizeObserver(resize);
  if (parent) observer.observe(parent);
  window.addEventListener("resize", resize);
  window.visualViewport?.addEventListener("resize", resize);
  return {
    resize,
    destroy() {
      observer.disconnect();
      window.removeEventListener("resize", resize);
      window.visualViewport?.removeEventListener("resize", resize);
    },
  };
}

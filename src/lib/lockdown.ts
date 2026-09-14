// 禁止用户缩放页面与全屏，配合固定窗口尺寸，保证界面始终小巧固定。
export function installLockdown() {
  // Ctrl/⌘ + (= - + 0) 缩放；F11 全屏。
  window.addEventListener(
    "keydown",
    (e) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && ["=", "-", "+", "0"].includes(e.key)) e.preventDefault();
      if (e.key === "F11") e.preventDefault();
    },
    { capture: true },
  );

  // Ctrl/⌘ + 滚轮 缩放。
  window.addEventListener(
    "wheel",
    (e) => {
      if (e.ctrlKey || e.metaKey) e.preventDefault();
    },
    { passive: false, capture: true },
  );

  // 触控板双指捏合缩放。
  window.addEventListener(
    "gesturestart",
    (e) => e.preventDefault(),
    { capture: true } as AddEventListenerOptions,
  );

  // 屏蔽系统右键菜单（改用自定义界面）。
  window.addEventListener("contextmenu", (e) => e.preventDefault());
}

import { isTauri } from "../lib/api";

async function winAction(action: "minimize" | "close") {
  if (!isTauri()) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (action === "minimize") await w.minimize();
  else await w.close();
}

// 自定义标题栏（替代系统操作栏）。整条可拖动，右侧为最小化 / 关闭。
export default function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="flex h-8 shrink-0 items-center justify-between pl-3 pr-1 select-none"
    >
      <span data-tauri-drag-region className="text-[11px] font-medium text-foreground-muted">
        灵网登录器
      </span>
      <div className="flex items-center gap-0.5">
        <button
          type="button"
          onClick={() => winAction("minimize")}
          aria-label="最小化"
          className="flex h-6 w-6 items-center justify-center rounded-md text-foreground-muted transition-colors hover:bg-background-muted hover:text-foreground-intense"
        >
          <svg viewBox="0 0 12 12" className="h-3 w-3" aria-hidden="true">
            <path d="M2 6h8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
          </svg>
        </button>
        <button
          type="button"
          onClick={() => winAction("close")}
          aria-label="关闭"
          className="flex h-6 w-6 items-center justify-center rounded-md text-foreground-muted transition-colors hover:bg-rose-500 hover:text-white"
        >
          <svg viewBox="0 0 12 12" className="h-3 w-3" aria-hidden="true">
            <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
          </svg>
        </button>
      </div>
    </div>
  );
}

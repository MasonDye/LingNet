import { useCallback, useEffect, useRef, useState } from "react";
import { Switch } from "@appica/ui-react/switch";
import { Badge } from "@appica/ui-react/badge";
import { Button } from "@appica/ui-react/button";
import { Separator } from "@appica/ui-react/separator";
import PowerButton, { type PowerState } from "./components/PowerButton";
import logo from "./assets/logo.png";
import Settings from "./components/Settings";
import Onboarding from "./components/Onboarding";
import TitleBar from "./components/TitleBar";
import {
  api,
  onAutoLoginResult,
  onDialProgress,
  type ConfigDto,
  type StatusResult,
} from "./lib/api";

// --- 小图标 ---
function WifiIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-3.5 w-3.5" aria-hidden="true">
      <path d="M2 8.5a15 15 0 0 1 20 0M5 12a10 10 0 0 1 14 0M8.5 15.5a5 5 0 0 1 7 0" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <circle cx="12" cy="19" r="1.4" fill="currentColor" />
    </svg>
  );
}
function CableIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-3.5 w-3.5" aria-hidden="true">
      <rect x="7" y="3" width="10" height="7" rx="2" stroke="currentColor" strokeWidth="1.8" />
      <path d="M12 10v6m0 0a3 3 0 0 0 3 3h2m-5-3a3 3 0 0 1-3 3H7" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}
function GearIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5" aria-hidden="true">
      <circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.8" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  );
}

type Toast = { text: string; ok: boolean } | null;

export default function App() {
  const [config, setConfig] = useState<ConfigDto | null>(null);
  const [status, setStatus] = useState<StatusResult | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [dialMsg, setDialMsg] = useState<string>("");
  const [view, setView] = useState<"main" | "settings" | "onboarding">("main");
  const [toast, setToast] = useState<Toast>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const flash = useCallback((text: string, ok: boolean) => {
    setToast({ text, ok });
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 3200);
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await api.status());
    } catch (e) {
      console.error(e);
    }
  }, []);

  // 初始加载 + 轮询 + 自动登录事件。
  useEffect(() => {
    (async () => {
      try {
        const c = await api.getConfig();
        setConfig(c);
        // 首次使用（未填学号）→ 进入引导。
        if (!c.username || !c.hasPassword) setView("onboarding");
      } catch (e) {
        console.error(e);
      }
      await refreshStatus();
    })();

    const poll = window.setInterval(refreshStatus, 8000);
    const unsubs: Array<() => void> = [];
    onAutoLoginResult((r) => {
      setDialMsg("");
      flash(r.message, r.success);
      refreshStatus();
    }).then((un) => unsubs.push(un));
    onDialProgress((p) => setDialMsg(p.message)).then((un) => unsubs.push(un));

    return () => {
      window.clearInterval(poll);
      unsubs.forEach((u) => u());
    };
  }, [refreshStatus, flash]);

  const online = status?.online ?? false;
  const allowed = status?.detect.allowed ?? false;

  const powerState: PowerState = connecting
    ? "connecting"
    : online
    ? "online"
    : allowed
    ? "offline"
    : "blocked";

  async function handlePower() {
    if (!config?.username || !config?.hasPassword) {
      flash("请先在设置中填写学号和密码", false);
      setView("settings");
      return;
    }
    setConnecting(true);
    setDialMsg("");
    try {
      const r = online ? await api.disconnect() : await api.connect();
      flash(r.message, r.success);
    } catch (e) {
      flash(String(e), false);
    } finally {
      setConnecting(false);
      setDialMsg("");
      await refreshStatus();
    }
  }

  async function updateConfig(patch: Partial<ConfigDto>) {
    if (!config) return;
    const next = { ...config, ...patch };
    setConfig(next);
    try {
      await api.saveConfig(next);
    } catch (e) {
      flash(String(e), false);
    }
  }

  async function saveFromSettings(cfg: ConfigDto) {
    await api.saveConfig(cfg);
    setConfig(await api.getConfig());
    await refreshStatus();
    flash("设置已保存", true);
  }

  async function finishOnboarding(cfg: ConfigDto) {
    await api.saveConfig(cfg);
    setConfig(await api.getConfig());
    setView("main");
    flash("设置完成，正在登录…", true);
    // 首次设置完成后立即尝试连接（不依赖异步的 config 状态）。
    setConnecting(true);
    setDialMsg("");
    try {
      const r = await api.connect();
      flash(r.message, r.success);
    } catch (e) {
      flash(String(e), false);
    } finally {
      setConnecting(false);
      setDialMsg("");
      await refreshStatus();
    }
  }

  // 网络徽标。
  const kind = status?.detect.kind ?? "none";
  const ssid = status?.detect.ssid;

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <TitleBar />
      <div className="flex flex-1 flex-col overflow-hidden px-5 pb-5 pt-1">
        {view === "onboarding" && config ? (
          <Onboarding config={config} onDone={finishOnboarding} />
        ) : view === "settings" && config ? (
          <Settings
            config={config}
            onSave={saveFromSettings}
            onClose={() => setView("main")}
          />
        ) : (
          <>
          {/* 顶栏 */}
          <header className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <img src={logo} alt="灵网" className="h-8 w-8 rounded-full" />
              <div className="leading-tight">
                <div className="text-sm font-semibold text-foreground-intense">灵网登录器</div>
                <div className="text-[11px] text-foreground-muted">河南经贸校园网专版</div>
              </div>
            </div>
            <Button variant="ghost" size="sm" onClick={() => setView("settings")} aria-label="设置">
              <GearIcon />
            </Button>
          </header>

          {/* 网络状态徽标 */}
          <div className="mt-4 flex justify-center">
            {kind === "wifi" ? (
              <Badge variant={allowed ? "success" : "warning"} data-icon="start">
                <WifiIcon /> {ssid ?? "WiFi"}
              </Badge>
            ) : kind === "wired" ? (
              <Badge variant="success" data-icon="start">
                <CableIcon /> 校园有线网络
              </Badge>
            ) : (
              <Badge variant="warning">未检测到校园网</Badge>
            )}
          </div>

          {/* 主开关 */}
          <div className="flex flex-1 flex-col items-center justify-center gap-5">
            <PowerButton
              state={powerState}
              disabled={!allowed && !online}
              onClick={handlePower}
            />

            <div className="min-h-[2.5rem] px-2 text-center">
              <p className={["text-sm font-medium", online ? "text-emerald-600 dark:text-emerald-400" : "text-foreground-intense"].join(" ")}>
                {connecting ? "连接中…" : online ? "网络已连接" : allowed ? "点击圆形按钮登录" : "请连接到校园网"}
              </p>
              <p className="mt-0.5 text-xs text-foreground-muted">
                {connecting && dialMsg ? dialMsg : status?.message ?? "检测中…"}
              </p>
              {online && status?.userName && (
                <p className="mt-0.5 text-[11px] text-foreground-muted">
                  账号 {status.userName} · 接入点 {config?.acId}
                </p>
              )}
            </div>
          </div>

          <Separator className="my-1" />

          {/* 快捷开关 */}
          <div className="flex flex-col gap-1 pt-2">
            <label className="flex items-center justify-between py-1.5">
              <div className="flex flex-col">
                <span className="text-sm text-foreground-intense">自动登录</span>
                <span className="text-[11px] text-foreground-muted">启动软件后自动连入</span>
              </div>
              <Switch
                checked={config?.autoLogin ?? false}
                onCheckedChange={(v) => updateConfig({ autoLogin: v })}
              />
            </label>
            <label className="flex items-center justify-between py-1.5">
              <div className="flex flex-col">
                <span className="text-sm text-foreground-intense">自动连接校园 WiFi</span>
                <span className="text-[11px] text-foreground-muted">不在校园网时自动加入 WiFi 再登录</span>
              </div>
              <Switch
                checked={config?.autoConnectWifi ?? false}
                onCheckedChange={(v) => updateConfig({ autoConnectWifi: v })}
              />
            </label>
            <label className="flex items-center justify-between py-1.5">
              <div className="flex flex-col">
                <span className="text-sm text-foreground-intense">开机自启</span>
                <span className="text-[11px] text-foreground-muted">开机时自动运行本程序</span>
              </div>
              <Switch
                checked={config?.autostart ?? false}
                onCheckedChange={(v) => updateConfig({ autostart: v })}
              />
            </label>
          </div>
          </>
        )}
      </div>

      {/* 轻量提示 */}
      {toast && (
        <div
          className={[
            "pointer-events-none fixed inset-x-0 bottom-4 z-50 mx-auto w-fit max-w-[90%] rounded-full px-4 py-2 text-center text-xs font-medium text-white shadow-lg",
            toast.ok ? "bg-emerald-500/95" : "bg-rose-500/95",
          ].join(" ")}
        >
          {toast.text}
        </div>
      )}
    </div>
  );
}

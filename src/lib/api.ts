// Tauri 命令封装 + 类型定义。
// 在非 Tauri 环境 (纯浏览器 `vite dev`) 下自动降级为 mock，便于预览 UI。

export interface ConfigDto {
  username: string;
  password: string;
  autoLogin: boolean;
  autostart: boolean;
  autoConnectWifi: boolean;
  acId: string;
  ssidAllowlist: string[];
  keepRunning: boolean;
  hasPassword: boolean;
}

export interface DetectResult {
  allowed: boolean;
  kind: "wifi" | "wired" | "none";
  ssid: string | null;
  interface: string | null;
  sourceIp: string | null;
  reason: string;
}

export interface StatusResult {
  detect: DetectResult;
  online: boolean;
  userName: string | null;
  message: string;
}

export interface ConnectResult {
  success: boolean;
  online: boolean;
  kind: "wifi" | "wired" | "none";
  ssid: string | null;
  userName: string | null;
  acId: string;
  message: string;
  /** 代拨结果提示（仅联通代拨账号有值），如「代拨成功」。 */
  dial?: string | null;
}

export interface DialProgress {
  attempt?: number;
  code?: number;
  message: string;
}

// 是否运行在 Tauri 容器内。
export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// ---------------------------------------------------------------------------
// 浏览器预览用 mock
// ---------------------------------------------------------------------------

const mock = (() => {
  let cfg: ConfigDto = {
    username: "20230000001",
    password: "",
    autoLogin: true,
    autostart: false,
    autoConnectWifi: false,
    acId: "3",
    ssidAllowlist: ["HNJM-Student-X"],
    keepRunning: true,
    hasPassword: true,
  };
  let online = false;
  const detect: DetectResult = {
    allowed: true,
    kind: "wired",
    ssid: null,
    interface: "eno1",
    sourceIp: "10.115.192.137",
    reason: "已识别校园有线网络 (网卡 eno1，门户可达)",
  };
  const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

  return {
    async get_config() {
      return { ...cfg };
    },
    async save_config(args: { dto: ConfigDto }) {
      cfg = { ...args.dto, hasPassword: cfg.hasPassword || !!args.dto.password };
    },
    async set_autostart(args: { enabled: boolean }) {
      cfg.autostart = args.enabled;
    },
    async detect_network() {
      await delay(400);
      return detect;
    },
    async status(): Promise<StatusResult> {
      await delay(300);
      return {
        detect,
        online,
        userName: online ? cfg.username : null,
        message: online ? `已在线：${cfg.username}` : "未登录",
      };
    },
    async connect(): Promise<ConnectResult> {
      await delay(1200);
      online = true;
      return {
        success: true,
        online: true,
        kind: "wired",
        ssid: null,
        userName: cfg.username,
        acId: "3",
        message: "代拨成功",
        dial: "代拨成功",
      };
    },
    async disconnect(): Promise<ConnectResult> {
      await delay(600);
      online = false;
      return {
        success: true,
        online: false,
        kind: "wired",
        ssid: null,
        userName: null,
        acId: "3",
        message: "已断开连接",
      };
    },
  } as Record<string, (args?: any) => Promise<any>>;
})();

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) return tauriInvoke<T>(cmd, args);
  return mock[cmd](args) as Promise<T>;
}

// ---------------------------------------------------------------------------
// 公共 API
// ---------------------------------------------------------------------------

export const api = {
  getConfig: () => call<ConfigDto>("get_config"),
  saveConfig: (dto: ConfigDto) => call<void>("save_config", { dto }),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  detect: () => call<DetectResult>("detect_network"),
  status: () => call<StatusResult>("status"),
  connect: () => call<ConnectResult>("connect"),
  disconnect: () => call<ConnectResult>("disconnect"),
};

export async function onAutoLoginResult(cb: (r: ConnectResult) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<ConnectResult>("auto-login-result", (e) => cb(e.payload));
  return un;
}

export async function onDialProgress(cb: (p: DialProgress) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<DialProgress>("dial-progress", (e) => cb(e.payload));
  return un;
}

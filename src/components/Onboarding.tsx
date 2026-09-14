import { useState } from "react";
import { Button } from "@appica/ui-react/button";
import { Input } from "@appica/ui-react/input";
import { Field, FieldLabel, FieldDescription } from "@appica/ui-react/field";
import type { ConfigDto } from "../lib/api";
import logo from "../assets/logo.png";

interface Props {
  config: ConfigDto;
  onDone: (cfg: ConfigDto) => Promise<void>;
}

type NetKind = "wired" | "wifi";

function EyeIcon({ off }: { off?: boolean }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-4 w-4" aria-hidden="true">
      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" stroke="currentColor" strokeWidth="1.8" />
      <circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.8" />
      {off && <path d="M4 4l16 16" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />}
    </svg>
  );
}

// 首次使用引导：学号 + 密码；若走有线，引导输入纯数字楼号（作为首次 ac_id）。
export default function Onboarding({ config, onDone }: Props) {
  const [username, setUsername] = useState(config.username);
  const [password, setPassword] = useState("");
  const [showPwd, setShowPwd] = useState(false);
  const [kind, setKind] = useState<NetKind>("wired");
  const [building, setBuilding] = useState(config.acId === "3" ? "" : config.acId);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState("");

  const buildingValid = kind !== "wired" || /^\d+$/.test(building.trim());
  const canSubmit = username.trim() !== "" && password !== "" && buildingValid;

  async function submit() {
    setErr("");
    if (!username.trim() || !password) {
      setErr("请填写学号和密码");
      return;
    }
    if (kind === "wired" && !/^\d+$/.test(building.trim())) {
      setErr("楼号必须是纯数字");
      return;
    }
    setSaving(true);
    try {
      await onDone({
        ...config,
        username: username.trim(),
        password,
        // 有线：楼号即首次尝试的 ac_id；WiFi：保持默认，登录时自动探测。
        acId: kind === "wired" ? building.trim() : config.acId,
        autoConnectWifi: kind === "wifi" ? true : config.autoConnectWifi,
      });
    } finally {
      setSaving(false);
    }
  }

  const seg = (k: NetKind, label: string, sub: string) => (
    <button
      type="button"
      onClick={() => setKind(k)}
      className={[
        "flex-1 rounded-xl border px-3 py-2.5 text-left transition-colors",
        kind === k
          ? "border-sky-500 bg-sky-50 dark:bg-sky-950/40"
          : "border-border bg-background-muted/40 hover:border-border-strong",
      ].join(" ")}
    >
      <div className="text-sm font-medium text-foreground-intense">{label}</div>
      <div className="text-[11px] text-foreground-muted">{sub}</div>
    </button>
  );

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex flex-col items-center gap-2 pt-1 text-center">
        <img src={logo} alt="灵网" className="h-14 w-14 rounded-full shadow-md" />
        <div>
          <h2 className="text-base font-semibold text-foreground-intense">欢迎使用灵网登录器</h2>
          <p className="text-xs text-foreground-muted">首次使用，请先完成设置</p>
        </div>
      </div>

      <div className="flex flex-1 flex-col gap-3.5 overflow-y-auto">
        <Field>
          <FieldLabel>学号</FieldLabel>
          <Input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="请输入学号"
            inputMode="numeric"
            autoComplete="username"
          />
        </Field>

        <Field>
          <FieldLabel>密码</FieldLabel>
          <Input
            type={showPwd ? "text" : "password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="请输入密码"
            autoComplete="current-password"
            endSlot={
              <button
                type="button"
                onClick={() => setShowPwd((v) => !v)}
                className="text-foreground-muted hover:text-foreground-intense"
                aria-label={showPwd ? "隐藏密码" : "显示密码"}
              >
                <EyeIcon off={!showPwd} />
              </button>
            }
          />
        </Field>

        <div className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-foreground-intense">连接方式</span>
          <div className="flex gap-2">
            {seg("wired", "有线", "网线接入")}
            {seg("wifi", "WiFi", "校园无线")}
          </div>
        </div>

        {kind === "wired" && (
          <Field>
            <FieldLabel>楼号</FieldLabel>
            <Input
              value={building}
              onChange={(e) => setBuilding(e.target.value.replace(/\D/g, ""))}
              placeholder="例如 3"
              inputMode="numeric"
            />
            <FieldDescription>有线网络请输入所在楼号（纯数字），作为接入点首次尝试。</FieldDescription>
          </Field>
        )}

        {err && <p className="text-xs text-rose-500">{err}</p>}
      </div>

      <Button onClick={submit} disabled={!canSubmit || saving} className="w-full" size="lg">
        {saving ? "保存中…" : "开始使用"}
      </Button>
    </div>
  );
}

import { useState } from "react";
import { Button } from "@appica/ui-react/button";
import { Input } from "@appica/ui-react/input";
import { Field, FieldLabel, FieldDescription } from "@appica/ui-react/field";
import { Separator } from "@appica/ui-react/separator";
import type { ConfigDto } from "../lib/api";

interface Props {
  config: ConfigDto;
  onSave: (cfg: ConfigDto) => Promise<void>;
  onClose: () => void;
}

function EyeIcon({ off }: { off?: boolean }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="h-4 w-4" aria-hidden="true">
      <path
        d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z"
        stroke="currentColor"
        strokeWidth="1.8"
      />
      <circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.8" />
      {off && <path d="M4 4l16 16" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />}
    </svg>
  );
}

export default function Settings({ config, onSave, onClose }: Props) {
  const [username, setUsername] = useState(config.username);
  const [password, setPassword] = useState("");
  const [ssid, setSsid] = useState(config.ssidAllowlist.join(", "));
  const [acId, setAcId] = useState(config.acId);
  const [showPwd, setShowPwd] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [saving, setSaving] = useState(false);
  const [qrError, setQrError] = useState(false);

  const passwordPlaceholder = config.hasPassword && !password ? "••••••••（已保存，留空不修改）" : "请输入密码";

  async function handleSave() {
    setSaving(true);
    try {
      await onSave({
        ...config,
        username: username.trim(),
        password, // 空 = 不修改
        acId: acId.trim() || "3",
        ssidAllowlist: ssid
          .split(/[,，\s]+/)
          .map((s) => s.trim())
          .filter(Boolean),
      });
      onClose();
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold text-foreground-intense">账号设置</h2>
        <Button variant="ghost" size="sm" onClick={onClose} aria-label="返回">
          返回
        </Button>
      </div>

      <div className="flex flex-1 flex-col gap-4 overflow-y-auto">
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
            placeholder={passwordPlaceholder}
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
          <FieldDescription>密码仅保存在本机，用于自动登录。</FieldDescription>
        </Field>

        <Field>
          <FieldLabel>校园 WiFi 名称 (SSID)</FieldLabel>
          <Input
            value={ssid}
            onChange={(e) => setSsid(e.target.value)}
            placeholder="HNJM-Student-X"
          />
          <FieldDescription>多个用逗号分隔；只有连到这些 WiFi 或校园有线网才会登录。</FieldDescription>
        </Field>

        <Separator />

        <button
          type="button"
          onClick={() => setShowAdvanced((v) => !v)}
          className="flex items-center gap-1 text-xs text-foreground-muted hover:text-foreground-intense"
        >
          <span>{showAdvanced ? "▾" : "▸"}</span> 高级选项
        </button>

        {showAdvanced && (
          <Field>
            <FieldLabel>楼号 / 接入点 ac_id</FieldLabel>
            <Input
              value={acId}
              onChange={(e) => setAcId(e.target.value)}
              placeholder="3"
              inputMode="numeric"
            />
            <FieldDescription>有线网络填所在楼号（纯数字）；登录失败时会自动尝试其它值并记住。</FieldDescription>
          </Field>
        )}

        <Separator />

        {/* 关于我们 */}
        <section className="flex flex-col gap-2.5 pb-1">
          <h3 className="text-sm font-semibold text-foreground-intense">关于我们</h3>

          <div className="text-xs leading-relaxed text-foreground">
            <p>
              <span className="text-foreground-muted">作者：</span>
              MasonLiu（某位物联网与通讯学院的 26 新生）
            </p>
            <p className="mt-1.5 text-foreground-muted">
              这个软件设计之初是为了方便同学们更简单地使用校园网。校园网虽然便利性差点意思，但实际体验其实不错，
              这个软件完美解决了这个痛点——同学们只需用它即可自动无感联网、开机直接上网，无须手动登录，方便快捷。
            </p>
            <p className="mt-1.5">如果你想和作者交流，没有 🚪（逃，直接加我吧。</p>
            <p className="mt-1">
              <span className="text-foreground-muted">微信：</span>
              <span className="font-medium select-text">MasonDye</span>
            </p>
          </div>

          <div className="flex flex-col items-center gap-1.5 pt-1">
            <span className="text-xs text-foreground-muted">加入我们的讨论群</span>
            {qrError ? (
              <div className="flex h-40 w-40 flex-col items-center justify-center rounded-xl border border-dashed border-border text-center text-[11px] leading-relaxed text-foreground-muted">
                二维码加载失败
                <br />
                请联网后重试，或访问
                <br />
                lingnet.voxtal.com
              </div>
            ) : (
              <img
                src="https://lingnet.voxtal.com/"
                alt="讨论群二维码"
                className="h-40 w-40 rounded-xl border border-border bg-white object-contain"
                onError={() => setQrError(true)}
              />
            )}
          </div>
        </section>
      </div>

      <Button onClick={handleSave} disabled={saving} className="w-full" size="lg">
        {saving ? "保存中…" : "保存"}
      </Button>
    </div>
  );
}

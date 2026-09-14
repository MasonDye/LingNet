import { Spinner } from "@appica/ui-react/spinner";

export type PowerState = "online" | "offline" | "connecting" | "blocked";

interface Props {
  state: PowerState;
  disabled?: boolean;
  onClick: () => void;
}

const RING: Record<PowerState, string> = {
  online: "from-emerald-400 to-teal-500 shadow-[0_10px_40px_-8px_rgba(16,185,129,0.6)]",
  offline: "from-sky-400 to-blue-600 shadow-[0_10px_40px_-8px_rgba(37,99,235,0.5)]",
  connecting: "from-sky-400 to-blue-600 shadow-[0_10px_40px_-8px_rgba(37,99,235,0.5)]",
  blocked: "from-rose-300 to-rose-500 shadow-[0_10px_40px_-8px_rgba(244,63,94,0.5)]",
};

const LABEL: Record<PowerState, string> = {
  online: "已连接",
  offline: "点击连接",
  connecting: "连接中",
  blocked: "不可用",
};

// 电源图标。
function PowerIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" className={className} aria-hidden="true">
      <path
        d="M12 3v9"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
      <path
        d="M6.3 6.7a8 8 0 1 0 11.4 0"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default function PowerButton({ state, disabled, onClick }: Props) {
  const isConnecting = state === "connecting";
  const isOnline = state === "online";

  return (
    <div className="relative flex items-center justify-center">
      {/* 在线时的呼吸光环 */}
      {isOnline && (
        <>
          <span className="absolute h-40 w-40 rounded-full bg-emerald-400/40 [animation:an-pulse_2.4s_ease-out_infinite]" />
          <span className="absolute h-40 w-40 rounded-full bg-emerald-400/30 [animation:an-pulse_2.4s_ease-out_infinite_1.2s]" />
        </>
      )}

      <button
        type="button"
        onClick={onClick}
        disabled={disabled || isConnecting}
        aria-label={LABEL[state]}
        className={[
          "group relative h-40 w-40 rounded-full",
          "bg-gradient-to-br",
          RING[state],
          "flex flex-col items-center justify-center gap-1.5",
          "text-white transition-all duration-300",
          "outline-none focus-visible:ring-4 focus-visible:ring-sky-300/60",
          disabled ? "opacity-60" : "hover:scale-[1.03] active:scale-[0.97] cursor-pointer",
        ].join(" ")}
      >
        {/* 内圈玻璃质感 */}
        <span className="absolute inset-2 rounded-full bg-white/10 backdrop-blur-sm" />
        <span className="absolute inset-0 rounded-full ring-1 ring-inset ring-white/30" />

        <span className="relative flex flex-col items-center gap-1.5">
          {isConnecting ? (
            <Spinner className="text-3xl" />
          ) : (
            <PowerIcon className="h-11 w-11" />
          )}
          <span className="text-sm font-semibold tracking-wide">{LABEL[state]}</span>
        </span>
      </button>
    </div>
  );
}

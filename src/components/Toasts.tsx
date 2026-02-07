import { memo } from "react";
import type { Toast } from "../types";

const colors: Record<Toast["type"], string> = {
  info: "rgba(100,160,255,0.85)",
  success: "rgba(80,200,120,0.85)",
  warning: "rgba(240,180,60,0.85)",
  danger: "rgba(220,80,80,0.85)",
};

export const Toasts = memo(function Toasts({ toasts }: { toasts: Toast[] }) {
  return (
    <div
      style={{
        position: "absolute",
        top: 50,
        right: 16,
        display: "flex",
        flexDirection: "column",
        gap: 8,
        zIndex: 20,
        pointerEvents: "none",
      }}
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          style={{
            padding: "8px 14px",
            borderRadius: 8,
            background: "rgba(0,0,0,0.6)",
            backdropFilter: "blur(8px)",
            borderLeft: `3px solid ${colors[t.type]}`,
            color: "#dde",
            fontSize: 12,
            fontFamily: "system-ui",
            animation: "fadeIn 0.3s ease",
          }}
        >
          {t.message}
        </div>
      ))}
    </div>
  );
});

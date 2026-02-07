import { memo } from "react";
import type { FrameUpdate } from "../types";

const barStyle: React.CSSProperties = {
  position: "absolute",
  top: 0,
  left: 0,
  right: 0,
  display: "flex",
  alignItems: "center",
  gap: 20,
  padding: "8px 16px",
  background: "rgba(0,0,0,0.4)",
  backdropFilter: "blur(8px)",
  color: "#dde",
  fontFamily: "system-ui",
  fontSize: 13,
  zIndex: 10,
  userSelect: "none",
};

const labelStyle: React.CSSProperties = {
  color: "rgba(255,255,255,0.5)",
  fontSize: 11,
  marginRight: 4,
};

const iconBtn: React.CSSProperties = {
  background: "none",
  border: "1px solid rgba(255,255,255,0.15)",
  borderRadius: 4,
  color: "rgba(255,255,255,0.5)",
  cursor: "pointer",
  padding: "3px 8px",
  fontSize: 11,
  fontFamily: "system-ui",
};

interface Props {
  frame: FrameUpdate | null;
  onStatsToggle?: () => void;
  onSettingsToggle?: () => void;
  onDecorateToggle?: () => void;
  onGalleryToggle?: () => void;
  onAchievementsToggle?: () => void;
  onReplayToggle?: () => void;
}

export const TopBar = memo(function TopBar({ frame, onStatsToggle, onSettingsToggle, onDecorateToggle, onGalleryToggle, onAchievementsToggle, onReplayToggle }: Props) {
  const wq = frame?.water_quality ?? 1;
  const wqColor = wq > 0.6 ? "#4a4" : wq > 0.4 ? "#aa4" : "#a44";
  const wqPct = Math.round(wq * 100);

  return (
    <div style={barStyle}>
      <span style={{ fontWeight: 600, fontSize: 15, letterSpacing: 1 }}>DeepTank</span>
      <div>
        <span style={labelStyle}>Gen</span>
        {frame?.max_generation ?? 0}
      </div>
      <div>
        <span style={labelStyle}>Pop</span>
        {frame?.population ?? 0}
      </div>
      <div>
        <span style={labelStyle}>Species</span>
        {frame?.species_count ?? 0}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        <span style={labelStyle}>Water</span>
        <div
          style={{
            width: 60,
            height: 8,
            background: "rgba(255,255,255,0.15)",
            borderRadius: 4,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              width: `${wqPct}%`,
              height: "100%",
              background: wqColor,
              borderRadius: 4,
              transition: "width 0.3s, background 0.3s",
            }}
          />
        </div>
        <span style={{ fontSize: 11, color: wqColor }}>{wqPct}%</span>
      </div>
      <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 6 }}>
        {onGalleryToggle && (
          <button style={iconBtn} onClick={onGalleryToggle} title="Species Gallery">
            Gallery
          </button>
        )}
        {onAchievementsToggle && (
          <button style={iconBtn} onClick={onAchievementsToggle} title="Achievements">
            Achievements
          </button>
        )}
        {onReplayToggle && (
          <button style={iconBtn} onClick={onReplayToggle} title="Time-Lapse Replay">
            Replay
          </button>
        )}
        {onDecorateToggle && (
          <button style={iconBtn} onClick={onDecorateToggle} title="Decorations [D]">
            Decorate
          </button>
        )}
        <button style={iconBtn} onClick={onStatsToggle} title="Stats [S]">
          Stats
        </button>
        <button style={iconBtn} onClick={onSettingsToggle} title="Settings">
          Settings
        </button>
        <span style={{ fontSize: 11, color: "rgba(255,255,255,0.3)", marginLeft: 4 }}>
          tick {frame?.tick ?? 0}
        </span>
      </div>
    </div>
  );
});

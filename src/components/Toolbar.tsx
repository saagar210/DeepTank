import { memo } from "react";

const barStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 0,
  left: 0,
  right: 0,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  gap: 8,
  padding: "8px 16px",
  background: "rgba(0,0,0,0.4)",
  backdropFilter: "blur(8px)",
  zIndex: 10,
  userSelect: "none",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  border: "1px solid rgba(255,255,255,0.2)",
  borderRadius: 6,
  background: "rgba(255,255,255,0.08)",
  color: "#ccd",
  fontSize: 12,
  fontFamily: "system-ui",
  cursor: "pointer",
  transition: "background 0.15s",
};

const activeBtnStyle: React.CSSProperties = {
  ...btnStyle,
  background: "rgba(100,160,255,0.25)",
  borderColor: "rgba(100,160,255,0.5)",
};

interface Props {
  paused: boolean;
  speed: number;
  feedMode: boolean;
  muted: boolean;
  onPauseToggle: () => void;
  onSpeedChange: (mult: number) => void;
  onFeedToggle: () => void;
  onMuteToggle: () => void;
  onStepForward?: () => void;
  onScreenshot?: () => void;
  foodType?: string;
  onFoodTypeChange?: (type: string) => void;
}

export const Toolbar = memo(function Toolbar({
  paused,
  speed,
  feedMode,
  muted,
  onPauseToggle,
  onSpeedChange,
  onFeedToggle,
  onMuteToggle,
  onStepForward,
  onScreenshot,
  foodType,
  onFoodTypeChange,
}: Props) {
  return (
    <div style={barStyle}>
      <button style={feedMode ? activeBtnStyle : btnStyle} onClick={onFeedToggle}>
        Feed {feedMode ? "(active)" : "[F]"}
      </button>
      {onFoodTypeChange && (
        <select
          value={foodType ?? "pellet"}
          onChange={(e) => onFoodTypeChange(e.target.value)}
          style={{
            ...btnStyle,
            appearance: "none",
            paddingRight: 20,
            background: "rgba(255,255,255,0.08)",
          }}
        >
          <option value="flake">Flake</option>
          <option value="pellet">Pellet</option>
          <option value="live">Live</option>
        </select>
      )}
      <button style={btnStyle} onClick={onPauseToggle}>
        {paused ? "Play" : "Pause"} [Space]
      </button>
      {paused && onStepForward && (
        <button style={btnStyle} onClick={onStepForward}>
          Step [.]
        </button>
      )}
      <select
        value={speed}
        onChange={(e) => onSpeedChange(Number(e.target.value))}
        style={{
          ...btnStyle,
          appearance: "none",
          paddingRight: 20,
          background: "rgba(255,255,255,0.08)",
        }}
      >
        <option value={0.5}>0.5x</option>
        <option value={1}>1x</option>
        <option value={2}>2x</option>
        <option value={4}>4x</option>
      </select>
      <button style={btnStyle} onClick={onMuteToggle}>
        {muted ? "Unmute" : "Mute"} [M]
      </button>
      {onScreenshot && (
        <button style={btnStyle} onClick={onScreenshot}>
          Screenshot [P]
        </button>
      )}
    </div>
  );
});

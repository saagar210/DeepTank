import { memo } from "react";

const types = [
  { id: "rock", label: "Rock" },
  { id: "tall_plant", label: "Tall Plant" },
  { id: "short_plant", label: "Short Plant" },
  { id: "coral", label: "Coral" },
] as const;

const paletteStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 50,
  left: "50%",
  transform: "translateX(-50%)",
  display: "flex",
  gap: 6,
  padding: "8px 12px",
  background: "rgba(10,15,30,0.9)",
  backdropFilter: "blur(8px)",
  borderRadius: 8,
  border: "1px solid rgba(255,255,255,0.15)",
  zIndex: 25,
  fontFamily: "system-ui",
};

const itemStyle: React.CSSProperties = {
  padding: "6px 12px",
  border: "1px solid rgba(255,255,255,0.15)",
  borderRadius: 6,
  background: "rgba(255,255,255,0.06)",
  color: "#ccd",
  fontSize: 11,
  cursor: "pointer",
};

const activeStyle: React.CSSProperties = {
  ...itemStyle,
  background: "rgba(100,160,255,0.25)",
  borderColor: "rgba(100,160,255,0.5)",
};

interface Props {
  selectedType: string;
  onSelect: (type: string) => void;
  onClose: () => void;
}

export const DecorationPalette = memo(function DecorationPalette({ selectedType, onSelect, onClose }: Props) {
  return (
    <div style={paletteStyle}>
      {types.map((t) => (
        <button
          key={t.id}
          style={selectedType === t.id ? activeStyle : itemStyle}
          onClick={() => onSelect(t.id)}
        >
          {t.label}
        </button>
      ))}
      <button
        style={{ ...itemStyle, color: "rgba(255,255,255,0.4)" }}
        onClick={onClose}
      >
        Cancel
      </button>
    </div>
  );
});

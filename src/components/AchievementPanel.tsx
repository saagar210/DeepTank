import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Achievement {
  id: string;
  name: string;
  description: string;
  unlocked_at_tick: number | null;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

const overlayStyle: React.CSSProperties = {
  position: "absolute",
  inset: 0,
  background: "rgba(0,0,0,0.7)",
  backdropFilter: "blur(6px)",
  zIndex: 30,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: "60px 20px 20px",
  overflow: "auto",
  fontFamily: "system-ui",
  color: "#dde",
};

const cardStyle: React.CSSProperties = {
  width: 220,
  padding: 14,
  background: "rgba(255,255,255,0.06)",
  borderRadius: 8,
  border: "1px solid rgba(255,255,255,0.1)",
  textAlign: "center",
};

export function AchievementPanel({ open, onClose }: Props) {
  const [achievements, setAchievements] = useState<Achievement[]>([]);

  const fetchData = useCallback(async () => {
    const data = await invoke<Achievement[]>("get_achievements").catch(() => []);
    setAchievements(data);
  }, []);

  useEffect(() => {
    if (!open) return;
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [open, fetchData]);

  if (!open) return null;

  const unlocked = achievements.filter((a) => a.unlocked_at_tick !== null);
  const locked = achievements.filter((a) => a.unlocked_at_tick === null);

  return (
    <div style={overlayStyle}>
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <span style={{ fontSize: 18, fontWeight: 600, marginRight: 16 }}>
          Achievements ({unlocked.length}/{achievements.length})
        </span>
        <button
          onClick={onClose}
          style={{
            background: "none",
            border: "1px solid rgba(255,255,255,0.2)",
            color: "rgba(255,255,255,0.5)",
            padding: "4px 12px",
            borderRadius: 4,
            cursor: "pointer",
            fontSize: 11,
            fontFamily: "system-ui",
          }}
        >
          Close
        </button>
      </div>

      {achievements.length === 0 && (
        <div style={{ color: "rgba(255,255,255,0.3)", marginTop: 40 }}>
          Loading achievements...
        </div>
      )}

      <div style={{ display: "flex", flexWrap: "wrap", gap: 12, justifyContent: "center" }}>
        {unlocked.map((a) => (
          <div
            key={a.id}
            style={{
              ...cardStyle,
              borderColor: "rgba(255,200,50,0.3)",
              background: "rgba(255,200,50,0.08)",
            }}
          >
            <div style={{ fontSize: 24, marginBottom: 4 }}>&#9733;</div>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 4, color: "#fd8" }}>
              {a.name}
            </div>
            <div style={{ fontSize: 10, color: "rgba(255,255,255,0.6)", lineHeight: 1.4 }}>
              {a.description}
            </div>
            <div style={{ fontSize: 9, color: "rgba(255,255,255,0.3)", marginTop: 6 }}>
              Unlocked at tick {a.unlocked_at_tick}
            </div>
          </div>
        ))}
        {locked.map((a) => (
          <div key={a.id} style={{ ...cardStyle, opacity: 0.4 }}>
            <div style={{ fontSize: 24, marginBottom: 4 }}>&#9734;</div>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 4 }}>
              {a.name}
            </div>
            <div style={{ fontSize: 10, color: "rgba(255,255,255,0.5)", lineHeight: 1.4 }}>
              {a.description}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

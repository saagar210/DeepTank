import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Snapshot {
  tick: number;
  population: number;
  species_count: number;
  water_quality: number;
  avg_hue: number;
  avg_speed: number;
  avg_size: number;
  avg_aggression: number;
}

interface Props {
  onClose: () => void;
  onPauseSimulation: () => void;
}

const overlayStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 50,
  left: "50%",
  transform: "translateX(-50%)",
  background: "rgba(10,15,30,0.92)",
  backdropFilter: "blur(12px)",
  border: "1px solid rgba(255,255,255,0.12)",
  borderRadius: 10,
  color: "#ccd",
  fontFamily: "system-ui",
  padding: "14px 20px",
  zIndex: 30,
  display: "flex",
  flexDirection: "column",
  gap: 10,
  minWidth: 520,
};

export function ReplayControls({ onClose, onPauseSimulation }: Props) {
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [index, setIndex] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const playRef = useRef(false);
  const speedRef = useRef(1);

  useEffect(() => {
    onPauseSimulation();
    invoke<Snapshot[]>("get_all_snapshots").then((data) => {
      if (data.length > 0) setSnapshots(data);
    });
  }, [onPauseSimulation]);

  useEffect(() => { playRef.current = playing; }, [playing]);
  useEffect(() => { speedRef.current = speed; }, [speed]);

  // Auto-play timer
  useEffect(() => {
    if (!playing || snapshots.length === 0) return;
    const interval = setInterval(() => {
      if (!playRef.current) return;
      setIndex((prev) => {
        const next = prev + speedRef.current;
        if (next >= snapshots.length - 1) {
          setPlaying(false);
          return snapshots.length - 1;
        }
        return next;
      });
    }, 200);
    return () => clearInterval(interval);
  }, [playing, snapshots.length]);

  const clampedIndex = Math.min(Math.max(0, Math.floor(index)), snapshots.length - 1);
  const snap = snapshots[clampedIndex];

  const handleClose = useCallback(() => {
    setPlaying(false);
    onClose();
  }, [onClose]);

  if (snapshots.length === 0) {
    return (
      <div style={overlayStyle}>
        <div style={{ textAlign: "center", fontSize: 13 }}>No snapshot data available yet. Run the simulation longer.</div>
        <button onClick={handleClose} style={btnStyle}>Close</button>
      </div>
    );
  }

  return (
    <div style={overlayStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontSize: 13, fontWeight: 600 }}>Time-Lapse Replay</span>
        <button onClick={handleClose} style={{ background: "none", border: "none", color: "rgba(255,255,255,0.4)", cursor: "pointer", fontSize: 16 }}>x</button>
      </div>

      {/* Timeline slider */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ fontSize: 10, color: "rgba(255,255,255,0.4)", width: 50 }}>
          Tick {snap?.tick ?? 0}
        </span>
        <input
          type="range"
          min={0}
          max={snapshots.length - 1}
          value={clampedIndex}
          onChange={(e) => { setIndex(Number(e.target.value)); setPlaying(false); }}
          style={{ flex: 1, accentColor: "#6af" }}
        />
        <span style={{ fontSize: 10, color: "rgba(255,255,255,0.4)", width: 50, textAlign: "right" }}>
          {snapshots.length} pts
        </span>
      </div>

      {/* Controls */}
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <button onClick={() => { setIndex(0); setPlaying(false); }} style={btnStyle}>|&lt;</button>
        <button onClick={() => setPlaying(!playing)} style={{ ...btnStyle, minWidth: 60 }}>
          {playing ? "Pause" : "Play"}
        </button>
        <button onClick={() => { setIndex(snapshots.length - 1); setPlaying(false); }} style={btnStyle}>&gt;|</button>
        <span style={{ fontSize: 10, color: "rgba(255,255,255,0.4)", marginLeft: 8 }}>Speed:</span>
        {[1, 2, 5, 10].map((s) => (
          <button
            key={s}
            onClick={() => setSpeed(s)}
            style={{
              ...btnStyle,
              background: speed === s ? "rgba(100,160,255,0.25)" : "rgba(255,255,255,0.06)",
              color: speed === s ? "#8bf" : "rgba(255,255,255,0.5)",
            }}
          >
            {s}x
          </button>
        ))}
      </div>

      {/* Stats display */}
      {snap && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 10, fontSize: 11 }}>
          <StatBox label="Population" value={snap.population} color="#6af" max={150} />
          <StatBox label="Species" value={snap.species_count} color="#fa6" max={10} />
          <StatBox label="Water" value={Math.round(snap.water_quality * 100)} suffix="%" color="#4d8" max={100} />
          <StatBox label="Aggression" value={Math.round(snap.avg_aggression * 100)} suffix="%" color="#f66" max={100} />
        </div>
      )}
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "4px 10px",
  border: "1px solid rgba(255,255,255,0.15)",
  borderRadius: 4,
  background: "rgba(255,255,255,0.06)",
  color: "rgba(255,255,255,0.6)",
  fontSize: 11,
  cursor: "pointer",
  fontFamily: "system-ui",
};

function StatBox({ label, value, color, max, suffix }: { label: string; value: number; color: string; max: number; suffix?: string }) {
  const pct = Math.min(100, Math.round((value / max) * 100));
  return (
    <div>
      <div style={{ color: "rgba(255,255,255,0.4)", fontSize: 9, textTransform: "uppercase", letterSpacing: 1 }}>{label}</div>
      <div style={{ fontSize: 16, fontWeight: 600, color }}>{value}{suffix ?? ""}</div>
      <div style={{ height: 3, background: "rgba(255,255,255,0.1)", borderRadius: 2, marginTop: 3 }}>
        <div style={{ width: `${pct}%`, height: "100%", background: color, borderRadius: 2, transition: "width 0.2s" }} />
      </div>
    </div>
  );
}

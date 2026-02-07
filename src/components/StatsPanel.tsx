import { useEffect, useState, useRef, useCallback } from "react";
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

interface JournalEntry {
  tick: number;
  text: string;
  timestamp: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

type Tab = "population" | "traits" | "journal";

const panelStyle: React.CSSProperties = {
  position: "absolute",
  top: 40,
  right: 0,
  bottom: 40,
  width: 380,
  background: "rgba(10,15,30,0.92)",
  backdropFilter: "blur(12px)",
  borderLeft: "1px solid rgba(255,255,255,0.1)",
  color: "#ccd",
  fontFamily: "system-ui",
  fontSize: 12,
  display: "flex",
  flexDirection: "column",
  zIndex: 20,
  overflow: "hidden",
};

const tabStyle: React.CSSProperties = {
  padding: "8px 14px",
  cursor: "pointer",
  border: "none",
  background: "transparent",
  color: "rgba(255,255,255,0.5)",
  fontSize: 12,
  fontFamily: "system-ui",
  borderBottom: "2px solid transparent",
};

const activeTabStyle: React.CSSProperties = {
  ...tabStyle,
  color: "#8bf",
  borderBottomColor: "#8bf",
};

function MiniChart({
  data,
  color,
  label,
  max,
}: {
  data: number[];
  color: string;
  label: string;
  max?: number;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || data.length < 2) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    const maxVal = max ?? data.reduce((a, b) => (a > b ? a : b), 1);
    const step = w / (data.length - 1);

    // Fill
    ctx.beginPath();
    ctx.moveTo(0, h);
    data.forEach((v, i) => {
      ctx.lineTo(i * step, h - (v / maxVal) * (h - 4));
    });
    ctx.lineTo(w, h);
    ctx.closePath();
    ctx.fillStyle = color + "20";
    ctx.fill();

    // Line
    ctx.beginPath();
    data.forEach((v, i) => {
      const x = i * step;
      const y = h - (v / maxVal) * (h - 4);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }, [data, color, max]);

  const latest = data.length > 0 ? data[data.length - 1] : 0;

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 2 }}>
        <span style={{ color: "rgba(255,255,255,0.5)", fontSize: 11 }}>{label}</span>
        <span style={{ color, fontSize: 11, fontWeight: 600 }}>
          {typeof latest === "number" && latest % 1 !== 0 ? latest.toFixed(2) : latest}
        </span>
      </div>
      <canvas
        ref={canvasRef}
        width={340}
        height={50}
        style={{ width: "100%", height: 50, borderRadius: 4, background: "rgba(255,255,255,0.03)" }}
      />
    </div>
  );
}

export function StatsPanel({ open, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("population");
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [journal, setJournal] = useState<JournalEntry[]>([]);

  const fetchData = useCallback(async () => {
    const snaps = await invoke<Snapshot[]>("get_snapshots").catch(() => []);
    setSnapshots(snaps as Snapshot[]);
    if (tab === "journal") {
      const entries = await invoke<JournalEntry[]>("get_journal_entries").catch(() => []);
      setJournal(entries as JournalEntry[]);
    }
  }, [tab]);

  useEffect(() => {
    if (!open) return;
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [open, fetchData]);

  if (!open) return null;

  const popData = snapshots.map((s) => s.population);
  const speciesData = snapshots.map((s) => s.species_count);
  const wqData = snapshots.map((s) => s.water_quality * 100);
  const hueData = snapshots.map((s) => s.avg_hue);
  const speedData = snapshots.map((s) => s.avg_speed);
  const sizeData = snapshots.map((s) => s.avg_size);
  const aggrData = snapshots.map((s) => s.avg_aggression);

  return (
    <div style={panelStyle}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          borderBottom: "1px solid rgba(255,255,255,0.1)",
        }}
      >
        <button
          style={tab === "population" ? activeTabStyle : tabStyle}
          onClick={() => setTab("population")}
        >
          Population
        </button>
        <button
          style={tab === "traits" ? activeTabStyle : tabStyle}
          onClick={() => setTab("traits")}
        >
          Traits
        </button>
        <button
          style={tab === "journal" ? activeTabStyle : tabStyle}
          onClick={() => setTab("journal")}
        >
          Journal
        </button>
        <button
          onClick={onClose}
          style={{
            marginLeft: "auto",
            background: "none",
            border: "none",
            color: "rgba(255,255,255,0.4)",
            cursor: "pointer",
            fontSize: 16,
            padding: "8px 12px",
          }}
        >
          x
        </button>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {tab === "population" && (
          <>
            <MiniChart data={popData} color="#6af" label="Population" />
            <MiniChart data={speciesData} color="#fa6" label="Species" />
            <MiniChart data={wqData} color="#6a6" label="Water Quality %" max={100} />
          </>
        )}

        {tab === "traits" && (
          <>
            <MiniChart data={hueData} color="#f6a" label="Avg Hue" max={360} />
            <MiniChart data={speedData} color="#6ff" label="Avg Speed" />
            <MiniChart data={sizeData} color="#af6" label="Avg Size" />
            <MiniChart data={aggrData} color="#f66" label="Avg Aggression" max={1} />
          </>
        )}

        {tab === "journal" && (
          <div>
            {journal.length === 0 && (
              <div style={{ color: "rgba(255,255,255,0.3)", textAlign: "center", padding: 20 }}>
                No journal entries yet. Enable Ollama in settings to generate field notes.
              </div>
            )}
            {journal.map((entry) => (
              <div
                key={entry.tick}
                style={{
                  marginBottom: 12,
                  padding: 10,
                  background: "rgba(255,255,255,0.04)",
                  borderRadius: 6,
                  borderLeft: "3px solid rgba(100,160,255,0.3)",
                }}
              >
                <div
                  style={{
                    fontSize: 10,
                    color: "rgba(255,255,255,0.3)",
                    marginBottom: 4,
                  }}
                >
                  Day {Math.floor(entry.tick / 1800)} - Tick {entry.tick}
                </div>
                <div style={{ lineHeight: 1.5, color: "rgba(255,255,255,0.8)" }}>
                  {entry.text}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

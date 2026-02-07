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

interface EventEntry {
  tick: number;
  event_type: string;
  fish_id: number | null;
  species_id: number | null;
  description: string;
  timestamp: string;
}

interface SpeciesSnapshot {
  tick: number;
  species_id: number;
  species_name: string;
  population: number;
}

type Tab = "population" | "species" | "traits" | "journal" | "events";

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

function SpeciesStackChart({ data }: { data: SpeciesSnapshot[] }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || data.length === 0) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // Group by tick, then by species
    const ticks = [...new Set(data.map((d) => d.tick))].sort((a, b) => a - b);
    const speciesIds = [...new Set(data.map((d) => d.species_id))];

    // Build species -> hue map from names
    const speciesHues = new Map<number, number>();
    for (const d of data) {
      if (!speciesHues.has(d.species_id)) {
        // Hash species_id into a hue
        speciesHues.set(d.species_id, (d.species_id * 137) % 360);
      }
    }

    // Build stacked data: for each tick, sum populations per species
    const tickData = ticks.map((tick) => {
      const entries = data.filter((d) => d.tick === tick);
      const pops = new Map<number, number>();
      for (const e of entries) pops.set(e.species_id, e.population);
      return { tick, pops };
    });

    if (ticks.length < 2) return;

    // Find max total
    let maxTotal = 0;
    for (const td of tickData) {
      let total = 0;
      for (const p of td.pops.values()) total += p;
      if (total > maxTotal) maxTotal = total;
    }
    if (maxTotal === 0) return;

    const step = w / (ticks.length - 1);

    // Draw stacked areas bottom-up
    for (let si = speciesIds.length - 1; si >= 0; si--) {
      const spId = speciesIds[si];
      const hue = speciesHues.get(spId) ?? 180;

      // Compute cumulative base + top for this species
      ctx.beginPath();
      for (let ti = 0; ti < ticks.length; ti++) {
        const td = tickData[ti];
        // Sum of species below this one
        let base = 0;
        for (let j = 0; j < si; j++) {
          base += td.pops.get(speciesIds[j]) ?? 0;
        }
        const top = base + (td.pops.get(spId) ?? 0);
        const x = ti * step;
        const yTop = h - (top / maxTotal) * (h - 4);
        if (ti === 0) ctx.moveTo(x, yTop);
        else ctx.lineTo(x, yTop);
      }
      // Close back along base
      for (let ti = ticks.length - 1; ti >= 0; ti--) {
        const td = tickData[ti];
        let base = 0;
        for (let j = 0; j < si; j++) {
          base += td.pops.get(speciesIds[j]) ?? 0;
        }
        const x = ti * step;
        const yBase = h - (base / maxTotal) * (h - 4);
        ctx.lineTo(x, yBase);
      }
      ctx.closePath();
      ctx.fillStyle = `hsla(${hue}, 60%, 50%, 0.5)`;
      ctx.fill();
    }
  }, [data]);

  // Get legend entries
  const speciesNames = new Map<number, string>();
  for (const d of data) {
    speciesNames.set(d.species_id, d.species_name);
  }

  return (
    <div>
      <div style={{ marginBottom: 4, fontSize: 11, color: "rgba(255,255,255,0.5)" }}>
        Population by Species
      </div>
      <canvas
        ref={canvasRef}
        width={340}
        height={120}
        style={{ width: "100%", height: 120, borderRadius: 4, background: "rgba(255,255,255,0.03)" }}
      />
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 6 }}>
        {[...speciesNames.entries()].map(([id, name]) => {
          const hue = (id * 137) % 360;
          return (
            <div key={id} style={{ display: "flex", alignItems: "center", gap: 3, fontSize: 9 }}>
              <div style={{ width: 8, height: 8, borderRadius: 2, background: `hsl(${hue}, 60%, 50%)` }} />
              <span style={{ color: "rgba(255,255,255,0.5)" }}>{name}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function StatsPanel({ open, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("population");
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [journal, setJournal] = useState<JournalEntry[]>([]);
  const [events, setEvents] = useState<EventEntry[]>([]);
  const [speciesSnapshots, setSpeciesSnapshots] = useState<SpeciesSnapshot[]>([]);

  const fetchData = useCallback(async () => {
    const snaps = await invoke<Snapshot[]>("get_snapshots").catch(() => []);
    setSnapshots(snaps as Snapshot[]);
    if (tab === "species") {
      const ss = await invoke<SpeciesSnapshot[]>("get_species_snapshots").catch(() => []);
      setSpeciesSnapshots(ss as SpeciesSnapshot[]);
    }
    if (tab === "journal") {
      const entries = await invoke<JournalEntry[]>("get_journal_entries").catch(() => []);
      setJournal(entries as JournalEntry[]);
    }
    if (tab === "events") {
      const evts = await invoke<EventEntry[]>("get_events", { eventType: null, limit: 100 }).catch(() => []);
      setEvents(evts as EventEntry[]);
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
          style={tab === "species" ? activeTabStyle : tabStyle}
          onClick={() => setTab("species")}
        >
          Species
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
          style={tab === "events" ? activeTabStyle : tabStyle}
          onClick={() => setTab("events")}
        >
          Events
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

        {tab === "species" && (
          <SpeciesStackChart data={speciesSnapshots} />
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

        {tab === "events" && (
          <div>
            {events.length === 0 && (
              <div style={{ color: "rgba(255,255,255,0.3)", textAlign: "center", padding: 20 }}>
                No events recorded yet.
              </div>
            )}
            {events.map((ev, i) => {
              const colors: Record<string, string> = {
                birth: "#4a4", death: "#a44", new_species: "#da4",
                extinction: "#a44", predation: "#d84",
              };
              const color = colors[ev.event_type] ?? "#68a";
              return (
                <div
                  key={i}
                  style={{
                    marginBottom: 6,
                    padding: "6px 8px",
                    background: "rgba(255,255,255,0.04)",
                    borderRadius: 4,
                    borderLeft: `3px solid ${color}`,
                    fontSize: 11,
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between" }}>
                    <span style={{ color, fontWeight: 600, fontSize: 10, textTransform: "uppercase" }}>
                      {ev.event_type.replace("_", " ")}
                    </span>
                    <span style={{ fontSize: 9, color: "rgba(255,255,255,0.3)" }}>
                      tick {ev.tick}
                    </span>
                  </div>
                  <div style={{ color: "rgba(255,255,255,0.7)", marginTop: 2 }}>{ev.description}</div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

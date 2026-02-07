import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Settings {
  // Boids
  separation_weight: number;
  alignment_weight: number;
  cohesion_weight: number;
  wander_strength: number;
  // Ecosystem
  hunger_rate: number;
  mutation_rate_small: number;
  mutation_rate_large: number;
  species_threshold: number;
  // Environment
  day_night_cycle: boolean;
  bubble_rate: number;
  current_strength: number;
  // Auto-feeder
  auto_feed_enabled: boolean;
  auto_feed_interval: number;
  auto_feed_amount: number;
  // Ollama
  ollama_enabled: boolean;
  ollama_url: string;
  ollama_model: string;
  // Audio
  master_volume: number;
  ambient_enabled: boolean;
  event_sounds_enabled: boolean;
  // Visual
  theme: string;
  // Disease
  disease_enabled: boolean;
  disease_infection_chance: number;
  disease_spontaneous_chance: number;
  disease_duration: number;
  disease_damage: number;
  disease_spread_radius: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
  settings: Settings;
  onUpdate: (key: string, value: number | boolean | string) => void;
}

const panelStyle: React.CSSProperties = {
  position: "absolute",
  top: 40,
  left: 0,
  bottom: 40,
  width: 340,
  background: "rgba(10,15,30,0.92)",
  backdropFilter: "blur(12px)",
  borderRight: "1px solid rgba(255,255,255,0.1)",
  color: "#ccd",
  fontFamily: "system-ui",
  fontSize: 12,
  display: "flex",
  flexDirection: "column",
  zIndex: 20,
  overflow: "hidden",
};

const sectionStyle: React.CSSProperties = {
  marginBottom: 16,
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: 10,
  fontWeight: 700,
  textTransform: "uppercase",
  letterSpacing: 1.5,
  color: "rgba(255,255,255,0.35)",
  marginBottom: 8,
};

function Slider({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  return (
    <div style={{ marginBottom: 8, display: "flex", alignItems: "center", gap: 8 }}>
      <span style={{ width: 120, fontSize: 11, color: "rgba(255,255,255,0.6)" }}>{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{ flex: 1, accentColor: "#6af" }}
      />
      <span style={{ width: 40, textAlign: "right", fontSize: 10, color: "rgba(255,255,255,0.4)" }}>
        {value.toFixed(step < 0.01 ? 4 : step < 1 ? 2 : 0)}
      </span>
    </div>
  );
}

function Toggle({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div
      style={{ marginBottom: 8, display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}
      onClick={() => onChange(!value)}
    >
      <div
        style={{
          width: 32,
          height: 18,
          borderRadius: 9,
          background: value ? "rgba(100,170,255,0.5)" : "rgba(255,255,255,0.15)",
          position: "relative",
          transition: "background 0.2s",
        }}
      >
        <div
          style={{
            width: 14,
            height: 14,
            borderRadius: "50%",
            background: value ? "#6af" : "rgba(255,255,255,0.4)",
            position: "absolute",
            top: 2,
            left: value ? 16 : 2,
            transition: "left 0.2s, background 0.2s",
          }}
        />
      </div>
      <span style={{ fontSize: 11, color: "rgba(255,255,255,0.6)" }}>{label}</span>
    </div>
  );
}

function DebouncedInput({ value, onUpdate, label }: { value: string; onUpdate: (v: string) => void; label: string }) {
  const [local, setLocal] = useState(value);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => { setLocal(value); }, [value]);
  useEffect(() => { return () => { if (timerRef.current) clearTimeout(timerRef.current); }; }, []);
  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value;
    setLocal(v);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => onUpdate(v), 400);
  }, [onUpdate]);
  return (
    <div style={{ marginBottom: 8 }}>
      <span style={{ fontSize: 11, color: "rgba(255,255,255,0.6)", display: "block", marginBottom: 4 }}>{label}</span>
      <input
        type="text"
        value={local}
        onChange={handleChange}
        style={{
          width: "100%",
          padding: "6px 8px",
          border: "1px solid rgba(255,255,255,0.15)",
          borderRadius: 4,
          background: "rgba(255,255,255,0.06)",
          color: "#ccd",
          fontSize: 12,
          fontFamily: "monospace",
          boxSizing: "border-box",
        }}
      />
    </div>
  );
}

export function SettingsPanel({ open, onClose, settings, onUpdate }: Props) {
  const [section, setSection] = useState<"sim" | "env" | "ai" | "audio">("sim");

  if (!open) return null;

  const tabBtn = (id: typeof section, label: string) => (
    <button
      onClick={() => setSection(id)}
      style={{
        padding: "8px 12px",
        border: "none",
        background: "transparent",
        color: section === id ? "#8bf" : "rgba(255,255,255,0.5)",
        borderBottom: section === id ? "2px solid #8bf" : "2px solid transparent",
        cursor: "pointer",
        fontSize: 12,
        fontFamily: "system-ui",
      }}
    >
      {label}
    </button>
  );

  return (
    <div style={panelStyle}>
      <div style={{ display: "flex", borderBottom: "1px solid rgba(255,255,255,0.1)" }}>
        {tabBtn("sim", "Simulation")}
        {tabBtn("env", "Environment")}
        {tabBtn("ai", "AI")}
        {tabBtn("audio", "Audio")}
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
        {section === "sim" && (
          <>
            <div style={sectionStyle}>
              <div style={sectionTitleStyle}>Boids</div>
              <Slider label="Separation" value={settings.separation_weight} min={0} max={5} step={0.1} onChange={(v) => onUpdate("separation_weight", v)} />
              <Slider label="Alignment" value={settings.alignment_weight} min={0} max={5} step={0.1} onChange={(v) => onUpdate("alignment_weight", v)} />
              <Slider label="Cohesion" value={settings.cohesion_weight} min={0} max={5} step={0.1} onChange={(v) => onUpdate("cohesion_weight", v)} />
              <Slider label="Wander" value={settings.wander_strength} min={0} max={2} step={0.05} onChange={(v) => onUpdate("wander_strength", v)} />
            </div>
            <div style={sectionStyle}>
              <div style={sectionTitleStyle}>Evolution</div>
              <Slider label="Hunger rate" value={settings.hunger_rate} min={0.0001} max={0.005} step={0.0001} onChange={(v) => onUpdate("hunger_rate", v)} />
              <Slider label="Mutation (small)" value={settings.mutation_rate_small} min={0} max={0.5} step={0.01} onChange={(v) => onUpdate("mutation_rate_small", v)} />
              <Slider label="Mutation (large)" value={settings.mutation_rate_large} min={0} max={0.2} step={0.005} onChange={(v) => onUpdate("mutation_rate_large", v)} />
              <Slider label="Species threshold" value={settings.species_threshold} min={0.5} max={5} step={0.1} onChange={(v) => onUpdate("species_threshold", v)} />
            </div>
            <div style={sectionStyle}>
              <div style={sectionTitleStyle}>Auto-Feeder</div>
              <Toggle label="Enabled" value={settings.auto_feed_enabled} onChange={(v) => onUpdate("auto_feed_enabled", v)} />
              <Slider label="Interval (ticks)" value={settings.auto_feed_interval} min={100} max={2000} step={50} onChange={(v) => onUpdate("auto_feed_interval", v)} />
              <Slider label="Amount" value={settings.auto_feed_amount} min={1} max={10} step={1} onChange={(v) => onUpdate("auto_feed_amount", v)} />
            </div>
          </>
        )}

        {section === "env" && (
          <div style={sectionStyle}>
            <div style={sectionTitleStyle}>Environment</div>
            <Toggle label="Day/Night cycle" value={settings.day_night_cycle} onChange={(v) => onUpdate("day_night_cycle", v)} />
            <Slider label="Bubble rate" value={settings.bubble_rate} min={0} max={3} step={0.1} onChange={(v) => onUpdate("bubble_rate", v)} />
            <Slider label="Current strength" value={settings.current_strength} min={0} max={1} step={0.05} onChange={(v) => onUpdate("current_strength", v)} />
            <div style={{ marginTop: 12 }}>
              <div style={sectionTitleStyle}>Theme</div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {(["aquarium", "tropical", "deep_ocean", "freshwater"] as const).map((t) => (
                  <button
                    key={t}
                    onClick={() => onUpdate("theme", t)}
                    style={{
                      padding: "5px 10px",
                      border: "1px solid rgba(255,255,255,0.15)",
                      borderRadius: 4,
                      background: settings.theme === t ? "rgba(100,160,255,0.25)" : "rgba(255,255,255,0.06)",
                      color: settings.theme === t ? "#8bf" : "rgba(255,255,255,0.5)",
                      fontSize: 11,
                      cursor: "pointer",
                      fontFamily: "system-ui",
                      textTransform: "capitalize",
                    }}
                  >
                    {t.replace("_", " ")}
                  </button>
                ))}
              </div>
            </div>
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitleStyle}>Disease</div>
              <Toggle label="Enable disease" value={settings.disease_enabled} onChange={(v) => onUpdate("disease_enabled", v)} />
              {settings.disease_enabled && (
                <>
                  <Slider label="Infection chance" value={settings.disease_infection_chance} min={0} max={1} step={0.05} onChange={(v) => onUpdate("disease_infection_chance", v)} />
                  <Slider label="Spontaneous rate" value={settings.disease_spontaneous_chance} min={0} max={0.001} step={0.00001} onChange={(v) => onUpdate("disease_spontaneous_chance", v)} />
                  <Slider label="Duration (ticks)" value={settings.disease_duration} min={100} max={2000} step={50} onChange={(v) => onUpdate("disease_duration", v)} />
                  <Slider label="Damage/tick" value={settings.disease_damage} min={0} max={0.005} step={0.0001} onChange={(v) => onUpdate("disease_damage", v)} />
                  <Slider label="Spread radius" value={settings.disease_spread_radius} min={10} max={100} step={5} onChange={(v) => onUpdate("disease_spread_radius", v)} />
                </>
              )}
            </div>
            <div style={{ marginTop: 16 }}>
              <div style={sectionTitleStyle}>Tank Sharing</div>
              <div style={{ display: "flex", gap: 8 }}>
                <button
                  onClick={() => invoke("export_tank").catch((e: unknown) => console.error("Export failed:", e))}
                  style={{
                    flex: 1, padding: "8px 0", border: "1px solid rgba(100,160,255,0.3)",
                    borderRadius: 4, background: "rgba(100,160,255,0.1)", color: "#8bf",
                    fontSize: 11, cursor: "pointer", fontFamily: "system-ui",
                  }}
                >
                  Export Tank
                </button>
                <button
                  onClick={() => invoke("import_tank").catch((e: unknown) => console.error("Import failed:", e))}
                  style={{
                    flex: 1, padding: "8px 0", border: "1px solid rgba(255,180,100,0.3)",
                    borderRadius: 4, background: "rgba(255,180,100,0.1)", color: "#fb8",
                    fontSize: 11, cursor: "pointer", fontFamily: "system-ui",
                  }}
                >
                  Import Tank
                </button>
              </div>
            </div>
          </div>
        )}

        {section === "ai" && (
          <div style={sectionStyle}>
            <div style={sectionTitleStyle}>Ollama Integration</div>
            <Toggle label="Enabled" value={settings.ollama_enabled} onChange={(v) => onUpdate("ollama_enabled", v)} />
            <DebouncedInput label="URL" value={settings.ollama_url} onUpdate={(v) => onUpdate("ollama_url", v)} />
            <DebouncedInput label="Model" value={settings.ollama_model} onUpdate={(v) => onUpdate("ollama_model", v)} />
          </div>
        )}

        {section === "audio" && (
          <div style={sectionStyle}>
            <div style={sectionTitleStyle}>Audio</div>
            <Slider label="Master volume" value={settings.master_volume} min={0} max={1} step={0.05} onChange={(v) => onUpdate("master_volume", v)} />
            <Toggle label="Ambient sounds" value={settings.ambient_enabled} onChange={(v) => onUpdate("ambient_enabled", v)} />
            <Toggle label="Event sounds" value={settings.event_sounds_enabled} onChange={(v) => onUpdate("event_sounds_enabled", v)} />
          </div>
        )}
      </div>
    </div>
  );
}

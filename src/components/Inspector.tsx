import { memo } from "react";
import { BASE_LIFESPAN } from "../types";
import type { FishDetail } from "../types";

const panelStyle: React.CSSProperties = {
  position: "absolute",
  top: 40,
  right: 0,
  bottom: 40,
  width: 260,
  background: "rgba(0,0,0,0.55)",
  backdropFilter: "blur(12px)",
  color: "#dde",
  fontFamily: "system-ui",
  fontSize: 12,
  padding: "12px 14px",
  overflowY: "auto",
  zIndex: 10,
  borderLeft: "1px solid rgba(255,255,255,0.1)",
};

const sectionStyle: React.CSSProperties = {
  borderTop: "1px solid rgba(255,255,255,0.1)",
  paddingTop: 8,
  marginTop: 8,
};

const labelStyle: React.CSSProperties = {
  color: "rgba(255,255,255,0.5)",
  fontSize: 10,
  textTransform: "uppercase",
  letterSpacing: 1,
};

function Bar({ value, max = 1, color }: { value: number; max?: number; color: string }) {
  const pct = Math.round((value / max) * 100);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <div style={{ flex: 1, height: 6, background: "rgba(255,255,255,0.1)", borderRadius: 3 }}>
        <div
          style={{
            width: `${pct}%`,
            height: "100%",
            background: color,
            borderRadius: 3,
            transition: "width 0.2s",
          }}
        />
      </div>
      <span style={{ fontSize: 10, width: 28, textAlign: "right" }}>{pct}%</span>
    </div>
  );
}

function TraitBar({ label, value, min, max }: { label: string; value: number; min: number; max: number }) {
  const range = max - min;
  const pct = range > 0 ? Math.round(((value - min) / range) * 100) : 50;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 3 }}>
      <span style={{ width: 65, fontSize: 10, color: "rgba(255,255,255,0.5)" }}>{label}</span>
      <div style={{ flex: 1, height: 4, background: "rgba(255,255,255,0.1)", borderRadius: 2 }}>
        <div style={{ width: `${pct}%`, height: "100%", background: "rgba(100,160,255,0.6)", borderRadius: 2 }} />
      </div>
      <span style={{ fontSize: 9, width: 28, textAlign: "right", color: "rgba(255,255,255,0.4)" }}>
        {value.toFixed(2)}
      </span>
    </div>
  );
}

function patternName(pattern: FishDetail["genome"]["pattern"]): string {
  if ("Solid" in pattern) return "Solid";
  if ("Striped" in pattern) return "Striped";
  if ("Spotted" in pattern) return "Spotted";
  if ("Gradient" in pattern) return "Gradient";
  if ("Bicolor" in pattern) return "Bicolor";
  return "Unknown";
}

function lifeStageName(ageFrac: number, maturityAge: number): string {
  if (ageFrac < maturityAge) return "Juvenile";
  if (ageFrac < 0.85) return "Adult";
  return "Elder";
}

export const Inspector = memo(function Inspector({ fish, onClose }: { fish: FishDetail; onClose: () => void }) {
  const g = fish.genome;
  const hueColor = `hsl(${g.base_hue}, ${Math.round(g.saturation * 100)}%, ${Math.round(g.lightness * 100)}%)`;

  return (
    <div style={panelStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>
          {fish.species_name ?? `Fish #${fish.id}`}
        </span>
        <button
          onClick={onClose}
          style={{
            background: "none",
            border: "none",
            color: "rgba(255,255,255,0.4)",
            cursor: "pointer",
            fontSize: 16,
          }}
        >
          x
        </button>
      </div>

      {/* Color swatch */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
        <div style={{ width: 20, height: 20, borderRadius: 4, background: hueColor, border: "1px solid rgba(255,255,255,0.2)" }} />
        <span style={{ fontSize: 11 }}>
          Gen {g.generation} | {g.sex} | ID #{fish.id}
        </span>
      </div>

      <div style={{ fontSize: 11, marginBottom: 4 }}>
        <span style={labelStyle}>Stage </span>
        {lifeStageName(fish.age / (BASE_LIFESPAN * g.lifespan_factor), g.maturity_age)}
        <span style={{ marginLeft: 8, ...labelStyle }}>State </span>
        {fish.behavior}
      </div>

      {/* Status bars */}
      <div style={sectionStyle}>
        <div style={labelStyle}>Status</div>
        <div style={{ marginTop: 4 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, marginBottom: 2 }}>
            <span>Hunger</span>
          </div>
          <Bar value={fish.hunger} color={fish.hunger > 0.7 ? "#c44" : "#4a8"} />
        </div>
        <div style={{ marginTop: 4 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, marginBottom: 2 }}>
            <span>Health</span>
          </div>
          <Bar value={fish.health} color={fish.health < 0.4 ? "#c44" : "#4a8"} />
        </div>
        <div style={{ marginTop: 4 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, marginBottom: 2 }}>
            <span>Energy</span>
          </div>
          <Bar value={fish.energy} color="#68a" />
        </div>
      </div>

      {/* Genome traits */}
      <div style={sectionStyle}>
        <div style={labelStyle}>Genome</div>
        <div style={{ marginTop: 6 }}>
          <TraitBar label="Speed" value={g.speed} min={0.5} max={2.0} />
          <TraitBar label="Aggression" value={g.aggression} min={0} max={1} />
          <TraitBar label="Schooling" value={g.school_affinity} min={0} max={1} />
          <TraitBar label="Curiosity" value={g.curiosity} min={0} max={1} />
          <TraitBar label="Boldness" value={g.boldness} min={0} max={1} />
          <TraitBar label="Metabolism" value={g.metabolism} min={0.5} max={2} />
          <TraitBar label="Fertility" value={g.fertility} min={0.3} max={1} />
          <TraitBar label="Lifespan" value={g.lifespan_factor} min={0.5} max={2} />
          <TraitBar label="Body size" value={g.body_length} min={0.6} max={2} />
        </div>
        <div style={{ fontSize: 10, color: "rgba(255,255,255,0.4)", marginTop: 4 }}>
          Pattern: {patternName(g.pattern)} | Hue: {Math.round(g.base_hue)}
        </div>
      </div>

      {/* Lineage */}
      <div style={sectionStyle}>
        <div style={labelStyle}>Lineage</div>
        <div style={{ fontSize: 11, marginTop: 4 }}>
          Parents: {g.parent_a ? `#${g.parent_a}, #${g.parent_b}` : "Initial population"}
        </div>
        <div style={{ fontSize: 11 }}>
          Meals: {fish.meals_eaten}
        </div>
      </div>
    </div>
  );
});

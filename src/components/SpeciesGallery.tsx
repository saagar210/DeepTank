import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SpeciesHistoryEntry } from "../types";

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
  width: 200,
  padding: 12,
  background: "rgba(255,255,255,0.06)",
  borderRadius: 8,
  border: "1px solid rgba(255,255,255,0.1)",
  textAlign: "center",
};

type Filter = "all" | "living" | "extinct";

export function SpeciesGallery({ open, onClose }: Props) {
  const [species, setSpecies] = useState<SpeciesHistoryEntry[]>([]);
  const [filter, setFilter] = useState<Filter>("all");

  const fetchData = useCallback(async () => {
    const data = await invoke<SpeciesHistoryEntry[]>("get_species_history").catch(() => []);
    setSpecies(data as SpeciesHistoryEntry[]);
  }, []);

  useEffect(() => {
    if (!open) return;
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [open, fetchData]);

  if (!open) return null;

  const filtered = species.filter((s) => {
    if (filter === "living") return s.extinct_at_tick === null;
    if (filter === "extinct") return s.extinct_at_tick !== null;
    return true;
  });

  const filterBtn = (f: Filter, label: string) => (
    <button
      onClick={() => setFilter(f)}
      style={{
        padding: "4px 12px",
        border: "1px solid rgba(255,255,255,0.15)",
        borderRadius: 4,
        background: filter === f ? "rgba(100,160,255,0.25)" : "rgba(255,255,255,0.06)",
        color: filter === f ? "#8bf" : "rgba(255,255,255,0.5)",
        fontSize: 11,
        cursor: "pointer",
        fontFamily: "system-ui",
      }}
    >
      {label}
    </button>
  );

  return (
    <div style={overlayStyle}>
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <span style={{ fontSize: 18, fontWeight: 600, marginRight: 16 }}>Species Gallery</span>
        {filterBtn("all", "All")}
        {filterBtn("living", "Living")}
        {filterBtn("extinct", "Extinct")}
        <button
          onClick={onClose}
          style={{
            marginLeft: 20,
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

      {filtered.length === 0 && (
        <div style={{ color: "rgba(255,255,255,0.3)", marginTop: 40 }}>
          No species discovered yet.
        </div>
      )}

      <div style={{ display: "flex", flexWrap: "wrap", gap: 12, justifyContent: "center" }}>
        {filtered.map((s) => {
          const hue = s.centroid_hue;
          const isExtinct = s.extinct_at_tick !== null;
          return (
            <div key={s.id} style={{ ...cardStyle, opacity: isExtinct ? 0.5 : 1 }}>
              <div
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: "50%",
                  background: `hsl(${hue}, 70%, 50%)`,
                  margin: "0 auto 8px",
                  border: "2px solid rgba(255,255,255,0.2)",
                }}
              />
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 4 }}>
                {s.name ?? `Species #${s.id}`}
              </div>
              {s.description && (
                <div style={{ fontSize: 10, color: "rgba(255,255,255,0.5)", marginBottom: 4, lineHeight: 1.4 }}>
                  {s.description}
                </div>
              )}
              <div style={{ fontSize: 10, color: "rgba(255,255,255,0.4)" }}>
                Day {Math.floor(s.discovered_at_tick / 1800)} | {s.member_count} members
              </div>
              {isExtinct && (
                <div style={{ fontSize: 10, color: "#a44", fontWeight: 600, marginTop: 4 }}>
                  EXTINCT
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

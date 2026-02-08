import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TraitRange {
  min: number;
  mid: number;
  max: number;
  parent_a: number;
  parent_b: number;
}

interface BreedPreview {
  speed: TraitRange;
  size: TraitRange;
  aggression: TraitRange;
  boldness: TraitRange;
  school_affinity: TraitRange;
  metabolism: TraitRange;
}

interface Props {
  fishAId: number | null;
  fishBId: number | null;
  genomeAId: number | null;
  genomeBId: number | null;
  onClose: () => void;
  onBred: () => void;
}

const panelStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 50,
  left: "50%",
  transform: "translateX(-50%)",
  width: 420,
  background: "rgba(10,15,30,0.95)",
  backdropFilter: "blur(12px)",
  border: "1px solid rgba(255,255,255,0.12)",
  borderRadius: 8,
  color: "#ccd",
  fontFamily: "system-ui",
  fontSize: 12,
  padding: 16,
  zIndex: 25,
};

function TraitBar({ label, trait: t }: { label: string; trait: TraitRange }) {
  const scale = (v: number) => Math.min(v / 2.5 * 100, 100);
  return (
    <div style={{ marginBottom: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, marginBottom: 2 }}>
        <span style={{ color: "rgba(255,255,255,0.5)" }}>{label}</span>
        <span style={{ color: "rgba(255,255,255,0.4)" }}>
          A:{t.parent_a.toFixed(2)} | Pred:{t.mid.toFixed(2)} | B:{t.parent_b.toFixed(2)}
        </span>
      </div>
      <div style={{ position: "relative", height: 12, background: "rgba(255,255,255,0.06)", borderRadius: 3 }}>
        {/* Min-max range band */}
        <div style={{
          position: "absolute", left: `${scale(t.min)}%`, width: `${scale(t.max) - scale(t.min)}%`,
          height: "100%", background: "rgba(100,160,255,0.15)", borderRadius: 3,
        }} />
        {/* Parent A marker */}
        <div style={{
          position: "absolute", left: `${scale(t.parent_a)}%`, top: 1, width: 2, height: 10,
          background: "#f88", borderRadius: 1,
        }} />
        {/* Predicted midpoint */}
        <div style={{
          position: "absolute", left: `${scale(t.mid)}%`, top: 0, width: 3, height: 12,
          background: "#8bf", borderRadius: 1,
        }} />
        {/* Parent B marker */}
        <div style={{
          position: "absolute", left: `${scale(t.parent_b)}%`, top: 1, width: 2, height: 10,
          background: "#8f8", borderRadius: 1,
        }} />
      </div>
    </div>
  );
}

export function BreedingPanel({ fishAId, fishBId, genomeAId, genomeBId, onClose, onBred }: Props) {
  const [preview, setPreview] = useState<BreedPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [breeding, setBreeding] = useState(false);

  useEffect(() => {
    if (genomeAId == null || genomeBId == null) { setPreview(null); return; }
    invoke<BreedPreview>("get_breed_preview", { genomeAId, genomeBId })
      .then((p) => { setPreview(p); setError(null); })
      .catch((e: unknown) => setError(String(e)));
  }, [genomeAId, genomeBId]);

  const handleBreed = useCallback(async () => {
    if (fishAId == null || fishBId == null) return;
    setBreeding(true);
    try {
      await invoke("breed_fish", { fishAId, fishBId });
      onBred();
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setBreeding(false);
    }
  }, [fishAId, fishBId, onBred]);

  if (fishAId == null) {
    return (
      <div style={panelStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 12 }}>
          <span style={{ fontWeight: 600, fontSize: 14 }}>Selective Breeding</span>
          <button onClick={onClose} style={{ background: "none", border: "none", color: "rgba(255,255,255,0.4)", cursor: "pointer", fontSize: 14 }}>x</button>
        </div>
        <div style={{ color: "rgba(255,255,255,0.4)", textAlign: "center", padding: 16 }}>
          Click on a fish in the tank to select it as Parent A, then click a second fish for Parent B.
        </div>
      </div>
    );
  }

  if (fishBId == null) {
    return (
      <div style={panelStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 12 }}>
          <span style={{ fontWeight: 600, fontSize: 14 }}>Selective Breeding</span>
          <button onClick={onClose} style={{ background: "none", border: "none", color: "rgba(255,255,255,0.4)", cursor: "pointer", fontSize: 14 }}>x</button>
        </div>
        <div style={{ color: "rgba(255,255,255,0.4)", textAlign: "center", padding: 16 }}>
          Fish A selected (#{fishAId}). Now click a second fish to preview offspring traits.
        </div>
      </div>
    );
  }

  return (
    <div style={panelStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Breed Preview</span>
        <button onClick={onClose} style={{ background: "none", border: "none", color: "rgba(255,255,255,0.4)", cursor: "pointer", fontSize: 14 }}>x</button>
      </div>
      <div style={{ fontSize: 11, color: "rgba(255,255,255,0.5)", marginBottom: 8 }}>
        <span style={{ color: "#f88" }}>Red</span> = Parent A (#{fishAId}) &nbsp;
        <span style={{ color: "#8bf" }}>Blue</span> = Predicted &nbsp;
        <span style={{ color: "#8f8" }}>Green</span> = Parent B (#{fishBId})
      </div>
      {error && <div style={{ color: "#f66", marginBottom: 8, fontSize: 11 }}>{error}</div>}
      {preview && (
        <>
          <TraitBar label="Speed" trait={preview.speed} />
          <TraitBar label="Size" trait={preview.size} />
          <TraitBar label="Aggression" trait={preview.aggression} />
          <TraitBar label="Boldness" trait={preview.boldness} />
          <TraitBar label="Schooling" trait={preview.school_affinity} />
          <TraitBar label="Metabolism" trait={preview.metabolism} />
        </>
      )}
      <button
        onClick={handleBreed}
        disabled={breeding || !preview}
        style={{
          width: "100%", marginTop: 8, padding: "8px 0",
          border: "1px solid rgba(100,200,100,0.3)", borderRadius: 4,
          background: "rgba(100,200,100,0.15)", color: "#8f8",
          fontSize: 12, cursor: breeding ? "default" : "pointer",
          fontFamily: "system-ui", opacity: breeding ? 0.5 : 1,
        }}
      >
        {breeding ? "Breeding..." : "Breed"}
      </button>
    </div>
  );
}

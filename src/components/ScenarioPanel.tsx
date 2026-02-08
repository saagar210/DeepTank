import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScenarioDef {
  id: string;
  name: string;
  description: string;
  goals: string[];
}

interface ScenarioProgress {
  scenario_id: string;
  scenario_name: string;
  goals: { description: string; complete: boolean }[];
  all_complete: boolean;
}

const panelStyle: React.CSSProperties = {
  position: "absolute",
  top: 68,
  left: "50%",
  transform: "translateX(-50%)",
  width: 440,
  maxHeight: "calc(100vh - 150px)",
  overflowY: "auto",
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

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  border: "1px solid rgba(100,160,255,0.3)",
  borderRadius: 4,
  background: "rgba(100,160,255,0.15)",
  color: "#8bf",
  fontSize: 11,
  cursor: "pointer",
  fontFamily: "system-ui",
};

interface Props {
  open: boolean;
  onClose: () => void;
}

export function ScenarioPanel({ open, onClose }: Props) {
  const [scenarios, setScenarios] = useState<ScenarioDef[]>([]);
  const [progress, setProgress] = useState<ScenarioProgress | null>(null);

  const refresh = useCallback(async () => {
    const [list, prog] = await Promise.all([
      invoke<ScenarioDef[]>("get_scenarios").catch(() => []),
      invoke<ScenarioProgress | null>("get_scenario_progress").catch(() => null),
    ]);
    setScenarios(list);
    setProgress(prog);
  }, []);

  useEffect(() => {
    if (open) refresh();
  }, [open, refresh]);

  // Poll progress every 5 seconds while active
  useEffect(() => {
    if (!open || !progress) return;
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, [open, progress, refresh]);

  if (!open) return null;

  return (
    <div style={panelStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>
          {progress ? `Scenario: ${progress.scenario_name}` : "Guided Scenarios"}
        </span>
        <button onClick={onClose} style={{ background: "none", border: "none", color: "rgba(255,255,255,0.4)", cursor: "pointer", fontSize: 14 }}>x</button>
      </div>

      {progress ? (
        <div>
          {progress.all_complete && (
            <div style={{ background: "rgba(100,200,100,0.15)", border: "1px solid rgba(100,200,100,0.3)", borderRadius: 4, padding: 8, marginBottom: 12, textAlign: "center", color: "#8f8" }}>
              Scenario Complete!
            </div>
          )}
          <div style={{ marginBottom: 12 }}>
            {progress.goals.map((g, i) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: 14 }}>{g.complete ? "\u2705" : "\u2B1C"}</span>
                <span style={{ color: g.complete ? "rgba(100,200,100,0.8)" : "rgba(255,255,255,0.5)" }}>{g.description}</span>
              </div>
            ))}
          </div>
          <button
            style={{ ...btnStyle, background: "rgba(200,100,100,0.15)", borderColor: "rgba(200,100,100,0.3)", color: "#f88" }}
            onClick={async () => {
              await invoke("abandon_scenario").catch(() => {});
              refresh();
            }}
          >
            Abandon Scenario
          </button>
        </div>
      ) : (
        <div>
          <p style={{ color: "rgba(255,255,255,0.4)", marginBottom: 12, fontSize: 11 }}>
            Choose a scenario to test your aquarium management skills. Each scenario creates a new tank with specific goals.
          </p>
          {scenarios.map((s) => (
            <div key={s.id} style={{ border: "1px solid rgba(255,255,255,0.08)", borderRadius: 6, padding: 10, marginBottom: 8 }}>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>{s.name}</div>
              <div style={{ color: "rgba(255,255,255,0.5)", marginBottom: 6, fontSize: 11 }}>{s.description}</div>
              <div style={{ marginBottom: 6 }}>
                {s.goals.map((g, i) => (
                  <div key={i} style={{ fontSize: 10, color: "rgba(255,255,255,0.4)" }}>
                    {"\u2022"} {g}
                  </div>
                ))}
              </div>
              <button
                style={btnStyle}
                onClick={async () => {
                  await invoke("start_scenario", { scenarioId: s.id }).catch(() => {});
                  refresh();
                }}
              >
                Start
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

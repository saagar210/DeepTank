import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Tank {
  name: string;
  active: boolean;
}

const barStyle: React.CSSProperties = {
  position: "absolute",
  top: 0,
  left: 0,
  right: 0,
  display: "flex",
  alignItems: "center",
  gap: 2,
  padding: "0 8px",
  height: 28,
  background: "rgba(0,0,0,0.6)",
  zIndex: 20,
  fontFamily: "system-ui",
  fontSize: 11,
  userSelect: "none",
};

const tabStyle: React.CSSProperties = {
  padding: "4px 12px",
  border: "none",
  borderBottom: "2px solid transparent",
  background: "none",
  color: "rgba(255,255,255,0.4)",
  cursor: "pointer",
  fontFamily: "system-ui",
  fontSize: 11,
};

const activeTabStyle: React.CSSProperties = {
  ...tabStyle,
  color: "rgba(255,255,255,0.85)",
  borderBottomColor: "rgba(100,160,255,0.6)",
};

export function TankSwitcher() {
  const [tanks, setTanks] = useState<Tank[]>([]);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");

  const refresh = useCallback(async () => {
    const list = await invoke<Tank[]>("list_tanks").catch(() => []);
    setTanks(list);
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const handleSwitch = useCallback(async (name: string) => {
    await invoke("switch_tank", { name }).catch(() => {});
    refresh();
  }, [refresh]);

  const handleCreate = useCallback(async () => {
    if (!newName.trim()) return;
    await invoke("create_tank", { name: newName.trim() }).catch(() => {});
    setNewName("");
    setCreating(false);
    refresh();
  }, [newName, refresh]);

  const handleDelete = useCallback(async (name: string) => {
    await invoke("delete_tank", { name }).catch(() => {});
    refresh();
  }, [refresh]);

  if (tanks.length <= 1 && !creating) {
    return (
      <div style={{ ...barStyle, justifyContent: "flex-end" }}>
        <button
          onClick={() => setCreating(true)}
          style={{ ...tabStyle, fontSize: 13, padding: "2px 8px" }}
          title="Create new tank"
        >
          +
        </button>
      </div>
    );
  }

  return (
    <div style={barStyle}>
      {tanks.map((t) => (
        <div key={t.name} style={{ display: "flex", alignItems: "center" }}>
          <button
            style={t.active ? activeTabStyle : tabStyle}
            onClick={() => !t.active && handleSwitch(t.name)}
          >
            {t.name}
          </button>
          {!t.active && t.name !== "My Aquarium" && (
            <button
              onClick={() => handleDelete(t.name)}
              style={{
                background: "none", border: "none", color: "rgba(255,255,255,0.2)",
                cursor: "pointer", fontSize: 9, padding: "0 2px",
              }}
              title={`Delete ${t.name}`}
            >
              x
            </button>
          )}
        </div>
      ))}
      {creating ? (
        <input
          autoFocus
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") setCreating(false); }}
          onBlur={() => { if (!newName.trim()) setCreating(false); }}
          maxLength={20}
          placeholder="Tank name..."
          style={{
            background: "rgba(255,255,255,0.1)", border: "1px solid rgba(255,255,255,0.2)",
            borderRadius: 3, color: "#dde", fontSize: 11, fontFamily: "system-ui",
            padding: "2px 6px", width: 120, outline: "none",
          }}
        />
      ) : (
        <button
          onClick={() => setCreating(true)}
          style={{ ...tabStyle, fontSize: 13, padding: "2px 8px" }}
          title="Create new tank"
        >
          +
        </button>
      )}
    </div>
  );
}

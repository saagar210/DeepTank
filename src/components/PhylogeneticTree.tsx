import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LineageNode {
  genome_id: number;
  generation: number;
  parent_a: number | null;
  parent_b: number | null;
  base_hue: number;
  speed: number;
  body_length: number;
  depth: number;
  is_alive: boolean;
}

interface Props {
  genomeId: number;
  onClose: () => void;
  onSelectFish?: (genomeId: number) => void;
}

const overlayStyle: React.CSSProperties = {
  position: "absolute",
  inset: 0,
  background: "rgba(0,0,0,0.75)",
  backdropFilter: "blur(6px)",
  zIndex: 30,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  padding: "60px 20px 20px",
  fontFamily: "system-ui",
  color: "#dde",
};

export function PhylogeneticTree({ genomeId, onClose, onSelectFish }: Props) {
  const [nodes, setNodes] = useState<LineageNode[]>([]);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const fetchLineage = useCallback(async () => {
    const data = await invoke<LineageNode[]>("get_lineage", { genomeId, depth: 5 }).catch(() => []);
    setNodes(data as LineageNode[]);
  }, [genomeId]);

  useEffect(() => {
    fetchLineage();
  }, [fetchLineage]);

  // Draw tree
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || nodes.length === 0) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // Organize nodes by depth
    const byDepth = new Map<number, LineageNode[]>();
    for (const n of nodes) {
      const arr = byDepth.get(n.depth) || [];
      arr.push(n);
      byDepth.set(n.depth, arr);
    }

    const maxDepth = Math.max(...nodes.map((n) => n.depth));
    const nodeRadius = 16;
    const levelHeight = h / (maxDepth + 2);

    // Position nodes: target fish at bottom, ancestors go upward
    const positions = new Map<number, { x: number; y: number }>();
    for (let d = 0; d <= maxDepth; d++) {
      const levelNodes = byDepth.get(d) || [];
      const count = levelNodes.length;
      for (let i = 0; i < count; i++) {
        const x = w / 2 + (i - (count - 1) / 2) * 80;
        const y = h - levelHeight * (0.5 + d);
        positions.set(levelNodes[i].genome_id, { x, y });
      }
    }

    // Draw connections
    ctx.lineWidth = 1.5;
    for (const n of nodes) {
      const pos = positions.get(n.genome_id);
      if (!pos) continue;
      for (const pid of [n.parent_a, n.parent_b]) {
        if (pid === null) continue;
        const ppos = positions.get(pid);
        if (!ppos) continue;
        ctx.beginPath();
        ctx.strokeStyle = "rgba(100,160,255,0.25)";
        ctx.moveTo(pos.x, pos.y - nodeRadius);
        ctx.lineTo(ppos.x, ppos.y + nodeRadius);
        ctx.stroke();
      }
    }

    // Draw nodes
    for (const n of nodes) {
      const pos = positions.get(n.genome_id);
      if (!pos) continue;

      const hue = n.base_hue;
      const isTarget = n.genome_id === genomeId;
      const alpha = n.is_alive ? 1 : 0.4;

      // Circle
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, nodeRadius, 0, Math.PI * 2);
      ctx.fillStyle = `hsla(${hue}, 70%, 50%, ${alpha})`;
      ctx.fill();

      if (isTarget) {
        ctx.lineWidth = 2;
        ctx.strokeStyle = "#fff";
        ctx.stroke();
      }

      // Label
      ctx.fillStyle = `rgba(255,255,255,${alpha * 0.8})`;
      ctx.font = "9px system-ui";
      ctx.textAlign = "center";
      ctx.fillText(`#${n.genome_id}`, pos.x, pos.y + nodeRadius + 12);
      ctx.fillText(`Gen ${n.generation}`, pos.x, pos.y + nodeRadius + 22);
    }
  }, [nodes, genomeId]);

  const handleCanvasClick = useCallback(
    (e: React.MouseEvent) => {
      if (!onSelectFish || nodes.length === 0) return;
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;

      // Recompute positions to find clicked node
      const h = canvas.height;
      const maxDepth = Math.max(...nodes.map((n) => n.depth));
      const levelHeight = h / (maxDepth + 2);

      const byDepth = new Map<number, LineageNode[]>();
      for (const n of nodes) {
        const arr = byDepth.get(n.depth) || [];
        arr.push(n);
        byDepth.set(n.depth, arr);
      }

      for (let d = 0; d <= maxDepth; d++) {
        const levelNodes = byDepth.get(d) || [];
        const count = levelNodes.length;
        for (let i = 0; i < count; i++) {
          const x = canvas.width / 2 + (i - (count - 1) / 2) * 80;
          const y = h - levelHeight * (0.5 + d);
          const dist = Math.sqrt((mx - x) ** 2 + (my - y) ** 2);
          if (dist < 16 && levelNodes[i].is_alive) {
            onSelectFish(levelNodes[i].genome_id);
            return;
          }
        }
      }
    },
    [nodes, onSelectFish],
  );

  return (
    <div style={overlayStyle}>
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <span style={{ fontSize: 18, fontWeight: 600, marginRight: 16 }}>
          Lineage - Genome #{genomeId}
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

      {nodes.length === 0 ? (
        <div style={{ color: "rgba(255,255,255,0.3)", marginTop: 40 }}>
          No lineage data available.
        </div>
      ) : (
        <canvas
          ref={canvasRef}
          width={600}
          height={400}
          onClick={handleCanvasClick}
          style={{
            width: 600,
            height: 400,
            borderRadius: 8,
            background: "rgba(255,255,255,0.03)",
            cursor: "pointer",
          }}
        />
      )}

      <div style={{ marginTop: 12, fontSize: 10, color: "rgba(255,255,255,0.3)" }}>
        Click a living node (bright) to select that fish. Current fish has white ring.
      </div>
    </div>
  );
}

import { useRef, useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { FrameUpdate, FishGenome, FishDetail, Toast } from "./types";
import { CanvasRenderer, type ThemeName } from "./renderer/canvasRenderer";
import { AudioEngine } from "./audio/audioEngine";
import { Inspector } from "./components/Inspector";
import { TopBar } from "./components/TopBar";
import { Toolbar } from "./components/Toolbar";
import { Toasts } from "./components/Toasts";
import { StatsPanel } from "./components/StatsPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { DecorationPalette } from "./components/DecorationPalette";
import { SpeciesGallery } from "./components/SpeciesGallery";
import { AchievementPanel } from "./components/AchievementPanel";
import { PhylogeneticTree } from "./components/PhylogeneticTree";
import { ReplayControls } from "./components/ReplayControls";
import { BreedingPanel } from "./components/BreedingPanel";
import { NarrationTicker } from "./components/NarrationTicker";
import { TankSwitcher } from "./components/TankSwitcher";
import { ScenarioPanel } from "./components/ScenarioPanel";

const defaultSettings = {
  separation_weight: 1.5,
  alignment_weight: 1.0,
  cohesion_weight: 1.0,
  wander_strength: 0.3,
  hunger_rate: 0.0005,
  mutation_rate_small: 0.1,
  mutation_rate_large: 0.02,
  species_threshold: 2.5,
  day_night_cycle: true,
  day_night_speed: 1.0,
  bubble_rate: 1.0,
  current_strength: 0.0,
  auto_feed_enabled: false,
  auto_feed_interval: 600,
  auto_feed_amount: 4,
  ollama_enabled: true,
  ollama_url: "http://localhost:11434",
  ollama_model: "llama3.2",
  master_volume: 0.3,
  ambient_enabled: true,
  event_sounds_enabled: true,
  theme: "aquarium",
  environmental_events_enabled: true,
  event_frequency: 1.0,
  territory_enabled: true,
  territory_claim_radius: 60,
  disease_enabled: false,
  disease_infection_chance: 0.3,
  disease_spontaneous_chance: 0.00005,
  disease_duration: 600,
  disease_damage: 0.0005,
  disease_spread_radius: 40.0,
};

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<CanvasRenderer | null>(null);
  const audioRef = useRef<AudioEngine | null>(null);
  const [paused, setPaused] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [selectedFish, setSelectedFish] = useState<FishDetail | null>(null);
  const [frame, setFrame] = useState<FrameUpdate | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [feedMode, setFeedMode] = useState(false);
  const [muted, setMuted] = useState(false);
  const [statsOpen, setStatsOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settings, setSettings] = useState(defaultSettings);
  const [decorationMode, setDecorationMode] = useState(false);
  const [decorationType, setDecorationType] = useState("rock");
  const [foodType, setFoodType] = useState("pellet");
  const [galleryOpen, setGalleryOpen] = useState(false);
  const [achievementsOpen, setAchievementsOpen] = useState(false);
  const [replayOpen, setReplayOpen] = useState(false);
  const [scenarioOpen, setScenarioOpen] = useState(false);
  const [widgetMode, setWidgetMode] = useState(false);
  const [lineageGenomeId, setLineageGenomeId] = useState<number | null>(null);
  const [breedingMode, setBreedingMode] = useState(false);
  const [breedFishA, setBreedFishA] = useState<{ id: number; genomeId: number } | null>(null);
  const [breedFishB, setBreedFishB] = useState<{ id: number; genomeId: number } | null>(null);
  const toastId = useRef(0);
  const lastUiUpdate = useRef(0);
  const pendingGenomes = useRef(new Set<number>());
  const lastActiveEvent = useRef<string | null>(null);
  const lowDiversityWarned = useRef(false);
  const [narrationText, setNarrationText] = useState<{ text: string; key: number } | null>(null);
  const narrationKey = useRef(0);

  const addToast = useCallback((message: string, type: Toast["type"] = "info") => {
    const id = ++toastId.current;
    setToasts((t) => {
      const next = [...t, { id, message, type, timestamp: Date.now() }];
      return next.length > 10 ? next.slice(-10) : next;
    });
    setTimeout(() => setToasts((t) => t.filter((toast) => toast.id !== id)), 5000);
  }, []);

  // Initialize renderer + audio
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const renderer = new CanvasRenderer(canvas);
    rendererRef.current = renderer;
    renderer.start();

    const audio = new AudioEngine();
    audioRef.current = audio;

    const handleResize = () => {
      renderer.resize();
      invoke("update_tank_size", { width: window.innerWidth, height: window.innerHeight }).catch(() => {});
    };
    window.addEventListener("resize", handleResize);
    handleResize();

    // Load config
    invoke<Record<string, unknown>>("get_config").then((cfg) => {
      setSettings((prev) => ({ ...prev, ...cfg }));
    }).catch(() => {});

    return () => {
      renderer.stop();
      audio.destroy();
      window.removeEventListener("resize", handleResize);
    };
  }, []);

  // Wheel zoom + alt-drag pan
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let isPanning = false;
    let lastPanX = 0;
    let lastPanY = 0;

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      rendererRef.current?.zoomAt(e.offsetX, e.offsetY, e.deltaY);
    };

    const handleMouseDown = (e: MouseEvent) => {
      if (e.altKey) {
        isPanning = true;
        lastPanX = e.clientX;
        lastPanY = e.clientY;
        e.preventDefault();
      }
    };

    const handleMouseMove = (e: MouseEvent) => {
      if (isPanning) {
        const dx = e.clientX - lastPanX;
        const dy = e.clientY - lastPanY;
        lastPanX = e.clientX;
        lastPanY = e.clientY;
        rendererRef.current?.pan(dx, dy);
      }
    };

    const handleMouseUp = () => {
      isPanning = false;
    };

    canvas.addEventListener("wheel", handleWheel, { passive: false });
    canvas.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      canvas.removeEventListener("wheel", handleWheel);
      canvas.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  // Init audio on first user interaction
  useEffect(() => {
    const initAudio = () => {
      audioRef.current?.init();
      window.removeEventListener("click", initAudio);
      window.removeEventListener("keydown", initAudio);
    };
    window.addEventListener("click", initAudio);
    window.addEventListener("keydown", initAudio);
    return () => {
      window.removeEventListener("click", initAudio);
      window.removeEventListener("keydown", initAudio);
    };
  }, []);

  // Sync muted state
  useEffect(() => {
    if (audioRef.current) audioRef.current.muted = muted;
  }, [muted]);

  // Sync selected fish & paused to renderer
  useEffect(() => {
    rendererRef.current?.setSelectedFish(selectedFish?.id ?? null);
  }, [selectedFish?.id]);
  useEffect(() => {
    rendererRef.current?.setPaused(paused);
  }, [paused]);

  // Load initial genomes
  useEffect(() => {
    invoke<FishGenome[]>("get_all_genomes").then((genomes) => {
      rendererRef.current?.cacheGenomes(genomes);
    });
  }, []);

  // Listen for frame updates
  useEffect(() => {
    const unlisten = listen<FrameUpdate>("frame-update", (event) => {
      const f = event.payload;
      rendererRef.current?.updateFrame(f);

      // Throttle React state updates to ~4Hz (renderer runs at 60fps independently)
      const now = performance.now();
      if (now - lastUiUpdate.current > 250) {
        lastUiUpdate.current = now;
        setFrame(f);
      }

      for (const ev of f.events) {
        if ("NewSpecies" in ev) {
          addToast("New species detected!", "success");
          audioRef.current?.playNewSpecies();
        } else if ("Extinction" in ev) {
          addToast("A species has gone extinct", "danger");
          audioRef.current?.playExtinction();
        } else if ("Birth" in ev) {
          audioRef.current?.playBirth();
        } else if ("Death" in ev) {
          audioRef.current?.playDeath();
          const d = ev.Death as { fish_id: number; cause: string; custom_name?: string; is_favorite?: boolean };
          if (d.custom_name || d.is_favorite) {
            const name = d.custom_name || `Fish #${d.fish_id}`;
            const causeMap: Record<string, string> = {
              OldAge: "old age", Starvation: "starvation", PoorWater: "poor water", Predation: "predation",
            };
            addToast(`${name} has died (${causeMap[d.cause] ?? d.cause})`, "danger");
          }
        }
      }

      // Environmental event banner
      const currentEvent = f.active_event ?? null;
      if (currentEvent !== lastActiveEvent.current) {
        if (currentEvent) {
          const eventNames: Record<string, string> = {
            algae_bloom: "Algae Bloom",
            cold_snap: "Cold Snap",
            heatwave: "Heatwave",
            current_surge: "Current Surge",
            plankton_bloom: "Plankton Bloom",
          };
          addToast(`Environmental event: ${eventNames[currentEvent] ?? currentEvent}`, "warning");
        } else if (lastActiveEvent.current) {
          addToast("Environmental event has ended", "info");
        }
        lastActiveEvent.current = currentEvent;
      }

      // Genetic diversity warning (check every ~300 ticks)
      if (f.tick % 300 === 0 && f.genetic_diversity < 0.3 && f.population > 5) {
        if (!lowDiversityWarned.current) {
          addToast("Genetic diversity is critically low!", "danger");
          lowDiversityWarned.current = true;
        }
      } else if (f.genetic_diversity >= 0.3) {
        lowDiversityWarned.current = false;
      }

      // Cache genomes only for fish we haven't seen yet
      const renderer = rendererRef.current;
      if (renderer) {
        const pending = pendingGenomes.current;
        for (const fish of f.fish) {
          const gid = fish.genome_id;
          if (!renderer.hasGenome(gid) && !pending.has(gid)) {
            pending.add(gid);
            invoke<FishGenome | null>("get_genome", { genomeId: gid }).then((g) => {
              if (g) renderer.cacheGenome(g);
            }).catch(() => {}).finally(() => pending.delete(gid));
          }
        }
      }
    });

    // Listen for achievement unlocks
    const unlistenAch = listen<string>("achievement-unlocked", (event) => {
      addToast(`Achievement unlocked: ${event.payload}`, "success");
    });

    // Listen for AI narration
    const unlistenNarr = listen<string>("narration", (event) => {
      setNarrationText({ text: event.payload, key: ++narrationKey.current });
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenAch.then((fn) => fn());
      unlistenNarr.then((fn) => fn());
    };
  }, [addToast]);

  // Update selected fish detail
  useEffect(() => {
    if (!selectedFish) return;
    const fishId = selectedFish.id;
    let cancelled = false;
    const interval = setInterval(async () => {
      const detail = await invoke<FishDetail | null>("get_fish_detail", { fishId }).catch(() => null);
      if (cancelled) return;
      if (detail) setSelectedFish(detail);
      else setSelectedFish(null);
    }, 200);
    return () => { cancelled = true; clearInterval(interval); };
  }, [selectedFish?.id]);

  // Canvas click handler
  const handleCanvasClick = useCallback(
    async (e: React.MouseEvent) => {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      // Transform screen coords to tank coords for zoom/pan
      const tank = rendererRef.current?.screenToTank(x, y) ?? { x, y };

      if (decorationMode) {
        await invoke("add_decoration", {
          decorationType: decorationType,
          x: tank.x,
          y: Math.max(tank.y, window.innerHeight * 0.6), // decorations sit near bottom
          scale: 1.0,
          flipX: Math.random() > 0.5,
        });
        return;
      }

      if (feedMode) {
        await invoke("feed", { x: tank.x, y: tank.y, foodType });
        audioRef.current?.playFeed();
        setFeedMode(false);
        return;
      }

      const clickedFish = rendererRef.current?.findFishAt(x, y);
      if (clickedFish && breedingMode) {
        const detail = await invoke<FishDetail | null>("get_fish_detail", { fishId: clickedFish.id }).catch(() => null);
        if (detail) {
          if (!breedFishA) {
            setBreedFishA({ id: detail.id, genomeId: detail.genome_id });
          } else if (!breedFishB && detail.id !== breedFishA.id) {
            setBreedFishB({ id: detail.id, genomeId: detail.genome_id });
          }
          setSelectedFish(detail);
          await invoke("select_fish", { id: clickedFish.id }).catch(() => {});
        } else {
          setSelectedFish(null);
        }
        return;
      }
      if (clickedFish) {
        const detail = await invoke<FishDetail | null>("get_fish_detail", { fishId: clickedFish.id }).catch(() => null);
        setSelectedFish(detail);
        await invoke("select_fish", { id: clickedFish.id }).catch(() => {});
      } else if (selectedFish) {
        setSelectedFish(null);
        await invoke("select_fish", { id: null }).catch(() => {});
      } else {
        // Click empty space with nothing selected = drop food
        await invoke("feed", { x: tank.x, y: tank.y, foodType });
        audioRef.current?.playFeed();
      }
    },
    [feedMode, selectedFish, decorationMode, decorationType, foodType, breedingMode, breedFishA, breedFishB],
  );

  // Double-click on empty space = tap the glass
  const handleCanvasDoubleClick = useCallback(
    async (e: React.MouseEvent) => {
      if (widgetMode) {
        setWidgetMode(false);
        rendererRef.current?.setWidgetMode(false);
        invoke("toggle_widget_mode", { enabled: false }).catch(() => {});
        return;
      }
      if (feedMode || decorationMode) return;
      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const clickedFish = rendererRef.current?.findFishAt(x, y);
      if (clickedFish) return; // Don't tap if clicking a fish
      const tank = rendererRef.current?.screenToTank(x, y) ?? { x, y };
      await invoke("tap_glass", { x: tank.x, y: tank.y }).catch(() => {});
      rendererRef.current?.addTapRipple(tank.x, tank.y);
      audioRef.current?.playTap();
    },
    [feedMode, decorationMode, widgetMode],
  );

  const handlePauseToggle = useCallback(async () => {
    try {
      if (paused) {
        await invoke("resume");
        setPaused(false);
      } else {
        await invoke("pause");
        setPaused(true);
      }
    } catch { /* prevent state desync on invoke failure */ }
  }, [paused]);

  const handleSpeedChange = useCallback(async (mult: number) => {
    await invoke("set_speed", { multiplier: mult });
    setSpeed(mult);
  }, []);

  const handleStepForward = useCallback(async () => {
    const frame = await invoke<FrameUpdate>("step_forward");
    rendererRef.current?.updateFrame(frame);
    setFrame(frame);
  }, []);

  const handleScreenshot = useCallback(async () => {
    const blob = await rendererRef.current?.captureScreenshot();
    if (!blob) return;
    try {
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      addToast("Screenshot copied to clipboard", "success");
    } catch {
      // Fallback: download
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `deeptank-${Date.now()}.png`;
      a.click();
      URL.revokeObjectURL(url);
      addToast("Screenshot saved", "success");
    }
  }, [addToast]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKey = async (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;

      switch (e.key) {
        case " ":
          e.preventDefault();
          handlePauseToggle();
          break;
        case "f":
        case "F":
          setFeedMode((m) => !m);
          break;
        case "Escape":
          if (widgetMode) {
            setWidgetMode(false);
            rendererRef.current?.setWidgetMode(false);
            invoke("toggle_widget_mode", { enabled: false }).catch(() => {});
          }
          setSelectedFish(null);
          setFeedMode(false);
          setStatsOpen(false);
          setSettingsOpen(false);
          setGalleryOpen(false);
          setAchievementsOpen(false);
          setReplayOpen(false);
          setScenarioOpen(false);
          setLineageGenomeId(null);
          invoke("select_fish", { id: null }).catch(() => {});
          break;
        case "1":
          handleSpeedChange(1);
          break;
        case "2":
          handleSpeedChange(2);
          break;
        case "3":
          handleSpeedChange(4);
          break;
        case "4":
          handleSpeedChange(0.5);
          break;
        case "m":
        case "M":
          setMuted((m) => !m);
          break;
        case "s":
        case "S":
          if (!e.metaKey && !e.ctrlKey) setStatsOpen((o) => !o);
          break;
        case ".":
          if (paused) handleStepForward();
          break;
        case "p":
        case "P":
          if (!e.metaKey && !e.ctrlKey) handleScreenshot();
          break;
        case "b":
        case "B":
          setBreedingMode((m) => {
            if (m) { setBreedFishA(null); setBreedFishB(null); }
            return !m;
          });
          break;
        case "0":
          rendererRef.current?.resetViewport();
          break;
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [paused, widgetMode, handlePauseToggle, handleSpeedChange, handleStepForward, handleScreenshot]);

  const handleSettingUpdate = useCallback((key: string, value: number | boolean | string) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
    invoke("update_config", { key, value }).catch(() => {});
    // Sync audio settings
    if (key === "master_volume" && audioRef.current) audioRef.current.masterVolume = value as number;
    if (key === "ambient_enabled" && audioRef.current) audioRef.current.ambientEnabled = value as boolean;
    if (key === "event_sounds_enabled" && audioRef.current) audioRef.current.eventEnabled = value as boolean;
    // Sync theme
    if (key === "theme") rendererRef.current?.setTheme(value as ThemeName);
  }, []);

  return (
    <div style={{ position: "relative", width: "100vw", height: "100vh", overflow: "hidden" }}>
      <canvas
        ref={canvasRef}
        onClick={handleCanvasClick}
        onDoubleClick={handleCanvasDoubleClick}
        onMouseMove={(e) => {
          const rect = canvasRef.current?.getBoundingClientRect();
          if (!rect) return;
          const x = e.clientX - rect.left;
          const y = e.clientY - rect.top;
          rendererRef.current?.updateMousePosition(x, y);
          // Set cursor imperatively — React state doesn't track hoveredFishId
          if (canvasRef.current && !decorationMode && !feedMode) {
            canvasRef.current.style.cursor = rendererRef.current?.getHoveredFishId() ? "pointer" : "default";
          }
        }}
        style={{
          display: "block",
          width: "100%",
          height: "100%",
          cursor: decorationMode ? "copy" : feedMode ? "crosshair" : "default",
        }}
      />

      {!widgetMode && (
        <>
          <TankSwitcher />

          <TopBar
            frame={frame}
            onStatsToggle={() => setStatsOpen((o) => !o)}
            onSettingsToggle={() => setSettingsOpen((o) => !o)}
            onDecorateToggle={() => setDecorationMode((m) => !m)}
            onGalleryToggle={() => setGalleryOpen((o) => !o)}
            onAchievementsToggle={() => setAchievementsOpen((o) => !o)}
            onReplayToggle={() => setReplayOpen((o) => !o)}
            onScenarioToggle={() => setScenarioOpen((o) => !o)}
            onWidgetToggle={() => {
              setWidgetMode(true);
              rendererRef.current?.setWidgetMode(true);
              invoke("toggle_widget_mode", { enabled: true }).catch(() => {});
            }}
          />

          <NarrationTicker text={narrationText?.text ?? null} key={narrationText?.key ?? 0} />

          <Toolbar
            paused={paused}
            speed={speed}
            feedMode={feedMode}
            muted={muted}
            onPauseToggle={handlePauseToggle}
            onSpeedChange={handleSpeedChange}
            onFeedToggle={() => setFeedMode((m) => !m)}
            onMuteToggle={() => setMuted((m) => !m)}
            onStepForward={handleStepForward}
            onScreenshot={handleScreenshot}
            foodType={foodType}
            onFoodTypeChange={setFoodType}
            breedingMode={breedingMode}
            onBreedToggle={() => {
              setBreedingMode((m) => {
                if (m) { setBreedFishA(null); setBreedFishB(null); }
                return !m;
              });
            }}
          />

          {selectedFish && (
            <Inspector
              fish={selectedFish}
              onClose={() => setSelectedFish(null)}
              onViewLineage={(genomeId) => setLineageGenomeId(genomeId)}
              onFishUpdated={async () => {
                const detail = await invoke<FishDetail | null>("get_fish_detail", { fishId: selectedFish.id }).catch(() => null);
                if (detail) setSelectedFish(detail);
              }}
            />
          )}

          <StatsPanel open={statsOpen} onClose={() => setStatsOpen(false)} />
          <SettingsPanel
            open={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            settings={settings}
            onUpdate={handleSettingUpdate}
          />

          {decorationMode && (
            <DecorationPalette
              selectedType={decorationType}
              onSelect={setDecorationType}
              onClose={() => setDecorationMode(false)}
            />
          )}

          <SpeciesGallery open={galleryOpen} onClose={() => setGalleryOpen(false)} />
          <AchievementPanel open={achievementsOpen} onClose={() => setAchievementsOpen(false)} />

          {lineageGenomeId !== null && (
            <PhylogeneticTree
              genomeId={lineageGenomeId}
              onClose={() => setLineageGenomeId(null)}
            />
          )}

          {replayOpen && (
            <ReplayControls
              onClose={() => setReplayOpen(false)}
              onPauseSimulation={() => invoke("pause")}
            />
          )}

          {breedingMode && (
            <BreedingPanel
              fishAId={breedFishA?.id ?? null}
              fishBId={breedFishB?.id ?? null}
              genomeAId={breedFishA?.genomeId ?? null}
              genomeBId={breedFishB?.genomeId ?? null}
              onClose={() => { setBreedingMode(false); setBreedFishA(null); setBreedFishB(null); }}
              onBred={() => { addToast("Egg produced!", "success"); setBreedingMode(false); setBreedFishA(null); setBreedFishB(null); }}
            />
          )}

          <ScenarioPanel open={scenarioOpen} onClose={() => setScenarioOpen(false)} />

          <Toasts toasts={toasts} />
        </>
      )}
    </div>
  );
}

export default App;

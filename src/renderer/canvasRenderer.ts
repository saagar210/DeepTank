import type { FishState, FoodState, BubbleState, DecorationState, FrameUpdate, FishGenome } from "../types";
import { getCachedSprite, renderFishSprite, hasCachedSprite, evictStaleSprites } from "./fishSprite";

interface PrevFrame {
  fish: Map<number, FishState>;
  timestamp: number;
}

export type ThemeName = "aquarium" | "tropical" | "deep_ocean" | "freshwater";

interface ThemeColors {
  topR: number; topG: number; topB: number;
  botR: number; botG: number; botB: number;
  sandR: number; sandG: number; sandB: number;
  lightRayR: number; lightRayG: number; lightRayB: number; lightRayAlpha: number;
  causticR: number; causticG: number; causticB: number; causticAlpha: number;
  particleR: number; particleG: number; particleB: number;
}

const THEMES: Record<ThemeName, ThemeColors> = {
  aquarium: {
    topR: 26, topG: 58, topB: 92,
    botR: 10, botG: 22, botB: 40,
    sandR: 194, sandG: 178, sandB: 128,
    lightRayR: 255, lightRayG: 255, lightRayB: 220, lightRayAlpha: 0.06,
    causticR: 200, causticG: 220, causticB: 255, causticAlpha: 0.04,
    particleR: 200, particleG: 220, particleB: 240,
  },
  tropical: {
    topR: 15, topG: 80, topB: 85,
    botR: 5, botG: 35, botB: 45,
    sandR: 210, sandG: 185, sandB: 120,
    lightRayR: 255, lightRayG: 245, lightRayB: 200, lightRayAlpha: 0.08,
    causticR: 180, causticG: 240, causticB: 230, causticAlpha: 0.05,
    particleR: 180, particleG: 230, particleB: 220,
  },
  deep_ocean: {
    topR: 8, topG: 12, topB: 35,
    botR: 2, botG: 4, botB: 12,
    sandR: 80, sandG: 75, sandB: 65,
    lightRayR: 100, lightRayG: 120, lightRayB: 200, lightRayAlpha: 0.03,
    causticR: 80, causticG: 100, causticB: 180, causticAlpha: 0.02,
    particleR: 100, particleG: 120, particleB: 180,
  },
  freshwater: {
    topR: 20, topG: 50, topB: 30,
    botR: 8, botG: 25, botB: 15,
    sandR: 160, sandG: 155, sandB: 110,
    lightRayR: 200, lightRayG: 230, lightRayB: 180, lightRayAlpha: 0.05,
    causticR: 170, causticG: 210, causticB: 170, causticAlpha: 0.03,
    particleR: 180, particleG: 210, particleB: 170,
  },
};

export class CanvasRenderer {
  private ctx: CanvasRenderingContext2D;
  private width = 0;
  private height = 0;
  private dpr = 1;
  private prevFrame: PrevFrame = { fish: new Map(), timestamp: 0 };
  private currentFrame: FrameUpdate | null = null;
  private genomeCache = new Map<number, FishGenome>();
  private animId = 0;
  private time = 0;
  private lastFrameTime = 0;
  private lastEviction = 0;

  // Hover & selection
  private hoveredFishId: number | null = null;
  private selectedFishId: number | null = null;
  // Pause overlay
  private _paused = false;

  // Theme
  private theme: ThemeColors = THEMES.aquarium;

  // Viewport (zoom & pan)
  private vpX = 0;
  private vpY = 0;
  private vpZoom = 1.0;

  constructor(private canvas: HTMLCanvasElement) {
    this.ctx = canvas.getContext("2d")!;
    this.resize();
  }

  resize() {
    this.dpr = window.devicePixelRatio || 1;
    this.width = window.innerWidth;
    this.height = window.innerHeight;
    this.canvas.width = this.width * this.dpr;
    this.canvas.height = this.height * this.dpr;
    this.canvas.style.width = `${this.width}px`;
    this.canvas.style.height = `${this.height}px`;
  }

  hasGenome(genomeId: number): boolean {
    return this.genomeCache.has(genomeId);
  }

  cacheGenome(genome: FishGenome) {
    if (!this.genomeCache.has(genome.id)) {
      this.genomeCache.set(genome.id, genome);
      renderFishSprite(genome);
    }
  }

  cacheGenomes(genomes: FishGenome[]) {
    for (const g of genomes) {
      this.cacheGenome(g);
    }
  }

  updateFrame(frame: FrameUpdate) {
    // Save previous fish positions for interpolation
    if (this.currentFrame) {
      this.prevFrame.fish.clear();
      for (const f of this.currentFrame.fish) {
        this.prevFrame.fish.set(f.id, f);
      }
      this.prevFrame.timestamp = this.lastFrameTime;
    }
    this.currentFrame = frame;
    this.lastFrameTime = performance.now();

    // Cache genomes for new fish
    for (const f of frame.fish) {
      if (!hasCachedSprite(f.genome_id)) {
        const genome = this.genomeCache.get(f.genome_id);
        if (genome) {
          renderFishSprite(genome);
        }
      }
    }

    // Evict stale genomes/sprites every 30 seconds
    const now = performance.now();
    if (now - this.lastEviction > 30_000) {
      this.lastEviction = now;
      const activeIds = new Set(frame.fish.map((f) => f.genome_id));
      evictStaleSprites(activeIds);
      for (const id of this.genomeCache.keys()) {
        if (!activeIds.has(id)) this.genomeCache.delete(id);
      }
    }
  }

  start() {
    const draw = () => {
      this.render();
      this.animId = requestAnimationFrame(draw);
    };
    this.animId = requestAnimationFrame(draw);
  }

  stop() {
    cancelAnimationFrame(this.animId);
  }

  updateMousePosition(x: number, y: number) {
    const fish = this.findFishAt(x, y);
    this.hoveredFishId = fish?.id ?? null;
  }

  getHoveredFishId(): number | null {
    return this.hoveredFishId;
  }

  setSelectedFish(id: number | null) {
    this.selectedFishId = id;
  }

  setPaused(paused: boolean) {
    this._paused = paused;
  }

  setTheme(name: ThemeName) {
    this.theme = THEMES[name] ?? THEMES.aquarium;
  }

  zoomAt(screenX: number, screenY: number, delta: number) {
    const oldZoom = this.vpZoom;
    const factor = delta > 0 ? 0.9 : 1.1;
    this.vpZoom = Math.max(0.5, Math.min(4.0, this.vpZoom * factor));
    // Adjust pan so that zoom centers on mouse position
    const zoomRatio = this.vpZoom / oldZoom;
    this.vpX = screenX - (screenX - this.vpX) * zoomRatio;
    this.vpY = screenY - (screenY - this.vpY) * zoomRatio;
    this.clampViewport();
  }

  pan(dx: number, dy: number) {
    this.vpX += dx;
    this.vpY += dy;
    this.clampViewport();
  }

  resetViewport() {
    this.vpX = 0;
    this.vpY = 0;
    this.vpZoom = 1.0;
  }

  screenToTank(sx: number, sy: number): { x: number; y: number } {
    return {
      x: (sx - this.vpX) / this.vpZoom,
      y: (sy - this.vpY) / this.vpZoom,
    };
  }

  getViewport() {
    return { x: this.vpX, y: this.vpY, zoom: this.vpZoom };
  }

  private clampViewport() {
    const maxPanX = this.width * (this.vpZoom - 1);
    const maxPanY = this.height * (this.vpZoom - 1);
    this.vpX = Math.max(-maxPanX, Math.min(0, this.vpX));
    this.vpY = Math.max(-maxPanY, Math.min(0, this.vpY));
  }

  private currentHour = 12;

  private render() {
    this.time = performance.now();
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    const now = new Date();
    this.currentHour = now.getHours() + now.getMinutes() / 60;

    // Apply viewport transform (zoom & pan)
    this.ctx.save();
    this.ctx.translate(this.vpX, this.vpY);
    this.ctx.scale(this.vpZoom, this.vpZoom);

    // Background
    this.drawBackground();

    if (!this.currentFrame) {
      this.ctx.restore();
      return;
    }

    const alpha = this.getInterpolationAlpha();

    // Light rays
    this.drawLightRays();

    // Caustics
    this.drawCaustics();

    // Decorations (drawn before food/fish — they sit on sand)
    this.drawDecorations(this.currentFrame.decorations);

    // Sort fish by z for proper depth rendering
    const sortedFish = [...this.currentFrame.fish].sort((a, b) => a.z - b.z);

    // Food
    this.drawFood(this.currentFrame.food);

    // Fish
    for (const fish of sortedFish) {
      this.drawFish(fish, alpha);
    }

    // Hover & selection highlight rings
    for (const fish of sortedFish) {
      if (fish.id === this.selectedFishId) {
        this.drawHighlightRing(fish, alpha, "selected");
      } else if (fish.id === this.hoveredFishId) {
        this.drawHighlightRing(fish, alpha, "hover");
      }
    }

    // Bubbles
    this.drawBubbles(this.currentFrame.bubbles);

    // Floating particles
    this.drawParticles();

    // Surface effect
    this.drawSurface();

    // Restore viewport transform before overlays
    this.ctx.restore();
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);

    // Pause overlay (screen-fixed, not affected by zoom/pan)
    if (this._paused) {
      this.drawPauseOverlay();
    }
  }

  private getInterpolationAlpha(): number {
    if (!this.prevFrame.timestamp) return 1.0;
    const elapsed = this.time - this.lastFrameTime;
    const tickDuration = 33.333; // 30Hz
    return Math.min(elapsed / tickDuration, 1.0);
  }

  private drawBackground() {
    const { ctx, width, height, theme } = this;
    const wq = this.currentFrame?.water_quality ?? 1.0;

    const hour = this.currentHour;
    let dayBrightness = 1.0;
    if (hour < 6 || hour > 20) dayBrightness = 0.7;
    else if (hour < 8) dayBrightness = 0.7 + (hour - 6) / 2 * 0.3;
    else if (hour > 18) dayBrightness = 1.0 - (hour - 18) / 2 * 0.3;

    // Water quality tint: pristine=themed, poor=greenish
    const greenShift = Math.max(0, (1.0 - wq) * 30);
    const topR = Math.round(theme.topR * dayBrightness);
    const topG = Math.round((theme.topG + greenShift) * dayBrightness);
    const topB = Math.round((theme.topB - greenShift * 0.3) * dayBrightness);
    const botR = Math.round(theme.botR * dayBrightness);
    const botG = Math.round((theme.botG + greenShift * 0.5) * dayBrightness);
    const botB = Math.round((theme.botB - greenShift * 0.2) * dayBrightness);

    const gradient = ctx.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, `rgb(${topR},${topG},${topB})`);
    gradient.addColorStop(1, `rgb(${botR},${botG},${botB})`);
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);

    // Sand floor
    const sandY = height - 35;
    const { sandR, sandG, sandB } = theme;
    const sandGrad = ctx.createLinearGradient(0, sandY, 0, height);
    sandGrad.addColorStop(0, `rgba(${sandR},${sandG},${sandB},0.6)`);
    sandGrad.addColorStop(0.3, `rgba(${Math.round(sandR * 0.88)},${Math.round(sandG * 0.87)},${Math.round(sandB * 0.86)},0.5)`);
    sandGrad.addColorStop(1, `rgba(${Math.round(sandR * 0.72)},${Math.round(sandG * 0.70)},${Math.round(sandB * 0.70)},0.4)`);
    ctx.fillStyle = sandGrad;
    ctx.fillRect(0, sandY, width, height - sandY);
  }

  private drawLightRays() {
    const { ctx, width, height, time, theme } = this;
    if (this.currentHour < 6 || this.currentHour > 19) return; // No rays at night

    const { lightRayR: r, lightRayG: g, lightRayB: b, lightRayAlpha: a } = theme;
    ctx.save();
    for (let i = 0; i < 3; i++) {
      const baseX = width * (0.2 + i * 0.3) + Math.sin(time * 0.0002 + i * 2) * 50;
      const grad = ctx.createLinearGradient(baseX, 0, baseX + 100, height * 0.7);
      grad.addColorStop(0, `rgba(${r},${g},${b},${a})`);
      grad.addColorStop(0.5, `rgba(${r},${g},${b},${a * 0.5})`);
      grad.addColorStop(1, `rgba(${r},${g},${b},0)`);

      ctx.beginPath();
      ctx.moveTo(baseX - 20, 0);
      ctx.lineTo(baseX + 60, 0);
      ctx.lineTo(baseX + 160, height * 0.7);
      ctx.lineTo(baseX + 40, height * 0.7);
      ctx.closePath();
      ctx.fillStyle = grad;
      ctx.fill();
    }
    ctx.restore();
  }

  private drawCaustics() {
    const { ctx, width, height, time, theme } = this;
    const floorY = height - 60;
    const { causticR: cr, causticG: cg, causticB: cb, causticAlpha: ca } = theme;

    ctx.save();
    ctx.globalAlpha = ca;
    for (let i = 0; i < 8; i++) {
      const x = (i * 180 + Math.sin(time * 0.001 + i * 1.5) * 40) % (width + 100) - 50;
      const y = floorY + Math.cos(time * 0.0008 + i) * 10;
      const r = 40 + Math.sin(time * 0.0015 + i * 0.7) * 15;

      const grad = ctx.createRadialGradient(x, y, 0, x, y, r);
      grad.addColorStop(0, `rgba(${cr},${cg},${cb},1)`);
      grad.addColorStop(1, `rgba(${cr},${cg},${cb},0)`);
      ctx.fillStyle = grad;
      ctx.fillRect(x - r, y - r, r * 2, r * 2);
    }
    ctx.restore();
  }

  private drawFood(food: FoodState[]) {
    const { ctx, time } = this;
    for (const f of food) {
      switch (f.food_type) {
        case "flake":
          // Flat oval
          ctx.save();
          ctx.translate(f.x, f.y);
          ctx.rotate(Math.sin(time * 0.003 + f.x) * 0.3);
          ctx.beginPath();
          ctx.ellipse(0, 0, 4, 1.5, 0, 0, Math.PI * 2);
          ctx.fillStyle = "#e8c070";
          ctx.fill();
          ctx.restore();
          break;
        case "live":
          // Wiggly worm shape
          ctx.beginPath();
          const wiggle = Math.sin(time * 0.01 + f.y) * 3;
          ctx.moveTo(f.x - 3, f.y);
          ctx.quadraticCurveTo(f.x + wiggle, f.y - 2, f.x + 3, f.y);
          ctx.strokeStyle = "#e07040";
          ctx.lineWidth = 2;
          ctx.stroke();
          ctx.beginPath();
          ctx.arc(f.x + 3, f.y, 1, 0, Math.PI * 2);
          ctx.fillStyle = "#e07040";
          ctx.fill();
          break;
        default: // pellet
          ctx.beginPath();
          ctx.arc(f.x, f.y, 3, 0, Math.PI * 2);
          ctx.fillStyle = "#d4a574";
          ctx.fill();
          ctx.beginPath();
          ctx.arc(f.x - 0.5, f.y - 0.5, 1.5, 0, Math.PI * 2);
          ctx.fillStyle = "#e8c49a";
          ctx.fill();
          break;
      }
    }
  }

  private drawFish(fish: FishState, alpha: number) {
    const { ctx, time } = this;

    // Interpolate position
    const prev = this.prevFrame.fish.get(fish.id);
    let x = fish.x;
    let y = fish.y;
    if (prev) {
      x = prev.x + (fish.x - prev.x) * alpha;
      y = prev.y + (fish.y - prev.y) * alpha;
    }

    // Z-depth effects
    const z = fish.z;
    const renderScale = 0.7 + z * 0.3;
    const brightness = 0.7 + z * 0.3;
    const sizeIdx = z < 0.33 ? 0 : z < 0.66 ? 1 : 2;

    const sprites = getCachedSprite(fish.genome_id);
    if (!sprites || !sprites[sizeIdx]) {
      // Fallback: draw simple circle
      ctx.beginPath();
      ctx.arc(x, y, 6 * renderScale, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(200,200,255,${brightness})`;
      ctx.fill();
      return;
    }

    const sprite = sprites[sizeIdx];
    const speedFactor = Math.sqrt(fish.vx * fish.vx + fish.vy * fish.vy);

    // Swimming oscillation
    const bodyOsc = Math.sin(time * 0.005 * (1 + speedFactor * 0.3) + (fish.id % 100) * 10) * 0.04;

    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(fish.heading + bodyOsc);
    ctx.scale(renderScale, renderScale);

    // Brightness/depth tint
    ctx.globalAlpha = brightness;

    // Dying effect
    if (fish.behavior === "dying") {
      ctx.globalAlpha *= 0.5 + Math.sin(time * 0.01) * 0.2;
      ctx.rotate(0.2); // list to one side
    }

    // Draw sprite centered
    ctx.drawImage(sprite, -sprite.width / 2, -sprite.height / 2);

    // Disease green particle overlay
    if (fish.is_infected) {
      const pCount = 5;
      for (let i = 0; i < pCount; i++) {
        const angle = (time * 0.003 + i * (Math.PI * 2) / pCount + fish.id) % (Math.PI * 2);
        const dist = 8 + Math.sin(time * 0.005 + i * 2) * 4;
        const px = Math.cos(angle) * dist;
        const py = Math.sin(angle) * dist;
        ctx.beginPath();
        ctx.arc(px, py, 1.5 + Math.sin(time * 0.008 + i) * 0.5, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(80,220,80,${0.4 + Math.sin(time * 0.006 + i * 1.5) * 0.2})`;
        ctx.fill();
      }
    }

    ctx.restore();
  }

  private drawBubbles(bubbles: BubbleState[]) {
    const { ctx } = this;
    for (const b of bubbles) {
      ctx.beginPath();
      ctx.arc(b.x, b.y, b.radius, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(180,210,240,0.2)";
      ctx.fill();
      ctx.strokeStyle = "rgba(200,230,255,0.3)";
      ctx.lineWidth = 0.5;
      ctx.stroke();

      // Highlight
      ctx.beginPath();
      ctx.arc(b.x - b.radius * 0.25, b.y - b.radius * 0.25, b.radius * 0.3, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(255,255,255,0.4)";
      ctx.fill();
    }
  }

  private drawParticles() {
    const { ctx, width, height, time, theme } = this;
    const { particleR: pr, particleG: pg, particleB: pb } = theme;
    ctx.save();
    ctx.globalAlpha = 0.15;
    for (let i = 0; i < 30; i++) {
      const px = (i * 137.5 + time * 0.01 * (i % 3 === 0 ? 1 : -0.5)) % width;
      const py = (i * 97.3 + time * 0.005 * (i % 2 === 0 ? 1 : -1)) % height;
      ctx.fillStyle = `rgba(${pr},${pg},${pb},1)`;
      ctx.fillRect(px, py, 1.5, 1.5);
    }
    ctx.restore();
  }

  private drawSurface() {
    const { ctx, width, time } = this;
    ctx.save();
    ctx.globalAlpha = 0.15;
    const surfGrad = ctx.createLinearGradient(0, 0, 0, 15);
    surfGrad.addColorStop(0, "rgba(100,180,230,0.4)");
    surfGrad.addColorStop(1, "rgba(100,180,230,0)");
    ctx.fillStyle = surfGrad;

    ctx.beginPath();
    ctx.moveTo(0, 0);
    for (let x = 0; x <= width; x += 10) {
      const y = 4 + Math.sin(x * 0.02 + time * 0.002) * 2 + Math.sin(x * 0.01 + time * 0.001) * 1;
      ctx.lineTo(x, y);
    }
    ctx.lineTo(width, 15);
    ctx.lineTo(0, 15);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  }

  private drawDecorations(decorations: DecorationState[]) {
    const { ctx, time } = this;
    for (const d of decorations) {
      ctx.save();
      ctx.translate(d.x, d.y);
      if (d.flip_x) ctx.scale(-1, 1);
      ctx.scale(d.scale, d.scale);

      switch (d.decoration_type) {
        case "rock":
          this.drawRock(ctx);
          break;
        case "tall_plant":
          this.drawPlant(ctx, 80, time);
          break;
        case "short_plant":
          this.drawPlant(ctx, 40, time);
          break;
        case "coral":
          this.drawCoral(ctx, time);
          break;
      }

      ctx.restore();
    }
  }

  private drawRock(ctx: CanvasRenderingContext2D) {
    ctx.beginPath();
    ctx.moveTo(-20, 0);
    ctx.bezierCurveTo(-22, -15, -10, -28, 0, -30);
    ctx.bezierCurveTo(10, -28, 24, -18, 22, 0);
    ctx.closePath();
    const g = ctx.createLinearGradient(0, -30, 0, 0);
    g.addColorStop(0, "rgba(120,115,100,0.8)");
    g.addColorStop(1, "rgba(90,85,70,0.7)");
    ctx.fillStyle = g;
    ctx.fill();
    ctx.strokeStyle = "rgba(60,55,40,0.3)";
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  private drawPlant(ctx: CanvasRenderingContext2D, plantHeight: number, time: number) {
    const segments = plantHeight > 50 ? 5 : 3;
    for (let s = 0; s < segments; s++) {
      const angle = (s - (segments - 1) / 2) * 0.3;
      const sway = Math.sin(time * 0.002 + s * 0.8) * 6;
      ctx.save();
      ctx.rotate(angle);
      ctx.beginPath();
      ctx.moveTo(-3, 0);
      const tipX = sway;
      const tipY = -plantHeight * (0.7 + s * 0.06);
      ctx.bezierCurveTo(-2, tipY * 0.4, tipX - 4, tipY * 0.7, tipX, tipY);
      ctx.bezierCurveTo(tipX + 4, tipY * 0.7, 2, tipY * 0.4, 3, 0);
      ctx.closePath();
      const g = ctx.createLinearGradient(0, 0, 0, tipY);
      g.addColorStop(0, "rgba(40,120,50,0.7)");
      g.addColorStop(1, "rgba(60,180,70,0.5)");
      ctx.fillStyle = g;
      ctx.fill();
      ctx.restore();
    }
  }

  private drawCoral(ctx: CanvasRenderingContext2D, time: number) {
    const branches = 4;
    for (let b = 0; b < branches; b++) {
      const angle = (b / branches - 0.5) * 1.2;
      const sway = Math.sin(time * 0.0015 + b * 1.2) * 3;
      ctx.save();
      ctx.rotate(angle);
      ctx.beginPath();
      ctx.moveTo(-4, 0);
      ctx.bezierCurveTo(-3, -15, sway - 5, -30, sway, -35);
      ctx.bezierCurveTo(sway + 5, -30, 3, -15, 4, 0);
      ctx.closePath();
      ctx.fillStyle = `hsla(${15 + b * 20}, 70%, 55%, 0.7)`;
      ctx.fill();
      ctx.restore();
    }
  }

  private drawHighlightRing(fish: FishState, alpha: number, type: "hover" | "selected") {
    const { ctx, time } = this;
    const prev = this.prevFrame.fish.get(fish.id);
    const x = prev ? prev.x + (fish.x - prev.x) * alpha : fish.x;
    const y = prev ? prev.y + (fish.y - prev.y) * alpha : fish.y;
    const z = fish.z;
    const renderScale = 0.7 + z * 0.3;
    const radius = 18 * renderScale;

    ctx.save();
    if (type === "selected") {
      const pulse = 0.6 + Math.sin(time * 0.004) * 0.4;
      ctx.strokeStyle = `rgba(100,180,255,${pulse * 0.7})`;
      ctx.lineWidth = 2;
      ctx.setLineDash([]);
    } else {
      ctx.strokeStyle = "rgba(100,180,255,0.35)";
      ctx.lineWidth = 1.5;
      ctx.setLineDash([4, 3]);
    }
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }

  captureScreenshot(): Promise<Blob | null> {
    return new Promise((resolve) => {
      this.canvas.toBlob((blob) => resolve(blob), "image/png");
    });
  }

  private drawPauseOverlay() {
    const { ctx, width, height, time } = this;
    ctx.save();
    ctx.fillStyle = "rgba(0,0,0,0.3)";
    ctx.fillRect(0, 0, width, height);
    const pulse = 0.3 + Math.sin(time * 0.003) * 0.1;
    ctx.globalAlpha = pulse;
    ctx.fillStyle = "#fff";
    ctx.font = "300 48px system-ui";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.letterSpacing = "8px";
    ctx.fillText("PAUSED", width / 2, height / 2);
    ctx.restore();
  }

  /** Find fish near a click point (uses interpolated positions) */
  findFishAt(screenX: number, screenY: number): FishState | null {
    if (!this.currentFrame) return null;
    // Convert screen coords to tank coords (accounting for zoom/pan)
    const { x, y } = this.screenToTank(screenX, screenY);
    const alpha = this.getInterpolationAlpha();
    let closest: FishState | null = null;
    let closestDist = 25 / this.vpZoom; // Adjust click radius for zoom level
    for (const f of this.currentFrame.fish) {
      const prev = this.prevFrame.fish.get(f.id);
      const fx = prev ? prev.x + (f.x - prev.x) * alpha : f.x;
      const fy = prev ? prev.y + (f.y - prev.y) * alpha : f.y;
      const dx = fx - x;
      const dy = fy - y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < closestDist) {
        closestDist = dist;
        closest = f;
      }
    }
    return closest;
  }
}

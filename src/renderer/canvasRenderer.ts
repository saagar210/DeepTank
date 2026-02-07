import type { FishState, FoodState, BubbleState, FrameUpdate, FishGenome } from "../types";
import { getCachedSprite, renderFishSprite, hasCachedSprite, evictStaleSprites } from "./fishSprite";

interface PrevFrame {
  fish: Map<number, FishState>;
  timestamp: number;
}

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

  private currentHour = 12;

  private render() {
    this.time = performance.now();
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    const now = new Date();
    this.currentHour = now.getHours() + now.getMinutes() / 60;

    // Background
    this.drawBackground();

    if (!this.currentFrame) return;

    const alpha = this.getInterpolationAlpha();

    // Light rays
    this.drawLightRays();

    // Caustics
    this.drawCaustics();

    // Sort fish by z for proper depth rendering
    const sortedFish = [...this.currentFrame.fish].sort((a, b) => a.z - b.z);

    // Food
    this.drawFood(this.currentFrame.food);

    // Fish
    for (const fish of sortedFish) {
      this.drawFish(fish, alpha);
    }

    // Bubbles
    this.drawBubbles(this.currentFrame.bubbles);

    // Floating particles
    this.drawParticles();

    // Surface effect
    this.drawSurface();
  }

  private getInterpolationAlpha(): number {
    if (!this.prevFrame.timestamp) return 1.0;
    const elapsed = this.time - this.lastFrameTime;
    const tickDuration = 33.333; // 30Hz
    return Math.min(elapsed / tickDuration, 1.0);
  }

  private drawBackground() {
    const { ctx, width, height } = this;
    const wq = this.currentFrame?.water_quality ?? 1.0;

    const hour = this.currentHour;
    let dayBrightness = 1.0;
    if (hour < 6 || hour > 20) dayBrightness = 0.7;
    else if (hour < 8) dayBrightness = 0.7 + (hour - 6) / 2 * 0.3;
    else if (hour > 18) dayBrightness = 1.0 - (hour - 18) / 2 * 0.3;

    // Water quality tint: pristine=blue, poor=greenish
    const greenShift = Math.max(0, (1.0 - wq) * 30);
    const topR = Math.round(26 * dayBrightness);
    const topG = Math.round((58 + greenShift) * dayBrightness);
    const topB = Math.round((92 - greenShift * 0.3) * dayBrightness);
    const botR = Math.round(10 * dayBrightness);
    const botG = Math.round((22 + greenShift * 0.5) * dayBrightness);
    const botB = Math.round((40 - greenShift * 0.2) * dayBrightness);

    const gradient = ctx.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, `rgb(${topR},${topG},${topB})`);
    gradient.addColorStop(1, `rgb(${botR},${botG},${botB})`);
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);

    // Sand floor
    const sandY = height - 35;
    const sandGrad = ctx.createLinearGradient(0, sandY, 0, height);
    sandGrad.addColorStop(0, "rgba(194,178,128,0.6)");
    sandGrad.addColorStop(0.3, "rgba(170,155,110,0.5)");
    sandGrad.addColorStop(1, "rgba(140,125,90,0.4)");
    ctx.fillStyle = sandGrad;
    ctx.fillRect(0, sandY, width, height - sandY);
  }

  private drawLightRays() {
    const { ctx, width, height, time } = this;
    if (this.currentHour < 6 || this.currentHour > 19) return; // No rays at night

    ctx.save();
    for (let i = 0; i < 3; i++) {
      const baseX = width * (0.2 + i * 0.3) + Math.sin(time * 0.0002 + i * 2) * 50;
      const grad = ctx.createLinearGradient(baseX, 0, baseX + 100, height * 0.7);
      grad.addColorStop(0, "rgba(255,255,220,0.06)");
      grad.addColorStop(0.5, "rgba(255,255,220,0.03)");
      grad.addColorStop(1, "rgba(255,255,220,0)");

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
    const { ctx, width, height, time } = this;
    const floorY = height - 60;

    ctx.save();
    ctx.globalAlpha = 0.04;
    for (let i = 0; i < 8; i++) {
      const x = (i * 180 + Math.sin(time * 0.001 + i * 1.5) * 40) % (width + 100) - 50;
      const y = floorY + Math.cos(time * 0.0008 + i) * 10;
      const r = 40 + Math.sin(time * 0.0015 + i * 0.7) * 15;

      const grad = ctx.createRadialGradient(x, y, 0, x, y, r);
      grad.addColorStop(0, "rgba(200,220,255,1)");
      grad.addColorStop(1, "rgba(200,220,255,0)");
      ctx.fillStyle = grad;
      ctx.fillRect(x - r, y - r, r * 2, r * 2);
    }
    ctx.restore();
  }

  private drawFood(food: FoodState[]) {
    const { ctx } = this;
    for (const f of food) {
      ctx.beginPath();
      ctx.arc(f.x, f.y, 3, 0, Math.PI * 2);
      ctx.fillStyle = "#d4a574";
      ctx.fill();
      ctx.beginPath();
      ctx.arc(f.x - 0.5, f.y - 0.5, 1.5, 0, Math.PI * 2);
      ctx.fillStyle = "#e8c49a";
      ctx.fill();
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
    const { ctx, width, height, time } = this;
    ctx.save();
    ctx.globalAlpha = 0.15;
    for (let i = 0; i < 30; i++) {
      const px = (i * 137.5 + time * 0.01 * (i % 3 === 0 ? 1 : -0.5)) % width;
      const py = (i * 97.3 + time * 0.005 * (i % 2 === 0 ? 1 : -1)) % height;
      ctx.fillStyle = "rgba(200,220,240,1)";
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

  /** Find fish near a click point (uses interpolated positions) */
  findFishAt(x: number, y: number): FishState | null {
    if (!this.currentFrame) return null;
    const alpha = this.getInterpolationAlpha();
    let closest: FishState | null = null;
    let closestDist = 25; // click radius
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

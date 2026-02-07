import type { FishGenome } from "../types";

const spriteCache = new Map<number, ImageBitmap[]>(); // genome_id -> [back, mid, front]
const pendingSprites = new Set<number>();

export function getCachedSprite(genomeId: number): ImageBitmap[] | undefined {
  return spriteCache.get(genomeId);
}

export function hasCachedSprite(genomeId: number): boolean {
  return spriteCache.has(genomeId) || pendingSprites.has(genomeId);
}

export async function renderFishSprite(genome: FishGenome): Promise<void> {
  if (spriteCache.has(genome.id) || pendingSprites.has(genome.id)) return;
  pendingSprites.add(genome.id);

  const sizes = [0.7, 0.85, 1.0]; // back, mid, front scale
  const bitmaps: ImageBitmap[] = [];

  for (const scale of sizes) {
    const baseLen = 30 * genome.body_length * scale;
    const baseWid = 16 * genome.body_width * scale;
    const padding = 20 * scale;
    const w = Math.ceil(baseLen + genome.tail_size * 15 * scale + padding * 2);
    const h = Math.ceil(baseWid * 2 + genome.dorsal_fin_size * 12 * scale + padding * 2);

    const canvas = new OffscreenCanvas(w, h);
    const ctx = canvas.getContext("2d")!;
    ctx.translate(padding + genome.tail_size * 10 * scale, h / 2);

    drawFishBody(ctx, genome, baseLen, baseWid, scale);
    drawFins(ctx, genome, baseLen, baseWid, scale);
    drawEye(ctx, genome, baseLen, baseWid, scale);
    drawPattern(ctx, genome, baseLen, baseWid, scale);

    const bitmap = await createImageBitmap(canvas);
    bitmaps.push(bitmap);
  }

  spriteCache.set(genome.id, bitmaps);
  pendingSprites.delete(genome.id);
}

function hsl(h: number, s: number, l: number): string {
  return `hsl(${h}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}

function genomeColor(genome: FishGenome, isMale: boolean): string {
  const satMod = isMale ? 1.1 : 1.0;
  return hsl(genome.base_hue, Math.min(genome.saturation * satMod, 1.0), genome.lightness);
}

function genomePatternColor(genome: FishGenome): string {
  const hue = (genome.base_hue + genome.pattern_color_offset) % 360;
  return hsl(hue, genome.saturation * 0.9, genome.lightness * 0.8);
}

function drawFishBody(
  ctx: OffscreenCanvasRenderingContext2D,
  genome: FishGenome,
  len: number,
  wid: number,
  _scale: number,
) {
  const isMale = genome.sex === "Male";
  const color = genomeColor(genome, isMale);

  ctx.beginPath();
  // Body using bezier curves — wider at 1/3 from nose, tapers to tail
  const noseX = len * 0.5;
  const tailX = -len * 0.5;
  const maxWidthX = len * 0.15; // widest point
  const topWid = wid;
  const botWid = wid * 0.9;

  // Start at nose
  ctx.moveTo(noseX, 0);

  // Top curve: nose → max width → tail
  ctx.bezierCurveTo(
    noseX - len * 0.1, -topWid * 0.7, // cp1
    maxWidthX + len * 0.1, -topWid,    // cp2
    maxWidthX, -topWid,                 // to max width point
  );
  ctx.bezierCurveTo(
    maxWidthX - len * 0.2, -topWid * 0.95, // cp1
    tailX + len * 0.15, -wid * 0.3,         // cp2
    tailX, 0,                                // to tail
  );

  // Bottom curve: tail → max width → nose
  ctx.bezierCurveTo(
    tailX + len * 0.15, botWid * 0.3,    // cp1
    maxWidthX - len * 0.2, botWid * 0.95, // cp2
    maxWidthX, botWid,                     // to max width
  );
  ctx.bezierCurveTo(
    maxWidthX + len * 0.1, botWid,    // cp1
    noseX - len * 0.1, botWid * 0.7,  // cp2
    noseX, 0,                          // back to nose
  );

  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();

  // Subtle body outline
  ctx.strokeStyle = hsl(genome.base_hue, genome.saturation * 0.6, genome.lightness * 0.6);
  ctx.lineWidth = 0.8;
  ctx.stroke();
}

function drawFins(
  ctx: OffscreenCanvasRenderingContext2D,
  genome: FishGenome,
  len: number,
  wid: number,
  scale: number,
) {
  const isMale = genome.sex === "Male";
  const finScale = isMale ? 1.15 : 1.0;
  const color = hsl(genome.base_hue, genome.saturation * 0.85, genome.lightness * 0.9);
  const tailX = -len * 0.5;

  // Tail fin
  const tailSize = genome.tail_size * 12 * scale * finScale;
  ctx.beginPath();
  ctx.moveTo(tailX, 0);
  ctx.lineTo(tailX - tailSize, -tailSize * 0.8);
  ctx.quadraticCurveTo(tailX - tailSize * 0.4, 0, tailX - tailSize, tailSize * 0.8);
  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();

  // Dorsal fin
  const dorsalSize = genome.dorsal_fin_size * 10 * scale * finScale;
  const dorsalX = len * 0.05;
  ctx.beginPath();
  ctx.moveTo(dorsalX + dorsalSize * 0.5, -wid * 0.9);
  ctx.quadraticCurveTo(dorsalX, -wid - dorsalSize, dorsalX - dorsalSize * 0.5, -wid * 0.9);
  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();

  // Pectoral fins (two small ones on sides)
  const pectSize = genome.pectoral_fin_size * 6 * scale * finScale;
  const pectX = len * 0.15;
  for (const side of [-1, 1]) {
    ctx.beginPath();
    ctx.moveTo(pectX, wid * 0.5 * side);
    ctx.quadraticCurveTo(
      pectX - pectSize * 0.3, (wid * 0.5 + pectSize) * side,
      pectX - pectSize, wid * 0.7 * side,
    );
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
  }
}

function drawEye(
  ctx: OffscreenCanvasRenderingContext2D,
  genome: FishGenome,
  len: number,
  wid: number,
  scale: number,
) {
  const eyeR = genome.eye_size * 3 * scale;
  const eyeX = len * 0.3;
  const eyeY = -wid * 0.2;

  // Eye white
  ctx.beginPath();
  ctx.arc(eyeX, eyeY, eyeR, 0, Math.PI * 2);
  ctx.fillStyle = "#e8e8e8";
  ctx.fill();

  // Pupil
  ctx.beginPath();
  ctx.arc(eyeX + eyeR * 0.2, eyeY, eyeR * 0.55, 0, Math.PI * 2);
  ctx.fillStyle = "#111";
  ctx.fill();

  // Highlight
  ctx.beginPath();
  ctx.arc(eyeX + eyeR * 0.3, eyeY - eyeR * 0.25, eyeR * 0.2, 0, Math.PI * 2);
  ctx.fillStyle = "#fff";
  ctx.fill();
}

function drawPattern(
  ctx: OffscreenCanvasRenderingContext2D,
  genome: FishGenome,
  len: number,
  wid: number,
  _scale: number,
) {
  if (genome.pattern_intensity < 0.05) return;

  const alpha = genome.pattern_intensity * 0.6;
  const patternColor = genomePatternColor(genome);
  ctx.globalAlpha = alpha;
  ctx.globalCompositeOperation = "source-atop";

  if (genome.pattern.Striped != null) {
    const angle = ((genome.pattern.Striped.angle ?? 0) * Math.PI) / 180;
    const stripeCount = 5;
    const spacing = len / stripeCount;
    ctx.strokeStyle = patternColor;
    ctx.lineWidth = 3;
    for (let i = 0; i < stripeCount; i++) {
      const x = -len * 0.4 + i * spacing;
      ctx.beginPath();
      ctx.moveTo(x + Math.cos(angle) * wid, -wid);
      ctx.lineTo(x - Math.cos(angle) * wid, wid);
      ctx.stroke();
    }
  } else if (genome.pattern.Spotted != null) {
    const density = genome.pattern.Spotted.density ?? 0.5;
    const spotCount = Math.floor(4 + density * 10);
    const spotR = (1.2 - density * 0.5) * 3;
    ctx.fillStyle = patternColor;
    // Deterministic spots based on genome id
    for (let i = 0; i < spotCount; i++) {
      const t = i / spotCount;
      const sx = -len * 0.35 + t * len * 0.7 + Math.sin(i * 7.3) * 5;
      const sy = Math.cos(i * 4.1) * wid * 0.6;
      ctx.beginPath();
      ctx.arc(sx, sy, spotR, 0, Math.PI * 2);
      ctx.fill();
    }
  } else if (genome.pattern.Gradient != null) {
    const dir = ((genome.pattern.Gradient.direction ?? 0) * Math.PI) / 180;
    const grad = ctx.createLinearGradient(
      Math.cos(dir) * -len * 0.5,
      Math.sin(dir) * -wid,
      Math.cos(dir) * len * 0.5,
      Math.sin(dir) * wid,
    );
    grad.addColorStop(0, "transparent");
    grad.addColorStop(1, patternColor);
    ctx.fillStyle = grad;
    ctx.fillRect(-len * 0.5, -wid * 1.5, len, wid * 3);
  } else if (genome.pattern.Bicolor != null) {
    const split = genome.pattern.Bicolor.split ?? 0.5;
    const splitX = -len * 0.5 + len * split;
    ctx.fillStyle = patternColor;
    ctx.fillRect(splitX, -wid * 1.5, len * (1 - split), wid * 3);
  }

  ctx.globalAlpha = 1.0;
  ctx.globalCompositeOperation = "source-over";
}

export function evictStaleSprites(activeGenomeIds: Set<number>) {
  for (const [id, bitmaps] of spriteCache) {
    if (!activeGenomeIds.has(id)) {
      for (const bm of bitmaps) bm.close();
      spriteCache.delete(id);
      pendingSprites.delete(id);
    }
  }
}

export function clearSpriteCache() {
  for (const bitmaps of spriteCache.values()) {
    for (const bm of bitmaps) bm.close();
  }
  spriteCache.clear();
  pendingSprites.clear();
}

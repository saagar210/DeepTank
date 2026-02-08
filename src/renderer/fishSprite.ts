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
  const noseX = len * 0.5;
  const tailX = -len * 0.5;

  // body_width affects bezier spread: low = sleek, high = round/chubby
  // body_length shifts the widest point: long fish have it further forward
  const widthFactor = genome.body_width; // 0.6 ~ 2.0
  const lenFactor = genome.body_length;  // 0.6 ~ 2.0
  // Widest point: long fish → further forward, short fish → more centered
  const maxWidthX = len * (0.05 + 0.15 * Math.min(lenFactor / 1.5, 1.0));
  // Control point spread: low width → tight (sleek), high width → wide (chubby)
  const spreadFactor = 0.6 + (widthFactor - 0.6) * 0.3;

  const topWid = wid;
  // Females get a rounder belly contour
  const botWid = isMale ? wid * 0.9 : wid * 1.0;

  // Start at nose
  ctx.moveTo(noseX, 0);

  // Top curve: nose → max width → tail
  ctx.bezierCurveTo(
    noseX - len * 0.1, -topWid * 0.7 * spreadFactor,
    maxWidthX + len * 0.1, -topWid,
    maxWidthX, -topWid,
  );
  ctx.bezierCurveTo(
    maxWidthX - len * 0.2, -topWid * 0.95,
    tailX + len * 0.15, -wid * 0.3,
    tailX, 0,
  );

  // Bottom curve: tail → max width → nose
  ctx.bezierCurveTo(
    tailX + len * 0.15, botWid * 0.3,
    maxWidthX - len * 0.2, botWid * 0.95 * spreadFactor,
    maxWidthX, botWid,
  );
  ctx.bezierCurveTo(
    maxWidthX + len * 0.1, botWid * spreadFactor,
    noseX - len * 0.1, botWid * 0.7 * spreadFactor,
    noseX, 0,
  );

  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();

  // Subtle body outline
  ctx.strokeStyle = hsl(genome.base_hue, genome.saturation * 0.6, genome.lightness * 0.6);
  ctx.lineWidth = 0.8;
  ctx.stroke();

  // Ventral line — subtle darker line along belly
  ctx.beginPath();
  ctx.moveTo(noseX * 0.7, botWid * 0.2);
  ctx.quadraticCurveTo(maxWidthX, botWid * 0.55, tailX * 0.6, botWid * 0.1);
  ctx.strokeStyle = hsl(genome.base_hue, genome.saturation * 0.5, genome.lightness * 0.45);
  ctx.lineWidth = 0.6;
  ctx.stroke();

  // Males: brighter ventral stripe
  if (isMale) {
    ctx.beginPath();
    ctx.moveTo(noseX * 0.5, botWid * 0.15);
    ctx.quadraticCurveTo(maxWidthX, botWid * 0.4, tailX * 0.5, botWid * 0.08);
    ctx.strokeStyle = hsl(genome.base_hue, Math.min(genome.saturation * 1.3, 1.0), Math.min(genome.lightness * 1.2, 0.9));
    ctx.lineWidth = 1.0;
    ctx.globalAlpha = 0.4;
    ctx.stroke();
    ctx.globalAlpha = 1.0;
  }
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
  const ts = genome.tail_size;

  // === Tail fin — shape varies by tail_size ===
  const tailMag = ts * 12 * scale * finScale;
  ctx.beginPath();
  if (ts < 0.8) {
    // Small rounded tail — single semicircle
    ctx.moveTo(tailX, 0);
    ctx.arc(tailX - tailMag * 0.5, 0, tailMag * 0.6, -Math.PI / 2, Math.PI / 2);
    ctx.closePath();
  } else if (ts <= 1.3) {
    // Medium forked tail — v-shape
    ctx.moveTo(tailX, 0);
    ctx.lineTo(tailX - tailMag, -tailMag * 0.8);
    ctx.quadraticCurveTo(tailX - tailMag * 0.4, 0, tailX - tailMag, tailMag * 0.8);
    ctx.closePath();
  } else {
    // Large lunate (crescent) tail — two curved lobes with deep fork
    ctx.moveTo(tailX, -wid * 0.15);
    ctx.bezierCurveTo(
      tailX - tailMag * 0.6, -tailMag * 0.5,
      tailX - tailMag * 0.9, -tailMag * 1.0,
      tailX - tailMag, -tailMag * 0.9,
    );
    ctx.quadraticCurveTo(tailX - tailMag * 0.5, -tailMag * 0.1, tailX - tailMag * 0.35, 0);
    ctx.quadraticCurveTo(tailX - tailMag * 0.5, tailMag * 0.1, tailX - tailMag, tailMag * 0.9);
    ctx.bezierCurveTo(
      tailX - tailMag * 0.9, tailMag * 1.0,
      tailX - tailMag * 0.6, tailMag * 0.5,
      tailX, wid * 0.15,
    );
    ctx.closePath();
  }
  ctx.fillStyle = color;
  ctx.fill();

  // === Dorsal fin — shape varies by dorsal_fin_size ===
  const ds = genome.dorsal_fin_size;
  const dorsalMag = ds * 10 * scale * finScale;
  const dorsalX = len * 0.05;
  ctx.beginPath();
  if (ds < 0.6) {
    // Low ridge — barely visible bump
    ctx.moveTo(dorsalX + dorsalMag * 0.8, -wid * 0.85);
    ctx.quadraticCurveTo(dorsalX, -wid - dorsalMag * 0.5, dorsalX - dorsalMag * 0.8, -wid * 0.85);
    ctx.closePath();
  } else if (ds <= 1.0) {
    // Triangular sail
    ctx.moveTo(dorsalX + dorsalMag * 0.5, -wid * 0.9);
    ctx.quadraticCurveTo(dorsalX, -wid - dorsalMag, dorsalX - dorsalMag * 0.5, -wid * 0.9);
    ctx.closePath();
  } else {
    // Flowing banner fin — extended backward with multiple bezier segments
    ctx.moveTo(dorsalX + dorsalMag * 0.4, -wid * 0.9);
    ctx.bezierCurveTo(
      dorsalX + dorsalMag * 0.2, -wid - dorsalMag * 0.8,
      dorsalX - dorsalMag * 0.1, -wid - dorsalMag,
      dorsalX - dorsalMag * 0.3, -wid - dorsalMag * 0.9,
    );
    ctx.bezierCurveTo(
      dorsalX - dorsalMag * 0.6, -wid - dorsalMag * 0.6,
      dorsalX - dorsalMag * 0.9, -wid * 0.95,
      dorsalX - dorsalMag * 1.0, -wid * 0.9,
    );
    ctx.closePath();
  }
  ctx.fillStyle = color;
  ctx.fill();

  // === Pectoral fins — size varies significantly ===
  const ps = genome.pectoral_fin_size;
  const pectMag = ps * 6 * scale * finScale;
  const pectX = len * 0.15;
  for (const side of [-1, 1]) {
    ctx.beginPath();
    if (ps < 0.6) {
      // Tiny nubs
      ctx.moveTo(pectX, wid * 0.4 * side);
      ctx.quadraticCurveTo(
        pectX - pectMag * 0.2, (wid * 0.4 + pectMag * 0.5) * side,
        pectX - pectMag * 0.6, wid * 0.5 * side,
      );
      ctx.closePath();
    } else if (ps <= 1.0) {
      // Normal pectoral fins
      ctx.moveTo(pectX, wid * 0.5 * side);
      ctx.quadraticCurveTo(
        pectX - pectMag * 0.3, (wid * 0.5 + pectMag) * side,
        pectX - pectMag, wid * 0.7 * side,
      );
      ctx.closePath();
    } else {
      // Large wing-like fins
      ctx.moveTo(pectX + pectMag * 0.1, wid * 0.4 * side);
      ctx.bezierCurveTo(
        pectX, (wid * 0.5 + pectMag * 0.8) * side,
        pectX - pectMag * 0.5, (wid * 0.5 + pectMag * 1.2) * side,
        pectX - pectMag * 1.1, (wid * 0.6 + pectMag * 0.3) * side,
      );
      ctx.quadraticCurveTo(
        pectX - pectMag * 0.5, wid * 0.5 * side,
        pectX, wid * 0.35 * side,
      );
      ctx.closePath();
    }
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

import * as THREE from "three";
import type { Costume, JerseyDesign, JerseyStripe } from "../protocol";
import { PUCKLAB_MARK } from "./pucklab-mark";

export const PUCKLAB_JERSEY = "shirt-pucklab";

/** Worn before a club design has synced: PuckLab's colours and mark, no team. */
// PuckLab's own default kit, worn by any Lab club that has not designed one.
export const DEFAULT_JERSEY: JerseyDesign = {
  body: "#23333F",
  collar: "#50E1D5",
  trim: "#FFFFFF",
  sleeve_stripes: [{ pattern: "double", color: "#50E1D5" }],
  hem_stripes: [{ pattern: "single", color: "#50E1D5" }],
};

export function jerseyDesign(
  costume: Costume | null | undefined,
): JerseyDesign | null {
  return costume?.shirt === PUCKLAB_JERSEY
    ? (costume.jersey ?? DEFAULT_JERSEY)
    : null;
}

// Jersey Lab's stripe patterns as [offset, height] in stripe units.
const PATTERNS: Record<JerseyStripe["pattern"], [number, number][]> = {
  single: [[0, 3]],
  double: [
    [0, 1.5],
    [2.5, 1.5],
  ],
  triple: [
    [0, 1.1],
    [1.9, 1.1],
    [3.8, 1.1],
  ],
  bold: [[0, 5.5]],
  thick_thin: [
    [0, 3],
    [3.8, 1],
  ],
  thin_thick_thin: [
    [0, 0.9],
    [1.6, 2.6],
    [4.9, 0.9],
  ],
};

// The garment's atlas, in its 1024 x 768 logical layout (see models/PROVENANCE.txt).
const BODY = { x: 0, y: 0, w: 1024, h: 512 };
const SLEEVE = { x: 0, y: 512, w: 384, h: 256 };
const COLLAR = { x: 640, y: 512, w: 384, h: 64 };
const YOKE_DEPTH = 96;
const CREST = { x: 256, y: 232 };
// The round body stretches the chest sideways; this keeps a crest round when worn.
const CREST_SQUASH = 0.53;

type Region = typeof BODY;

function bands(
  ctx: CanvasRenderingContext2D,
  region: Region,
  bottom: number,
  unit: number,
  stripes: JerseyStripe[] | undefined,
  gap: string,
) {
  const rows: { height: number; color: string }[] = [];
  for (const stripe of stripes ?? []) {
    let previous = 0;
    for (const [offset, size] of PATTERNS[stripe.pattern] ?? []) {
      if (offset > previous)
        rows.push({ height: (offset - previous) * unit, color: gap });
      rows.push({ height: size * unit, color: stripe.color });
      previous = offset + size;
    }
  }
  const total = rows.reduce((sum, row) => sum + row.height, 0);
  let at = Math.max(
    region.y,
    Math.min(bottom - total, region.y + region.h - total),
  );
  for (const row of rows) {
    ctx.fillStyle = row.color;
    ctx.fillRect(region.x, at, region.w, row.height);
    at += row.height;
  }
}

function drawPaths(
  ctx: CanvasRenderingContext2D,
  viewBox: readonly number[],
  parts: { d: string; fill: string }[],
  height: number,
) {
  const [vx, vy, vw, vh] = viewBox;
  const scale = height / Math.max(vw, vh);
  ctx.save();
  ctx.translate(CREST.x, CREST.y);
  ctx.scale(scale * CREST_SQUASH, scale);
  ctx.translate(-vx - vw / 2, -vy - vh / 2);
  for (const part of parts) {
    ctx.fillStyle = part.fill;
    ctx.fill(new Path2D(part.d));
  }
  ctx.restore();
}

function backing(ctx: CanvasRenderingContext2D, color: string, radius: number) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.ellipse(
    CREST.x,
    CREST.y,
    radius * CREST_SQUASH,
    radius,
    0,
    0,
    Math.PI * 2,
  );
  ctx.fill();
}

function crest(
  ctx: CanvasRenderingContext2D,
  design: JerseyDesign,
  mark: boolean,
) {
  if (mark) {
    drawPaths(ctx, PUCKLAB_MARK.viewBox, PUCKLAB_MARK.parts, 210);
    return;
  }
  const emblem = design.crest?.kind === "emblem" ? design.crest : null;
  if (!emblem) {
    // Jersey Lab's plain mark when a club has no emblem.
    ctx.fillStyle = design.trim ?? "#FFFFFF";
    ctx.beginPath();
    const r = 52;
    ctx.moveTo(CREST.x, CREST.y - r);
    ctx.lineTo(CREST.x + r * CREST_SQUASH, CREST.y);
    ctx.lineTo(CREST.x, CREST.y + r);
    ctx.lineTo(CREST.x - r * CREST_SQUASH, CREST.y);
    ctx.closePath();
    ctx.fill();
    return;
  }
  if (emblem.back) backing(ctx, emblem.back, 80);
  drawPaths(
    ctx,
    emblem.view_box,
    emblem.paths.map((d) => ({ d, fill: emblem.color })),
    emblem.back ? 110 : 150,
  );
}

/** Paints a design into the garment's atlas. Only colours and path data reach the canvas. */
export function paintJerseyAtlas(
  design: JerseyDesign,
  scale = 2,
): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = 1024 * scale;
  canvas.height = 768 * scale;
  const ctx = canvas.getContext("2d")!;
  ctx.scale(scale, scale);
  const body = design.body;
  const sleeve = design.sleeves ?? body;
  ctx.fillStyle = body;
  ctx.fillRect(0, 0, 1024, 768);
  if (design.yoke) {
    ctx.fillStyle = design.yoke;
    ctx.fillRect(BODY.x, BODY.y, BODY.w, YOKE_DEPTH);
  }
  bands(ctx, BODY, BODY.y + BODY.h - 30, 11, design.hem_stripes, body);
  crest(ctx, design, design === DEFAULT_JERSEY);
  ctx.fillStyle = sleeve;
  ctx.fillRect(SLEEVE.x, SLEEVE.y, SLEEVE.w, SLEEVE.h);
  bands(
    ctx,
    SLEEVE,
    SLEEVE.y + SLEEVE.h - (140 * SLEEVE.h) / 300,
    8,
    design.sleeve_stripes,
    sleeve,
  );
  ctx.fillStyle = design.collar ?? design.yoke ?? body;
  ctx.fillRect(COLLAR.x, COLLAR.y, COLLAR.w, COLLAR.h);
  return canvas;
}

const atlases = new Map<
  string,
  { texture: THREE.CanvasTexture; users: number }
>();

/** A shared texture per design; call the returned release when the wearer goes. */
export function jerseyAtlas(design: JerseyDesign): {
  texture: THREE.Texture;
  release: () => void;
} {
  const key = design === DEFAULT_JERSEY ? "default" : JSON.stringify(design);
  let entry = atlases.get(key);
  if (!entry) {
    const texture = new THREE.CanvasTexture(paintJerseyAtlas(design));
    texture.flipY = false;
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = 8;
    entry = { texture, users: 0 };
    atlases.set(key, entry);
  }
  entry.users++;
  const held = entry;
  let released = false;
  return {
    texture: held.texture,
    release: () => {
      if (released) return;
      released = true;
      if (--held.users > 0) return;
      held.texture.dispose();
      atlases.delete(key);
    },
  };
}

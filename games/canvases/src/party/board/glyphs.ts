import * as THREE from "three";
import type { SpaceKind } from "../protocol";

/**
 * A symbol painted on top of each space, so what a space does reads at a
 * glance. Shapes are Lucide icons (ISC licence), drawn on a 24 unit grid.
 */

export type Shape = { d: string } | { cx: number; cy: number; r: number };

/** Canvas paths for an icon's shapes, on its 24 unit grid. */
export function iconPaths(shapes: Shape[]) {
  return shapes.map((shape) => {
    if ("d" in shape) return new Path2D(shape.d);
    const circle = new Path2D();
    circle.arc(shape.cx, shape.cy, shape.r, 0, Math.PI * 2);
    return circle;
  });
}

const ICONS: Record<SpaceKind, Shape[]> = {
  start: [
    {
      d: "M4 22V4a1 1 0 0 1 .4-.8A6 6 0 0 1 8 2c3 0 5 2 7.333 2q2 0 3.067-.8A1 1 0 0 1 20 4v10a1 1 0 0 1-.4.8A6 6 0 0 1 16 16c-3 0-5-2-8-2a6 6 0 0 0-4 1.528",
    },
  ],
  coin: [{ cx: 12, cy: 12, r: 10 }, { d: "M8 12h8" }, { d: "M12 8v8" }],
  loss: [{ cx: 12, cy: 12, r: 10 }, { d: "M8 12h8" }],
  item: [
    {
      d: "M3 9a1 1 0 0 1 1-1h16a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z",
    },
    { d: "M12 8v13" },
    { d: "M19 12v7a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-7" },
    {
      d: "M7.5 8a2.5 2.5 0 0 1 0-5A4.8 8 0 0 1 12 8a4.8 8 0 0 1 4.5-5 2.5 2.5 0 0 1 0 5",
    },
  ],
  event: [
    { d: "M11 15h2a2 2 0 1 0 0-4h-3c-.6 0-1.1.2-1.4.6L3 17" },
    {
      d: "m7 21 1.6-1.4c.3-.4.8-.6 1.4-.6h4c1.1 0 2.1-.4 2.8-1.2l4.6-4.4a2 2 0 0 0-2.75-2.91l-4.2 3.9",
    },
    { d: "m2 16 6 6" },
    { cx: 16, cy: 9, r: 2.9 },
    { cx: 6, cy: 5, r: 3 },
  ],
  shop: [
    { d: "M15 21v-5a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v5" },
    {
      d: "M17.774 10.31a1.12 1.12 0 0 0-1.549 0 2.5 2.5 0 0 1-3.451 0 1.12 1.12 0 0 0-1.548 0 2.5 2.5 0 0 1-3.452 0 1.12 1.12 0 0 0-1.549 0 2.5 2.5 0 0 1-3.77-3.248l2.889-4.184A2 2 0 0 1 7 2h10a2 2 0 0 1 1.653.873l2.895 4.192a2.5 2.5 0 0 1-3.774 3.244",
    },
    { d: "M4 10.95V19a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8.05" },
  ],
  hazard: [
    { d: "m12.5 17-.5-1-.5 1h1z" },
    {
      d: "M15 22a1 1 0 0 0 1-1v-1a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20v1a1 1 0 0 0 1 1z",
    },
    { cx: 15, cy: 12, r: 1 },
    { cx: 9, cy: 12, r: 1 },
  ],
  teleport: [
    { d: "M20.341 6.484A10 10 0 0 1 10.266 21.85" },
    { d: "M3.659 17.516A10 10 0 0 1 13.74 2.152" },
    { cx: 12, cy: 12, r: 3 },
    { cx: 19, cy: 5, r: 2 },
    { cx: 5, cy: 19, r: 2 },
  ],
  prize_spot: [
    {
      d: "M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",
    },
  ],
  cannon: [
    {
      d: "M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z",
    },
    {
      d: "m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z",
    },
    { d: "M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0" },
    { d: "M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5" },
  ],
  tide: [
    {
      d: "M2 6c.6.5 1.2 1 2.5 1C7 7 7 5 9.5 5c2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1",
    },
    {
      d: "M2 12c.6.5 1.2 1 2.5 1 2.5 0 2.5-2 5-2 2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1",
    },
    {
      d: "M2 18c.6.5 1.2 1 2.5 1 2.5 0 2.5-2 5-2 2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1",
    },
  ],
  ice: [
    { d: "m10 20-1.25-2.5L6 18" },
    { d: "M10 4 8.75 6.5 6 6" },
    { d: "m14 20 1.25-2.5L18 18" },
    { d: "m14 4 1.25 2.5L18 6" },
    { d: "m17 21-3-6h-4" },
    { d: "m17 3-3 6 1.5 3" },
    { d: "M2 12h6.5L10 9" },
    { d: "m20 10-1.5 2 1.5 2" },
    { d: "M22 12h-6.5L14 15" },
    { d: "m4 10 1.5 2L4 14" },
    { d: "m7 21 3-6-1.5-3" },
    { d: "m7 3 3 6h4" },
  ],
};

/** Pale spaces carry a dark symbol; everything else a white one. */
const DARK_INK = new Set<SpaceKind>(["prize_spot", "ice", "start"]);

const SIZE = 256;
const textures = new Map<SpaceKind, THREE.CanvasTexture>();

function draw(kind: SpaceKind) {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = SIZE;
  const ctx = canvas.getContext("2d")!;
  // The icon fills the middle of the tile with room to spare at the rim.
  const scale = (SIZE * 0.7) / 24;
  ctx.translate(SIZE * 0.15, SIZE * 0.15);
  ctx.scale(scale, scale);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  const paths = iconPaths(ICONS[kind]);
  const dark = DARK_INK.has(kind);
  // A soft halo first, so the symbol holds up against any tile colour.
  ctx.strokeStyle = dark ? "rgba(255,255,255,0.55)" : "rgba(0,0,0,0.35)";
  ctx.lineWidth = 5;
  for (const path of paths) ctx.stroke(path);
  ctx.strokeStyle = dark ? "#1f2937" : "#ffffff";
  ctx.lineWidth = 3;
  for (const path of paths) ctx.stroke(path);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = 4;
  return texture;
}

/** The symbol for a kind of space, shared by every space of that kind. */
export function glyphTexture(kind: SpaceKind) {
  let texture = textures.get(kind);
  if (!texture) {
    texture = draw(kind);
    textures.set(kind, texture);
  }
  return texture;
}

export function disposeGlyphs() {
  for (const texture of textures.values()) texture.dispose();
  textures.clear();
}

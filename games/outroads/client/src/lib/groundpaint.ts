/**
 * Paints a tile's land use, parks and water into a texture for its ground
 * grid, so the colours follow hills without being cut into the mesh.
 */

import { polygons, type Layer } from './mvt';
import { AREAS, GROUND } from './style';

type Ctx = OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D;

const css = (c: readonly number[]) => `rgb(${c[0]},${c[1]},${c[2]})`;
const str = (v: unknown) => (typeof v === 'string' ? v : '');

function fill(ctx: Ctx, layer: Layer | undefined, px: number, colorOf: (p: Record<string, unknown>) => readonly number[] | undefined) {
  if (!layer) return;
  const k = px / layer.extent;
  for (const f of layer.features) {
    if (f.type !== 3) continue;
    const color = colorOf(f.props);
    if (!color) continue;
    ctx.fillStyle = css(color);
    for (const poly of polygons(f.parts)) {
      ctx.beginPath();
      for (const ring of [poly.outer, ...poly.holes]) {
        for (let i = 0; i + 1 < ring.length; i += 2) {
          if (i === 0) ctx.moveTo(ring[i] * k, ring[i + 1] * k);
          else ctx.lineTo(ring[i] * k, ring[i + 1] * k);
        }
        ctx.closePath();
      }
      ctx.fill('evenodd');
    }
  }
}

/** RGBA pixels, north row first, or null where workers cannot draw. */
export function paintGround(layers: Map<string, Layer>, px: number): Uint8ClampedArray | null {
  if (typeof OffscreenCanvas === 'undefined') return null;
  const canvas = new OffscreenCanvas(px, px);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return null;
  ctx.fillStyle = css(GROUND);
  ctx.fillRect(0, 0, px, px);
  fill(ctx, layers.get('landuse'), px, (p) => AREAS[str(p.class)]);
  fill(ctx, layers.get('landcover'), px, (p) => AREAS[str(p.subclass)] ?? AREAS[str(p.class)]);
  fill(ctx, layers.get('park'), px, () => AREAS.park);
  fill(ctx, layers.get('water'), px, (p) => (str(p.brunnel) === 'tunnel' ? undefined : AREAS.water));
  return ctx.getImageData(0, 0, px, px).data;
}

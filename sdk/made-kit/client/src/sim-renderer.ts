import type { Shape, SimColor, SimWorld } from './contract';

/** Palette names to colours for the theme the page is in. Chart colours and the brand come from the Tawks theme. */
export type SimPalette = Record<SimColor, string> & { background: string; grid: string };

const FIXED = {
  light: { teal: '#0f9a9a', sky: '#d6e9fb', earth: '#b08457', water: '#6fb6e9', surface: '#d9d9d9' },
  dark: { teal: '#2bb5b5', sky: '#1a3048', earth: '#7b5a3b', water: '#2f78aa', surface: '#3a3a3a' },
};

function cssVar(el: Element, name: string, fallback: string) {
  const value = getComputedStyle(el).getPropertyValue(name).trim();
  return value || fallback;
}

export function readPalette(el: Element): SimPalette {
  const dark = !!el.closest('.dark') || document.documentElement.classList.contains('dark');
  const fixed = dark ? FIXED.dark : FIXED.light;
  const ink = cssVar(el, '--color-foreground', dark ? '#fafafa' : '#0a0a0a');
  return {
    ink,
    muted: cssVar(el, '--color-muted-foreground', dark ? '#a3a3a3' : '#737373'),
    faint: dark ? 'rgba(255,255,255,0.16)' : 'rgba(0,0,0,0.14)',
    surface: fixed.surface,
    accent: cssVar(el, '--color-primary', '#ff66a3'),
    blue: cssVar(el, '--chart-1', '#2a78d6'),
    orange: cssVar(el, '--chart-2', '#eb6834'),
    green: cssVar(el, '--chart-3', '#1baf7a'),
    yellow: cssVar(el, '--chart-4', '#eda100'),
    pink: cssVar(el, '--chart-5', '#e87ba4'),
    purple: cssVar(el, '--chart-7', '#6250d6'),
    red: cssVar(el, '--chart-8', '#e34948'),
    teal: fixed.teal,
    sky: fixed.sky,
    earth: fixed.earth,
    water: fixed.water,
    background: cssVar(el, '--color-card', dark ? '#1a1a1a' : '#ffffff'),
    grid: dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)',
  };
}

/** World units to canvas pixels: the world box fitted inside the canvas, centred, y up. */
export class WorldView {
  scale = 1;
  ox = 0;
  oy = 0;
  constructor(
    public world: SimWorld,
    public width: number,
    public height: number,
    pad = 8,
  ) {
    const wx = world.x[1] - world.x[0];
    const wy = world.y[1] - world.y[0];
    this.scale = Math.max(1e-9, Math.min((width - pad * 2) / wx, (height - pad * 2) / wy));
    this.ox = (width - wx * this.scale) / 2 - world.x[0] * this.scale;
    this.oy = (height + wy * this.scale) / 2 + world.y[0] * this.scale;
  }
  px(x: number) {
    return this.ox + x * this.scale;
  }
  py(y: number) {
    return this.oy - y * this.scale;
  }
  wx(px: number) {
    return (px - this.ox) / this.scale;
  }
  wy(py: number) {
    return (this.oy - py) / this.scale;
  }
  /** The canvas rectangle the world box fills. */
  box() {
    const x = this.px(this.world.x[0]);
    const y = this.py(this.world.y[1]);
    return { x, y, w: this.px(this.world.x[1]) - x, h: this.py(this.world.y[0]) - y };
  }
}

const FONT = "'Noto Sans Variable', 'Noto Sans', sans-serif";

function withAlpha(ctx: CanvasRenderingContext2D, alpha: number | null | undefined, draw: () => void) {
  if (alpha === null || alpha === undefined || alpha >= 1) return draw();
  ctx.save();
  ctx.globalAlpha = Math.max(0, alpha);
  draw();
  ctx.restore();
}

function label(ctx: CanvasRenderingContext2D, pal: SimPalette, text: string, x: number, y: number, align: CanvasTextAlign = 'left') {
  ctx.font = `500 12px ${FONT}`;
  ctx.textAlign = align;
  ctx.textBaseline = 'middle';
  ctx.lineWidth = 3;
  ctx.strokeStyle = pal.background;
  ctx.strokeText(text, x, y);
  ctx.fillStyle = pal.muted;
  ctx.fillText(text, x, y);
}

function arrowHead(ctx: CanvasRenderingContext2D, x1: number, y1: number, x2: number, y2: number, size: number) {
  const a = Math.atan2(y2 - y1, x2 - x1);
  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(x2 - size * Math.cos(a - 0.45), y2 - size * Math.sin(a - 0.45));
  ctx.lineTo(x2 - size * Math.cos(a + 0.45), y2 - size * Math.sin(a + 0.45));
  ctx.closePath();
  ctx.fill();
}

export function drawShape(ctx: CanvasRenderingContext2D, v: WorldView, pal: SimPalette, s: Shape, hovered: boolean) {
  const stroke = s.color ? pal[s.color] : null;
  const fill = s.fill ? pal[s.fill] : null;
  const width = s.width ?? 2;
  ctx.setLineDash(s.dash ? [6, 4] : []);
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  withAlpha(ctx, s.alpha, () => {
    const x = v.px(s.x);
    const y = v.py(s.y);
    switch (s.kind) {
      case 'circle': {
        const r = Math.max(1.5, (s.r ?? 0) * v.scale);
        ctx.beginPath();
        ctx.arc(x, y, r, 0, Math.PI * 2);
        if (fill) {
          ctx.fillStyle = fill;
          ctx.fill();
        }
        if (stroke) {
          ctx.strokeStyle = stroke;
          ctx.lineWidth = width;
          ctx.stroke();
        }
        if (s.label) label(ctx, pal, s.label, x + r + 6, y - r * 0.6);
        break;
      }
      case 'rect': {
        const w = (s.w ?? 0) * v.scale;
        const h = (s.h ?? 0) * v.scale;
        ctx.save();
        ctx.translate(x, y);
        if (s.angle) ctx.rotate(-s.angle);
        if (fill) {
          ctx.fillStyle = fill;
          ctx.fillRect(-w / 2, -h / 2, w, h);
        }
        if (stroke) {
          ctx.strokeStyle = stroke;
          ctx.lineWidth = width;
          ctx.strokeRect(-w / 2, -h / 2, w, h);
        }
        ctx.restore();
        if (s.label) label(ctx, pal, s.label, x + w / 2 + 6, y);
        break;
      }
      case 'line': {
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(v.px(s.x2 ?? s.x), v.py(s.y2 ?? s.y));
        ctx.strokeStyle = stroke ?? pal.ink;
        ctx.lineWidth = width;
        ctx.stroke();
        if (s.label) label(ctx, pal, s.label, v.px(s.x2 ?? s.x) + 6, v.py(s.y2 ?? s.y));
        break;
      }
      case 'path': {
        const pts = s.points ?? [];
        if (pts.length < 2) break;
        ctx.beginPath();
        ctx.moveTo(v.px(pts[0][0]), v.py(pts[0][1]));
        for (let i = 1; i < pts.length; i++) ctx.lineTo(v.px(pts[i][0]), v.py(pts[i][1]));
        if (s.closed) ctx.closePath();
        if (fill && s.closed) {
          ctx.fillStyle = fill;
          ctx.fill();
        }
        if (stroke || !fill) {
          ctx.strokeStyle = stroke ?? pal.ink;
          ctx.lineWidth = width;
          ctx.stroke();
        }
        if (s.label) {
          const last = pts[pts.length - 1];
          label(ctx, pal, s.label, v.px(last[0]) + 6, v.py(last[1]));
        }
        break;
      }
      case 'arrow': {
        const x2 = v.px(s.x2 ?? s.x);
        const y2 = v.py(s.y2 ?? s.y);
        const len = Math.hypot(x2 - x, y2 - y);
        if (len < 1) break;
        const lw = s.width ?? 3;
        const head = Math.min(len * 0.6, 8 + lw * 2.2);
        const k = (len - head * 0.8) / len;
        ctx.setLineDash([]);
        ctx.strokeStyle = stroke ?? pal.orange;
        ctx.fillStyle = stroke ?? pal.orange;
        ctx.lineWidth = lw;
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x + (x2 - x) * k, y + (y2 - y) * k);
        ctx.stroke();
        arrowHead(ctx, x, y, x2, y2, head);
        if (s.label) label(ctx, pal, s.label, x2 + (x2 >= x ? 6 : -6), y2, x2 >= x ? 'left' : 'right');
        break;
      }
      case 'spring': {
        const x2 = v.px(s.x2 ?? s.x);
        const y2 = v.py(s.y2 ?? s.y);
        const len = Math.hypot(x2 - x, y2 - y);
        if (len < 1) break;
        const coils = s.coils ?? 10;
        const amp = Math.max(3, (s.r ?? 0.1) * v.scale);
        const ux = (x2 - x) / len;
        const uy = (y2 - y) / len;
        const lead = Math.min(len * 0.1, 12);
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x + ux * lead, y + uy * lead);
        const body = len - lead * 2;
        const n = coils * 2;
        for (let i = 1; i <= n; i++) {
          const along = lead + (body * (i - 0.5)) / n;
          const side = (i % 2 ? 1 : -1) * amp;
          ctx.lineTo(x + ux * along - uy * side, y + uy * along + ux * side);
        }
        ctx.lineTo(x2 - ux * lead, y2 - uy * lead);
        ctx.lineTo(x2, y2);
        ctx.strokeStyle = stroke ?? pal.muted;
        ctx.lineWidth = s.width ?? 2;
        ctx.stroke();
        break;
      }
      case 'text': {
        ctx.font = `500 ${s.size ?? 13}px ${FONT}`;
        ctx.textAlign = (s.align ?? 'left') as CanvasTextAlign;
        ctx.textBaseline = 'middle';
        ctx.fillStyle = stroke ?? pal.ink;
        ctx.fillText(s.text ?? '', x, y);
        break;
      }
    }
  });
  ctx.setLineDash([]);
  if (s.drag && hovered) {
    const r = hitRadius(v, s) + 4;
    ctx.beginPath();
    ctx.arc(v.px(s.x), v.py(s.y), r, 0, Math.PI * 2);
    ctx.strokeStyle = pal.accent;
    ctx.lineWidth = 2;
    ctx.stroke();
  }
}

/** The radius, in pixels, round a shape's x, y that grabs it. At least 18 px so a finger can. */
export function hitRadius(v: WorldView, s: Shape) {
  let r = 0;
  if (s.kind === 'circle') r = (s.r ?? 0) * v.scale;
  else if (s.kind === 'rect') r = (Math.max(s.w ?? 0, s.h ?? 0) / 2) * v.scale;
  return Math.max(18, r);
}

/** The topmost draggable shape under the pixel (px, py), if any. */
export function hitShape(v: WorldView, shapes: Shape[], px: number, py: number): Shape | null {
  for (let i = shapes.length - 1; i >= 0; i--) {
    const s = shapes[i];
    if (!s.drag) continue;
    if (Math.hypot(v.px(s.x) - px, v.py(s.y) - py) <= hitRadius(v, s)) return s;
  }
  return null;
}

export function drawWorld(ctx: CanvasRenderingContext2D, v: WorldView, pal: SimPalette, world: SimWorld) {
  const box = v.box();
  ctx.fillStyle = pal.background;
  ctx.fillRect(0, 0, v.width, v.height);
  if (world.grid && world.grid > 0) {
    const g = world.grid;
    if ((g * v.scale) >= 8) {
      ctx.strokeStyle = pal.grid;
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (let x = Math.ceil(world.x[0] / g) * g; x <= world.x[1] + 1e-9; x += g) {
        const px = Math.round(v.px(x)) + 0.5;
        ctx.moveTo(px, box.y);
        ctx.lineTo(px, box.y + box.h);
      }
      for (let y = Math.ceil(world.y[0] / g) * g; y <= world.y[1] + 1e-9; y += g) {
        const py = Math.round(v.py(y)) + 0.5;
        ctx.moveTo(box.x, py);
        ctx.lineTo(box.x + box.w, py);
      }
      ctx.stroke();
    }
  }
}

/** A ruler from (x1, y1) to (x2, y2) in world units, labelled with its length. */
export type Ruler = { x1: number; y1: number; x2: number; y2: number };

export function drawRuler(ctx: CanvasRenderingContext2D, v: WorldView, pal: SimPalette, ruler: Ruler, unit: string) {
  const ax = v.px(ruler.x1);
  const ay = v.py(ruler.y1);
  const bx = v.px(ruler.x2);
  const by = v.py(ruler.y2);
  const len = Math.hypot(bx - ax, by - ay);
  if (len < 2) return;
  const ux = (bx - ax) / len;
  const uy = (by - ay) / len;
  const nx = -uy;
  const ny = ux;
  const thick = 22;
  ctx.save();
  ctx.fillStyle = pal.yellow;
  ctx.globalAlpha = 0.85;
  ctx.beginPath();
  ctx.moveTo(ax, ay);
  ctx.lineTo(bx, by);
  ctx.lineTo(bx + nx * thick, by + ny * thick);
  ctx.lineTo(ax + nx * thick, ay + ny * thick);
  ctx.closePath();
  ctx.fill();
  ctx.globalAlpha = 1;
  ctx.strokeStyle = '#1f1a10';
  ctx.lineWidth = 1;
  ctx.stroke();
  const world = Math.hypot(ruler.x2 - ruler.x1, ruler.y2 - ruler.y1);
  const step = niceStep(world / 10);
  const n = Math.floor(world / step + 1e-9);
  ctx.beginPath();
  for (let i = 0; i <= n && i <= 200; i++) {
    const at = (i * step * v.scale);
    const tick = i % 5 === 0 ? 10 : 5;
    ctx.moveTo(ax + ux * at, ay + uy * at);
    ctx.lineTo(ax + ux * at + nx * tick, ay + uy * at + ny * tick);
  }
  ctx.stroke();
  ctx.translate((ax + bx) / 2 + nx * (thick * 0.62), (ay + by) / 2 + ny * (thick * 0.62));
  let angle = Math.atan2(uy, ux);
  if (angle > Math.PI / 2 || angle < -Math.PI / 2) angle += Math.PI;
  ctx.rotate(angle);
  ctx.font = `600 12px ${FONT}`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = '#1f1a10';
  ctx.fillText(`${formatNumber(world, 3)} ${unit}`, 0, 0);
  ctx.restore();
  for (const [x, y] of [[ax, ay], [bx, by]]) {
    ctx.beginPath();
    ctx.arc(x, y, 6, 0, Math.PI * 2);
    ctx.fillStyle = pal.background;
    ctx.fill();
    ctx.strokeStyle = '#1f1a10';
    ctx.lineWidth = 2;
    ctx.stroke();
  }
}

export function niceStep(raw: number) {
  if (!(raw > 0)) return 1;
  const p = 10 ** Math.floor(Math.log10(raw));
  const m = raw / p;
  return (m < 1.5 ? 1 : m < 3.5 ? 2 : m < 7.5 ? 5 : 10) * p;
}

/** A number with about `digits` significant figures, without a trailing run of zeros. */
export function formatNumber(n: number, digits = 3) {
  if (!Number.isFinite(n)) return '';
  if (n === 0) return '0';
  const abs = Math.abs(n);
  if (abs >= 1e6 || abs < 1e-3) return n.toExponential(Math.max(0, digits - 1));
  const decimals = Math.max(0, digits - 1 - Math.floor(Math.log10(abs)));
  return n.toFixed(Math.min(6, decimals));
}

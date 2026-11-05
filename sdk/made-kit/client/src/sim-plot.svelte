<script lang="ts">
  import { onMount } from 'svelte';
  import type { PlotSpec } from './contract';
  import type { PlotLine } from './sim-scene';
  import { formatNumber, niceStep, type SimPalette } from './sim-renderer';

  /** One live plot: each series against sim time, the newest `window` seconds. */
  let { spec, line, t, palette, version }: { spec: PlotSpec; line: PlotLine | undefined; t: number; palette: SimPalette | null; version: number } =
    $props();

  let canvas: HTMLCanvasElement;
  let width = $state(0);
  let height = $state(0);
  const FONT = "'Noto Sans Variable', 'Noto Sans', sans-serif";

  onMount(() => {
    const observer = new ResizeObserver(([entry]) => {
      width = Math.round(entry.contentRect.width);
      height = Math.round(entry.contentRect.height);
    });
    observer.observe(canvas.parentElement!);
    return () => observer.disconnect();
  });

  $effect(() => {
    void version;
    draw(line, t, palette, width, height);
  });

  function draw(line: PlotLine | undefined, now: number, pal: SimPalette | null, w: number, h: number) {
    if (!canvas || !pal || w < 10 || h < 10) return;
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.round(w * ratio);
    canvas.height = Math.round(h * ratio);
    const ctx = canvas.getContext('2d')!;
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const left = 44;
    const right = 8;
    const top = 6;
    const bottom = 20;
    const pw = w - left - right;
    const ph = h - top - bottom;
    const t1 = Math.max(spec.window, now);
    const t0 = t1 - spec.window;
    const points = (line?.points ?? []).filter((p) => p[0] >= t0 - 1e-9);
    let lo = spec.y ? spec.y[0] : Infinity;
    let hi = spec.y ? spec.y[1] : -Infinity;
    if (!spec.y) {
      for (const p of points) for (let i = 1; i < p.length; i++) {
        lo = Math.min(lo, p[i]);
        hi = Math.max(hi, p[i]);
      }
      if (!Number.isFinite(lo)) {
        lo = 0;
        hi = 1;
      }
      if (hi - lo < 1e-9) {
        const pad = Math.abs(hi) * 0.1 || 1;
        lo -= pad;
        hi += pad;
      } else {
        const pad = (hi - lo) * 0.08;
        lo -= pad;
        hi += pad;
      }
    }
    const px = (tt: number) => left + ((tt - t0) / (t1 - t0)) * pw;
    const py = (v: number) => top + ph - ((v - lo) / (hi - lo)) * ph;
    ctx.font = `11px ${FONT}`;
    ctx.fillStyle = pal.muted;
    ctx.strokeStyle = pal.faint;
    ctx.lineWidth = 1;
    const ys = niceStep((hi - lo) / 4);
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    for (let v = Math.ceil(lo / ys) * ys; v <= hi + 1e-9; v += ys) {
      const y = Math.round(py(v)) + 0.5;
      ctx.beginPath();
      ctx.moveTo(left, y);
      ctx.lineTo(left + pw, y);
      ctx.stroke();
      ctx.fillText(formatNumber(v, 3), left - 6, y);
    }
    const xs = niceStep(spec.window / 5);
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    for (let tt = Math.ceil(t0 / xs) * xs; tt <= t1 + 1e-9; tt += xs) ctx.fillText(`${formatNumber(tt, 3)} s`, px(tt), top + ph + 5);
    ctx.save();
    ctx.beginPath();
    ctx.rect(left, top, pw, ph);
    ctx.clip();
    spec.series.forEach((series, i) => {
      ctx.beginPath();
      let started = false;
      for (const p of points) {
        const x = px(p[0]);
        const y = py(p[i + 1]);
        if (started) ctx.lineTo(x, y);
        else {
          ctx.moveTo(x, y);
          started = true;
        }
      }
      ctx.strokeStyle = pal[series.color];
      ctx.lineWidth = 2;
      ctx.lineJoin = 'round';
      ctx.stroke();
    });
    ctx.restore();
  }
</script>

<figure class="space-y-1.5" data-sim-plot={spec.id}>
  <figcaption class="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs">
    <span class="text-foreground font-semibold">{spec.title}{spec.y_label ? ` (${spec.y_label})` : ''}</span>
    {#each spec.series as series (series.key)}
      <span class="text-muted-foreground inline-flex items-center gap-1.5">
        <span class="inline-block h-0.5 w-3 rounded-full" style:background={palette?.[series.color]}></span>{series.label}
      </span>
    {/each}
  </figcaption>
  <div class="relative h-32 w-full">
    <canvas bind:this={canvas} class="absolute inset-0 block h-full w-full" aria-label="{spec.title} over time"></canvas>
  </div>
</figure>

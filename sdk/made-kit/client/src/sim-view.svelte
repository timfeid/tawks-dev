<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '@tawks/ui/button/index.js';
  import { Badge } from '@tawks/ui/badge/index.js';
  import { Hand, LoaderCircle, Pause, Play, RotateCcw, Ruler as RulerIcon, StepForward, Timer } from '@lucide/svelte';
  import type { LogLine, ParamValue, Shape, SimControl, SimFrame, SimMeta, SimRenderShot, SimViewer, SimWelcome } from './contract';
  import type { MadeSource, SourceStatus } from './connection';
  import { SimScene, blendShape, type PlotLine } from './sim-scene';
  import { WorldView, drawRuler, drawShape, drawWorld, formatNumber, hitShape, readPalette, type Ruler, type SimPalette } from './sim-renderer';
  import SimControls from './sim-controls.svelte';
  import SimPlot from './sim-plot.svelte';

  /**
   * An interactive simulation: the canvas, the transport, the controls, the
   * readouts and the plots. Live from the relay (`source`), or one recorded
   * frame for a screenshot (`still`).
   */
  let {
    source = null,
    still = null,
    onwelcome,
    onlog,
    onready,
  }: {
    source?: MadeSource | null;
    still?: SimRenderShot | null;
    onwelcome?: (welcome: SimWelcome) => void;
    onlog?: (line: LogLine) => void;
    /** Called once the first frame is drawn. */
    onready?: () => void;
  } = $props();

  const SPEEDS = [0.25, 0.5, 1, 2, 4];
  const BLEND_MS = 50;
  const GRAB_GAP_MS = 33;

  let root: HTMLDivElement;
  let stage: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let status = $state<SourceStatus>('connecting');
  let meta = $state<SimMeta | null>(null);
  let frame = $state<SimFrame | null>(null);
  let plots = $state<Map<string, PlotLine>>(new Map());
  let plotVersion = $state(0);
  let me = $state<string | null>(null);
  let controller = $state<SimViewer | null>(null);
  let viewers = $state<SimViewer[]>([]);
  let error = $state<string | null>(null);
  let palette = $state<SimPalette | null>(null);
  let rulerOn = $state(false);
  let watchOn = $state(false);
  let ruler = $state<Ruler | null>(null);
  let watch = $state({ running: false, from: 0, held: 0, x: 16, y: 16 });
  let hovering = $state(false);

  const driving = $derived(!still && !!me && controller?.user_id === me);
  /** On a narrow screen the canvas is as tall as the world needs and the panel takes the rest. */
  const aspect = $derived(meta ? String((meta.world.x[1] - meta.world.x[0]) / (meta.world.y[1] - meta.world.y[0])) : '16 / 9');
  const driverName = $derived(controller?.name ?? null);
  const watchTime = $derived(watch.running && frame ? watch.held + Math.max(0, frame.t - watch.from) : watch.held);

  function send(control: SimControl) {
    if (!driving || !source) return false;
    return source.send({ type: 'sim', control });
  }

  onMount(() => {
    const scene = new SimScene();
    let from = new Map<string, Shape>();
    let to: Shape[] = [];
    let arrived = 0;
    let shown: Shape[] = [];
    let view: WorldView | null = null;
    let grab: { id: string; sentAt: number; last: { x: number; y: number } | null } | null = null;
    let rulerDrag: { part: 'a' | 'b' | 'body'; x: number; y: number } | null = null;
    let readyCalled = false;
    let hoverId: string | null = null;

    palette = readPalette(root);
    const themeWatch = new MutationObserver(() => (palette = readPalette(root)));
    themeWatch.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });

    const take = (next: SimFrame) => {
      from = new Map(shown.map((s) => [s.id, s]));
      to = next.shapes;
      arrived = performance.now();
      frame = next;
      plots = scene.plots;
      plotVersion += 1;
      if (watch.running && next.t + 1e-9 < watch.from) watch = { ...watch, running: false, held: 0, from: 0 };
    };

    let stopMessages = () => {};
    let stopStatus = () => {};
    if (still) {
      meta = still.meta;
      for (const chunk of still.frame.plots) scene.absorb(chunk);
      scene.frame = still.frame;
      take(still.frame);
      from = new Map();
      arrived = 0;
    } else if (source) {
      stopMessages = source.subscribe((message) => {
        switch (message.type) {
          case 'sim_welcome':
            meta = message.welcome.meta;
            me = message.welcome.user_id;
            controller = message.welcome.controller;
            viewers = message.welcome.viewers;
            error = null;
            scene.reset();
            onwelcome?.(message.welcome);
            break;
          case 'controller':
            controller = message.controller;
            viewers = message.viewers;
            break;
          case 'sim_frame':
            if (scene.push(message.frame)) take(scene.frame!);
            else source!.send({ type: 'resync' });
            break;
          case 'log':
            onlog?.(message.line);
            break;
          case 'error':
            if (!/^[a-z_]+$/.test(message.message)) {
              const shownError = message.message;
              error = shownError;
              setTimeout(() => error === shownError && (error = null), 4000);
            }
            break;
        }
      });
      stopStatus = source.onStatus((next) => (status = next));
    }

    let width = 0;
    let height = 0;
    // Resizing clears the canvas, so it is drawn again straight away.
    const resize = () => {
      const rect = stage.getBoundingClientRect();
      width = Math.max(1, Math.round(rect.width));
      height = Math.max(1, Math.round(rect.height));
      const ratio = still ? 1 : window.devicePixelRatio || 1;
      canvas.width = Math.round(width * ratio);
      canvas.height = Math.round(height * ratio);
      draw();
    };

    const draw = () => {
      const ctx = canvas.getContext('2d');
      if (!ctx || !meta || !palette) return;
      const ratio = canvas.width / width;
      ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
      view = new WorldView(meta.world, width, height);
      drawWorld(ctx, view, palette, meta.world);
      const k = Math.min(1, (performance.now() - arrived) / BLEND_MS);
      shown = to.map((s) => blendShape(from.get(s.id), s, k));
      for (const s of shown) drawShape(ctx, view, palette, s, driving && s.id === hoverId);
      const handles = shown.filter((s) => s.drag).map((s) => `${s.id}@${Math.round(view!.px(s.x))},${Math.round(view!.py(s.y))}`).join(' ');
      if (canvas.dataset.handles !== handles) canvas.dataset.handles = handles;
      if (rulerOn && ruler) drawRuler(ctx, view, palette, ruler, meta.world.unit);
      if (!readyCalled && frame) {
        readyCalled = true;
        onready?.();
      }
    };

    const observer = new ResizeObserver(resize);
    observer.observe(stage);
    resize();

    let raf = 0;
    const loop = () => {
      raf = requestAnimationFrame(loop);
      draw();
    };
    if (still) {
      requestAnimationFrame(() => draw());
    } else raf = requestAnimationFrame(loop);

    const local = (event: PointerEvent) => {
      const rect = canvas.getBoundingClientRect();
      return { px: event.clientX - rect.left, py: event.clientY - rect.top };
    };
    const rulerPart = (px: number, py: number): 'a' | 'b' | 'body' | null => {
      if (!rulerOn || !ruler || !view) return null;
      const ax = view.px(ruler.x1);
      const ay = view.py(ruler.y1);
      const bx = view.px(ruler.x2);
      const by = view.py(ruler.y2);
      if (Math.hypot(px - ax, py - ay) < 16) return 'a';
      if (Math.hypot(px - bx, py - by) < 16) return 'b';
      const len = Math.hypot(bx - ax, by - ay);
      const ux = (bx - ax) / len;
      const uy = (by - ay) / len;
      const along = (px - ax) * ux + (py - ay) * uy;
      const across = -(px - ax) * uy + (py - ay) * ux;
      return along >= 0 && along <= len && across >= -6 && across <= 28 ? 'body' : null;
    };
    const down = (event: PointerEvent) => {
      if (!view) return;
      const { px, py } = local(event);
      const part = rulerPart(px, py);
      if (part) {
        rulerDrag = { part, x: view.wx(px), y: view.wy(py) };
        canvas.setPointerCapture(event.pointerId);
        event.preventDefault();
        return;
      }
      if (!driving) return;
      const hit = hitShape(view, shown, px, py);
      if (!hit) return;
      canvas.setPointerCapture(event.pointerId);
      event.preventDefault();
      grab = { id: hit.id, sentAt: performance.now(), last: null };
      send({ do: 'grab', id: hit.id, x: view.wx(px), y: view.wy(py), phase: 'start' });
    };
    const move = (event: PointerEvent) => {
      if (!view) return;
      const { px, py } = local(event);
      const x = view.wx(px);
      const y = view.wy(py);
      if (rulerDrag && ruler) {
        const dx = x - rulerDrag.x;
        const dy = y - rulerDrag.y;
        rulerDrag = { ...rulerDrag, x, y };
        if (rulerDrag.part === 'a') ruler = { ...ruler, x1: x, y1: y };
        else if (rulerDrag.part === 'b') ruler = { ...ruler, x2: x, y2: y };
        else ruler = { x1: ruler.x1 + dx, y1: ruler.y1 + dy, x2: ruler.x2 + dx, y2: ruler.y2 + dy };
        return;
      }
      if (grab) {
        grab.last = { x, y };
        if (performance.now() - grab.sentAt >= GRAB_GAP_MS) {
          grab.sentAt = performance.now();
          send({ do: 'grab', id: grab.id, x, y, phase: 'move' });
          grab.last = null;
        }
        return;
      }
      const hit = driving ? hitShape(view, shown, px, py) : null;
      hoverId = hit?.id ?? null;
      hovering = !!hit || !!rulerPart(px, py);
    };
    const up = (event: PointerEvent) => {
      if (rulerDrag) {
        rulerDrag = null;
        return;
      }
      if (!grab || !view) return;
      const { px, py } = local(event);
      send({ do: 'grab', id: grab.id, x: view.wx(px), y: view.wy(py), phase: 'end' });
      grab = null;
    };
    canvas.addEventListener('pointerdown', down);
    canvas.addEventListener('pointermove', move);
    canvas.addEventListener('pointerup', up);
    canvas.addEventListener('pointercancel', up);

    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      themeWatch.disconnect();
      stopMessages();
      stopStatus();
      canvas.removeEventListener('pointerdown', down);
      canvas.removeEventListener('pointermove', move);
      canvas.removeEventListener('pointerup', up);
      canvas.removeEventListener('pointercancel', up);
    };
  });

  function toggleRuler() {
    rulerOn = !rulerOn;
    if (rulerOn && !ruler && meta) {
      const [x0, x1] = meta.world.x;
      const [y0, y1] = meta.world.y;
      const y = y0 + (y1 - y0) * 0.2;
      ruler = { x1: x0 + (x1 - x0) * 0.25, y1: y, x2: x0 + (x1 - x0) * 0.55, y2: y };
    }
  }

  function toggleWatch() {
    if (!frame) return;
    watch = watch.running ? { ...watch, running: false, held: watchTime } : { ...watch, running: true, from: frame.t };
  }

  function watchDrag(event: PointerEvent) {
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    const start = { x: event.clientX, y: event.clientY, wx: watch.x, wy: watch.y };
    const moveWatch = (e: PointerEvent) => {
      const bounds = stage.getBoundingClientRect();
      watch = {
        ...watch,
        x: Math.max(0, Math.min(bounds.width - 140, start.wx + e.clientX - start.x)),
        y: Math.max(0, Math.min(bounds.height - 80, start.wy + e.clientY - start.y)),
      };
    };
    const stop = () => {
      target.removeEventListener('pointermove', moveWatch);
      target.removeEventListener('pointerup', stop);
    };
    target.addEventListener('pointermove', moveWatch);
    target.addEventListener('pointerup', stop);
  }

  const readout = (r: { value?: number | null; text?: string | null; digits?: number | null }) =>
    r.text ?? (r.value === null || r.value === undefined ? '' : r.digits !== null && r.digits !== undefined ? r.value.toFixed(r.digits) : formatNumber(r.value, 3));
  const setParam = (key: string, value: ParamValue) => send({ do: 'set', key, value });
</script>

<div
  bind:this={root}
  class="sim-view bg-background text-foreground @container relative flex h-full min-h-0 w-full flex-col overflow-hidden"
  data-sim-view
  data-status={still ? 'still' : status}
  data-driving={driving ? '1' : '0'}
  role="application"
  aria-label={meta?.name ?? 'Simulation'}
>
  <div class="flex min-h-0 flex-1 flex-col @3xl:flex-row">
    <div class="flex min-h-0 min-w-0 flex-1 flex-col @max-3xl:flex-none">
      <div
        bind:this={stage}
        class="relative min-h-48 flex-1 @max-3xl:max-h-[60cqh] @max-3xl:flex-none @max-3xl:[aspect-ratio:var(--sim-aspect)] @3xl:min-h-0"
        style:--sim-aspect={aspect}
        data-sim-stage
      >
        <canvas
          bind:this={canvas}
          class="absolute inset-0 block h-full w-full touch-none select-none"
          class:cursor-grab={hovering}
          data-sim-canvas
        ></canvas>
        {#if frame?.notice}
          <div class="pointer-events-none absolute inset-x-0 top-3 flex justify-center px-4">
            <p class="bg-card/90 rounded-md border px-3 py-1.5 text-sm shadow-sm" data-sim-notice>{frame.notice}</p>
          </div>
        {/if}
        {#if watchOn}
          <div
            class="bg-card/95 absolute w-36 rounded-lg border p-2 shadow-md"
            style:left="{watch.x}px"
            style:top="{watch.y}px"
            data-sim-stopwatch
          >
            <div class="text-muted-foreground mb-1 flex cursor-move items-center gap-1 text-xs" onpointerdown={watchDrag} role="presentation">
              <Timer class="size-3.5" />Stopwatch
            </div>
            <p class="text-center font-mono text-xl tabular-nums">{watchTime.toFixed(2)} s</p>
            <div class="mt-1 flex gap-1">
              <Button size="sm" variant="secondary" class="flex-1" onclick={toggleWatch}>{watch.running ? 'Stop' : 'Start'}</Button>
              <Button size="sm" variant="ghost" class="flex-1" onclick={() => (watch = { ...watch, running: false, held: 0, from: frame?.t ?? 0 })}>Clear</Button>
            </div>
          </div>
        {/if}
        {#if !frame && !still}
          <div class="absolute inset-0 grid place-items-center">
            <div class="bg-card flex items-center gap-2 rounded-lg border px-4 py-2 text-sm" role="status">
              <LoaderCircle class="size-4 animate-spin" />
              {status === 'offline' ? 'Reconnecting' : 'Starting the simulation'}
            </div>
          </div>
        {/if}
        {#if error}
          <div class="absolute inset-x-0 bottom-3 flex justify-center px-4">
            <p class="bg-destructive text-destructive-foreground rounded-md px-3 py-1.5 text-sm" role="alert">{error}</p>
          </div>
        {/if}
      </div>

      <div class="bg-card flex shrink-0 flex-wrap items-center gap-1 border-t px-2 py-1.5" data-sim-transport>
        <Button
          size="icon-md"
          variant="ghost"
          disabled={!driving && !still}
          aria-label={frame?.playing ? 'Pause' : 'Play'}
          data-sim-play
          onclick={() => send(frame?.playing ? { do: 'pause' } : { do: 'play' })}
        >
          {#if frame?.playing ?? true}<Pause />{:else}<Play />{/if}
        </Button>
        <Button size="icon-md" variant="ghost" disabled={!driving && !still} aria-label="Step" data-sim-step onclick={() => send({ do: 'step' })}><StepForward /></Button>
        <Button size="icon-md" variant="ghost" disabled={!driving && !still} aria-label="Reset" data-sim-reset onclick={() => send({ do: 'reset' })}><RotateCcw /></Button>
        <div class="bg-muted ml-1 flex items-center rounded-md p-0.5" role="group" aria-label="Speed">
          {#each SPEEDS as speed (speed)}
            <button
              type="button"
              class="h-7 rounded px-2 text-xs tabular-nums transition-colors disabled:opacity-60 {frame?.speed === speed || (!frame && speed === 1)
                ? 'bg-background text-foreground font-semibold shadow-sm'
                : 'text-muted-foreground hover:text-foreground'}"
              disabled={!driving && !still}
              aria-pressed={frame?.speed === speed}
              onclick={() => send({ do: 'speed', speed })}>{speed}x</button
            >
          {/each}
        </div>
        <span class="text-muted-foreground ml-2 font-mono text-xs tabular-nums" data-sim-time>{(frame?.t ?? 0).toFixed(1)} s</span>
        <div class="ml-auto flex items-center gap-1">
          {#if meta?.tools.ruler}
            <Button size="icon-md" variant={rulerOn ? 'secondary' : 'ghost'} aria-label="Ruler" aria-pressed={rulerOn} data-sim-ruler onclick={toggleRuler}><RulerIcon /></Button>
          {/if}
          {#if meta?.tools.stopwatch}
            <Button size="icon-md" variant={watchOn ? 'secondary' : 'ghost'} aria-label="Stopwatch" aria-pressed={watchOn} data-sim-watch onclick={() => (watchOn = !watchOn)}><Timer /></Button>
          {/if}
        </div>
      </div>
    </div>

    <aside class="bg-card min-h-0 flex-1 overflow-y-auto border-t @3xl:w-80 @3xl:flex-none @3xl:border-t-0 @3xl:border-l" aria-label="Controls" data-sim-panel>
      <div class="space-y-5 p-4">
        {#if !still}
          <div class="flex items-center justify-between gap-2" data-sim-controller>
            {#if driving}
              <Badge variant="success" class="gap-1.5"><Hand class="size-3.5" />You have control</Badge>
              {#if viewers.length > 1}<span class="text-muted-foreground text-xs">{viewers.length} here</span>{/if}
            {:else}
              <span class="text-muted-foreground min-w-0 truncate text-sm">{driverName ? `${driverName} has control` : 'Nobody has control'}</span>
              <Button size="sm" variant="secondary" data-sim-take onclick={() => source?.send({ type: 'take_control' })}><Hand />Take control</Button>
            {/if}
          </div>
        {/if}
        {#if meta}
          <section class="space-y-1">
            <h2 class="text-base font-semibold">{meta.name}</h2>
            <p class="text-muted-foreground text-sm">{meta.about}</p>
          </section>
          {#if meta.params.length}
            <SimControls params={meta.params} values={frame?.params ?? {}} disabled={!driving && !still} onset={setParam} />
          {/if}
          {#if frame?.readouts.length}
            <dl class="grid grid-cols-2 gap-2" data-sim-readouts>
              {#each frame.readouts as r (r.label)}
                <div class="bg-muted/60 rounded-md px-2.5 py-1.5">
                  <dt class="text-muted-foreground truncate text-xs">{r.label}</dt>
                  <dd class="font-mono text-sm tabular-nums" style:color={r.color && palette ? palette[r.color] : undefined}>
                    {readout(r)}{r.unit ? ` ${r.unit}` : ''}
                  </dd>
                </div>
              {/each}
            </dl>
          {/if}
          {#each meta.plots as spec (spec.id)}
            <SimPlot {spec} line={plots.get(spec.id)} t={frame?.t ?? 0} {palette} version={plotVersion} />
          {/each}
          <p class="text-muted-foreground border-t pt-3 text-xs leading-relaxed">{meta.explain}</p>
        {/if}
      </div>
    </aside>
  </div>
</div>

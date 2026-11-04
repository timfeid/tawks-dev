<script lang="ts">
  import { onMount } from 'svelte';
  import type { Hud, Meta, Placements, RenderShot, SceneFrame, Seat } from '../src/contract';
  import { MadeRenderer } from '../src/renderer';
  import { toState } from '../src/scene-buffer';
  import MadeHud from '../src/made-hud.svelte';
  import MadePodium from '../src/made-podium.svelte';

  type Request = { frame: SceneFrame; roster: Seat[]; meta: Meta | null; seat: number | null; placements?: Placements | null };

  let canvas: HTMLCanvasElement;
  let hud = $state<Hud | null>(null);
  let meta = $state<Meta | null>(null);
  let roster = $state<Seat[]>([]);
  let mySeat = $state<number | null>(null);
  let placements = $state<Placements | null>(null);
  let renderer: MadeRenderer | null = null;

  const nextPaint = () => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));

  onMount(() => {
    renderer = new MadeRenderer(canvas, { preserve: true, pixelRatio: 1 });
    renderer.resize(window.innerWidth, window.innerHeight);
    const show = async (request: Request) => {
      const view = renderer!;
      view.resize(window.innerWidth, window.innerHeight);
      view.setRoster(request.roster);
      view.mySeat = request.seat;
      roster = request.roster;
      meta = request.meta;
      mySeat = request.seat;
      hud = request.frame.hud;
      placements = request.placements ?? null;
      view.showPodium(placements);
      view.recamera(true);
      const state = toState(request.frame);
      const sample = { state, entities: [...state.entities.values()], speeds: new Map<string, number>() };
      for (let i = 0; i < 3; i++) view.draw(sample, request.frame.t + i / 60, 1 / 60);
      await view.settle();
      view.draw(sample, request.frame.t + 0.04, 1 / 60);
      await nextPaint();
      view.draw(sample, request.frame.t + 0.05, 1 / 60);
      return view.measure();
    };
    const page = window as unknown as { madeShow: typeof show; __madeReady?: boolean; __madeError?: string; __madeStats?: unknown };
    page.madeShow = show;
    // The page contract in made-protocol's simd.rs: `?shot=<url>` draws that one RenderShot.
    const shotUrl = new URLSearchParams(location.search).get('shot');
    if (shotUrl) {
      fetch(shotUrl)
        .then((response) => (response.ok ? (response.json() as Promise<RenderShot>) : Promise.reject(new Error(`the shot is ${response.status}`))))
        .then((shot) => show({ frame: shot.frame, roster: shot.roster, meta: null, seat: null }))
        .then((stats) => {
          page.__madeStats = stats;
          page.__madeReady = true;
        })
        .catch((error: unknown) => (page.__madeError = String((error as Error)?.message ?? error)));
    }
    document.body.dataset.ready = '1';
  });
</script>

<div class="relative h-screen w-screen overflow-hidden">
  <canvas bind:this={canvas} class="absolute inset-0 block h-full w-full"></canvas>
  <MadeHud {hud} {meta} {roster} {mySeat} status="online" colorOf={(seat) => renderer?.seatColor(seat) ?? '#ffd23f'} showBrief={false} ended={!!placements} muted={false} fullscreen={false} ontogglemute={() => {}} />
  {#if placements}
    <MadePodium {placements} {roster} {mySeat} colorOf={(seat) => renderer?.seatColor(seat) ?? '#ffd23f'} />
  {/if}
</div>

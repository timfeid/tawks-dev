<script lang="ts">
  import { Map as MapIcon, ChevronDown, ChevronUp, Plus, Minus } from '@lucide/svelte';
  import geography from './north-america.json';
  import { MAP_WIDTH, MAP_HEIGHT, project } from './geography';
  import type { Game } from './types';

  let { game, footprint, center, playerColor, navigate, zoom, highlighted }: {
    game: Game;
    footprint: string;
    center: { x: number; z: number };
    playerColor: (id: string) => string;
    navigate: (x: number, z: number) => void;
    zoom: (factor: number) => void;
    highlighted: Set<number>;
  } = $props();
  let collapsed = $state(false);
  let surface = $state<HTMLButtonElement>();
  let dragging: number | null = null;
  const land = geography.land.flatMap(country => country.polygons).map(rings => rings.map(ring =>
    ring.map(([lon, lat], i) => { const p = project(lon, lat); return `${i ? 'L' : 'M'}${p.x},${p.z}`; }).join(' ') + 'Z'
  ).join(' '));
  const cities = $derived(new Map(game.cities.map(city => [city.id, project(city.longitude, city.latitude)])));
  const visibleFootprint = $derived.by(() => {
    // Clip the camera polygon to the board, so the outline is still visible
    // when the camera is zoomed out far enough to contain the whole continent.
    let points = footprint ? footprint.split(' ').map(point => point.split(',').map(Number)) : [];
    for (const [axis, edge, direction] of [[0, -MAP_WIDTH / 2, 1], [0, MAP_WIDTH / 2, -1], [1, -MAP_HEIGHT / 2, 1], [1, MAP_HEIGHT / 2, -1]]) {
      const clipped: number[][] = [];
      for (let i = 0; i < points.length; i++) {
        const a = points[i], b = points[(i + 1) % points.length];
        const insideA = (a[axis] - edge) * direction >= 0;
        const insideB = (b[axis] - edge) * direction >= 0;
        if (insideA) clipped.push(a);
        if (insideA !== insideB) {
          const t = (edge - a[axis]) / (b[axis] - a[axis]);
          clipped.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
      }
      points = clipped;
    }
    return points.map(point => point.join(',')).join(' ');
  });
  function go(event: PointerEvent) {
    if (!surface) return;
    const rect = surface.getBoundingClientRect();
    navigate((Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)) - 0.5) * MAP_WIDTH,
      (Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height)) - 0.5) * MAP_HEIGHT);
  }
  function keydown(event: KeyboardEvent) {
    const steps: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    const step = steps[event.key];
    if (!step) return;
    event.preventDefault();
    navigate(center.x + step[0], center.z + step[1]);
  }
</script>

<aside class="minimap" aria-label="Minimap">
  <button class="toggle" onclick={() => collapsed = !collapsed} aria-expanded={!collapsed} aria-label={collapsed ? 'Show minimap' : 'Hide minimap'}>
    <MapIcon size={13} /><span>OVERVIEW</span>{#if collapsed}<ChevronDown size={14} />{:else}<ChevronUp size={14} />{/if}
  </button>
  {#if !collapsed}
    <button bind:this={surface} class="surface" aria-label="Map navigator. Tap or drag to pan. Use arrow keys to move."
      onkeydown={keydown}
      onclick={event => { if (event.detail === 0) navigate(0, 0); }}
      onpointerdown={event => { if (event.button !== 0) return; dragging = event.pointerId; surface?.setPointerCapture(event.pointerId); go(event); }}
      onpointermove={event => { if (dragging === event.pointerId) go(event); }}
      onpointerup={() => dragging = null} onpointercancel={() => dragging = null} onlostpointercapture={() => dragging = null}>
      <svg viewBox="{-MAP_WIDTH / 2} {-MAP_HEIGHT / 2} {MAP_WIDTH} {MAP_HEIGHT}" aria-hidden="true">
        {#each land as d}<path {d} fill="#6b8778" stroke="#a6bb981f" stroke-width="0.04" fill-rule="evenodd" />{/each}
        {#each game.routes as route (route.id)}
          {@const from = cities.get(route.from)}{@const to = cities.get(route.to)}
          {#if from && to}<line x1={from.x} y1={from.z} x2={to.x} y2={to.z} stroke={highlighted.has(route.id) ? '#ffe2a1' : route.owner ? playerColor(route.owner) : '#d0d9be55'} stroke-width={route.owner || highlighted.has(route.id) ? 0.13 : 0.055} />{/if}
        {/each}
        {#each [...cities.values()] as city}<circle cx={city.x} cy={city.z} r="0.09" fill="#f0e7c9" />{/each}
        <polygon points={visibleFootprint} fill="#ffe1a51a" stroke="#ffe1a5" stroke-width="0.18" stroke-linejoin="round" />
        <circle cx={center.x} cy={center.z} r="0.23" fill="#ffe1a5" stroke="#10252a" stroke-width="0.1" />
      </svg>
    </button>
    <div class="zoom"><span>Tap to travel</span><button onclick={() => zoom(1.3)} aria-label="Zoom out"><Minus size={15} /></button><button onclick={() => zoom(1 / 1.3)} aria-label="Zoom in"><Plus size={15} /></button></div>
  {/if}
</aside>

<style>
  .minimap { position:absolute; left:20px; bottom:180px; width:176px; z-index:2; color:#dce6d7; background:#10252af2; border:1px solid #a4b59940; border-radius:7px; overflow:hidden; box-shadow:0 6px 20px #0003; }
  button { cursor:pointer; color:inherit; background:transparent; border:0; }button:focus-visible,.surface:focus-visible { outline:2px solid #edca87; outline-offset:-2px; }
  .toggle { display:flex; align-items:center; gap:7px; width:100%; min-height:32px; padding:5px 9px; }.toggle span { flex:1; text-align:left; font-size:9px; letter-spacing:0.1em; }
  .surface { display:block; width:100%; padding:0; aspect-ratio:23/16; background:#2b5360; overflow:hidden; touch-action:none; cursor:crosshair; }svg { display:block; width:100%; height:100%; }
  .zoom { display:flex; align-items:center; padding-left:9px; }.zoom span { flex:1; font-size:9px; color:#a4b6ad; }.zoom button { display:grid; place-items:center; width:36px; height:36px; }
  @container (max-width:760px) or ((max-height:500px) and (max-width:1000px)) {
    .minimap { left:auto; right:max(10px,env(safe-area-inset-right)); top:calc(102px + env(safe-area-inset-top)); bottom:auto; width:120px; }
    .toggle { min-height:44px; }.zoom button { width:44px; height:44px; }.zoom span { display:none; }.zoom { padding:0; justify-content:space-around; }
  }
  @container (max-height:500px) and (min-width:601px) {
    .minimap { top:auto; right:auto; left:max(10px,env(safe-area-inset-left)); bottom:calc(76px + env(safe-area-inset-bottom)); width:120px; }
  }
</style>

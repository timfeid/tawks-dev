<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';
  import type { WorkflowResource } from '@tawks/workflow-types';
  import type { WorkflowStateProvider } from '@tawks/workflow-ui/workflow';
  import { submitServerAction } from '@tawks/workflow-ui/components/workflow';
  import { SceneManager } from './battlefield/engine/SceneManager';
  import { TracklinesBoard } from './tracklines/board';
  import TracklinesHud from './tracklines/hud.svelte';
  import TracklinesMinimap from './tracklines/minimap.svelte';
  import { MAP_HEIGHT, MAP_WIDTH } from './tracklines/geography';

  let { workflow }: { workflow: WorkflowResource } = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  import type { Card, Game, RouteView, TicketView } from './tracklines/types';

  let game = $derived((workflow?.responses?.game as Game | null) ?? null);
  let viewer = $derived(game?.viewer ?? null);
  let players = $derived(game?.players ?? []);
  let claimableIds = $derived(new Set((viewer?.claimable ?? []).map((c) => c.route)));

  const CARD_COLORS: Record<Card | 'any', string> = {
    red: '#ef4444',
    orange: '#f97316',
    yellow: '#eab308',
    green: '#22c55e',
    blue: '#3b82f6',
    purple: '#a855f7',
    black: '#475569',
    white: '#e2e8f0',
    locomotive: '#94a3b8',
    any: '#cbd5e1',
  };
  const CARD_LABELS: Record<Card | 'any', string> = {
    red: 'Red',
    orange: 'Orange',
    yellow: 'Yellow',
    green: 'Green',
    blue: 'Blue',
    purple: 'Purple',
    black: 'Black',
    white: 'White',
    locomotive: 'Locomotive',
    any: 'Any color',
  };
  const PLAYER_COLORS = ['#f43f5e', '#3b82f6', '#22c55e', '#f59e0b', '#a855f7'];
  const playerColor = (id: string | null | undefined) => {
    const seat = players.find((p) => p.id === id)?.seat ?? -1;
    return seat >= 0 ? PLAYER_COLORS[seat % PLAYER_COLORS.length] : '#64748b';
  };

  // ---- actions ---------------------------------------------------------------

  function submit(action: string, payload: Record<string, any> = {}) {
    if (!workflow?.instance_id) return;
    const lobbyId = workflow?.responses?.lobby_id;
    submitServerAction(provider, workflow.instance_id, { ...payload, lobby_id: lobbyId ?? null }, `games.tracklines.${action}`);
  }

  let lastHeartbeatInstance = '';
  $effect(() => {
    const iid = workflow?.instance_id;
    if (!iid || iid === lastHeartbeatInstance) return;
    lastHeartbeatInstance = iid;
    submit('view');
    const interval = setInterval(() => submit('view'), 20000);
    return () => clearInterval(interval);
  });

  let selectedRoute = $state<number | null>(null);
  let offerKeep = $state<Set<number>>(new Set());
  $effect(() => {
    // A new offer starts with nothing ticked.
    const ids = (viewer?.ticket_offer ?? []).map((t) => t.id).join(',');
    void ids;
    offerKeep = new Set();
  });

  function claim(route: number, card: Card) {
    submit('claim_route', { route, color: card });
    selectedRoute = null;
  }
  function keepTickets() {
    submit('keep_tickets', { keep: [...offerKeep] });
  }
  function toggleOffer(id: number) {
    const next = new Set(offerKeep);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    offerKeep = next;
  }

  // ---- 3D board --------------------------------------------------------------

  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;
  let sceneManager: SceneManager | null = null;
  let board: TracklinesBoard | null = null;
  let sceneReady = $state(false);
  let hovered = $state<number | null>(null);
  let ticketHighlight = $state<Set<number>>(new Set());
  let trainsPaused = $state(false);
  let nativeFullscreen = $state(false);
  let expanded = $state(false);
  let fullscreen = $derived(nativeFullscreen || expanded);
  let resetCamera = () => {};
  let navigateMap = (_x: number, _z: number) => {};
  let zoomMap = (_factor: number) => {};
  let footprint = $state('');
  let mapCenter = $state({ x: 0, z: 0 });
  let mobilePanel = $state<'map' | 'cards' | 'tickets' | 'crews'>('map');
  let motionTime = 0;
  let boardBuiltFor = '';

  async function toggleFullscreen() {
    if (nativeFullscreen) {
      await document.exitFullscreen();
    } else if (expanded) {
      expanded = false;
    } else {
      try {
        await container.requestFullscreen();
      } catch {
        // Browsers without the Fullscreen API can still fill the app viewport.
        expanded = true;
      }
    }
  }

  $effect(() => {
    if (!expanded) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => { document.body.style.overflow = previous; };
  });

  function cheapestRoutes(from: string, to: string, cost: (route: RouteView) => number | null): Set<number> | null {
    if (!game) return null;
    const distances = new Map<string, number>(game.cities.map((city) => [city.id, Number.POSITIVE_INFINITY]));
    const previous = new Map<string, { city: string; route: number }>();
    const pending = new Set(game.cities.map((city) => city.id));
    distances.set(from, 0);
    while (pending.size) {
      let current: string | undefined;
      let best = Number.POSITIVE_INFINITY;
      for (const city of pending) {
        const distance = distances.get(city) ?? Number.POSITIVE_INFINITY;
        if (distance < best) { best = distance; current = city; }
      }
      if (!current || current === to) break;
      pending.delete(current);
      for (const route of game.routes) {
        const next = route.from === current ? route.to : route.to === current ? route.from : null;
        if (!next || !pending.has(next)) continue;
        const step = cost(route);
        if (step === null) continue;
        const distance = best + step;
        if (distance < (distances.get(next) ?? Number.POSITIVE_INFINITY)) {
          distances.set(next, distance);
          previous.set(next, { city: current, route: route.id });
        }
      }
    }
    if (!Number.isFinite(distances.get(to) ?? Number.POSITIVE_INFINITY)) return null;
    const highlighted = new Set<number>();
    let current = to;
    while (current !== from) {
      const step = previous.get(current);
      if (!step) break;
      highlighted.add(step.route);
      current = step.city;
    }
    return highlighted;
  }

  // Your own track is nearly free and other players' track is closed, so a finished
  // ticket lights the line you built and an open one the cheapest way to finish it.
  function ticketRouteIds(ticket: TicketView): Set<number> {
    if (!game) return new Set();
    const me = viewer?.id;
    const owners = new Map(game.routes.map((route) => [route.id, route.owner]));
    const fewPlayers = players.length < 4;
    const playable = cheapestRoutes(ticket.from, ticket.to, (route) => {
      if (route.owner) return route.owner === me ? 0.01 : null;
      const twinOwner = route.twin === null ? null : owners.get(route.twin);
      if (twinOwner && (twinOwner === me || fewPlayers)) return null;
      return route.length;
    });
    return playable ?? cheapestRoutes(ticket.from, ticket.to, (route) => route.length) ?? new Set();
  }

  function showTicketRoute(ticket: TicketView) {
    ticketHighlight = ticketRouteIds(ticket);
    if (game) paintRoutes(game);
  }

  function clearTicketRoute() {
    ticketHighlight = new Set();
    if (game) paintRoutes(game);
  }

  function paintRoutes(g: Game) {
    board?.paint(g.routes, CARD_COLORS, playerColor, claimableIds, selectedRoute, hovered, ticketHighlight);
    board?.animate(motionTime);
  }

  $effect(() => {
    if (!game || !sceneReady || !sceneManager) return;
    // Older cached views refresh through the normal heartbeat before we draw them.
    if (!game.cities.every((city) => Number.isFinite(city.longitude) && Number.isFinite(city.latitude))) return;
    const key = workflow.instance_id + JSON.stringify(game.cities);
    if (!board || boardBuiltFor !== key) {
      board?.dispose();
      board = new TracklinesBoard(game.cities, game.routes, CARD_COLORS);
      sceneManager.scene.add(board.group);
      sceneManager.renderer.shadowMap.needsUpdate = true;
      boardBuiltFor = key;
    }
    paintRoutes(game);
  });

  onMount(() => {
    sceneManager = new SceneManager({ canvas, useBloom: false, useSmaa: true });
    sceneManager.composer.addPass(new OutputPass());
    const { scene, camera } = sceneManager;
    scene.fog = null;
    scene.background = new THREE.Color(0x0d2028);
    sceneManager.renderer.shadowMap.enabled = true;
    sceneManager.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    sceneManager.renderer.shadowMap.autoUpdate = false;
    const mobileRender = window.matchMedia('(pointer: coarse)').matches || window.innerWidth < 760;
    sceneManager.renderer.setPixelRatio(Math.min(window.devicePixelRatio, mobileRender ? 1.35 : 2));
    sceneManager.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    sceneManager.renderer.toneMappingExposure = 1.1;
    camera.position.set(0, 15, 11);
    camera.lookAt(0, 0, 0);
    scene.add(new THREE.HemisphereLight(0xd6eaf2, 0x294341, 1.7));
    const sun = new THREE.DirectionalLight(0xffe1ac, 3.2);
    sun.position.set(-8, 12, 5);
    sun.castShadow = true;
    sun.shadow.mapSize.set(mobileRender ? 1024 : 2048, mobileRender ? 1024 : 2048);
    sun.shadow.camera.left = -16;
    sun.shadow.camera.right = 16;
    sun.shadow.camera.top = 13;
    sun.shadow.camera.bottom = -13;
    sun.shadow.camera.near = 0.5;
    sun.shadow.camera.far = 45;
    sun.shadow.normalBias = 0.035;
    sun.shadow.bias = -0.00015;
    sun.shadow.radius = 2;
    scene.add(sun);
    const fill = new THREE.DirectionalLight(0x9ccee0, 0.8);
    fill.position.set(10, 8, -8);
    scene.add(fill);
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(200, 200), new THREE.MeshStandardMaterial({ color: '#10242b', roughness: 1 }));
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = -1.12;
    floor.receiveShadow = true;
    scene.add(floor);

    const controls = new OrbitControls(camera, canvas);
    navigateMap = (x, z) => {
      const target = new THREE.Vector3(THREE.MathUtils.clamp(x, -MAP_WIDTH / 2, MAP_WIDTH / 2), 0, THREE.MathUtils.clamp(z, -MAP_HEIGHT / 2, MAP_HEIGHT / 2));
      camera.position.add(target.clone().sub(controls.target));
      controls.target.copy(target);
      controls.update();
    };
    zoomMap = (factor) => {
      const offset = camera.position.clone().sub(controls.target);
      offset.setLength(THREE.MathUtils.clamp(offset.length() * factor, controls.minDistance, controls.maxDistance));
      camera.position.copy(controls.target).add(offset);
      controls.update();
    };
    // Frame the geographic board whatever the panel's aspect ratio.
    const fitCamera = (fitVertical = false) => {
      const aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
      const halfFov = THREE.MathUtils.degToRad(camera.fov / 2);
      const mobile = aspect < 0.75;
      // A full-width mobile board otherwise becomes a tiny postcard because
      // the portrait viewport has to fit the continent's wide aspect ratio.
      // Let the edges breathe outside the viewport and use the extra height.
      const widthNeed = MAP_WIDTH / 2 / Math.max(aspect, 0.1) * (mobile ? 0.82 : 1);
      const needed = fitVertical && !mobile ? MAP_HEIGHT / 2 * 1.08 : Math.max(MAP_HEIGHT / 2, widthNeed);
      const distance = (needed / Math.tan(halfFov)) * 1.14;
      const pitch = THREE.MathUtils.degToRad(mobile ? 64 : 57);
      controls.maxDistance = Math.max(18, distance * 1.16);
      controls.target.set(0, 0, 0);
      camera.position.set(0, Math.sin(pitch) * distance, Math.cos(pitch) * distance);
      camera.lookAt(0, 0, 0);
    };
    resetCamera = () => fitCamera(true);
    fitCamera();
    const resizeObserver = new ResizeObserver(() => fitCamera());
    resizeObserver.observe(canvas.parentElement ?? canvas);

    controls.target.set(0, 0, 0);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = 6;
    controls.enableRotate = false;
    controls.maxPolarAngle = 1.35;
    // The board is a map, not a model viewer: primary drag pans, middle drag
    // also pans for trackpad/mouse parity, and wheel remains zoom.
    controls.mouseButtons = { LEFT: THREE.MOUSE.PAN, MIDDLE: THREE.MOUSE.PAN, RIGHT: THREE.MOUSE.PAN };
    controls.touches = { ONE: THREE.TOUCH.PAN, TWO: THREE.TOUCH.DOLLY_PAN };
    controls.screenSpacePanning = false;

    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    let downAt: { x: number; y: number } | null = null;
    const activePointers = new Set<number>();
    let dragged = false;
    const pick = (ev: PointerEvent) => {
      const rect = canvas.getBoundingClientRect();
      pointer.x = ((ev.clientX - rect.left) / rect.width) * 2 - 1;
      pointer.y = -((ev.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObjects(board?.pickObjects ?? [], false)[0];
      return hit ? (hit.object.userData.routeId as number) : null;
    };
    const onDown = (ev: PointerEvent) => {
      activePointers.add(ev.pointerId);
      if (activePointers.size > 1) { downAt = null; dragged = true; return; }
      dragged = false;
      downAt = { x: ev.clientX, y: ev.clientY };
    };
    const onUp = (ev: PointerEvent) => {
      activePointers.delete(ev.pointerId);
      if (!downAt) return;
      const moved = Math.hypot(ev.clientX - downAt.x, ev.clientY - downAt.y);
      downAt = null;
      if (dragged || moved > 6 || ev.button !== 0) return;
      const id = pick(ev);
      selectedRoute = id !== null && id === selectedRoute ? null : id;
      if (game) paintRoutes(game);
    };
    let lastHover = 0;
    const onMove = (ev: PointerEvent) => {
      if (downAt && Math.hypot(ev.clientX - downAt.x, ev.clientY - downAt.y) > 6) dragged = true;
      if (ev.pointerType === 'touch' || activePointers.size) return;
      const now = performance.now();
      if (now - lastHover < 40) return;
      lastHover = now;
      const id = pick(ev);
      if (id !== hovered) {
        hovered = id;
        canvas.style.cursor = id !== null ? 'pointer' : 'default';
        if (game) paintRoutes(game);
      }
    };
    canvas.addEventListener('pointerdown', onDown);
    canvas.addEventListener('pointerup', onUp);
    canvas.addEventListener('pointermove', onMove);
    const onCancel = (ev: PointerEvent) => { activePointers.delete(ev.pointerId); downAt = null; };
    const onLeave = () => { hovered = null; if (game) paintRoutes(game); };
    canvas.addEventListener('pointercancel', onCancel);
    canvas.addEventListener('lostpointercapture', onCancel);
    canvas.addEventListener('pointerleave', onLeave);

    const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
    const onMotionChange = () => { trainsPaused = motionPreference.matches; };
    onMotionChange();
    motionPreference.addEventListener('change', onMotionChange);
    const onFullscreenChange = () => { nativeFullscreen = document.fullscreenElement === container; };
    const onEscape = (event: KeyboardEvent) => { if (event.key === 'Escape') expanded = false; };
    document.addEventListener('fullscreenchange', onFullscreenChange);
    document.addEventListener('keydown', onEscape);
    let frame = 0;
    let previousTime = performance.now();
    let lastOverview = 0;
    const overviewRay = new THREE.Raycaster();
    const boardPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
    const corner = new THREE.Vector3();
    const overviewCorners = [new THREE.Vector2(-1, 1), new THREE.Vector2(1, 1), new THREE.Vector2(1, -1), new THREE.Vector2(-1, -1)];
    let framingKey = '';
    const animate = (now: number) => {
      frame = requestAnimationFrame(animate);
      const elapsed = Math.min((now - previousTime) / 1000, 0.05);
      previousTime = now;
      if (document.hidden) return;
      if (!trainsPaused) motionTime += elapsed;
      board?.animate(motionTime);
      controls.update();
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      const rootHeight = container.clientHeight;
      const compact = width <= 760 || (rootHeight <= 500 && width <= 1000);
      const drawer = compact && mobilePanel !== 'map';
      const landscape = rootHeight <= 500 && width > 600;
      const nextFraming = `${width}:${height}:${drawer}:${landscape}`;
      if (nextFraming !== framingKey) {
        framingKey = nextFraming;
        // Shift the board into the space beside/above the mobile drawer without
        // resetting the player's zoom, orbit, or location when switching panels.
        if (drawer) camera.setViewOffset(width, height, landscape ? 155 : 0, landscape ? 0 : Math.min(rootHeight * 0.46, 360) / 2, width, height);
        else camera.clearViewOffset();
      }
      // Keep touch panning on the board, and share a lightweight top-down view.
      const bounded = new THREE.Vector3(THREE.MathUtils.clamp(controls.target.x, -MAP_WIDTH / 2 + 2.2, MAP_WIDTH / 2 - 2.2), 0, THREE.MathUtils.clamp(controls.target.z, -MAP_HEIGHT / 2 + 1.8, MAP_HEIGHT / 2 - 1.8));
      camera.position.add(bounded.clone().sub(controls.target));
      controls.target.copy(bounded);
      if (now - lastOverview > 100) {
        lastOverview = now;
        camera.updateMatrixWorld();
        const next = overviewCorners.map(point => {
          overviewRay.setFromCamera(point, camera);
          if (!overviewRay.ray.intersectPlane(boardPlane, corner)) overviewRay.ray.at(150, corner);
          return `${corner.x.toFixed(2)},${corner.z.toFixed(2)}`;
        }).join(' ');
        if (footprint !== next) footprint = next;
        if (Math.abs(mapCenter.x - bounded.x) + Math.abs(mapCenter.z - bounded.z) > 0.01) mapCenter = { x: bounded.x, z: bounded.z };
      }
      sceneManager?.render();
    };
    frame = requestAnimationFrame(animate);
    sceneReady = true;

    return () => {
      cancelAnimationFrame(frame);
      canvas.removeEventListener('pointerdown', onDown);
      canvas.removeEventListener('pointerup', onUp);
      canvas.removeEventListener('pointermove', onMove);
      canvas.removeEventListener('pointercancel', onCancel);
      canvas.removeEventListener('lostpointercapture', onCancel);
      canvas.removeEventListener('pointerleave', onLeave);
      motionPreference.removeEventListener('change', onMotionChange);
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      document.removeEventListener('keydown', onEscape);
      resizeObserver.disconnect();
      controls.dispose();
      sun.shadow.dispose();
      floor.geometry.dispose();
      floor.material.dispose();
      board?.dispose();
      board = null;
      sceneReady = false;
      sceneManager?.composer.passes.forEach((pass) => pass.dispose());
      sceneManager?.dispose();
      sceneManager = null;
    };
  });

</script>
<div bind:this={container} class="tracklines-root {expanded ? 'expanded' : ''}" class:panel-open={mobilePanel !== 'map'}>
  <div class="map-viewport">
    <canvas bind:this={canvas} aria-label="North America rail map. Mouse: drag to pan, middle drag to pan, scroll to zoom. Touch: drag to pan and pinch to zoom. Tap a route to claim it."></canvas>
  </div>
  {#if game}
    <TracklinesMinimap {game} {footprint} center={mapCenter} {playerColor} navigate={(x, z) => navigateMap(x, z)} zoom={factor => zoomMap(factor)} highlighted={ticketHighlight} />
    <TracklinesHud
      bind:mobilePanel
      {game} colors={CARD_COLORS} labels={CARD_LABELS} {playerColor}
      paused={trainsPaused} {fullscreen}
      togglePause={() => (trainsPaused = !trainsPaused)} {toggleFullscreen} fitMap={() => resetCamera()}
      {submit} selectedId={selectedRoute} closeRoute={() => (selectedRoute = null)} {claim}
      {offerKeep} toggleOffer={toggleOffer} {keepTickets} previewTicket={showTicketRoute} clearTicket={clearTicketRoute}
    />
  {:else}
    <div class="loading-state">Preparing your journey...</div>
  {/if}
</div>

<style>
  .tracklines-root { position:absolute; inset:0; width:100%; height:100%; overflow:hidden; background:#0d2028; font-family:system-ui,sans-serif; container-type:size; }
  .tracklines-root.expanded { position:fixed; z-index:100; height:100dvh; }
  .tracklines-root:fullscreen { height:100dvh; }
  .map-viewport { position:absolute; top:64px; bottom:125px; left:0; right:0; }
  canvas { display:block; width:100%; height:100%; touch-action:none; }
  .loading-state { position:absolute; bottom:30px; left:24px; color:#c3d0c0; font-size:13px; }
  @container (max-width:760px) or ((max-height:500px) and (max-width:1000px)) {
    .map-viewport { top:0; bottom:0; }
    .panel-open :global(.minimap) { display:none; }
  }
</style>

<script lang="ts">
  import { connect, onFullscreenChange, type Activity } from '@tawks/activity-sdk';
  import { onDestroy } from 'svelte';
  import { Camera, MapPin, Maximize, Minimize, Navigation, RotateCcw, Settings, Users, Volume2, VolumeX, X } from '@lucide/svelte';
  import Hud from './components/Hud.svelte';
  import Minimap from './components/Minimap.svelte';
  import PlacePicker from './components/PlacePicker.svelte';
  import TouchControls from './components/TouchControls.svelte';
  import { Game, type Hud as HudState } from './lib/game';
  import { DriveConnection } from './lib/net.svelte';
  import type { Pass } from './lib/types';
  import { QUALITY } from './lib/world';

  const JOIN_ACTION = 'games.outroads.join';

  let conn = $state<DriveConnection | null>(null);
  let game = $state<Game | null>(null);
  let activity = $state<Activity | null>(null);
  let problem = $state<string | null>(null);
  let canvas = $state<HTMLCanvasElement>();
  let labels = $state<HTMLDivElement>();
  let hud = $state<HudState | null>(null);
  let picking = $state(false);
  let panel = $state<'none' | 'drivers' | 'settings'>('none');
  let photo = $state<string | null>(null);
  let photoMode = $state(false);
  let fullscreen = $state<boolean | null>(null);
  let width = $state(1200);
  let units = $state<'mph' | 'kmh'>(navigator.language?.startsWith('en-US') ? 'mph' : 'kmh');
  let muted = $state(false);
  let assist = $state(true);
  let satellite = $state(false);
  let lights = $state<'auto' | 'on' | 'off'>('auto');
  let quality = $state<'high' | 'low'>(matchMedia('(pointer: coarse)').matches ? 'low' : 'high');
  const query = new URLSearchParams(location.search);
  let showStats = $state(query.has('debug') || query.has('stats'));
  let storage = $state<boolean | null>(null);
  let hourDraft = $state<number | null>(null);
  let hourTimer: ReturnType<typeof setTimeout> | null = null;
  const touch = matchMedia('(pointer: coarse)').matches || new URLSearchParams(location.search).has('touch');

  function isPass(value: unknown): value is Pass {
    return !!value && typeof value === 'object' && 'token' in value && 'ws_url' in value;
  }

  function passFromAddress(): Pass | null {
    const params = new URLSearchParams(location.search);
    const token = params.get('token');
    const ws = params.get('ws');
    return token && ws ? { token, ws_url: ws, lobby_id: params.get('lobby') ?? 'dev' } : null;
  }

  function nextPass(frame: Activity, previous: string | null): Promise<Pass> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        stop();
        reject(new Error('No seat arrived'));
      }, 15_000);
      const stop = frame.onState((state) => {
        if (isPass(state) && state.token !== previous) {
          clearTimeout(timeout);
          stop();
          resolve(state);
        }
      });
      frame.call(JOIN_ACTION, { lobby_id: frame.lobby?.id ?? null }).catch((error: Error) => {
        clearTimeout(timeout);
        stop();
        reject(error);
      });
    });
  }

  async function start() {
    const fromAddress = passFromAddress();
    if (fromAddress) {
      conn = new DriveConnection(fromAddress, () => Promise.reject(new Error('expired')));
      return;
    }
    try {
      const frame = await connect();
      activity = frame;
      fullscreen = false;
      frame.onFullscreen((on) => (fullscreen = on));
      const first = isPass(frame.state) ? frame.state : await nextPass(frame, null);
      let token = first.token;
      conn = new DriveConnection(first, async () => {
        const next = await nextPass(frame, token);
        token = next.token;
        return next;
      });
    } catch (error) {
      problem = (error as Error).message;
    }
  }

  $effect(() => {
    if (!conn || !canvas || !labels || game) return;
    const g = new Game(canvas, labels, conn);
    g.onHud = (h) => (hud = h);
    g.onPress = (press) => {
      if (press === 'photo') togglePhoto();
      else if (press === 'map') picking = !picking;
      else if (press === 'units') units = units === 'mph' ? 'kmh' : 'mph';
      else if (press === 'lights') lights = g.lightsMode;
    };
    g.world.storage((persistent) => (storage = persistent));
    game = g;
    (window as unknown as { __outroads: unknown }).__outroads = {
      game: g,
      conn,
      stats: () => ({ ...g.hud, ...g.world.stats(), car: g.car && { x: g.car.x, z: g.car.z, h: g.car.h } }),
    };
  });

  let seenEpoch = -1;
  $effect(() => {
    const room = conn?.room;
    if (room && game) game.sync();
    if (room && room.epoch !== seenEpoch) {
      if (seenEpoch >= 0) picking = false;
      seenEpoch = room.epoch;
    }
  });

  $effect(() => {
    if (!game) return;
    game.assist = assist;
    game.sound.muted = muted;
    game.lightsMode = lights;
    game.world.setSatellite(satellite);
  });

  $effect(() => {
    if (game) game.world.setQuality(QUALITY[quality]);
  });

  $effect(() => {
    if (game && showStats) game.world.measureGpu();
  });

  $effect(() => {
    if (game) game.input.enabled = !(picking || needsPlace) && !photoMode;
  });

  const room = $derived(conn?.room ?? null);
  const needsPlace = $derived(!!room && !room.place);
  const hostName = $derived(room?.drivers.find((d) => d.user_id === room.host)?.name ?? null);
  const narrow = $derived(width < 640);
  const hour = $derived(hourDraft ?? room?.hour ?? 15.5);

  function setHour(value: number) {
    hourDraft = value;
    if (hourTimer) clearTimeout(hourTimer);
    hourTimer = setTimeout(() => {
      conn?.act({ kind: 'set_hour', hour: value });
      hourDraft = null;
    }, 250);
  }

  function hourLabel(h: number) {
    const hh = Math.floor(h) % 24;
    const mm = Math.round((h - Math.floor(h)) * 60) % 60;
    const suffix = hh < 12 ? 'am' : 'pm';
    return `${hh % 12 === 0 ? 12 : hh % 12}:${String(mm).padStart(2, '0')} ${suffix}`;
  }

  function togglePhoto() {
    if (!game) return;
    photoMode = !photoMode;
    game.setPhoto(photoMode);
    if (!photoMode) photo = null;
  }

  function honk() {
    game?.input.presses.push('honk');
  }

  function distance(id: string) {
    const d = game?.nearby().find((n) => n.id === id)?.distance;
    if (d === undefined) return 'Not driving yet';
    if (units === 'mph') return d < 400 ? `${Math.round(d * 3.281)} ft away` : `${(d / 1609).toFixed(1)} mi away`;
    return d < 1000 ? `${Math.round(d)} m away` : `${(d / 1000).toFixed(1)} km away`;
  }

  async function toggleFullscreen() {
    if (activity) fullscreen = await activity.fullscreen(!fullscreen);
  }

  void start();
  const stopFullscreen = onFullscreenChange((on) => (fullscreen = activity ? fullscreen : on));
  onDestroy(() => {
    game?.dispose();
    conn?.close();
    stopFullscreen();
  });
</script>

<svelte:window bind:innerWidth={width} />

<main>
  <canvas bind:this={canvas} class="view"></canvas>
  <div class="labels" bind:this={labels}></div>

  {#if problem}
    <div class="center panel message">
      <h2>Outroads could not start</h2>
      <p>{problem}</p>
    </div>
  {:else if !conn || conn.status === 'connecting' || !room}
    <div class="center panel message"><p>Starting the engine...</p></div>
  {/if}

  {#if game && room && !photoMode}
    <div class="top-left">
      <button class="chip place" onclick={() => (picking = true)} title="Change place (M)">
        <MapPin size={16} />
        <span>{room.place?.name ?? 'Pick a place'}</span>
      </button>
      <button class="chip icon" class:on={panel === 'drivers'} onclick={() => (panel = panel === 'drivers' ? 'none' : 'drivers')} aria-label="Drivers">
        <Users size={18} />
      </button>
      <button class="chip icon" class:on={panel === 'settings'} onclick={() => (panel = panel === 'settings' ? 'none' : 'settings')} aria-label="Settings">
        <Settings size={18} />
      </button>
      <button class="chip icon" onclick={togglePhoto} aria-label="Photo mode (P)"><Camera size={18} /></button>
      <button class="chip icon" onclick={() => game?.respawn()} aria-label="Back on the road (R)"><RotateCcw size={18} /></button>
      {#if fullscreen !== null}
        <button class="chip icon" onclick={toggleFullscreen} aria-label="Full screen">
          {#if fullscreen}<Minimize size={18} />{:else}<Maximize size={18} />{/if}
        </button>
      {/if}
    </div>

    {#if panel === 'drivers'}
      <section class="panel side">
        <h3>Drivers</h3>
        {#if hostName}<p class="hint">{hostName} is hosting</p>{/if}
        <ul>
          {#each room.drivers as d (d.user_id)}
            <li>
              <span class="dot" style:background={d.color}></span>
              <div class="who">
                <strong>{d.name}{d.user_id === conn?.me ? ' (you)' : ''}</strong>
                {#if d.user_id !== conn?.me}<span>{distance(d.user_id)}</span>{/if}
              </div>
              {#if d.user_id !== conn?.me}
                <button class="ghost small" onclick={() => game?.teleportTo(d.user_id)} title="Teleport to {d.name}">
                  <Navigation size={14} /> Join
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {:else if panel === 'settings'}
      <section class="panel side settings">
        <h3>Settings</h3>
        <label class="field">
          <span>Time of day <em>{hourLabel(hour)}</em></span>
          <input type="range" min="0" max="23.75" step="0.25" value={hour} oninput={(e) => setHour(Number((e.currentTarget as HTMLInputElement).value))} />
          <small>Changes it for the whole lobby</small>
        </label>
        <div class="field">
          <span>Headlights</span>
          <div class="seg">
            {#each ['auto', 'on', 'off'] as const as mode}
              <button class:on={lights === mode} onclick={() => (lights = mode)}>{mode === 'auto' ? 'Auto' : mode === 'on' ? 'On' : 'Off'}</button>
            {/each}
          </div>
        </div>
        <label class="toggle"><input type="checkbox" bind:checked={assist} /> Snap to road assist</label>
        <label class="toggle"><input type="checkbox" bind:checked={satellite} /> Satellite ground (2016, about 10 m detail)</label>
        <div class="field">
          <span>Speed in</span>
          <div class="seg">
            <button class:on={units === 'mph'} onclick={() => (units = 'mph')}>mph</button>
            <button class:on={units === 'kmh'} onclick={() => (units = 'kmh')}>km/h</button>
          </div>
        </div>
        <div class="field">
          <span>Detail</span>
          <div class="seg">
            <button class:on={quality === 'high'} onclick={() => (quality = 'high')}>High</button>
            <button class:on={quality === 'low'} onclick={() => (quality = 'low')}>Light</button>
          </div>
        </div>
        <label class="toggle"><input type="checkbox" bind:checked={showStats} /> Show frame stats</label>
        <p class="keys">WASD or arrows drive, Space drifts, H honks, L lights, R back on the road, C camera, P photo, M places. Gamepads work too.</p>
      </section>
    {/if}

    <div class="top-right">
      <Minimap {game} size={narrow ? 112 : 168} heading={hud?.heading ?? 0} />
      <button class="chip icon" onclick={() => (muted = !muted)} aria-label={muted ? 'Sound on' : 'Sound off'}>
        {#if muted}<VolumeX size={18} />{:else}<Volume2 size={18} />{/if}
      </button>
    </div>

    {#if hud && room.place}
      <Hud {hud} {units} />
      {#if hud.spawning}
        <div class="center panel message"><p>Loading streets around {room.place.name}...</p></div>
      {/if}
    {/if}

    {#if touch}
      <TouchControls input={game.input} onhonk={honk} />
    {/if}
  {/if}

  {#if photoMode}
    <div class="photo-bar panel">
      <span>Photo mode. Drag to look around, scroll or pinch to zoom.</span>
      <button class="primary" onclick={() => (photo = game?.capture() ?? null)}><Camera size={16} /> Capture</button>
      <button class="ghost" onclick={togglePhoto}><X size={16} /> Done</button>
    </div>
    {#if photo}
      <div class="shot panel">
        <img src={photo} alt="The captured view" />
        <div class="row">
          <a class="primary link" href={photo} download="outroads-photo.png">Save</a>
          <button class="ghost" onclick={() => (photo = null)}>Discard</button>
        </div>
        <small>If saving is blocked here, take a screenshot of this picture.</small>
      </div>
    {/if}
  {/if}

  {#if showStats && hud}
    <div class="stats">
      {hud.fps} fps, {hud.frameMs} ms cpu{hud.gpuMs !== null ? `, ${hud.gpuMs} ms gpu` : ''}, {hud.calls} calls, {Math.round(hud.triangles / 1000)}k tris, {hud.tiles} tiles{hud.loading ? `, ${hud.loading} loading` : ''}{storage === false ? ', memory cache' : storage ? ', disk cache' : ''}
    </div>
  {/if}

  {#if conn?.error}
    <div class="toast panel">{conn.error}</div>
  {/if}

  {#if conn && room && (needsPlace || picking)}
    <PlacePicker {conn} here={() => game?.here() ?? null} onclose={needsPlace ? null : () => (picking = false)} />
  {/if}

  <footer class="credits" class:raised={touch}>
    © OpenStreetMap contributors, OpenFreeMap, © OpenMapTiles{satellite ? ', Sentinel-2 cloudless 2016 by EOX IT Services GmbH (contains modified Copernicus Sentinel data 2016)' : ''}, search by Photon
  </footer>
</main>

<style>
  main {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .view {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    touch-action: none;
  }
  .labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .center {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
  }
  .message {
    padding: 16px 22px;
    text-align: center;
    max-width: calc(100% - 32px);
    z-index: 5;
  }
  .message h2 {
    margin: 0 0 6px;
    font-size: 18px;
  }
  .message p {
    margin: 0;
    color: var(--text-dim);
  }
  .top-left {
    position: absolute;
    top: calc(14px + env(safe-area-inset-top));
    left: 16px;
    right: 200px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    z-index: 4;
  }
  .place {
    max-width: 280px;
  }
  .place span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .top-right {
    position: absolute;
    top: calc(14px + env(safe-area-inset-top));
    right: 16px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 10px;
    z-index: 4;
  }
  .side {
    position: absolute;
    top: calc(66px + env(safe-area-inset-top));
    left: 16px;
    width: min(320px, calc(100% - 32px));
    max-height: calc(100% - 180px);
    overflow-y: auto;
    padding: 14px 16px;
    z-index: 4;
  }
  .side h3 {
    margin: 0 0 4px;
    font-family: var(--font-sign);
    font-size: 18px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .hint {
    margin: 0 0 8px;
    color: var(--text-faint);
    font-size: 13px;
  }
  .side ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .side li {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.2);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .who span {
    font-size: 12px;
    color: var(--text-faint);
  }
  .small {
    height: 32px;
    padding: 0 10px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
  }
  .settings {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field > span {
    font-weight: 600;
    display: flex;
    justify-content: space-between;
  }
  .field em {
    font-style: normal;
    color: var(--signal);
  }
  .field small {
    color: var(--text-faint);
  }
  input[type='range'] {
    accent-color: var(--signal);
    width: 100%;
  }
  .seg {
    display: flex;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-small);
    overflow: hidden;
  }
  .seg button {
    flex: 1;
    height: 34px;
    border: none;
    background: none;
    font-weight: 600;
  }
  .seg button.on {
    background: var(--signal);
    color: #1b1307;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .toggle input {
    accent-color: var(--signal);
    width: 18px;
    height: 18px;
  }
  .keys {
    margin: 0;
    color: var(--text-faint);
    font-size: 13px;
    line-height: 1.45;
  }
  .photo-bar {
    position: absolute;
    left: 50%;
    bottom: calc(18px + env(safe-area-inset-bottom));
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 10px 16px;
    max-width: calc(100% - 32px);
    flex-wrap: wrap;
    z-index: 8;
  }
  .photo-bar span {
    color: var(--text-dim);
  }
  .photo-bar button,
  .row .link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .shot {
    position: absolute;
    right: 16px;
    top: 16px;
    width: min(380px, calc(100% - 32px));
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 9;
  }
  .shot img {
    width: 100%;
    border-radius: var(--radius-small);
    user-select: auto;
    -webkit-user-select: auto;
  }
  .shot small {
    color: var(--text-faint);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .link {
    text-decoration: none;
    line-height: var(--control);
  }
  .stats {
    position: absolute;
    left: 16px;
    top: calc(66px + env(safe-area-inset-top));
    max-width: calc(100% - 220px);
    padding: 4px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.6);
    font-family: ui-monospace, monospace;
    font-size: 12px;
    pointer-events: none;
    z-index: 3;
  }
  .toast {
    position: absolute;
    left: 50%;
    top: 70px;
    transform: translateX(-50%);
    padding: 10px 16px;
    z-index: 30;
    max-width: calc(100% - 32px);
  }
  .credits {
    position: absolute;
    right: 8px;
    bottom: 4px;
    max-width: calc(100% - 16px);
    font-size: 10.5px;
    color: rgba(238, 242, 246, 0.72);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
    text-align: right;
    pointer-events: none;
    z-index: 3;
  }
  .credits.raised {
    bottom: auto;
    top: calc(4px + env(safe-area-inset-top));
    right: auto;
    left: 50%;
    transform: translateX(-50%);
    white-space: nowrap;
    opacity: 0.8;
  }
  @media (max-width: 640px) {
    .top-left {
      right: 136px;
      left: 12px;
      gap: 6px;
    }
    .top-right {
      right: 12px;
    }
    main {
      --control: 36px;
    }
    .place {
      max-width: 150px;
    }
    .credits.raised {
      white-space: normal;
      width: max-content;
      max-width: calc(100% - 24px);
      text-align: center;
      top: auto;
      bottom: calc(150px + env(safe-area-inset-bottom));
    }
    .stats {
      bottom: auto;
      top: calc(220px + env(safe-area-inset-top));
      left: 12px;
      right: 12px;
    }
  }
</style>

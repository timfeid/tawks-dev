<script lang="ts">
  import { connect, onFullscreenChange, type Activity } from '@tawks/activity-sdk';
  import { onDestroy } from 'svelte';
  import { Dices, ListOrdered, ScrollText } from '@lucide/svelte';
  import Credits from './components/Credits.svelte';
  import DiceTray from './components/DiceTray.svelte';
  import Header from './components/Header.svelte';
  import Initiative from './components/Initiative.svelte';
  import Sheets from './components/Sheets.svelte';
  import { TableConnection } from './lib/table.svelte';
  import type { Pass } from './lib/types';

  const JOIN_ACTION = 'games.longtable.join';

  let conn = $state<TableConnection | null>(null);
  let activity = $state<Activity | null>(null);
  let problem = $state<string | null>(null);
  let width = $state(1200);
  let tab = $state<'dice' | 'order' | 'sheets'>('dice');
  let side = $state<'dice' | 'order' | 'sheets'>('dice');
  let creditsOpen = $state(false);
  let fullscreen = $state<boolean | null>(null);
  let lobbyOwner = $state(false);

  function isPass(value: unknown): value is Pass {
    return !!value && typeof value === 'object' && 'token' in value && 'ws_url' in value;
  }

  /** Reads the pass's own claims; the server checks the signature. */
  function ownsLobby(pass: Pass) {
    try {
      const payload = pass.token.split('.')[0].replace(/-/g, '+').replace(/_/g, '/');
      return !!JSON.parse(atob(payload)).owner;
    } catch {
      return false;
    }
  }

  /** Outside a tawks frame a pass can come from the address, for local work. */
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

  function seat(pass: Pass, renew: () => Promise<Pass>) {
    lobbyOwner = ownsLobby(pass);
    conn = new TableConnection(pass, async () => {
      const next = await renew();
      lobbyOwner = ownsLobby(next);
      return next;
    });
  }

  async function start() {
    const fromAddress = passFromAddress();
    if (fromAddress) {
      seat(fromAddress, () => Promise.reject(new Error('expired')));
      return;
    }
    try {
      const frame = await connect();
      activity = frame;
      fullscreen = false;
      frame.onFullscreen((on) => (fullscreen = on));
      const first = isPass(frame.state) ? frame.state : await nextPass(frame, null);
      let token = first.token;
      seat(first, async () => {
        const next = await nextPass(frame, token);
        token = next.token;
        return next;
      });
    } catch (error) {
      problem = (error as Error).message;
    }
  }

  async function toggleFullscreen() {
    if (activity) fullscreen = await activity.fullscreen(!fullscreen);
  }

  void start();
  const stopFullscreen = onFullscreenChange((on) => (fullscreen = activity ? fullscreen : on));
  onDestroy(() => {
    conn?.close();
    stopFullscreen();
  });

  const layout = $derived(width >= 1040 ? 'wide' : width >= 720 ? 'middle' : 'narrow');
  const tabs = [
    { key: 'dice', label: 'Dice', icon: Dices },
    { key: 'order', label: 'Initiative', icon: ListOrdered },
    { key: 'sheets', label: 'Sheets', icon: ScrollText },
  ] as const;
  const upNow = $derived.by(() => {
    const order = conn?.table?.initiative;
    return order?.entries.find((c) => c.id === order.current)?.name ?? null;
  });
</script>

<svelte:window bind:innerWidth={width} />

{#if problem}
  <main class="notice"><p>{problem}</p></main>
{:else if !conn?.table}
  <main class="notice">
    <span class="candle" aria-hidden="true"></span>
    <p>{conn?.status === 'offline' ? 'Finding the table again...' : 'Pulling up a chair...'}</p>
  </main>
{:else}
  <div class="app {layout}">
    <Header {conn} {lobbyOwner} {fullscreen} onFullscreen={toggleFullscreen} onCredits={() => (creditsOpen = true)} />

    {#if conn.status === 'offline'}
      <div class="status" role="status">Lost the table for a moment. Reconnecting...</div>
    {/if}

    {#if layout === 'narrow'}
      <main class="single">
        <div class="board">
          {#if tab === 'dice'}<DiceTray {conn} />{:else if tab === 'order'}<Initiative {conn} />{:else}<Sheets {conn} />{/if}
        </div>
      </main>
      <nav class="tabs" aria-label="Panels">
        {#each tabs as t (t.key)}
          {@const Icon = t.icon}
          <button class:on={tab === t.key} aria-current={tab === t.key ? 'page' : undefined} onclick={() => (tab = t.key)}>
            <Icon size={20} />
            <span>{t.label}</span>
            {#if t.key === 'order' && upNow && tab !== 'order'}<span class="pip" aria-label="A turn is running"></span>{/if}
          </button>
        {/each}
      </nav>
    {:else if layout === 'middle'}
      <main class="grid two">
        <div class="board split">
          <nav class="tabs mini" aria-label="Panels">
            {#each tabs.slice(0, 2) as t (t.key)}
              {@const Icon = t.icon}
              <button class:on={side === t.key} aria-current={side === t.key ? 'page' : undefined} onclick={() => (side = t.key)}>
                <Icon size={16} />
                <span>{t.label}</span>
                {#if t.key === 'order' && upNow && side !== 'order'}<span class="pip" aria-label="A turn is running"></span>{/if}
              </button>
            {/each}
          </nav>
          <div class="pane">{#if side === 'dice'}<DiceTray {conn} />{:else}<Initiative {conn} />{/if}</div>
        </div>
        <div class="board"><Sheets {conn} /></div>
      </main>
    {:else}
      <main class="grid three">
        <div class="board"><DiceTray {conn} /></div>
        <div class="board"><Initiative {conn} /></div>
        <div class="board"><Sheets {conn} /></div>
      </main>
    {/if}

    {#if conn.error}
      <div class="flash" role="alert">{conn.error}</div>
    {/if}
  </div>
  {#if creditsOpen}
    <Credits attribution={conn.attribution} compatibility={conn.compatibility} onClose={() => (creditsOpen = false)} />
  {/if}
{/if}

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    gap: 14px;
    padding: 14px;
  }

  .grid.three {
    grid-template-columns: minmax(300px, 1.05fr) minmax(260px, 0.85fr) minmax(320px, 1.1fr);
  }

  .grid.two {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }

  .split {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .pane {
    flex: 1;
    min-height: 0;
  }

  .tabs.mini {
    grid-template-columns: repeat(2, 1fr);
    padding: 0;
  }

  .tabs.mini button {
    flex-direction: row;
    justify-content: center;
    gap: 6px;
    font-size: 13px;
  }

  .tabs.mini .pip {
    top: 4px;
    right: 8px;
  }

  .single {
    flex: 1;
    min-height: 0;
    padding: 10px 10px 0;
    display: flex;
  }

  .single .board {
    flex: 1;
    padding: 10px;
  }

  .board {
    min-height: 0;
    overflow: hidden;
    padding: 14px;
    border-radius: 16px;
    background: linear-gradient(180deg, rgba(42, 27, 17, 0.92), rgba(29, 19, 12, 0.92));
    border: 1px solid rgba(214, 165, 79, 0.22);
    box-shadow:
      inset 0 1px 0 rgba(255, 220, 170, 0.07),
      0 10px 28px rgba(0, 0, 0, 0.45);
  }

  .tabs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
    padding: 8px 10px calc(8px + env(safe-area-inset-bottom));
  }

  .tabs button {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 6px 0;
    border-radius: 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-dim);
    font-weight: 700;
    font-size: 12px;
  }

  .tabs button.on {
    color: var(--brass-bright);
    border-color: rgba(214, 165, 79, 0.35);
    background: rgba(214, 165, 79, 0.1);
  }

  .tabs .pip {
    position: absolute;
    top: 6px;
    right: calc(50% - 18px);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--brass-bright);
    box-shadow: 0 0 6px var(--brass-bright);
  }

  .status {
    padding: 6px 14px;
    font-size: 13px;
    color: var(--wood-950);
    background: var(--frost);
  }

  .flash {
    position: fixed;
    left: 50%;
    bottom: 76px;
    transform: translateX(-50%);
    z-index: 30;
    max-width: calc(100vw - 32px);
    padding: 10px 16px;
    border-radius: 999px;
    color: var(--wood-950);
    background: var(--parchment);
    border: 1px solid var(--parchment-edge);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    font-weight: 700;
    font-size: 14px;
  }

  .notice {
    height: 100%;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 14px;
    color: var(--text-dim);
    font-family: var(--font-display);
    font-size: 20px;
  }

  .candle {
    width: 14px;
    height: 22px;
    border-radius: 50% 50% 40% 40%;
    background: radial-gradient(circle at 50% 70%, #fff3c4, var(--brass-bright) 45%, transparent 70%);
    animation: flicker 1.4s infinite alternate;
  }

  @keyframes flicker {
    from {
      transform: scale(1) rotate(-2deg);
      opacity: 0.9;
    }
    to {
      transform: scale(1.12) rotate(3deg);
      opacity: 1;
    }
  }
</style>

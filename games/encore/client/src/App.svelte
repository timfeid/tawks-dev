<script lang="ts">
  import { connect, type Activity } from '@tawks/activity-sdk';
  import { onDestroy, untrack } from 'svelte';
  import Lobby from './components/Lobby.svelte';
  import Results from './components/Results.svelte';
  import Setup from './components/Setup.svelte';
  import Stage from './components/Stage.svelte';
  import { RoomConnection } from './lib/room.svelte';
  import { host } from './lib/voice';
  import { readSettings, type Settings } from './lib/settings';
  import { presenceFor } from './lib/presence';
  import type { Pass, SongInfo } from './lib/types';

  const JOIN_ACTION = 'games.encore.join';

  let conn = $state<RoomConnection | null>(null);
  let problem = $state<string | null>(null);
  let catalog = $state<SongInfo[]>([]);
  let catalogFailed = $state(false);
  let catalogLoaded = $state(false);
  let setupOpen = $state(false);

  const settings: Settings = $derived(readSettings(conn?.settings));

  function isPass(value: unknown): value is Pass {
    return !!value && typeof value === 'object' && 'token' in value && 'ws_url' in value;
  }

  /** Outside a tawks frame, a pass can come from the address for local testing. */
  function passFromAddress(): Pass | null {
    const params = new URLSearchParams(location.search);
    const token = params.get('token');
    const ws = params.get('ws');
    const api = params.get('api');
    return token && ws && api ? { token, ws_url: ws, api_url: api, lobby_id: params.get('lobby') ?? 'dev' } : null;
  }

  function nextPass(activity: Activity, previous: string | null): Promise<Pass> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        stop();
        reject(new Error('No pass arrived'));
      }, 15_000);
      const stop = activity.onState((state) => {
        if (isPass(state) && state.token !== previous) {
          clearTimeout(timeout);
          stop();
          resolve(state);
        }
      });
      activity.call(JOIN_ACTION, { lobby_id: activity.lobby?.id ?? null }).catch(() => {});
    });
  }

  async function start() {
    const fromAddress = passFromAddress();
    if (fromAddress) {
      conn = new RoomConnection(fromAddress, () => Promise.reject(new Error('expired')));
      return;
    }
    try {
      const activity = await connect();
      host.activity = activity;
      const first = isPass(activity.state) ? activity.state : await nextPass(activity, null);
      conn = new RoomConnection(first, () => nextPass(activity, conn?.current.token ?? null));
    } catch (error) {
      problem = (error as Error).message;
    }
  }

  void start();
  onDestroy(() => conn?.close());

  // Uploads come and go with the people in the room, so the list follows it.
  const roster = $derived(
    (conn?.room?.players ?? [])
      .filter((p) => !p.away)
      .map((p) => p.user_id)
      .sort()
      .join(','),
  );

  $effect(() => {
    if (!conn || conn.status !== 'online') return;
    void roster;
    const current = conn;
    const timer = setTimeout(() => void loadCatalog(current), untrack(() => catalogLoaded) ? 400 : 0);
    return () => clearTimeout(timer);
  });

  async function loadCatalog(conn: RoomConnection) {
    for (let attempt = 0; attempt < 4; attempt++) {
      const pass = conn.current;
      try {
        const response = await fetch(`${pass.api_url}/songs?token=${encodeURIComponent(pass.token)}`);
        if (response.ok) {
          catalog = (await response.json()) as SongInfo[];
          catalogFailed = false;
          catalogLoaded = true;
          return;
        }
      } catch {
        // Retried below.
      }
      await new Promise((r) => setTimeout(r, 1000 * 2 ** attempt));
    }
    if (!catalogLoaded) catalogFailed = true;
  }

  const presence = $derived(JSON.stringify(presenceFor(conn?.room ?? null, conn?.conn ?? null)));
  $effect(() => {
    host.activity?.setPresence(JSON.parse(presence));
  });

  const phase = $derived(conn?.room?.phase ?? null);
  const inBand = $derived(!!conn?.me?.playing);
</script>

<main class="relative h-full w-full select-none">
  {#if problem}
    <div class="flex h-full items-center justify-center text-sm text-neutral-400">Encore could not start: {problem}</div>
  {:else if !conn || !conn.room}
    <div class="flex h-full items-center justify-center text-sm text-neutral-400">
      {conn?.status === 'offline' ? 'Reconnecting to the band...' : 'Tuning up...'}
    </div>
  {:else if (phase === 'loading' || phase === 'playing') && inBand}
    <Stage {conn} {settings} />
  {:else if phase === 'results'}
    <Results {conn} />
  {:else}
    <Lobby
      {conn}
      {catalog}
      {catalogFailed}
      {catalogLoaded}
      {settings}
      onSetup={() => (setupOpen = true)}
      onCatalogChange={() => conn && loadCatalog(conn)}
    />
  {/if}

  {#if setupOpen && conn}
    <Setup {conn} {settings} onClose={() => (setupOpen = false)} />
  {/if}

  {#if conn?.error}
    <div class="pointer-events-none absolute top-4 left-1/2 -translate-x-1/2 rounded-lg bg-red-500/90 px-4 py-2 text-sm font-medium text-white shadow-lg">
      {conn.error}
    </div>
  {/if}
</main>

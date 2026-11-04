<script lang="ts">
  import { Badge } from '@tawks/ui/badge/index.js';
  import { Button } from '@tawks/ui/button/index.js';
  import { Progress } from '@tawks/ui/progress/index.js';
  import { Maximize2, Minimize2, Skull, Volume2, VolumeX } from '@lucide/svelte';
  import type { Hud, Meta, Seat } from './contract';
  import type { SourceStatus } from './connection';
  import { keyName, settings } from './party/settings';
  import { PALETTE } from './renderer';

  let {
    hud,
    meta,
    roster,
    mySeat,
    status,
    colorOf,
    showBrief,
    ended = false,
    muted,
    fullscreen,
    ontogglemute,
    ontogglefullscreen,
  }: {
    hud: Hud | null;
    meta: Meta | null;
    roster: Seat[];
    mySeat: number | null;
    status: SourceStatus;
    colorOf: (seat: number) => string;
    showBrief: boolean;
    /** The podium is up, so the bottom of the screen is left to it. */
    ended?: boolean;
    muted: boolean;
    fullscreen: boolean;
    ontogglemute: () => void;
    ontogglefullscreen?: () => void;
  } = $props();

  const scores = $derived(new Map((hud?.scores ?? []).map((score) => [score.seat, score])));
  const objective = $derived(hud?.objective ?? meta?.goal ?? meta?.rules ?? '');
  const goal = $derived(meta?.goal ?? hud?.objective ?? null);
  const nameOf = (seat: number) => roster.find((member) => member.seat === seat)?.name ?? `Seat ${seat}`;

  type Notice = { id: number; text: string; tone: 'out' | 'score' | 'back'; seat: number };
  let notices = $state<Notice[]>([]);
  let noticeId = 0;
  let before = new Map<number, { up?: boolean | null; score?: number | null }>();
  const NOTICE_MS = 2600;
  const meOut = $derived(mySeat !== null && scores.get(mySeat)?.up === false && !ended);

  function notice(text: string, tone: Notice['tone'], seat: number) {
    const id = ++noticeId;
    notices = [...notices.slice(-2), { id, text, tone, seat }];
    setTimeout(() => (notices = notices.filter((n) => n.id !== id)), NOTICE_MS);
  }

  // Every game gets the same feedback when a player is knocked out or scores,
  // whatever its own view shows.
  $effect(() => {
    const next = new Map((hud?.scores ?? []).map((s) => [s.seat, { up: s.up, score: s.score }]));
    for (const [seat, now] of next) {
      const was = before.get(seat);
      if (!was) continue;
      const me = seat === mySeat;
      if (was.up === true && now.up === false) notice(me ? 'You are out' : `${nameOf(seat)} is out`, 'out', seat);
      else if (was.up === false && now.up === true && me) notice('You are back in', 'back', seat);
      // A score that ticks up continuously (time survived) is not news.
      if (typeof was.score === 'number' && typeof now.score === 'number' && now.score - was.score >= 1) {
        const gain = Math.round((now.score - was.score) * 100) / 100;
        notice(me ? `You scored +${gain}` : `${nameOf(seat)} +${gain}`, 'score', seat);
      }
    }
    before = next;
  });
  const clock = (seconds: number) => {
    const whole = Math.max(0, Math.ceil(seconds));
    return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`;
  };
  const keys = $derived(settings().keys);
  const cap = (code: string | undefined) => (code ? keyName(code) : '');
</script>

<div class="made-hud pointer-events-none absolute inset-0 select-none font-sans text-white" style="--hud-accent:#fcd34d">
  <header class="match-hud absolute left-0 top-0 max-w-[min(36rem,calc(100%-7rem))]">
    <div class="match-hud__title">
      <span class="match-hud__status {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
      <!-- A still drawn for the agent has no meta, so its objective stands in for the name. -->
      <span class="truncate">{meta?.name ?? objective ?? 'Minigame'}</span>
      {#if meta && objective}<span class="match-hud__objective max-md:hidden">{objective}</span>{/if}
    </div>
    {#if roster.length}
      <ol class="match-hud__roster" aria-label="Players">
        {#each roster as member (member.seat)}
          {@const score = scores.get(member.seat)}
          <li class="match-hud__player" class:match-hud__player--me={member.seat === mySeat} class:match-hud__player--out={score?.up === false}>
            <span class="match-hud__swatch" style="background:{colorOf(member.seat)}; border-color:{colorOf(member.seat)}"></span>
            <span class="max-w-28 truncate font-bold">{member.name}</span>
            {#if member.bot && !/\bbot\b/i.test(member.name)}<Badge variant="glass">Bot</Badge>{/if}
            {#if score?.up === false}<Skull class="size-3 text-white/60" aria-label="Out" />{/if}
            {#if score?.label}
              <output class="ml-auto truncate tabular-nums text-amber-200">{score.label}</output>
            {:else if score?.score !== undefined}
              <output class="ml-auto tabular-nums text-amber-200">{score.score}</output>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  </header>

  {#if hud?.timer !== null && hud?.timer !== undefined}
    <div class="made-clock absolute left-1/2 top-3 -translate-x-1/2 text-center">
      <output class="hud-clock text-2xl font-black tabular-nums" class:hud-clock--danger={hud.timer <= 10}>{clock(hud.timer)}</output>
      {#if hud.alert}<div class="made-alert--top mt-1 text-xs font-bold uppercase tracking-wider text-rose-200 drop-shadow">{hud.alert}</div>{/if}
    </div>
  {:else if hud?.alert}
    <div class="made-clock made-alert--top absolute left-1/2 top-4 -translate-x-1/2 text-xs font-bold uppercase tracking-wider text-rose-200 drop-shadow">{hud.alert}</div>
  {/if}

  <div class="pointer-events-auto absolute right-3 top-3 flex gap-2">
    <Button variant="outline" size="icon" aria-label={muted ? 'Sound on' : 'Mute'} title={muted ? 'Sound on' : 'Mute'} onclick={ontogglemute}>
      {#if muted}<VolumeX />{:else}<Volume2 />{/if}
    </Button>
    {#if ontogglefullscreen}
      <Button variant="outline" size="icon" aria-label={fullscreen ? 'Leave fullscreen' : 'Play fullscreen'} title={fullscreen ? 'Leave fullscreen' : 'Play fullscreen'} onclick={ontogglefullscreen}>
        {#if fullscreen}<Minimize2 />{:else}<Maximize2 />{/if}
      </Button>
    {/if}
  </div>

  {#if (notices.length || (hud?.alert && !ended)) && !(showBrief && meta)}
    <ol class="absolute bottom-24 left-1/2 flex w-max max-w-[calc(100%-2rem)] -translate-x-1/2 flex-col items-center gap-1" aria-live="polite">
      {#if hud?.alert && !ended}<li class="made-notice made-notice--out made-alert--bottom">{hud.alert}</li>{/if}
      {#each notices as item (item.id)}
        <li class="made-notice made-notice--{item.tone}" class:made-notice--me={item.seat === mySeat}>
          <span class="match-hud__swatch" style="background:{colorOf(item.seat)}; border-color:{colorOf(item.seat)}"></span>{item.text}
        </li>
      {/each}
    </ol>
  {/if}

  {#if meOut}
    <div class="made-out absolute inset-x-0 bottom-14 flex justify-center">
      <div><Skull class="size-4" aria-hidden="true" /> You are out. Watch the rest of the round.</div>
    </div>
  {/if}

  {#if showBrief && meta}
    <div class="game-brief absolute bottom-4 left-1/2 w-[min(32rem,calc(100%-2rem))] -translate-x-1/2 p-4">
      <div class="text-[11px] font-bold uppercase tracking-[0.16em] text-white/50">How to play {meta.name}</div>
      {#if goal}<div class="mt-0.5 text-lg font-black leading-snug">{goal}</div>{/if}
      {#if mySeat !== null}
        <div class="mt-1.5 flex items-center gap-2 text-sm font-bold">
          <span class="made-you" style="background:{colorOf(mySeat)}">YOU</span>
          That tag in your colour marks your character.
        </div>
      {/if}
      {#if meta.win || meta.lose}
        <div class="mt-1.5 grid gap-0.5 text-sm">
          {#if meta.win}<div><span class="font-bold text-emerald-300">Win:</span> {meta.win}</div>{/if}
          {#if meta.lose}<div><span class="font-bold text-rose-300">Out:</span> {meta.lose}</div>{/if}
        </div>
      {:else}
        <div class="mt-1 text-sm text-amber-200">{meta.rules}</div>
      {/if}
      <div class="mt-3 flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-white/80">
        {#if meta.steer === 'tank'}
          <span class="flex items-center gap-1.5"><kbd class="made-key font-mono">{cap(keys.up[0])}</kbd> Throttle</span>
          <span class="flex items-center gap-1.5"><kbd class="made-key font-mono">{cap(keys.down[0])}</kbd> Reverse</span>
          <span class="flex items-center gap-1.5">
            <span class="flex gap-0.5"><kbd class="made-key font-mono">{cap(keys.left[0])}</kbd><kbd class="made-key font-mono">{cap(keys.right[0])}</kbd></span>
            Steer
          </span>
        {:else}
          <span class="flex items-center gap-1.5">
            <span class="flex gap-0.5">
              {#each [keys.up[0], keys.left[0], keys.down[0], keys.right[0]] as code, index (index)}
                <kbd class="made-key font-mono">{cap(code)}</kbd>
              {/each}
            </span>
            Move
          </span>
        {/if}
        {#if meta.controls === 'move_action' && !meta.unused?.includes('action')}
          <span class="flex items-center gap-1.5"><kbd class="made-key font-mono">{cap(keys.action[0])}</kbd> or click to act</span>
        {:else if meta.controls === 'move_aim'}
          <span class="flex items-center gap-1.5"><kbd class="made-key">Mouse</kbd> Aim</span>
          <span class="flex items-center gap-1.5"><kbd class="made-key font-mono">{cap(keys.action[0])}</kbd> or click to act</span>
        {/if}
        <span class="text-white/45">A controller works too.</span>
      </div>
    </div>
  {/if}

  {#if ended}
    <!-- The podium panel holds the bottom. -->
  {:else if hud?.meter}
    <footer class="match-objective absolute bottom-4 left-1/2 -translate-x-1/2">
      <span>{hud.meter.label}</span>
      <div class="w-28" style="--meter:{PALETTE[hud.meter.color]}">
        <Progress value={hud.meter.value * 100} progressClass="bg-[var(--meter)]" aria-label={hud.meter.label} />
      </div>
    </footer>
  {:else if objective && !showBrief}
    <footer class="match-objective absolute bottom-4 left-1/2 -translate-x-1/2"><span>OBJECTIVE</span>{objective}</footer>
  {/if}
</div>

<style>
  .made-hud {
    container-type: size;
  }
  .match-hud {
    filter: drop-shadow(0 8px 14px rgb(2 6 23 / 0.28));
  }
  .match-hud__title {
    display: flex;
    min-width: 16rem;
    align-items: center;
    gap: 0.55rem;
    padding: 0.55rem 2.2rem 0.55rem 0.9rem;
    background: linear-gradient(100deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.82));
    border-bottom: 1px solid color-mix(in srgb, var(--hud-accent) 75%, transparent);
    clip-path: polygon(0 0, 100% 0, calc(100% - 1.2rem) 100%, 0 100%);
    font-size: 0.78rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .match-hud__status {
    width: 0.5rem;
    height: 0.5rem;
    flex: none;
    box-shadow: 0 0 0.7rem currentColor;
  }
  .match-hud__objective {
    overflow: hidden;
    margin-left: 0.25rem;
    color: rgb(255 255 255 / 0.48);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .match-hud__roster {
    display: grid;
    width: min(28rem, calc(100cqw - 8rem));
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
    background: rgb(255 255 255 / 0.08);
  }
  .match-hud__player {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.45rem;
    padding: 0.35rem 0.7rem;
    background: linear-gradient(90deg, rgb(8 15 29 / 0.86), rgb(15 23 42 / 0.72));
    border-bottom: 1px solid transparent;
    font-size: 0.7rem;
  }
  .match-hud__player--me {
    border-bottom-color: var(--hud-accent);
    background: linear-gradient(90deg, color-mix(in srgb, var(--hud-accent) 18%, rgb(8 15 29 / 0.9)), rgb(15 23 42 / 0.72));
  }
  .match-hud__player--out {
    filter: grayscale(0.8);
    opacity: 0.42;
  }
  .match-hud__swatch {
    width: 0.58rem;
    height: 0.58rem;
    flex: none;
    border: 2px solid;
    transform: rotate(45deg);
  }
  .hud-clock {
    display: block;
    min-width: 6.5rem;
    padding: 0.28rem 1.35rem 0.42rem;
    border-top: 2px solid var(--hud-accent);
    background: linear-gradient(180deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.76));
    clip-path: polygon(0.7rem 0, calc(100% - 0.7rem) 0, 100% 100%, 0 100%);
    filter: drop-shadow(0 6px 8px rgb(2 6 23 / 0.3));
  }
  .hud-clock--danger {
    border-top-color: #fb7185;
    color: #fda4af;
    animation: hud-alert 0.8s ease-in-out infinite alternate;
  }
  .game-brief {
    border-left: 3px solid var(--hud-accent);
    background: linear-gradient(100deg, rgb(4 12 24 / 0.92), rgb(4 12 24 / 0.55));
    filter: drop-shadow(0 4px 12px rgb(0 0 0 / 0.2));
  }
  .made-notice {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.3rem 0.9rem;
    background: rgb(8 15 29 / 0.82);
    border-left: 3px solid rgb(255 255 255 / 0.4);
    font-size: 0.8rem;
    font-weight: 800;
    animation: made-notice-in 0.18s ease-out;
  }
  .made-alert--bottom {
    display: none;
  }
  .made-notice--out {
    border-left-color: #fb7185;
  }
  .made-notice--score,
  .made-notice--back {
    border-left-color: #34d399;
  }
  .made-notice--me {
    font-size: 1rem;
    padding: 0.4rem 1.1rem;
  }
  .made-out > div {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 1.1rem;
    background: rgb(76 5 25 / 0.85);
    color: #ffe4e6;
    font-size: 0.85rem;
    font-weight: 800;
  }
  .made-you {
    padding: 0.05rem 0.45rem;
    border-radius: 999px;
    color: #0f172a;
    font-size: 0.7rem;
    font-weight: 900;
    letter-spacing: 0.06em;
  }
  @keyframes made-notice-in {
    from {
      opacity: 0;
      transform: translateY(-0.4rem);
    }
  }
  .made-key {
    border: 1px solid rgb(255 255 255 / 0.25);
    border-radius: 0.25rem;
    background: rgb(255 255 255 / 0.1);
    padding: 0.125rem 0.375rem;
    font-size: 11px;
    font-weight: 700;
  }
  .match-objective {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.35rem 1.2rem;
    color: rgb(255 255 255 / 0.72);
    border-bottom: 1px solid rgb(255 255 255 / 0.15);
    background: linear-gradient(90deg, transparent, rgb(8 15 29 / 0.78) 15%, rgb(8 15 29 / 0.78) 85%, transparent);
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    white-space: nowrap;
  }
  .match-objective span {
    color: var(--hud-accent);
    font-weight: 950;
    letter-spacing: 0.14em;
  }
  @keyframes hud-alert {
    to {
      filter: drop-shadow(0 0 0.6rem rgb(251 113 133 / 0.58));
    }
  }
  /* The game is often a pane beside the Builder's chat, so the HUD fits the game, not the window. */
  @container (max-width: 760px) {
    .match-hud__title {
      min-width: 10rem;
      padding-left: 0.65rem;
      font-size: 0.68rem;
    }
    .match-hud__roster {
      width: min(17rem, calc(100cqw - 10.5rem));
    }
    .match-hud__player {
      gap: 0.3rem;
      padding: 0.25rem 0.45rem;
      font-size: 0.62rem;
    }
    .hud-clock {
      min-width: 5rem;
      font-size: 1.15rem;
    }
    .made-alert--top {
      display: none;
    }
    .match-objective {
      max-width: calc(100cqw - 1.5rem);
      white-space: normal;
      text-align: center;
    }
    .made-alert--bottom {
      display: flex;
    }
    /* Beside the roster there is no room in the middle, so the clock moves under the buttons. */
    .made-clock {
      left: auto;
      right: 0.75rem;
      top: 3.6rem;
      max-width: 9rem;
      transform: none;
      text-align: right;
    }
  }
</style>

<script lang="ts">
  import { ChevronsDown, Bot, ChevronLeft, ChevronRight, CircleDot, Coins, Crosshair, Crown, Dices, Eye, Gamepad2, Gem, Heart, LoaderCircle, Maximize2, Minimize2, MousePointerClick, Play, Plus, Skull, Swords, Target, Trophy, Volume2, VolumeX, X, Zap } from '@lucide/svelte';
  import type { BoutHud } from '@tawks/bout-sdk';
  import type { StickValue } from './input';
  import Joystick from './joystick.svelte';
  import type { BoutControls, CharacterDef, ClientMsg, Item, RoomView } from './protocol';
  import type { ConnectionStatus } from './connection';

  let {
    room,
    roster,
    me,
    spectating,
    status,
    hasPass,
    timeLeft,
    showGo,
    muted,
    filming = false,
    guest = false,
    touch,
    overview,
    fullscreen,
    pointerLocked,
    hud,
    firstPerson,
    errorMessage,
    send,
    toggleMute,
    toggleOverview,
    toggleFullscreen,
    onaim,
    onlook,
    onfire,
    oncrouch,
    onstick,
  }: {
    room: RoomView | null;
    roster: CharacterDef[];
    me: string | null;
    spectating: boolean;
    status: ConnectionStatus;
    hasPass: boolean;
    timeLeft: number | null;
    showGo: boolean;
    muted: boolean;
    filming?: boolean;
    guest?: boolean;
    touch: boolean;
    overview: boolean;
    fullscreen: boolean;
    pointerLocked: boolean;
    hud: BoutHud | null;
    firstPerson: boolean;
    errorMessage: string | null;
    send: (msg: ClientMsg) => void;
    toggleMute: () => void;
    toggleOverview: () => void;
    toggleFullscreen: () => void;
    onaim: (item: Item | null) => void;
    onlook: (dx: number, dy: number) => void;
    onfire: (down: boolean) => void;
    oncrouch: (down: boolean) => void;
    onstick: (value: StickValue) => void;
  } = $props();

  const byId = $derived(new Map(roster.map((c) => [c.id, c])));
  const mine = $derived(room?.members.find((m) => m.user_id === me) ?? null);
  const isHost = $derived(!!me && room?.host_id === me);
  const host = $derived(room?.members.find((m) => m.user_id === room?.host_id) ?? null);
  const phase = $derived(room?.phase.name ?? 'lobby');
  const seatsFull = $derived(!!room && room.members.length >= room.max_seats);
  const bySeat = (seat: number) => room?.members.find((m) => m.seat === seat);
  const bouts = $derived(new Map((room?.bouts ?? []).map((bout) => [bout.key, bout])));
  const allGames = $derived([
    ...(room?.bouts ?? []).map((bout) => [bout.key, bout.name] as [string, string]),
    ...(room?.table_games ?? []),
  ]);
  const gameChoice = $derived(allGames.find(([key]) => key === room?.minigame) ?? null);
  const tableGame = $derived((room?.table_games ?? []).some(([key]) => key === room?.minigame));
  const TABLE_MIN: Record<string, number> = { coven: 4, landlord: 2 };
  const tableMin = $derived(TABLE_MIN[room?.minigame ?? ''] ?? 2);
  const people = $derived((room?.members ?? []).filter((m) => !m.is_bot).length);
  const TABLE_COPY: Record<string, string> = {
    coven: 'Hidden witches, whispered accusations, four to twelve players at one table.',
    landlord: 'Deal, rent and steal your way to three districts. Two to five players.',
  };
  const gameCopy = (key: string) => bouts.get(key)?.rules ?? TABLE_COPY[key] ?? 'A quick bout.';
  const CONTROLS: Record<BoutControls, string> = {
    move: 'Move',
    move_action: 'Move and dash',
    move_aim: 'Move and aim',
    first_person: 'First person',
  };
  const vote = $derived(room?.phase.name === 'vote' ? room.phase : null);
  const myVote = $derived(vote && mine ? (vote.votes.find((v) => v.seat === mine.seat)?.key ?? null) : null);
  const canVote = $derived(!!vote && !!mine?.in_round && !spectating);
  function cycleGame(change: number) {
    const games = allGames;
    const index = games.findIndex(([key]) => key === room?.minigame);
    if (!games.length) return;
    const next = games[(index + change + games.length) % games.length];
    if (next) send({ type: 'choose_game', key: next[0] });
  }

  const board = $derived(room?.board ?? null);
  const myToken = $derived(board && mine ? (board.tokens.find((t) => t.seat === mine.seat) ?? null) : null);
  const myTurn = $derived(!!board && !!mine && board.turn_seat === mine.seat && room?.phase.name === 'board');
  const turnHolder = $derived(board ? (bySeat(board.turn_seat)?.name ?? 'Someone') : '');
  const rivals = $derived(board && mine ? board.tokens.filter((t) => t.seat !== mine.seat) : []);
  const ITEM_HELP: Record<Item, string> = {
    bat: 'Knock a rival back three spaces and take a heart.',
    bomb: 'Blast everyone near a rival for coins and a heart.',
    grapple: 'Steal coins from a rival.',
    trap: 'Leave a trap on your space.',
  };
  let aiming = $state<Item | null>(null);
  const reachOf = (item: Item) => board?.reach.find((r) => r.item === item)?.spaces ?? null;
  const gapTo = (seat: number) => board?.aim.find((a) => a.seat === seat)?.spaces ?? null;
  const inReach = (item: Item, seat: number) => {
    const reach = reachOf(item);
    const spaces = gapTo(seat);
    return reach === null || spaces === null || spaces <= reach;
  };
  const spacesLabel = (spaces: number | null) =>
    spaces === null ? '' : spaces === 0 ? 'same space' : `${spaces} ${spaces === 1 ? 'space' : 'spaces'}`;
  $effect(() => onaim(aiming));

  const SPACE_COLORS: Record<string, string> = {
    start: '#ffd23f',
    coin: '#4cc9f0',
    loss: '#ff5f6d',
    item: '#9b5de5',
    event: '#62c370',
    shop: '#ffa62b',
    hazard: '#7f1d3a',
    teleport: '#22d3ee',
    prize_spot: '#f8f0d0',
  };
  const SPACE_LABELS: Record<string, string> = {
    start: 'onto the start',
    coin: 'onto a coin space',
    loss: 'onto a rough space',
    item: 'onto an item space',
    event: 'onto a toll space',
    shop: 'past the shop',
    hazard: 'through the spikes',
    teleport: 'onto the teleport',
    prize_spot: 'toward the prize',
  };
  const BRANCH_LETTERS = ['A', 'B', 'C'];

  const inArena = $derived(phase === 'playing' || phase === 'countdown');
  const scoreOf = (seat: number) => hud?.scores?.find((s) => s.seat === seat) ?? null;
  const combat = $derived(hud?.combat ?? null);
  const hudAccent = $derived(byId.get(mine?.character ?? '')?.accent ?? '#facc15');
  const objective = $derived(hud?.objective ?? 'Outlast the field');

  let looking = $state(false);
  let lookAt: { x: number; y: number } | null = null;
  function lookStart(event: PointerEvent) {
    looking = true;
    lookAt = { x: event.clientX, y: event.clientY };
    (event.target as HTMLElement).setPointerCapture(event.pointerId);
  }
  function lookMove(event: PointerEvent) {
    if (!looking || !lookAt) return;
    onlook(event.clientX - lookAt.x, event.clientY - lookAt.y);
    lookAt = { x: event.clientX, y: event.clientY };
  }
  function lookEnd() {
    looking = false;
    lookAt = null;
  }

  function pump(side: -1 | 1) {
    onstick({ x: side, z: 0 });
    setTimeout(() => onstick({ x: 0, z: 0 }), 55);
  }

  const onBoard = $derived(!!board && (phase === 'board' || phase === 'vote' || phase === 'game_over'));
  const mapBox = $derived.by(() => {
    if (!board) return null;
    const xs = board.spaces.map((space) => space.x);
    const zs = board.spaces.map((space) => space.z);
    const minX = Math.min(...xs) - 2;
    const minZ = Math.min(...zs) - 2;
    return { minX, minZ, w: Math.max(...xs) + 2 - minX, h: Math.max(...zs) + 2 - minZ };
  });
  const spaceAt = (id: number) => board?.spaces.find((space) => space.id === id) ?? null;

  const placements = $derived(room?.phase.name === 'results' ? room.phase.placements : []);
  const headline = $derived.by(() => {
    const winners = (placements[0] ?? []).map((seat) => bySeat(seat)?.name ?? 'Someone');
    if (!winners.length) return 'Nobody stayed on';
    if (winners.length === 1) return `${winners[0]} wins!`;
    return `${winners.slice(0, -1).join(', ')} and ${winners[winners.length - 1]} tie!`;
  });
  const ordinal = (n: number) => ['1st', '2nd', '3rd', '4th'][n] ?? `${n + 1}th`;
  const clock = (seconds: number) => `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;

  const recording = $derived(room?.recording ?? null);
  let now = $state(Date.now() / 1000);
  $effect(() => {
    if (!recording) return;
    const timer = setInterval(() => (now = Date.now() / 1000), 1000);
    return () => clearInterval(timer);
  });
  const recordedFor = $derived(recording ? clock(Math.max(0, Math.floor(now - recording.since))) : '');
  const canRecord = $derived(!!mine && !spectating && !guest);

  const panel = 'rounded-2xl border border-white/15 bg-slate-900/80 text-white shadow-xl backdrop-blur';
</script>

<div class="pointer-events-none absolute inset-0 select-none font-sans" style="--hud-accent:{hudAccent}">
  {#if inArena && !onBoard}
    <header class="match-hud absolute left-0 top-0 max-w-[min(36rem,calc(100%-7rem))] text-white">
      <div class="match-hud__title">
        <span class="match-hud__status {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
        <span class="truncate">{room?.minigame_name ?? 'Bout'}</span>
        <span class="match-hud__objective max-md:hidden">{objective}</span>
      </div>
      <ol class="match-hud__roster" aria-label="Players">
        {#if hud?.teams}
          {#each hud.teams as teamName, team (teamName)}
            <li class="team-rail team-rail--{team}">
              <span class="team-rail__name">{teamName}</span>
              {#each (room?.members ?? []).filter((member) => scoreOf(member.seat)?.team === team) as member (member.user_id)}
                {@const character = byId.get(member.character)}
                {@const score = scoreOf(member.seat)}
                <span class:team-rail__member--out={score && !score.up} class:team-rail__member--me={member.user_id === me} class="team-rail__member">
                  <span class="match-hud__swatch" style="background:{character?.body}; border-color:{character?.accent}"></span>
                  <span class="max-w-24 truncate">{member.name}</span>
                  {#if !score?.up}<Skull class="size-3" />{/if}
                </span>
              {/each}
            </li>
          {/each}
        {:else}
          {#each room?.members ?? [] as member (member.user_id)}
            {@const character = byId.get(member.character)}
            {@const score = scoreOf(member.seat)}
            <li
              class:match-hud__player--me={member.user_id === me}
              class:match-hud__player--out={score && !score.up}
              class="match-hud__player"
            >
              <span class="match-hud__swatch" style="background:{character?.body}; border-color:{character?.accent}"></span>
              <span class="max-w-28 truncate font-bold">{member.name}</span>
              {#if member.is_bot}<span class="text-[9px] font-black uppercase tracking-wider text-white/45">bot</span>{/if}
              {#if firstPerson && score}
                <output class="ml-auto tabular-nums text-amber-200">{score.hits}</output>
                {#if score.downs !== undefined}<Skull class="size-3 text-white/35" />{/if}
              {:else}
                <output class="ml-auto tabular-nums text-amber-200">{member.wins}</output>
              {/if}
            </li>
          {/each}
        {/if}
      </ol>
    </header>
  {:else if phase === 'lobby' && room}
    <header class="lobby-hud absolute left-0 top-0 text-white">
      <div class="lobby-hud__title">
        <span class="match-hud__status {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
        <span class="truncate">{room.minigame_name}</span>
        <span class="lobby-hud__state">LOBBY</span>
      </div>
      <div class="lobby-hud__players">
        <strong>{room.members.length}/{room.max_seats}</strong>
        {#each room.members as member (member.user_id)}
          {@const character = byId.get(member.character)}
          <span class="lobby-hud__player" title={member.name} aria-label={member.name} style="background:{character?.body}; border-color:{character?.accent}"></span>
        {/each}
        <span class="text-[10px] font-bold uppercase tracking-[0.12em] text-white/45">players ready</span>
      </div>
    </header>
  {:else if (phase !== 'lobby' && phase !== 'table') || !room}
  <div class="pointer-events-auto absolute left-3 top-3 w-60 {panel} p-3">
    <div class="flex items-center gap-2">
      <span class="size-2.5 rounded-full {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
      <span class="text-sm font-bold tracking-wide">{onBoard ? 'Tawkathon' : (room?.minigame_name ?? 'Bout')}</span>
    </div>
    <div class="mt-2 flex flex-col gap-1">
      {#each room?.members ?? [] as member (member.user_id)}
        {@const character = byId.get(member.character)}
        <div class="flex items-center gap-2 rounded-lg px-1.5 py-1 text-sm {member.connected ? '' : 'opacity-45'} {placements[0]?.includes(member.seat) ? 'bg-amber-300/20' : ''}">
          <span class="size-3.5 shrink-0 rounded-full border-2" style="background:{character?.body}; border-color:{character?.accent}"></span>
          <span class="truncate {member.user_id === me ? 'font-bold' : ''}">{member.name}</span>
          {#if member.user_id === room?.host_id}<Crown class="size-3.5 shrink-0 text-amber-300" aria-label="Host" />{/if}
          {#if member.is_bot}<span class="rounded bg-white/15 px-1 text-[10px] font-semibold uppercase">Bot</span>{/if}
          {#if board}
            {@const token = board.tokens.find((t) => t.seat === member.seat)}
            {#if token}
              <span class="ml-auto flex items-center gap-1 tabular-nums text-pink-300"><Gem class="size-3.5" />{token.prizes}</span>
              <span class="flex items-center gap-1 tabular-nums text-amber-200"><Coins class="size-3.5" />{token.coins}</span>
              <span class="flex items-center gap-0.5 tabular-nums text-rose-300"><Heart class="size-3.5" />{token.health}</span>
            {/if}
          {:else if firstPerson && (inArena || phase === 'results') && scoreOf(member.seat)}
            {@const score = scoreOf(member.seat)}
            <span class="ml-auto flex items-center gap-1 tabular-nums text-amber-200"><Target class="size-3.5" />{score?.hits ?? 0}</span>
            <span class="flex items-center gap-1 tabular-nums text-white/50"><Skull class="size-3.5" />{score?.downs ?? 0}</span>
          {:else}
            <span class="ml-auto flex items-center gap-1 tabular-nums text-amber-200"><Trophy class="size-3.5" />{member.wins}</span>
          {/if}
          {#if member.is_bot && isHost && phase === 'lobby'}
            <button class="rounded p-0.5 text-white/60 hover:bg-white/10 hover:text-white" aria-label="Remove {member.name}" onclick={() => send({ type: 'remove_bot', seat: member.seat })}>
              <X class="size-3.5" />
            </button>
          {/if}
        </div>
      {/each}
      {#if !room}
        <div class="flex items-center gap-2 py-1 text-sm text-white/70">
          <LoaderCircle class="size-4 animate-spin" />
          {hasPass ? 'Joining the room' : 'Setting up the room'}
        </div>
      {/if}
    </div>
  </div>
  {/if}

  {#if !filming}
  <div class="pointer-events-auto absolute right-3 top-3 flex items-center gap-2">
    {#if recording}
      <button
        class="{inArena ? 'hud-control' : panel + ' px-3 py-2 hover:bg-slate-800/90'} flex items-center gap-2 text-sm font-bold"
        title={canRecord ? `Recording by ${recording.by}. Stop recording` : `Recording by ${recording.by}`}
        aria-label="Stop recording"
        disabled={!canRecord}
        onclick={() => send({ type: 'record', on: false })}
      >
        <span class="rec-dot size-2.5 rounded-full {recording.live ? 'bg-red-500' : 'bg-red-500/40'}"></span>
        {recording.live ? recordedFor : 'Starting'}
      </button>
    {:else if canRecord}
      <button
        class="{inArena ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}"
        title="Record this game to the channel's files"
        aria-label="Record this game"
        onclick={() => send({ type: 'record', on: true })}
      >
        <CircleDot class="size-5 text-red-400" />
      </button>
    {/if}
    <button class="{inArena ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}" aria-label={muted ? 'Turn sound on' : 'Turn sound off'} onclick={toggleMute}>
      {#if muted}<VolumeX class="size-5" />{:else}<Volume2 class="size-5" />{/if}
    </button>
    <button
      class="{inArena ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}"
      aria-label={fullscreen ? 'Leave fullscreen' : 'Play fullscreen'}
      title={fullscreen ? 'Leave fullscreen' : 'Play fullscreen'}
      onclick={toggleFullscreen}
    >
      {#if fullscreen}<Minimize2 class="size-5" />{:else}<Maximize2 class="size-5" />{/if}
    </button>
  </div>
  {/if}

  {#if timeLeft !== null}
    <div class="absolute left-1/2 top-3 -translate-x-1/2 text-center">
      <output class="{inArena ? 'hud-clock' : panel + ' px-5 py-1.5'} text-2xl font-black tabular-nums {timeLeft <= 10 ? 'hud-clock--danger text-rose-300' : ''}">{clock(timeLeft)}</output>
      {#if hud?.alert}<div class="mt-1 text-xs font-bold uppercase tracking-wider text-rose-200 drop-shadow">{hud.alert}</div>{/if}
    </div>
  {/if}

  {#if board && room?.phase.name === 'board'}
    <div class="absolute left-1/2 top-3 flex -translate-x-1/2 flex-col items-center gap-1">
      <div class="{panel} px-4 py-1.5 text-sm font-bold">Round {board.round} of {board.rounds}</div>
      <div class="rounded-full bg-slate-900/70 px-3 py-1 text-xs font-semibold text-white/90">
        {myTurn ? 'Your turn' : `${turnHolder} is playing`}{board.last_roll ? ` · rolled ${board.last_roll}` : ''}
      </div>
      <div class="flex items-center gap-1.5 rounded-full bg-pink-500/25 px-3 py-1 text-xs font-semibold text-pink-100">
        <Gem class="size-3.5" /> Most prizes after {board.rounds} rounds wins. Land on the pink gem with {board.prize_cost} coins to buy one.
      </div>
      <div class="flex items-center gap-1 rounded-full bg-slate-900/70 px-2 py-1 text-[11px] font-semibold text-white/70">
        <span class="px-1">Turn order</span>
        {#each board.order as seat, index (seat)}
          {@const member = bySeat(seat)}
          <span
            class="flex items-center gap-1 rounded-full px-2 py-0.5 {seat === board.turn_seat ? 'bg-white text-slate-900' : 'bg-white/10'}"
          >
            <span class="tabular-nums">{index + 1}</span>
            <span class="size-2.5 rounded-full" style="background:{byId.get(member?.character ?? '')?.body}"></span>
            {member?.name ?? 'Player'}
          </span>
        {/each}
      </div>
      {#if board.round === 1}
        <div class="rounded-full bg-slate-900/70 px-3 py-1 text-[11px] text-white/70">
          Win a bout to earn an item and go first next round.
        </div>
      {/if}
    </div>

    {#if myTurn}
      <div class="pointer-events-auto absolute bottom-3 left-1/2 w-[min(42rem,calc(100%-1.5rem))] -translate-x-1/2 {panel} p-4">
        {#if board.turn.step === 'branch'}
          <div class="text-sm font-bold">Which way? The two paths are marked on the board.</div>
          <div class="mt-2 flex flex-wrap gap-2">
            {#each board.turn.options as option, index (option.node)}
              <button
                class="flex items-center gap-2 rounded-xl bg-white/10 px-4 py-2 text-sm font-semibold hover:bg-white/20"
                onclick={() => send({ type: 'choose_branch', node: option.node })}
              >
                <span class="flex size-6 items-center justify-center rounded-full bg-white text-sm font-black text-slate-900">
                  {BRANCH_LETTERS[index] ?? '?'}
                </span>
                <span class="size-3 rounded-full" style="background:{SPACE_COLORS[option.kind]}"></span>
                {option.shortcut ? `Shortcut, saves ${option.saves} ${option.saves === 1 ? 'space' : 'spaces'}` : 'Stay on the lap'}
                <span class="text-white/60">{SPACE_LABELS[option.kind]}</span>
              </button>
            {/each}
          </div>
        {:else if board.turn.step === 'done'}
          <div class="flex items-center gap-3">
            <div class="text-sm text-white/80">That is your turn.</div>
            <button class="ml-auto rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300" onclick={() => send({ type: 'end_turn' })}>
              Hand it on
            </button>
          </div>
        {:else}
          <div class="flex flex-wrap items-center gap-2">
            {#if board.turn.step === 'items'}
              {#each myToken?.items ?? [] as item, index (index)}
                <button
                  class="flex items-center gap-1.5 rounded-xl px-3 py-2 text-sm font-semibold
                    {aiming === item ? 'bg-rose-500 text-white' : 'bg-white/10 hover:bg-white/20'}"
                  title={ITEM_HELP[item]}
                  onclick={() => {
                    if (item === 'trap') {
                      send({ type: 'use_item', item, target: null });
                      aiming = null;
                    } else {
                      aiming = aiming === item ? null : item;
                    }
                  }}
                >
                  <Swords class="size-4" />{item[0].toUpperCase() + item.slice(1)}
                  {#if reachOf(item) !== null}
                    <span class="text-xs font-normal text-white/60">reaches {reachOf(item)}</span>
                  {/if}
                </button>
              {/each}
              {#if !(myToken?.items ?? []).length}
                <span class="text-sm text-white/60">No items yet. Win a bout or land on a purple space to earn one.</span>
              {/if}
            {/if}
            <button
              class="ml-auto flex items-center gap-1.5 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300"
              onclick={() => {
                aiming = null;
                send({ type: 'roll' });
              }}
            >
              <Dices class="size-4" /> Roll
            </button>
          </div>
          {#if aiming}
            {@const reach = reachOf(aiming)}
            <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-white/10 pt-3">
              <span class="text-sm text-white/80">
                {ITEM_HELP[aiming]}
                {#if reach !== null}It reaches {spacesLabel(reach)}.{/if}
              </span>
              {#each rivals as rival (rival.seat)}
                {@const member = bySeat(rival.seat)}
                {@const spaces = gapTo(rival.seat)}
                {@const reachable = inReach(aiming, rival.seat)}
                <button
                  class="flex items-center gap-1.5 rounded-xl px-3 py-1.5 text-sm font-semibold
                    {reachable ? 'bg-white/10 hover:bg-white/20' : 'cursor-not-allowed bg-white/5 text-white/40'}"
                  disabled={!reachable}
                  title={reachable ? `${spacesLabel(spaces)} away` : 'Out of reach'}
                  onclick={() => {
                    send({ type: 'use_item', item: aiming!, target: rival.seat });
                    aiming = null;
                  }}
                >
                  <span class="size-3 rounded-full {reachable ? '' : 'opacity-40'}" style="background:{byId.get(member?.character ?? '')?.body}"></span>
                  {member?.name ?? 'Player'}
                  <span class="tabular-nums {reachable ? 'text-white/60' : 'text-rose-300/80'}">
                    {spacesLabel(spaces)}{reachable ? '' : ' away, too far'}
                  </span>
                </button>
              {/each}
              <button class="rounded-xl px-3 py-1.5 text-sm text-white/70 hover:text-white" onclick={() => (aiming = null)}>Cancel</button>
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  {/if}

  {#if firstPerson && inArena && mine?.in_round && !spectating}
    {#if combat?.up}
      <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
        <div class="relative size-8">
          <span class="absolute left-1/2 top-1/2 size-1 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white shadow"></span>
          {#each [[0, -14], [0, 14], [-14, 0], [14, 0]] as [dx, dy] (dx + ':' + dy)}
            <span
              class="absolute left-1/2 top-1/2 rounded-full bg-white/85 {dx === 0 ? 'h-2.5 w-0.5' : 'h-0.5 w-2.5'}"
              style="transform: translate(calc(-50% + {dx}px), calc(-50% + {dy}px))"
            ></span>
          {/each}
          {#if hud?.hitMarker}
            <span class="absolute inset-0 flex items-center justify-center text-2xl font-black text-rose-300">x</span>
          {/if}
        </div>
      </div>
    {:else if combat}
      <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 {panel} px-6 py-4 text-center">
        <div class="text-lg font-black text-rose-300">{combat.outTitle}</div>
        <div class="mt-1 text-sm text-white/80">
          {combat.back === null ? 'Watch your team finish the round' : `Back in ${Math.max(1, Math.ceil(combat.back))}`}
        </div>
      </div>
    {/if}

    {#if combat}
      <aside class="combat-hud absolute bottom-4 left-4 text-white">
        {#if combat.meter === 'score'}
          <div class="combat-hud__label"><Target class="size-4" /> SCORE</div>
          <output class="combat-hud__value text-emerald-200">{combat.hits}</output>
          <div class="combat-hud__hint">Find the lit target</div>
        {:else if combat.meter === 'health'}
          <div class="combat-hud__label"><Heart class="size-4 text-rose-300" /> HEALTH</div>
          <div class="flex items-end gap-3">
            <output class="combat-hud__value">{Math.max(0, combat.hp)}</output>
            <div class="mb-2 h-2 w-32 overflow-hidden bg-white/15 max-md:w-20">
              <div
                class="h-full transition-[width] {combat.hp > combat.hpMax / 3 ? 'bg-emerald-400' : 'bg-rose-400'}"
                style="width: {Math.max(0, (combat.hp / Math.max(1, combat.hpMax)) * 100)}%"
              ></div>
            </div>
          </div>
        {:else}
          <div class="combat-hud__label {combat.holding ? 'text-amber-200' : 'text-white/60'}">
            <CircleDot class="size-5" />
            {combat.holding ? 'BALL READY' : 'FIND A BALL'}
          </div>
        {/if}
        {#if combat.meter !== 'score'}
          <div class="combat-hud__stats">
            <span class="flex items-center gap-1.5 font-bold text-amber-200">
              <Target class="size-4" />{combat.hits} {combat.hitsLabel}
            </span>
            <span class="flex items-center gap-1.5 text-white/60"><Skull class="size-4" />{combat.downs}</span>
          </div>
        {/if}
      </aside>

      <footer class="match-objective absolute bottom-4 left-1/2 -translate-x-1/2">
        {#if hud?.meter}
          <span>{hud.meter.label}</span>
          <div class="objective-meter"><i style="width:{hud.meter.progress * 100}%"></i></div>
          {hud.meter.text}
        {:else}
          <span>OBJECTIVE</span>{objective}
        {/if}
      </footer>
    {/if}

    {#if touch}
      <div
        class="pointer-events-auto absolute bottom-0 right-0 top-0 w-1/2 touch-none"
        role="presentation"
        onpointerdown={lookStart}
        onpointermove={lookMove}
        onpointerup={lookEnd}
        onpointercancel={lookEnd}
      ></div>
      {#if hud?.crouch}
        <button
          class="pointer-events-auto absolute bottom-10 right-32 flex size-14 items-center justify-center rounded-full border-4 border-white/25 bg-slate-900/70 text-white active:bg-slate-700"
          aria-label="Crouch"
          onpointerdown={() => oncrouch(true)}
          onpointerup={() => oncrouch(false)}
          onpointercancel={() => oncrouch(false)}
        >
          <ChevronsDown class="size-7" />
        </button>
      {/if}
      <button
        class="pointer-events-auto absolute bottom-8 right-8 flex size-20 items-center justify-center rounded-full border-4 border-white/30 bg-rose-500/80 text-white active:bg-rose-400"
        aria-label={hud?.action ?? 'Fire'}
        onpointerdown={() => onfire(true)}
        onpointerup={() => onfire(false)}
        onpointercancel={() => onfire(false)}
      >
        <Zap class="size-8" />
      </button>
    {:else if !pointerLocked && phase === 'playing'}
      <div class="hud-prompt absolute left-1/2 top-1/2 flex -translate-x-1/2 translate-y-16 items-center gap-2 px-4 py-2 text-sm font-semibold text-white">
        <MousePointerClick class="size-4" />
        {hud?.hint ?? 'Click to take aim.'}
      </div>
    {/if}
  {/if}

  {#if board && mapBox && onBoard}
    <div class="pointer-events-auto absolute right-3 top-[4.25rem] w-44 {panel} p-2 max-md:hidden">
      <svg viewBox="{mapBox.minX} {mapBox.minZ} {mapBox.w} {mapBox.h}" class="w-full" style="aspect-ratio: {mapBox.w} / {mapBox.h}" aria-label="Board map">
        {#each board.spaces as space (space.id)}
          {#each space.next as nextId (nextId)}
            {#if spaceAt(nextId)}
              <line x1={space.x} y1={space.z} x2={spaceAt(nextId)?.x} y2={spaceAt(nextId)?.z} stroke="rgba(255,255,255,0.28)" stroke-width="0.6" />
            {/if}
          {/each}
        {/each}
        {#each board.spaces as space (space.id)}
          <circle
            cx={space.x}
            cy={space.z}
            r={space.id === board.prize_at ? 1.6 : 1}
            fill={space.id === board.prize_at ? '#ff5fa2' : SPACE_COLORS[space.kind]}
            opacity={space.id === board.prize_at ? 1 : 0.8}
          />
        {/each}
        {#each board.tokens as token (token.seat)}
          {@const at = spaceAt(token.space)}
          {#if at}
            <circle
              cx={at.x}
              cy={at.z}
              r={token.seat === board.turn_seat ? 2.2 : 1.7}
              fill={byId.get(bySeat(token.seat)?.character ?? '')?.body ?? '#ffffff'}
              stroke={token.seat === board.turn_seat ? '#ffffff' : 'rgba(15,23,42,0.7)'}
              stroke-width="0.6"
            />
          {/if}
        {/each}
      </svg>
      <button class="mt-1.5 flex w-full items-center justify-center gap-1.5 rounded-lg bg-white/10 px-2 py-1.5 text-xs font-semibold hover:bg-white/20" onclick={toggleOverview}>
        {#if overview}<Crosshair class="size-3.5" /> Follow the turn{:else}<Maximize2 class="size-3.5" /> Whole board{/if}
      </button>
    </div>
  {/if}

  {#if vote && board}
    <div class="pointer-events-auto absolute bottom-3 left-1/2 w-[min(44rem,calc(100%-1.5rem))] -translate-x-1/2 {panel} p-4">
      <div class="flex items-baseline justify-between gap-3">
        <div class="text-lg font-black">Round {board.round} bout</div>
        <div class="text-sm tabular-nums text-white/70">{canVote && !myVote ? 'Pick one' : 'Waiting for votes'} · {Math.ceil(vote.seconds)}</div>
      </div>
      <div class="mt-3 grid gap-2 sm:grid-cols-3">
        {#each vote.options as key (key)}
          {@const bout = bouts.get(key)}
          {@const voters = vote.votes.filter((v) => v.key === key)}
          <button
            class="flex flex-col gap-1.5 rounded-2xl border p-3 text-left transition-colors disabled:cursor-default
              {myVote === key ? 'border-amber-300 bg-amber-400/20' : 'border-white/10 bg-white/5 enabled:hover:bg-white/10'}"
            disabled={!canVote}
            aria-pressed={myVote === key}
            onclick={() => send({ type: 'vote', key })}
          >
            <span class="text-base font-black">{bout?.name ?? key}</span>
            <span class="text-xs text-white/65">{bout?.rules ?? ''}</span>
            <span class="mt-auto flex items-center justify-between gap-2 pt-1">
              <span class="rounded-full bg-white/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-white/60">
                {bout ? CONTROLS[bout.controls] : ''}{bout?.format === 'teams' ? ' · teams' : ''}
              </span>
              <span class="flex -space-x-1">
                {#each voters as voter (voter.seat)}
                  <span
                    class="size-3.5 rounded-full ring-2 ring-slate-900"
                    style="background:{byId.get(bySeat(voter.seat)?.character ?? '')?.body}"
                    title={bySeat(voter.seat)?.name}
                  ></span>
                {/each}
              </span>
            </span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if room?.phase.name === 'game_over'}
    <div class="absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-5 text-center">
      <div class="text-3xl font-black text-amber-300">
        {(room.phase.standings[0] ?? []).map((seat) => bySeat(seat)?.name ?? 'Someone').join(' and ')} {tableGame ? 'take the night' : 'wins the Tawkathon!'}
      </div>
      <div class="mt-3 flex flex-col gap-1 text-left text-sm">
        {#each room.phase.standings as group, place}
          {#each group as seat (seat)}
            {@const token = board?.tokens.find((t) => t.seat === seat)}
            <div class="flex items-center gap-2">
              <span class="w-9 font-bold text-white/70">{ordinal(place)}</span>
              <span class="size-3 rounded-full" style="background:{byId.get(bySeat(seat)?.character ?? '')?.body}"></span>
              <span class="flex-1 truncate">{bySeat(seat)?.name ?? 'Left the room'}</span>
              {#if !tableGame}
                <span class="flex items-center gap-1 tabular-nums text-pink-300"><Gem class="size-3.5" />{token?.prizes ?? 0}</span>
                <span class="flex items-center gap-1 tabular-nums text-amber-200"><Coins class="size-3.5" />{token?.coins ?? 0}</span>
              {/if}
            </div>
          {/each}
        {/each}
      </div>
      <div class="mt-3 text-xs text-white/60">Back to the lobby in {Math.ceil(room.phase.seconds)}</div>
    </div>
  {/if}

  {#if status === 'offline' && room}
    <div class="absolute left-1/2 top-16 -translate-x-1/2 {panel} flex items-center gap-2 px-4 py-2 text-sm">
      <LoaderCircle class="size-4 animate-spin" /> Reconnecting
    </div>
  {/if}

  {#if errorMessage}
    <div class="absolute left-1/2 top-28 -translate-x-1/2 rounded-xl bg-rose-600/90 px-4 py-2 text-sm font-semibold text-white shadow-lg">{errorMessage}</div>
  {/if}

  {#if room?.phase.name === 'countdown'}
    {#key room.phase.seconds}
      <div class="party-pop absolute inset-0 flex items-center justify-center text-[9rem] font-black text-white drop-shadow-[0_6px_0_rgba(15,23,42,0.55)]">
        {Math.ceil(room.phase.seconds)}
      </div>
    {/key}
  {:else if showGo}
    <div class="party-pop absolute inset-0 flex items-center justify-center text-[8rem] font-black text-amber-300 drop-shadow-[0_6px_0_rgba(15,23,42,0.55)]">Go!</div>
  {/if}

  {#if room?.phase.name === 'results'}
    <div class="absolute left-1/2 top-3 w-72 -translate-x-1/2 {panel} p-4 text-center max-md:top-auto max-md:bottom-3">
      <div class="text-3xl font-black text-amber-300">{headline}</div>
      <div class="mt-3 flex flex-col gap-1 text-left text-sm">
        {#each placements as group, place}
          {#each group as seat (seat)}
            {@const member = bySeat(seat)}
            <div class="flex items-center gap-2">
              <span class="w-9 font-bold text-white/70">{ordinal(place)}</span>
              <span class="size-3 rounded-full" style="background:{byId.get(member?.character ?? '')?.body}"></span>
              <span class="truncate">{member?.name ?? 'Left the room'}</span>
            </div>
          {/each}
        {/each}
      </div>
      <div class="mt-3 text-xs text-white/60">{board ? 'Back to the board' : 'Back to the lobby'} in {Math.ceil(room.phase.seconds)}</div>
    </div>
  {/if}

  {#if room && phase === 'lobby'}
    <div class="lobby-panel pointer-events-auto absolute bottom-3 left-1/2 w-[min(40rem,calc(100%-1.5rem))] -translate-x-1/2 {panel} p-4">
      {#if filming}
        <div class="text-center text-sm font-semibold text-white/80">Waiting for the next game</div>
      {:else if spectating || !mine}
        <div class="flex items-center gap-2 text-sm text-white/80">
          <Eye class="size-4" /> The room is full. You are watching and get a seat when one opens.
        </div>
      {:else}
        <div class="text-sm font-bold">Pick your character</div>
        <div class="mt-2 grid grid-cols-4 gap-2">
          {#each roster as character (character.id)}
            {@const holder = room.members.find((m) => m.character === character.id)}
            {@const taken = !!holder && holder.user_id !== me}
            <button
              class="flex flex-col items-center gap-1 rounded-xl border-2 px-1 py-2 text-xs font-semibold transition
                {holder?.user_id === me ? 'border-amber-300 bg-white/15' : 'border-transparent bg-white/5 hover:bg-white/10'}
                disabled:cursor-not-allowed disabled:opacity-40"
              disabled={taken}
              onclick={() => {
                try {
                  localStorage.setItem('party-character', character.id);
                } catch {
                  // Remembering the pick is a convenience only.
                }
                send({ type: 'pick', character: character.id });
              }}
            >
              <span class="size-8 rounded-full border-4" style="background:{character.body}; border-color:{character.accent}"></span>
              <span>{character.name}</span>
              {#if taken}<span class="max-w-full truncate text-[10px] font-normal text-white/70">{holder?.name}</span>{/if}
            </button>
          {/each}
        </div>
        {#if isHost && allGames.length > 1 && gameChoice}
          <div class="mt-3 border-t border-white/10 pt-3">
            <div class="mb-1 text-[11px] font-bold uppercase tracking-[0.16em] text-white/50">Tonight's bout</div>
            <div class="flex items-center gap-2 rounded-2xl border border-white/10 bg-gradient-to-r from-cyan-400/15 via-indigo-500/15 to-fuchsia-500/15 p-2">
              <button class="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-950/35 hover:bg-white/10" aria-label="Previous bout" onclick={() => cycleGame(-1)}><ChevronLeft class="size-5" /></button>
              <button class="min-w-0 flex-1 rounded-xl px-2 py-1 text-left hover:bg-white/10" onclick={() => send({ type: 'choose_game', key: gameChoice[0] })}>
                <div class="flex items-center gap-2 text-base font-black"><Gamepad2 class="size-4 text-amber-300" />{gameChoice[1]}</div>
                <div class="mt-0.5 truncate text-xs text-white/65">{gameCopy(gameChoice[0])}</div>
              </button>
              <button class="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-950/35 hover:bg-white/10" aria-label="Next bout" onclick={() => cycleGame(1)}><ChevronRight class="size-5" /></button>
            </div>
            <div class="mt-1.5 text-center text-xs text-white/50">{tableGame ? 'A card game at one table. People make it; bots only fill chairs.' : 'Use the arrows to browse. In a Tawkathon everyone votes on each round\'s bout.'}</div>
          </div>
        {/if}
        {#if isHost && (room.skills ?? []).length > 1}
          <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-white/10 pt-3">
            <span class="text-sm text-white/70">Bots</span>
            {#each room.skills as [level] (level)}
              <button
                class="rounded-xl px-3 py-1.5 text-sm font-semibold capitalize
                  {room.bot_skill_name === level ? 'bg-amber-400 text-slate-900' : 'bg-white/10 hover:bg-white/20'}"
                onclick={() => send({ type: 'choose_skill', level })}
              >
                {level}
              </button>
            {/each}
          </div>
        {/if}
        <div class="mt-3 flex flex-wrap items-center gap-2">
          {#if isHost && tableGame}
            <button
              class="flex items-center gap-1.5 rounded-xl bg-white/10 px-3 py-2 text-sm font-semibold hover:bg-white/20 disabled:opacity-40"
              disabled={seatsFull}
              onclick={() => send({ type: 'add_bot' })}
            >
              <Plus class="size-4" /><Bot class="size-4" /> Add a bot
            </button>
            <button
              class="ml-auto flex items-center gap-1.5 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300 disabled:opacity-40"
              disabled={room.members.length < tableMin}
              onclick={() => send({ type: 'start' })}
            >
              <Play class="size-4" /> Deal the cards
            </button>
            {#if room.members.length < tableMin}
              <div class="w-full text-xs text-white/60">{room.members.length} of {tableMin} seats filled. Bots can fill in, but they play simply.</div>
            {/if}
          {:else if isHost}
            <button
              class="flex items-center gap-1.5 rounded-xl bg-white/10 px-3 py-2 text-sm font-semibold hover:bg-white/20 disabled:opacity-40"
              disabled={seatsFull}
              onclick={() => send({ type: 'add_bot' })}
            >
              <Plus class="size-4" /><Bot class="size-4" /> Add a bot
            </button>
            <button
              class="flex items-center gap-1.5 rounded-xl bg-white/10 px-3 py-2 text-sm font-semibold hover:bg-white/20 disabled:opacity-40"
              disabled={room.members.length < 2}
              onclick={() => send({ type: 'start' })}
            >
              <Play class="size-4" /> Just this bout
            </button>
            <button
              class="ml-auto flex items-center gap-1.5 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300 disabled:opacity-40"
              disabled={room.members.length < 2}
              onclick={() => send({ type: 'start_party' })}
            >
              <Dices class="size-4" /> Start a Tawkathon
            </button>
            {#if room.members.length < 2}
              <div class="w-full text-xs text-white/60">Invite someone into the lobby or add a bot to start.</div>
            {/if}
          {:else}
            <div class="text-sm text-white/70">Waiting for {host?.name ?? 'the host'} to start the round.</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if mine?.in_round && inArena}
    {#if hud?.touch === 'pump' && touch}
      <div class="pointer-events-auto absolute bottom-5 left-4 right-4 flex gap-3">
        <button class="flex h-20 flex-1 items-center justify-center rounded-2xl border-2 border-white/25 bg-indigo-500/85 text-lg font-black text-white active:bg-indigo-400" aria-label="Pump left" onpointerdown={() => pump(-1)}>LEFT</button>
        <button class="flex h-20 flex-1 items-center justify-center rounded-2xl border-2 border-white/25 bg-fuchsia-500/85 text-lg font-black text-white active:bg-fuchsia-400" aria-label="Pump right" onpointerdown={() => pump(1)}>RIGHT</button>
      </div>
    {:else if touch}
      <div class="pointer-events-auto absolute bottom-6 left-6"><Joystick onchange={onstick} /></div>
    {:else if !firstPerson && hud?.hint}
      <div class="absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full bg-slate-900/60 px-4 py-1.5 text-xs font-semibold text-white/85">
        {hud.hint}
      </div>
    {/if}
  {/if}
</div>

<style>
  .rec-dot {
    animation: rec-pulse 1.6s ease-in-out infinite;
  }

  @keyframes rec-pulse {
    50% { opacity: 0.35; }
  }

  .lobby-hud {
    filter: drop-shadow(0 7px 12px rgb(2 6 23 / 0.28));
  }
  .lobby-hud__title {
    display: flex;
    min-width: 15rem;
    align-items: center;
    gap: 0.55rem;
    padding: 0.55rem 2rem 0.55rem 0.85rem;
    border-bottom: 1px solid var(--hud-accent);
    background: linear-gradient(100deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.78));
    clip-path: polygon(0 0, 100% 0, calc(100% - 1rem) 100%, 0 100%);
    font-size: 0.76rem;
    font-weight: 900;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }
  .lobby-hud__state {
    margin-left: auto;
    color: var(--hud-accent);
    font-size: 0.55rem;
    letter-spacing: 0.16em;
  }
  .lobby-hud__players {
    display: flex;
    width: fit-content;
    align-items: center;
    gap: 0.45rem;
    padding: 0.4rem 1.35rem 0.4rem 0.85rem;
    background: linear-gradient(90deg, rgb(8 15 29 / 0.86), rgb(15 23 42 / 0.62), transparent);
    font-size: 0.7rem;
  }
  .lobby-hud__player {
    width: 0.65rem;
    height: 0.65rem;
    border: 2px solid;
    transform: rotate(45deg);
  }
  .lobby-panel {
    max-height: calc(100% - 5.5rem);
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
    width: min(28rem, calc(100vw - 8rem));
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
  .team-rail {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.5rem;
    grid-column: 1 / -1;
    min-height: 2rem;
    padding: 0.3rem 0.7rem;
    background: linear-gradient(90deg, rgb(8 15 29 / 0.92), rgb(15 23 42 / 0.72));
    font-size: 0.68rem;
  }
  .team-rail--0 { border-left: 3px solid #f59e0b; }
  .team-rail--1 { border-left: 3px solid #22d3ee; }
  .team-rail__name {
    width: 3.4rem;
    flex: none;
    color: rgb(255 255 255 / 0.52);
    font-size: 0.58rem;
    font-weight: 950;
    letter-spacing: 0.13em;
  }
  .team-rail__member {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.35rem;
    font-weight: 750;
  }
  .team-rail__member--me { color: var(--hud-accent); }
  .team-rail__member--out { filter: grayscale(1); opacity: 0.35; text-decoration: line-through; }
  .hud-control {
    display: grid;
    width: 2.65rem;
    height: 2.65rem;
    place-items: center;
    color: white;
    border: 1px solid rgb(255 255 255 / 0.16);
    background: linear-gradient(145deg, rgb(15 23 42 / 0.92), rgb(8 15 29 / 0.76));
    clip-path: polygon(0.45rem 0, 100% 0, 100% calc(100% - 0.45rem), calc(100% - 0.45rem) 100%, 0 100%, 0 0.45rem);
    filter: drop-shadow(0 4px 8px rgb(2 6 23 / 0.3));
  }
  .hud-control:hover {
    color: var(--hud-accent);
    border-color: color-mix(in srgb, var(--hud-accent) 55%, transparent);
  }
  .hud-clock {
    display: block;
    min-width: 6.5rem;
    padding: 0.28rem 1.35rem 0.42rem;
    color: white;
    border-top: 2px solid var(--hud-accent);
    background: linear-gradient(180deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.76));
    clip-path: polygon(0.7rem 0, calc(100% - 0.7rem) 0, 100% 100%, 0 100%);
    filter: drop-shadow(0 6px 8px rgb(2 6 23 / 0.3));
  }
  .hud-clock--danger {
    border-top-color: #fb7185;
    animation: hud-alert 0.8s ease-in-out infinite alternate;
  }
  .combat-hud {
    min-width: 13rem;
    padding: 0.55rem 1.8rem 0.6rem 0.8rem;
    border-left: 3px solid var(--hud-accent);
    background: linear-gradient(90deg, rgb(8 15 29 / 0.9), rgb(8 15 29 / 0.55) 72%, transparent);
    filter: drop-shadow(0 5px 10px rgb(2 6 23 / 0.3));
  }
  .combat-hud__label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    color: rgb(255 255 255 / 0.62);
    font-size: 0.62rem;
    font-weight: 900;
    letter-spacing: 0.16em;
  }
  .combat-hud__value {
    display: block;
    font-size: 2.45rem;
    font-weight: 950;
    line-height: 1;
    letter-spacing: -0.06em;
    text-shadow: 0 2px 10px rgb(0 0 0 / 0.35);
  }
  .combat-hud__hint {
    margin-top: 0.15rem;
    color: rgb(255 255 255 / 0.62);
    font-size: 0.66rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .combat-hud__stats {
    display: flex;
    gap: 1rem;
    margin-top: 0.2rem;
    font-size: 0.7rem;
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
  .objective-meter {
    width: 7rem;
    height: 0.32rem;
    overflow: hidden;
    background: rgb(255 255 255 / 0.14);
  }
  .objective-meter i {
    display: block;
    height: 100%;
    background: #22d3ee;
    box-shadow: 0 0 0.6rem #22d3ee;
    transition: width 100ms linear;
  }
  .hud-prompt {
    border-top: 1px solid var(--hud-accent);
    background: linear-gradient(90deg, transparent, rgb(8 15 29 / 0.88) 12%, rgb(8 15 29 / 0.88) 88%, transparent);
    white-space: nowrap;
  }
  .party-pop {
    animation: party-pop 0.6s cubic-bezier(0.2, 1.6, 0.4, 1) both;
  }
  @keyframes party-pop {
    from {
      transform: scale(0.3);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
  @keyframes hud-alert {
    to { filter: drop-shadow(0 0 0.6rem rgb(251 113 133 / 0.58)); }
  }
  @media (max-width: 640px) {
    .match-hud__title {
      min-width: 10rem;
      padding-left: 0.65rem;
      font-size: 0.68rem;
    }
    .match-hud__roster {
      width: min(19rem, calc(100vw - 7rem));
      grid-template-columns: 1fr;
    }
    .match-hud__player:nth-child(n + 4) {
      display: none;
    }
    .combat-hud {
      bottom: 0.65rem;
      left: 0.65rem;
      min-width: 10rem;
    }
    .combat-hud__value { font-size: 2rem; }
    .hud-clock {
      min-width: 5rem;
      font-size: 1.15rem;
    }
    .match-objective {
      bottom: auto;
      top: 7.25rem;
      max-width: calc(100vw - 1rem);
      padding-inline: 0.7rem;
      font-size: 0.55rem;
    }
    .objective-meter { width: 4.5rem; }
    .lobby-hud__title { min-width: 12rem; }
    .lobby-hud__players { max-width: calc(100vw - 6.5rem); }
  }
  @media (max-height: 760px) {
    .lobby-panel {
      bottom: 0.5rem;
      padding: 0.65rem;
    }
    :global(.lobby-panel button) { padding-block: 0.35rem; }
    :global(.lobby-panel .mt-3) { margin-top: 0.45rem; }
  }
</style>

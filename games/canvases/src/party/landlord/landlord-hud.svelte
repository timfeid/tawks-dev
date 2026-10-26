<script lang="ts">
  import { BookOpen, Coins, Eye, Gavel, ListChecks, ScrollText, Users, X } from '@lucide/svelte';
  import type { ClientMsg, RoomView } from '../protocol';
  import {
    ACTION_TEXT,
    GROUPS,
    GROUP_INFO,
    card,
    cardColor,
    cardText,
    cardTitle,
    describe,
    describeEffect,
    groupsOf,
    isProperty,
    type Group,
    type LandlordAction,
    type LandlordPlayer,
    type LandlordView,
  } from './rules';

  let {
    view,
    room,
    hit,
    send,
  }: {
    view: LandlordView | null;
    room: RoomView | null;
    hit: { seat: number; index?: number; at: number } | null;
    send: (msg: ClientMsg) => void;
  } = $props();

  const act = (action: LandlordAction) => send({ type: 'table', action });

  const mySeat = $derived(view?.me ?? null);
  const meP = $derived(mySeat !== null && view ? (view.players[mySeat] ?? null) : null);
  const phase = $derived(view?.phase ?? null);
  const myTurn = $derived(!!phase && phase.kind === 'turn' && phase.seat === mySeat);
  const playsLeft = $derived(phase?.kind === 'turn' ? phase.plays_left : 0);
  const canPlay = $derived(myTurn && playsLeft > 0);
  const nameOf = (seat: number | null | undefined) =>
    seat === null || seat === undefined ? 'Someone' : (view?.seats[seat]?.name ?? 'Someone');
  const hand = $derived(view?.hand ?? []);
  const hasHike = $derived(hand.find((id) => card(id).kind.kind === 'rent_hike'));
  const hasLawyer = $derived(hand.find((id) => card(id).kind.kind === 'lawyer_up'));
  const others = $derived((view?.players ?? []).filter((p) => p.seat !== mySeat));
  const gameOver = $derived(phase?.kind === 'over');

  type Flow =
    | { step: 'menu'; id: number }
    | { step: 'group'; id: number; groups: Group[]; then: 'place' | 'rent' | 'build' }
    | {
        step: 'target';
        id: number;
        then: 'rent' | 'repo' | 'backroom' | 'trade' | 'takeover';
        group?: Group;
        double?: number;
      }
    | { step: 'take'; id: number; target: number; then: 'backroom' | 'trade' }
    | { step: 'give'; id: number; target: number; take: number }
    | { step: 'takeover_set'; id: number; target: number };
  let flow = $state<Flow | null>(null);
  let paySelection = $state<number[]>([]);
  let discardSelection = $state<number[]>([]);
  let moving = $state<number | null>(null);
  let showRules = $state(false);
  let seenHit = 0;

  $effect(() => {
    if (!myTurn) {
      flow = null;
      moving = null;
    }
    if (phase?.kind !== 'pay') paySelection = [];
    if (phase?.kind !== 'discard') discardSelection = [];
  });

  const wantsPlayer = $derived(flow?.step === 'target');

  function canTarget(seat: number): boolean {
    if (!view || flow?.step !== 'target' || seat === mySeat) return false;
    const player = view.players[seat];
    if (!player) return false;
    switch (flow.then) {
      case 'backroom':
      case 'trade':
        return player.sets.some((s) => !s.complete);
      case 'takeover':
        return player.sets.some((s) => s.complete);
      default:
        return true;
    }
  }

  function choosePlayer(seat: number) {
    if (!flow || flow.step !== 'target' || !canTarget(seat)) return;
    const f = flow;
    switch (f.then) {
      case 'rent':
        act({ type: 'rent', card: f.id, group: f.group!, target: seat, double: f.double });
        flow = null;
        break;
      case 'repo':
        act({ type: 'repo', card: f.id, target: seat });
        flow = null;
        break;
      case 'backroom':
      case 'trade':
        flow = { step: 'take', id: f.id, target: seat, then: f.then };
        break;
      case 'takeover':
        flow = { step: 'takeover_set', id: f.id, target: seat };
        break;
    }
  }

  $effect(() => {
    if (!hit || hit.at === seenHit) return;
    seenHit = hit.at;
    if (wantsPlayer) choosePlayer(hit.seat);
  });

  function open(id: number) {
    if (!canPlay) return;
    if (flow?.step === 'menu' && flow.id === id) {
      flow = null;
      return;
    }
    moving = null;
    flow = { step: 'menu', id };
  }

  function play(id: number) {
    const spec = card(id);
    const k = spec.kind;
    if (isProperty(spec)) {
      const groups = groupsOf(spec);
      if (groups.length === 1) {
        act({ type: 'place', card: id, group: groups[0] });
        flow = null;
      } else {
        flow = { step: 'group', id, groups, then: 'place' };
      }
      return;
    }
    switch (k.kind) {
      case 'rent': {
        const mine = (meP?.sets ?? []).map((s) => s.group);
        const groups = groupsOf(spec).filter((g) => mine.includes(g));
        if (!groups.length) return;
        flow = { step: 'group', id, groups, then: 'rent' };
        break;
      }
      case 'payday':
        act({ type: 'payday', card: id });
        flow = null;
        break;
      case 'repo_man':
        flow = { step: 'target', id, then: 'repo' };
        break;
      case 'fundraiser':
        act({ type: 'fundraiser', card: id });
        flow = null;
        break;
      case 'backroom_deal':
        flow = { step: 'target', id, then: 'backroom' };
        break;
      case 'trade_up':
        flow = { step: 'target', id, then: 'trade' };
        break;
      case 'takeover':
        flow = { step: 'target', id, then: 'takeover' };
        break;
      case 'renovation':
      case 'tower': {
        const groups = (meP?.sets ?? [])
          .filter(
            (s) =>
              s.complete &&
              !['rails', 'works'].includes(s.group) &&
              (k.kind === 'renovation' ? !s.house : s.house && !s.hotel),
          )
          .map((s) => s.group);
        if (!groups.length) return;
        flow = { step: 'group', id, groups, then: 'build' };
        break;
      }
      default:
        break;
    }
  }

  function chooseGroup(group: Group, double = false) {
    if (flow?.step !== 'group') return;
    const f = flow;
    if (f.then === 'place') {
      act({ type: 'place', card: f.id, group });
      flow = null;
    } else if (f.then === 'build') {
      act({ type: 'build', card: f.id, group });
      flow = null;
    } else {
      const spec = card(f.id);
      const everyone = spec.kind.kind === 'rent' && spec.kind.groups.length > 0;
      const hike = double ? hasHike : undefined;
      if (everyone) {
        act({ type: 'rent', card: f.id, group, double: hike });
        flow = null;
      } else {
        flow = { step: 'target', id: f.id, then: 'rent', group, double: hike };
      }
    }
  }

  const playable = (id: number) => {
    const spec = card(id);
    if (isProperty(spec)) return true;
    const k = spec.kind;
    if (k.kind === 'rent') return (meP?.sets ?? []).some((s) => groupsOf(spec).includes(s.group));
    if (k.kind === 'renovation')
      return (meP?.sets ?? []).some(
        (s) => s.complete && !['rails', 'works'].includes(s.group) && !s.house,
      );
    if (k.kind === 'tower') return (meP?.sets ?? []).some((s) => s.house && !s.hotel);
    if (k.kind === 'takeover') return others.some((p) => p.sets.some((s) => s.complete));
    if (k.kind === 'backroom_deal' || k.kind === 'trade_up')
      return others.some((p) => p.sets.some((s) => !s.complete));
    if (k.kind === 'cash' || k.kind === 'lawyer_up' || k.kind === 'rent_hike') return false;
    return true;
  };

  const payTotal = $derived(paySelection.reduce((sum, id) => sum + card(id).value, 0));
  const payOptions = $derived(phase?.kind === 'pay' ? (phase.options ?? []) : []);
  const payOk = $derived(
    phase?.kind === 'pay' &&
      (payTotal >= phase.amount || paySelection.length === payOptions.length),
  );
  const toggle = (list: number[], id: number) =>
    list.includes(id) ? list.filter((c) => c !== id) : [...list, id];

  const banner = $derived.by(() => {
    if (!view || !phase) return '';
    switch (phase.kind) {
      case 'turn':
        return phase.seat === mySeat
          ? `Your turn: ${phase.plays_left} ${phase.plays_left === 1 ? 'play' : 'plays'} left.`
          : `${nameOf(phase.seat)} is playing.`;
      case 'respond':
        return phase.target === mySeat
          ? 'An action is aimed at you.'
          : `${nameOf(phase.target)} is deciding whether to lawyer up.`;
      case 'pay':
        return phase.from === mySeat
          ? `You owe ${nameOf(phase.to)} $${phase.amount}.`
          : `${nameOf(phase.from)} owes ${nameOf(phase.to)} $${phase.amount}.`;
      case 'discard':
        return phase.seat === mySeat
          ? `Discard ${phase.over} down to seven.`
          : `${nameOf(phase.seat)} is discarding.`;
      case 'over':
        return `${nameOf(phase.winner)} owns three districts.`;
    }
  });
  const prompt = $derived.by(() => {
    if (!flow) return '';
    switch (flow.step) {
      case 'menu':
        return '';
      case 'group':
        return flow.then === 'place'
          ? 'Which district?'
          : flow.then === 'rent'
            ? 'Charge rent on which district?'
            : 'Build in which district?';
      case 'target':
        return flow.then === 'rent'
          ? 'Who pays?'
          : flow.then === 'repo'
            ? 'Who pays the $5?'
            : flow.then === 'takeover'
              ? 'Whose district?'
              : 'Whose deed?';
      case 'take':
        return `Which of ${nameOf(flow.target)}'s deeds?`;
      case 'give':
        return 'Which of yours goes in return?';
      case 'takeover_set':
        return `Which of ${nameOf(flow.target)}'s districts?`;
    }
  });

  let showLog = $state(false);

  /** What pressing Play does with this card, in words. */
  function playLabel(id: number): string {
    const spec = card(id);
    if (isProperty(spec)) {
      const groups = groupsOf(spec);
      return groups.length === 1 ? `Lay it in ${GROUP_INFO[groups[0]].title}` : 'Lay it in a district';
    }
    switch (spec.kind.kind) {
      case 'rent':
        return 'Charge rent';
      case 'payday':
        return 'Draw two cards';
      case 'fundraiser':
        return 'Collect $2 from everyone';
      case 'repo_man':
        return 'Collect $5 from a rival';
      case 'backroom_deal':
        return 'Take a rival deed';
      case 'trade_up':
        return 'Swap deeds with a rival';
      case 'takeover':
        return 'Take a whole district';
      case 'renovation':
      case 'tower':
        return 'Build in a district';
      default:
        return 'Play it';
    }
  }

  /** Why a card that looks like an action cannot be played right now. */
  function blockedReason(id: number): string | null {
    const k = card(id).kind.kind;
    if (k === 'lawyer_up') return 'Lawyer Up is only played when an action is aimed at you.';
    if (k === 'rent_hike') return 'Rent Hike is added while you charge rent.';
    if (k === 'rent') return 'You need a deed in one of its districts first.';
    if (k === 'renovation' || k === 'tower') return 'You need a complete district to build on.';
    if (k === 'takeover') return 'No rival has a complete district yet.';
    if (k === 'backroom_deal' || k === 'trade_up') return 'No rival has a loose deed to take.';
    return null;
  }

  const hint = $derived.by(() => {
    if (!view || !phase || !myTurn || flow) return '';
    if (playsLeft === 0) return 'Out of plays. End your turn.';
    return 'Tap a card. Green ones can be played; the rest can go in your bank.';
  });

  const inspecting = $derived(flow?.step === 'menu' ? flow.id : null);

  function onKey(event: KeyboardEvent) {
    const typing = (event.target as HTMLElement | null)?.closest('input, textarea, [contenteditable]');
    if (typing) return;
    if (event.key === 'Escape' && flow) {
      flow = null;
      return;
    }
    if (event.code === 'Space' && myTurn && !flow) {
      event.preventDefault();
      act({ type: 'end_turn' });
    }
  }

  const panel =
    'rounded-2xl border border-white/15 bg-slate-900/85 text-white shadow-xl backdrop-blur';
  const chip = (group: Group) =>
    `background:${GROUP_INFO[group].color}; color:${['strip', 'cannery', 'works', 'market'].includes(group) ? '#0f172a' : '#fff'}`;
</script>

<svelte:window onkeydown={onKey} />

{#snippet face(id: number, big: boolean)}
  {@const spec = card(id)}
  <span
    class="flex h-full w-full flex-col text-left {big ? 'p-3' : 'px-1.5 py-1'}"
    style="color:{['strip', 'cannery', 'works', 'market'].includes((spec.kind as { group?: Group }).group ?? '')
      ? '#0f172a'
      : '#fff'}"
  >
    <span class="flex items-start justify-between gap-1 font-black leading-tight {big ? 'text-lg' : 'text-[11px]'}">
      <span>{cardTitle(spec)}</span>
      <span class="shrink-0 rounded bg-black/30 px-1 {big ? 'text-base' : 'text-[10px]'}">${spec.value}</span>
    </span>
    <span class="mt-auto leading-tight opacity-90 {big ? 'text-sm' : 'line-clamp-4 text-[9px]'}">{cardText(spec)}</span>
  </span>
{/snippet}

{#snippet districts(sets: LandlordPlayer['sets'], small: boolean)}
  {#each sets as set, i (i)}
    <span
      class="inline-flex items-center gap-1 rounded font-bold {small ? 'px-1 text-[10px]' : 'px-1.5 py-0.5 text-[11px]'} {set.complete
        ? 'ring-1 ring-white'
        : ''}"
      style={chip(set.group)}
    >
      {GROUP_INFO[set.group].title}
      <span class="inline-flex gap-0.5">
        {#each Array(GROUP_INFO[set.group].size) as _, n (n)}
          <span class="size-1.5 rounded-full {n < set.cards.length ? 'bg-current' : 'bg-current opacity-30'}"></span>
        {/each}
      </span>
      {#if set.house}<span>+R</span>{/if}{#if set.hotel}<span>+T</span>{/if}
    </span>
  {/each}
{/snippet}

{#if view && room && room.phase.name !== 'lobby'}
  <div
    class="pointer-events-none absolute left-1/2 top-3 flex w-[min(40rem,calc(100%-1.5rem))] -translate-x-1/2 flex-col items-center gap-1 text-center max-md:top-14"
  >
    <div class="{panel} flex items-center gap-3 px-4 py-2 text-sm font-semibold">
      <ScrollText class="size-4 text-amber-200" />
      <span>{banner}</span>
      {#if !gameOver}<span class="tabular-nums text-white/60">{Math.ceil(view.seconds_left)}s</span>{/if}
    </div>
    {#if prompt && flow?.step !== 'target'}
      <div
        class="pointer-events-auto flex items-center gap-2 rounded-full bg-amber-400 px-3 py-1 text-xs font-bold text-slate-900"
      >
        {prompt}
        <button class="ml-1 rounded-full bg-slate-900/20 p-0.5" aria-label="Cancel" onclick={() => (flow = null)}
          ><X class="size-3" /></button
        >
      </div>
    {:else if hint}
      <div class="rounded-full bg-slate-900/70 px-3 py-1 text-xs text-white/80">{hint}</div>
    {/if}
    <div class="pointer-events-auto flex w-full flex-wrap justify-center gap-1 md:hidden">
      {#each others as player (player.seat)}
        <div class="{panel} flex flex-wrap items-center gap-1 px-2 py-1 text-[11px]">
          <span class="font-semibold">{view.seats[player.seat]?.name}</span>
          <span class="flex items-center gap-0.5 text-emerald-200"><Coins class="size-3" />{player.bank_total}</span>
          <span class="text-white/50">{player.hand_size} cards</span>
          {@render districts(player.sets, true)}
        </div>
      {/each}
    </div>
  </div>

  <aside class="pointer-events-auto absolute right-3 top-28 flex w-64 max-w-[45%] flex-col gap-2 max-md:hidden">
    <div class="{panel} p-2">
      <div
        class="mb-1 flex items-center justify-between px-1 text-[10px] font-black uppercase tracking-[0.16em] text-white/50"
      >
        <span>Landlords</span><span>Round {view.round}, deck {view.deck}</span>
      </div>
      {#each view.players as player (player.seat)}
        {@const seat = view.seats[player.seat]}
        <div class="flex w-full flex-col gap-1 rounded-lg px-2 py-1 text-left text-xs {player.seat === mySeat ? 'bg-white/10' : ''}">
          <span class="flex w-full items-center gap-2">
            <span class="size-2 shrink-0 rounded-full {seat?.connected ? 'bg-emerald-400' : 'bg-slate-500'}"></span>
            <span class="min-w-0 flex-1 truncate font-semibold">{seat?.name}</span>
            <span class="flex items-center gap-1 text-emerald-200"><Coins class="size-3" />{player.bank_total}</span>
            <span class="text-white/50">{player.hand_size} in hand</span>
          </span>
          {#if player.sets.length}<span class="flex flex-wrap gap-1">{@render districts(player.sets, true)}</span>{/if}
        </div>
      {/each}
    </div>
    <div class="{panel} max-h-44 overflow-y-auto p-2 text-[11px] leading-snug text-white/75">
      {#each [...view.log].reverse() as entry, i (i)}
        <div class="border-b border-white/5 py-0.5 last:border-0">{describe(entry, view.seats)}</div>
      {/each}
    </div>
  </aside>

  {#if showLog}
    <div class="pointer-events-auto absolute inset-x-3 top-24 max-h-[45%] overflow-y-auto {panel} p-3 text-xs md:hidden">
      <div class="mb-1 flex items-center justify-between font-black">
        What happened <button aria-label="Close" onclick={() => (showLog = false)}><X class="size-4" /></button>
      </div>
      {#each [...view.log].reverse() as entry, i (i)}
        <div class="border-b border-white/5 py-1 text-white/75 last:border-0">{describe(entry, view.seats)}</div>
      {/each}
    </div>
  {/if}

  {#if inspecting !== null}
    {@const id = inspecting}
    {@const spec = card(id)}
    {@const reason = playable(id) ? null : blockedReason(id)}
    <div class="pointer-events-auto absolute inset-0 flex items-center justify-center bg-black/40 p-4">
      <div class="{panel} flex w-[min(22rem,100%)] flex-col items-center gap-3 p-4">
        <div class="h-64 w-44 overflow-hidden rounded-xl border border-white/30 shadow-2xl" style="background:{cardColor(spec)}">
          {@render face(id, true)}
        </div>
        <div class="flex w-full flex-col gap-2">
          {#if playable(id)}
            <button class="rounded-xl bg-amber-400 px-4 py-2.5 text-sm font-black text-slate-900" onclick={() => play(id)}
              >{playLabel(id)}</button
            >
          {:else if reason}
            <div class="rounded-lg bg-white/5 px-3 py-2 text-center text-xs text-white/70">{reason}</div>
          {/if}
          {#if !isProperty(spec)}
            <button
              class="rounded-xl bg-emerald-500 px-4 py-2.5 text-sm font-black text-slate-900"
              onclick={() => {
                act({ type: 'bank', card: id });
                flow = null;
              }}>Bank it for ${spec.value}</button
            >
          {/if}
          <button class="rounded-xl bg-white/10 px-4 py-2 text-sm font-bold" onclick={() => (flow = null)}>Keep it</button>
        </div>
      </div>
    </div>
  {/if}

  {#if flow?.step === 'target'}
    <div class="pointer-events-auto absolute inset-0 flex items-center justify-center bg-black/40 p-4">
      <div class="{panel} w-[min(26rem,100%)] p-4">
        <div class="mb-3 flex items-center justify-between text-sm font-black">
          <span class="flex items-center gap-2"><Users class="size-4 text-amber-300" />{prompt}</span>
          <button aria-label="Cancel" onclick={() => (flow = null)}><X class="size-4" /></button>
        </div>
        <div class="flex flex-col gap-2">
          {#each others as player (player.seat)}
            {@const ok = canTarget(player.seat)}
            <button
              class="flex flex-col gap-1 rounded-xl border px-3 py-2 text-left text-xs {ok
                ? 'border-amber-300/60 bg-amber-400/10 hover:bg-amber-400/20'
                : 'border-white/10 opacity-40'}"
              disabled={!ok}
              onclick={() => choosePlayer(player.seat)}
            >
              <span class="flex items-center gap-2 text-sm font-bold">
                {view.seats[player.seat]?.name}
                <span class="ml-auto flex items-center gap-1 text-emerald-200"><Coins class="size-3.5" />{player.bank_total}</span>
              </span>
              {#if player.sets.length}
                <span class="flex flex-wrap gap-1">{@render districts(player.sets, true)}</span>
              {:else}
                <span class="text-white/50">No deeds yet</span>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  {#if flow?.step === 'group'}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">{prompt}</div>
      <div class="mt-3 flex flex-wrap justify-center gap-2">
        {#each flow.groups as group (group)}
          <button
            class="rounded-lg px-3 py-1.5 text-xs font-bold"
            style={chip(group)}
            onclick={() => chooseGroup(group)}>{GROUP_INFO[group].title}</button
          >
        {/each}
      </div>
      {#if flow.then === 'rent' && hasHike !== undefined && playsLeft >= 2}
        <div class="mt-3 border-t border-white/10 pt-2 text-xs text-white/70">
          Add Rent Hike to double it, for a second play:
        </div>
        <div class="mt-1 flex flex-wrap justify-center gap-2">
          {#each flow.groups as group (group)}
            <button
              class="rounded-lg px-3 py-1.5 text-xs font-bold ring-2 ring-amber-300"
              style={chip(group)}
              onclick={() => chooseGroup(group, true)}>{GROUP_INFO[group].title} x2</button
            >
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if flow?.step === 'take' || flow?.step === 'give' || flow?.step === 'takeover_set'}
    {@const f = flow}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-96 max-w-[95%] -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">{prompt}</div>
      <div class="mt-3 flex flex-wrap justify-center gap-2">
        {#if f.step === 'takeover_set'}
          {#each view.players[f.target].sets.filter((s) => s.complete) as set, i (i)}
            <button
              class="rounded-lg px-3 py-1.5 text-xs font-bold"
              style={chip(set.group)}
              onclick={() => {
                act({ type: 'takeover', card: f.id, target: f.target, group: set.group });
                flow = null;
              }}>{GROUP_INFO[set.group].title}</button
            >
          {/each}
        {:else}
          {@const owner = f.step === 'take' ? view.players[f.target] : meP}
          {#each (owner?.sets ?? []).filter((s) => f.step === 'give' || !s.complete) as set, i (i)}
            {#each set.cards as id (id)}
              <button
                class="rounded-lg px-2 py-1 text-[11px] font-bold"
                style="background:{cardColor(card(id))}"
                onclick={() => {
                  if (f.step === 'take') {
                    if (f.then === 'backroom') {
                      act({ type: 'backroom', card: f.id, target: f.target, take: id });
                      flow = null;
                    } else flow = { step: 'give', id: f.id, target: f.target, take: id };
                  } else {
                    act({ type: 'trade', card: f.id, target: f.target, take: f.take, give: id });
                    flow = null;
                  }
                }}>{card(id).title}</button
              >
            {/each}
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  {#if phase?.kind === 'respond' && phase.target === mySeat}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="flex items-center justify-center gap-2 text-sm font-bold">
        <Gavel class="size-4 text-amber-300" />{phase.cancelled
          ? 'Your lawyer cancelled it. They may counter.'
          : describeEffect(phase.effect, view.seats)}
      </div>
      <div class="mt-3 flex justify-center gap-2">
        {#if hasLawyer !== undefined}
          <button
            class="rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900"
            onclick={() => act({ type: 'lawyer', card: hasLawyer! })}>Lawyer Up</button
          >
        {/if}
        <button
          class="rounded-xl bg-white/10 px-4 py-2 text-sm font-bold"
          onclick={() => act({ type: 'accept' })}
          >{phase.cancelled ? 'Let it stand' : 'Accept'}</button
        >
      </div>
    </div>
  {:else if phase?.kind === 'respond'}
    <div
      class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 {panel} px-4 py-2 text-sm"
    >
      {describeEffect(phase.effect, view.seats)}
    </div>
  {/if}

  {#if phase?.kind === 'pay' && phase.from === mySeat}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-96 max-w-[95%] -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">Pay {nameOf(phase.to)} ${phase.amount}</div>
      <div class="mt-1 text-xs text-white/60">Bank first if you can. Change is never given.</div>
      <div class="mt-3 flex flex-wrap justify-center gap-2">
        {#each payOptions as id (id)}
          <button
            class="rounded-lg px-2 py-1 text-[11px] font-bold {paySelection.includes(id)
              ? 'ring-2 ring-amber-300'
              : 'opacity-80'}"
            style="background:{cardColor(card(id))}"
            onclick={() => (paySelection = toggle(paySelection, id))}
            >{card(id).title} ${card(id).value}</button
          >
        {/each}
      </div>
      <button
        class="mt-3 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 disabled:opacity-40"
        disabled={!payOk}
        onclick={() => act({ type: 'pay', cards: paySelection })}>Pay ${payTotal}</button
      >
    </div>
  {/if}

  {#if phase?.kind === 'discard' && phase.seat === mySeat}
    <div
      class="pointer-events-auto absolute bottom-40 left-1/2 -translate-x-1/2 {panel} p-3 text-center"
    >
      <div class="text-xs">Pick {phase.over} to discard, then confirm.</div>
      <button
        class="mt-2 rounded-xl bg-amber-400 px-4 py-1.5 text-sm font-bold text-slate-900 disabled:opacity-40"
        disabled={discardSelection.length !== phase.over}
        onclick={() => act({ type: 'discard', cards: discardSelection })}>Discard</button
      >
    </div>
  {/if}

  {#if gameOver && phase?.kind === 'over'}
    <div
      class="absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-5 text-center"
    >
      <div class="text-3xl font-black text-amber-300">{nameOf(phase.winner)} wins</div>
      <div class="mt-2 text-sm text-white/70">Three districts, fully owned.</div>
    </div>
  {/if}

  {#if meP}
    <div class="pointer-events-auto absolute bottom-2 left-1/2 flex w-[min(56rem,calc(100%-1rem))] -translate-x-1/2 flex-col gap-2">
      <div class="{panel} p-2">
        <div class="flex flex-wrap items-center gap-2 px-1">
          <span class="text-[10px] font-black uppercase tracking-[0.16em] text-white/50">Your hand</span>
          <span class="flex items-center gap-1 text-xs text-emerald-200"><Coins class="size-3" /> ${meP.bank_total} banked</span>
          {#if myTurn}
            <span class="flex items-center gap-1" aria-label="{playsLeft} plays left">
              {#each [0, 1, 2] as n (n)}
                <span class="size-2.5 rounded-full {n < playsLeft ? 'bg-amber-300' : 'bg-white/15'}"></span>
              {/each}
            </span>
          {/if}
          <span class="ml-auto flex gap-1">
            <button class="rounded-lg bg-white/10 px-2 py-1 text-[11px] font-bold hover:bg-white/20 md:hidden" onclick={() => (showLog = !showLog)}
              ><ListChecks class="mr-1 inline size-3" />Log</button
            >
            <button class="rounded-lg bg-white/10 px-2 py-1 text-[11px] font-bold hover:bg-white/20" onclick={() => (showRules = !showRules)}
              ><BookOpen class="mr-1 inline size-3" />Rules</button
            >
            <button
              class="rounded-lg px-3 py-1 text-xs font-black text-slate-900 disabled:opacity-30 {myTurn && playsLeft === 0
                ? 'bg-amber-300 ring-2 ring-amber-200'
                : 'bg-amber-400'}"
              disabled={!myTurn}
              onclick={() => act({ type: 'end_turn' })}
              >End turn <span class="ml-1 rounded bg-slate-900/20 px-1 text-[9px] max-md:hidden">SPACE</span></button
            >
          </span>
        </div>
        {#if meP.sets.length}
          <div class="mt-1 flex flex-wrap items-center gap-1 px-1 text-[10px] text-white/60">
            <span>Your districts:</span>
            {@render districts(meP.sets, false)}
            {#if myTurn}
              {#each meP.sets as set, i (i)}
                {#each set.cards.filter((c) => card(c).kind.kind === 'wild') as wild (wild)}
                  <button class="rounded bg-white/10 px-1" onclick={() => (moving = moving === wild ? null : wild)}
                    >move {card(wild).title}</button
                  >
                  {#if moving === wild}
                    {#each groupsOf(card(wild)).filter((g) => g !== set.group) as g (g)}
                      <button
                        class="rounded px-1 font-bold"
                        style={chip(g)}
                        onclick={() => {
                          act({ type: 'move', card: wild, group: g });
                          moving = null;
                        }}>{GROUP_INFO[g].title}</button
                      >
                    {/each}
                  {/if}
                {/each}
              {/each}
            {/if}
          </div>
        {/if}
        <div class="mt-1 flex gap-1.5 overflow-x-auto px-1 pb-1 pt-2">
          {#each hand as id (id)}
            {@const spec = card(id)}
            {@const marked = phase?.kind === 'discard' && discardSelection.includes(id)}
            {@const green = canPlay && playable(id)}
            <button
              class="h-28 w-20 shrink-0 overflow-hidden rounded-lg border transition {marked
                ? '-translate-y-2 ring-2 ring-amber-300'
                : ''} {green ? 'border-emerald-300 shadow-[0_0_12px_rgba(110,231,183,0.6)]' : 'border-white/20'} {canPlay ||
              phase?.kind === 'discard'
                ? 'hover:-translate-y-1'
                : 'opacity-70'}"
              style="background:{cardColor(spec)}"
              onclick={() =>
                phase?.kind === 'discard' && phase.seat === mySeat
                  ? (discardSelection = toggle(discardSelection, id))
                  : open(id)}
              title={cardText(spec)}
            >
              {@render face(id, false)}
            </button>
          {/each}
          {#if !hand.length}<div class="px-2 py-6 text-xs text-white/50">No cards in hand.</div>{/if}
        </div>
      </div>
    </div>
  {/if}

  {#if showRules}
    <div
      class="pointer-events-auto absolute inset-x-3 bottom-36 top-16 overflow-y-auto {panel} p-4 text-xs"
    >
      <div class="flex items-center justify-between">
        <span class="text-sm font-black">How Landlord plays</span><button
          aria-label="Close"
          onclick={() => (showRules = false)}><X class="size-4" /></button
        >
      </div>
      <p class="mt-2 text-white/75">
        Draw two at the start of your turn, then make up to three plays: bank a card as cash, lay a
        deed in its district, or play an action. Rent and debts are paid from your bank or your
        deeds, never your hand, and nobody gives change. Lawyer Up cancels an action played on you.
        End your turn with at most seven cards. Own three complete districts and you win.
      </p>
      <div class="mt-3 grid gap-1 sm:grid-cols-2">
        {#each GROUPS as g (g)}
          <div class="flex items-center gap-2 rounded-lg bg-white/5 p-1.5">
            <span class="rounded px-1 font-bold" style={chip(g)}>{GROUP_INFO[g].title}</span><span
              class="text-white/70"
              >{GROUP_INFO[g].size} deeds, rent {GROUP_INFO[g].rents
                .map((r) => `$${r}`)
                .join(' / ')}</span
            >
          </div>
        {/each}
      </div>
      <div class="mt-3 grid gap-1 sm:grid-cols-2">
        {#each Object.entries(ACTION_TEXT).filter(([k]) => k !== 'cash') as [key, text] (key)}
          <div class="rounded-lg bg-white/5 p-1.5">
            <b class="capitalize">{key.replace('_', ' ')}.</b>
            {text}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if mySeat === null}
    <div
      class="absolute bottom-3 left-1/2 -translate-x-1/2 {panel} flex items-center gap-2 px-3 py-1.5 text-xs"
    >
      <Eye class="size-3.5" /> Watching the table.
    </div>
  {/if}
{/if}

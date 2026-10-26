<script lang="ts">
  import { BookOpen, Cat, Eye, Moon, ScrollText, Skull, Sparkles, Sun, X } from '@lucide/svelte';
  import type { ClientMsg, CovenAction, CovenCard, CovenView, RoomView } from '../protocol';
  import {
    CARDS,
    CHARACTERS,
    COLOR_HEX,
    VERDICTS,
    describe,
    needsBlueChoice,
    needsSecond,
  } from './rules';

  let {
    view,
    room,
    hit,
    send,
  }: {
    view: CovenView | null;
    room: RoomView | null;
    /** A seat or verdict card clicked on the table. */
    hit: { seat: number; index?: number; at: number } | null;
    send: (msg: ClientMsg) => void;
  } = $props();

  const act = (action: CovenAction) => send({ type: 'table', action });

  const mySeat = $derived(view?.me ?? null);
  const meP = $derived(mySeat !== null && view ? (view.players[mySeat] ?? null) : null);
  const phase = $derived(view?.phase ?? null);
  const myTurn = $derived(!!phase && phase.kind === 'turn' && phase.seat === mySeat);
  const canDraw = $derived(myTurn && phase?.kind === 'turn' && !phase.played && !phase.drawing);
  const canPlay = $derived(myTurn && phase?.kind === 'turn' && !phase.drawing && !!meP?.alive);
  const canEnd = $derived(myTurn && phase?.kind === 'turn' && phase.played);
  const iAmWitch = $derived(!!meP?.alive && !!view && view.witches.includes(mySeat ?? -1));
  const nameOf = (seat: number | null | undefined) =>
    seat === null || seat === undefined ? 'Someone' : (view?.seats[seat]?.name ?? 'Someone');
  const hidden = (seat: number) =>
    (view?.players[seat]?.verdicts ?? []).map((v, i) => ({ ...v, i })).filter((v) => !v.revealed);
  const alive = $derived((view?.players ?? []).filter((p) => p.alive));
  const others = $derived(alive.filter((p) => p.seat !== mySeat));
  const character = $derived(meP ? CHARACTERS[meP.character] : null);
  const abilityReady = $derived(!!character?.ability && !!meP && !meP.ability_used && myTurn);
  const night = $derived(phase?.kind === 'night' || phase?.kind === 'confess');
  const gameOver = $derived(phase?.kind === 'over');

  let selected = $state<number | null>(null);
  let picking = $state<{
    step: 'target' | 'second' | 'blue';
    card: CovenCard;
    target?: number;
  } | null>(null);
  let ability = $state<'peek' | 'cure' | null>(null);
  let peekTarget = $state<number | null>(null);
  let confessing = $state(false);
  let showRules = $state(false);
  let seenHit = 0;

  function clearPick() {
    selected = null;
    picking = null;
    ability = null;
    peekTarget = null;
  }

  $effect(() => {
    if (!myTurn) {
      selected = null;
      picking = null;
      ability = null;
      peekTarget = null;
    }
    if (phase?.kind !== 'confess') confessing = false;
  });

  function clickCard(index: number) {
    if (!canPlay || !view?.hand) return;
    if (selected === index) {
      clearPick();
      return;
    }
    ability = null;
    selected = index;
    picking = { step: 'target', card: view.hand[index] };
  }

  const wantsPlayer = $derived(
    (!!picking && picking.step !== 'blue') ||
      ability === 'cure' ||
      (ability === 'peek' && peekTarget === null) ||
      (phase?.kind === 'dawn' && iAmWitch) ||
      (phase?.kind === 'night' && (iAmWitch || phase.constable)),
  );

  function canTarget(seat: number): boolean {
    if (!view) return false;
    const player = view.players[seat];
    if (!player?.alive) return false;
    if (picking?.step === 'target') {
      const card = picking.card;
      if (seat === mySeat) return card === 'vouching' && meP?.character === 'innkeeper';
      if (CARDS[card].color === 'red' && player.table.includes('good_standing')) return false;
      if (needsSecond(card) && player.character === 'servant') return false;
      if (card === 'hex' && !player.table.some((c) => CARDS[c].color === 'blue')) return false;
      return true;
    }
    if (picking?.step === 'second')
      return seat !== mySeat && seat !== picking.target && player.character !== 'servant';
    if (ability === 'cure') return true;
    if (ability === 'peek') return seat !== mySeat && hidden(seat).length > 0;
    if (phase?.kind === 'dawn') return true;
    if (phase?.kind === 'night') return phase.constable ? seat !== mySeat : true;
    return false;
  }

  function choosePlayer(seat: number) {
    if (!canTarget(seat)) return;
    if (picking?.step === 'target') {
      if (selected === null) return;
      if (needsSecond(picking.card)) {
        picking = { ...picking, step: 'second', target: seat };
        return;
      }
      if (needsBlueChoice(picking.card)) {
        picking = { ...picking, step: 'blue', target: seat };
        return;
      }
      act({ type: 'play', card: selected, target: seat });
      clearPick();
      return;
    }
    if (picking?.step === 'second') {
      if (selected === null || picking.target === undefined) return;
      act({ type: 'play', card: selected, target: picking.target, second: seat });
      clearPick();
      return;
    }
    if (ability === 'cure') {
      act({ type: 'cure', target: seat });
      clearPick();
      return;
    }
    if (ability === 'peek') {
      peekTarget = seat;
      return;
    }
    if (phase?.kind === 'dawn') act({ type: 'place_familiar', target: seat });
    else if (phase?.kind === 'night')
      act(
        phase.constable && !iAmWitch
          ? { type: 'guard', target: seat }
          : { type: 'kill', target: seat },
      );
  }

  function chooseBlue(index: number) {
    if (picking?.step !== 'blue' || selected === null || picking.target === undefined) return;
    act({ type: 'play', card: selected, target: picking.target, second: index });
    clearPick();
  }

  function useAbility() {
    if (!character?.ability || !abilityReady) return;
    selected = null;
    picking = null;
    if (character.ability === 'tide') {
      act({ type: 'tide' });
      return;
    }
    ability = character.ability;
    peekTarget = null;
  }

  $effect(() => {
    if (!hit || hit.at === seenHit) return;
    seenHit = hit.at;
    if (
      ability === 'peek' &&
      peekTarget !== null &&
      hit.seat === peekTarget &&
      hit.index !== undefined
    ) {
      act({ type: 'peek', target: peekTarget, index: hit.index });
      clearPick();
      return;
    }
    if (
      phase?.kind === 'reveal' &&
      phase.chooser === mySeat &&
      hit.seat === phase.accused &&
      hit.index !== undefined
    ) {
      act({ type: 'reveal', index: hit.index });
      return;
    }
    if (wantsPlayer) choosePlayer(hit.seat);
  });

  const banner = $derived.by(() => {
    if (!view || !phase) return '';
    switch (phase.kind) {
      case 'dawn':
        return iAmWitch
          ? 'Dawn. Choose where the Familiar starts.'
          : 'Dawn. The witches are stirring.';
      case 'turn':
        return phase.seat === mySeat
          ? phase.drawing
            ? 'Drawing'
            : phase.played
              ? 'Play more cards or end your turn.'
              : 'Your turn: draw two cards, or play.'
          : `${nameOf(phase.seat)} is taking a turn.`;
      case 'reveal':
        return phase.chooser === mySeat
          ? `Choose which of ${nameOf(phase.accused)}'s verdict cards turns over.`
          : `${nameOf(phase.chooser)} chooses a verdict card of ${nameOf(phase.accused)}.`;
      case 'conspiracy':
        return 'Conspiracy. Take a face-down card from the player on your left.';
      case 'night':
        return 'Night. The witches choose. The constable guards.';
      case 'confess':
        return 'The deed is chosen. Confess to save yourself, or hold your nerve.';
      case 'over':
        return phase.winner === 'town' ? 'The town wins.' : 'The coven wins.';
    }
  });
  const prompt = $derived.by(() => {
    if (picking?.step === 'target') return `Play ${CARDS[picking.card].title} on whom?`;
    if (picking?.step === 'second') return `${CARDS[picking.card].title}: give to whom?`;
    if (picking?.step === 'blue')
      return `Hex: which blue card in front of ${nameOf(picking.target)}?`;
    if (ability === 'cure') return 'Clear every accusation from whom?';
    if (ability === 'peek')
      return peekTarget === null
        ? 'Look at whose verdict card?'
        : `Which of ${nameOf(peekTarget)}'s cards?`;
    if (phase?.kind === 'dawn' && iAmWitch) return 'Place the Familiar in front of whom?';
    if (phase?.kind === 'night' && iAmWitch) return 'Who dies tonight?';
    if (phase?.kind === 'night' && phase.constable) return 'Whom do you guard tonight?';
    return '';
  });
  const myVote = $derived.by(() => {
    if (!phase || mySeat === null) return null;
    if (phase.kind === 'dawn') return phase.voted?.[String(mySeat)] ?? null;
    if (phase.kind === 'night') return phase.votes?.[String(mySeat)] ?? null;
    return null;
  });

  const panel =
    'rounded-2xl border border-white/15 bg-slate-900/85 text-white shadow-xl backdrop-blur';
  const cardClass = (card: CovenCard) =>
    `border-white/20 text-white` + ` bg-[${COLOR_HEX[CARDS[card].color]}]`;
</script>

{#if view && room && room.phase.name !== 'lobby'}
  {#if night && !gameOver}
    <div
      class="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(2,6,23,0.15),rgba(2,6,23,0.75))] transition-opacity"
    ></div>
  {/if}

  <div
    class="absolute left-1/2 top-3 flex w-[min(44rem,calc(100%-1.5rem))] -translate-x-1/2 flex-col items-center gap-1 text-center"
  >
    <div class="{panel} flex items-center gap-3 px-4 py-2 text-sm font-semibold">
      {#if night}<Moon class="size-4 text-indigo-300" />{:else if phase?.kind === 'dawn'}<Sun
          class="size-4 text-amber-300"
        />{:else}<ScrollText class="size-4 text-amber-200" />{/if}
      <span>{banner}</span>
      {#if !gameOver}<span class="tabular-nums text-white/60">{Math.ceil(view.seconds_left)}s</span
        >{/if}
    </div>
    {#if prompt}
      <div
        class="pointer-events-auto flex items-center gap-2 rounded-full bg-amber-400 px-3 py-1 text-xs font-bold text-slate-900"
      >
        {prompt}
        {#if myVote !== null}<span class="font-normal">Chosen: {nameOf(myVote)}</span>{/if}
        {#if picking || ability}<button
            class="ml-1 rounded-full bg-slate-900/20 p-0.5"
            aria-label="Cancel"
            onclick={clearPick}><X class="size-3" /></button
          >{/if}
      </div>
    {/if}
  </div>

  <aside
    class="pointer-events-auto absolute right-3 top-16 flex w-64 max-w-[45%] flex-col gap-2 max-md:hidden"
  >
    <div class="{panel} p-2">
      <div
        class="mb-1 flex items-center justify-between px-1 text-[10px] font-black uppercase tracking-[0.16em] text-white/50"
      >
        <span>The town</span><span>Round {view.round}, deck {view.deck}</span>
      </div>
      {#each view.players as player (player.seat)}
        {@const seat = view.seats[player.seat]}
        {@const target = wantsPlayer && canTarget(player.seat)}
        <button
          class="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left text-xs transition
            {player.seat === mySeat ? 'bg-white/10' : ''} {target
            ? 'ring-1 ring-amber-300 hover:bg-amber-400/20'
            : wantsPlayer
              ? 'opacity-40'
              : ''}
            {player.alive ? '' : 'text-white/40 line-through'}"
          disabled={!target}
          onclick={() => choosePlayer(player.seat)}
        >
          <span
            class="size-2 shrink-0 rounded-full {seat?.connected
              ? 'bg-emerald-400'
              : 'bg-slate-500'}"
          ></span>
          <span class="min-w-0 flex-1 truncate font-semibold">{seat?.name}</span>
          <span class="truncate text-white/60">{player.character_title}</span>
          {#if player.witch}<Cat class="size-3 text-fuchsia-300" />{/if}
          {#if view.familiar === player.seat}<span class="text-[10px] text-fuchsia-200">cat</span
            >{/if}
          {#if player.accusations}<span class="rounded bg-rose-600/80 px-1 font-bold tabular-nums"
              >{player.accusations}</span
            >{/if}
          <span class="flex gap-0.5">
            {#each player.verdicts as v, i (i)}
              <span
                class="size-2 rounded-sm"
                style="background:{v.revealed && v.card ? VERDICTS[v.card].color : '#475569'}"
              ></span>
            {/each}
          </span>
          {#if !player.alive}<Skull class="size-3" />{/if}
        </button>
      {/each}
    </div>
    <div class="{panel} max-h-48 overflow-y-auto p-2 text-[11px] leading-snug text-white/75">
      {#each [...view.log].reverse() as entry, i (i)}
        <div class="border-b border-white/5 py-0.5 last:border-0">
          {describe(entry, view.seats)}
        </div>
      {/each}
    </div>
  </aside>

  {#if phase?.kind === 'reveal' && phase.chooser === mySeat}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">Turn over one of {nameOf(phase.accused)}'s cards</div>
      <div class="mt-3 flex justify-center gap-2">
        {#each hidden(phase.accused) as v (v.i)}
          <button
            class="h-16 w-11 rounded-md border border-white/30 bg-[#3b2a1e] text-xs font-bold hover:bg-[#5a3f2c]"
            onclick={() => act({ type: 'reveal', index: v.i })}>{v.i + 1}</button
          >
        {/each}
      </div>
    </div>
  {/if}

  {#if phase?.kind === 'conspiracy' && phase.from !== null && !phase.picked && meP?.alive}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">Take a face-down card from {nameOf(phase.from)}</div>
      <div class="mt-3 flex justify-center gap-2">
        {#each hidden(phase.from) as v (v.i)}
          <button
            class="h-16 w-11 rounded-md border border-white/30 bg-[#3b2a1e] text-xs font-bold hover:bg-[#5a3f2c]"
            onclick={() => act({ type: 'conspiracy_pick', index: v.i })}>{v.i + 1}</button
          >
        {/each}
      </div>
    </div>
  {:else if phase?.kind === 'conspiracy'}
    <div
      class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 {panel} px-4 py-2 text-sm"
    >
      Waiting on {phase.waiting}
      {phase.waiting === 1 ? 'player' : 'players'}.
    </div>
  {/if}

  {#if phase?.kind === 'confess' && meP?.alive && mySeat !== null && !phase.confessed.includes(mySeat)}
    <div
      class="pointer-events-auto absolute bottom-40 left-1/2 -translate-x-1/2 {panel} p-3 text-center"
    >
      {#if confessing}
        <div class="text-xs text-white/70">
          Turn over one of your own cards to be spared tonight.
        </div>
        <div class="mt-2 flex justify-center gap-2">
          {#each hidden(mySeat) as v (v.i)}
            <button
              class="h-14 w-10 rounded-md border border-white/30 text-[10px] font-bold"
              style="background:{v.card ? VERDICTS[v.card].color : '#3b2a1e'}; color:#0f172a"
              onclick={() => act({ type: 'confess', index: v.i })}
              >{v.card ? VERDICTS[v.card].title : ''}</button
            >
          {/each}
          <button class="rounded-md bg-white/10 px-2 text-xs" onclick={() => (confessing = false)}
            >Hold</button
          >
        </div>
      {:else}
        <button
          class="rounded-xl bg-rose-500 px-4 py-2 text-sm font-bold text-white hover:bg-rose-400"
          onclick={() => (confessing = true)}>Confess</button
        >
      {/if}
    </div>
  {/if}

  {#if ability === 'peek' && peekTarget !== null}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">Look at which of {nameOf(peekTarget)}'s cards?</div>
      <div class="mt-3 flex justify-center gap-2">
        {#each hidden(peekTarget) as v (v.i)}
          <button
            class="h-16 w-11 rounded-md border border-white/30 bg-[#3b2a1e] text-xs font-bold hover:bg-[#5a3f2c]"
            onclick={() => {
              act({ type: 'peek', target: peekTarget!, index: v.i });
              clearPick();
            }}>{v.i + 1}</button
          >
        {/each}
      </div>
    </div>
  {/if}

  {#if picking?.step === 'blue' && picking.target !== undefined}
    <div
      class="pointer-events-auto absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-4 text-center"
    >
      <div class="text-sm font-bold">Which blue card burns?</div>
      <div class="mt-3 flex flex-wrap justify-center gap-2">
        {#each view.players[picking.target].table as card, i (i)}
          {#if CARDS[card].color === 'blue'}
            <button
              class="rounded-md px-2 py-1 text-xs font-bold"
              style="background:{COLOR_HEX.blue}"
              onclick={() => chooseBlue(i)}>{CARDS[card].title}</button
            >
          {/if}
        {/each}
      </div>
    </div>
  {/if}

  {#if gameOver && phase?.kind === 'over'}
    <div
      class="absolute left-1/2 top-1/2 w-80 -translate-x-1/2 -translate-y-1/2 {panel} p-5 text-center"
    >
      <div
        class="text-3xl font-black {phase.winner === 'town'
          ? 'text-amber-300'
          : 'text-fuchsia-300'}"
      >
        {phase.winner === 'town' ? 'The town wins' : 'The coven wins'}
      </div>
      <div class="mt-3 flex flex-col gap-1 text-left text-sm">
        {#each view.players as player (player.seat)}
          <div class="flex items-center gap-2">
            <span class="flex-1 truncate">{view.seats[player.seat]?.name}</span
            >{#if player.witch}<Cat class="size-3.5 text-fuchsia-300" /><span
                class="text-fuchsia-200">witch</span
              >{:else}<span class="text-white/60">townsperson</span>{/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if meP}
    <div
      class="pointer-events-auto absolute bottom-3 left-1/2 flex w-[min(52rem,calc(100%-1rem))] -translate-x-1/2 flex-col gap-2"
    >
      <div class="flex items-end gap-2 max-md:flex-col max-md:items-stretch">
        <div class="{panel} w-44 shrink-0 p-2 text-xs max-md:w-auto">
          <div class="flex items-center justify-between">
            <span class="font-black">{character?.title}</span>
            {#if iAmWitch}<span class="flex items-center gap-1 text-fuchsia-300"
                ><Cat class="size-3" /> witch</span
              >{:else if view.constable}<span class="text-amber-300">constable</span>{/if}
          </div>
          <div class="mt-1 text-white/70">{character?.text}</div>
          {#if character?.ability}
            <button
              class="mt-2 w-full rounded-lg bg-amber-400 px-2 py-1 font-bold text-slate-900 disabled:opacity-40"
              disabled={!abilityReady}
              onclick={useAbility}
            >
              <Sparkles class="mr-1 inline size-3" />{meP.ability_used
                ? 'Ability used'
                : 'Use ability'}
            </button>
          {/if}
          <div class="mt-2 flex gap-1">
            {#each meP.verdicts as v, i (i)}
              <span
                class="flex h-8 flex-1 items-center justify-center rounded border text-[9px] font-bold {v.revealed
                  ? 'border-white/40'
                  : 'border-dashed border-white/15'}"
                style="background:{v.card
                  ? VERDICTS[v.card].color + '33'
                  : '#1e1b2e'}; color:{v.card ? VERDICTS[v.card].color : '#94a3b8'}"
                >{v.card ? VERDICTS[v.card].title : '?'}</span
              >
            {/each}
          </div>
        </div>
        <div class="{panel} min-w-0 flex-1 p-2">
          <div
            class="flex flex-wrap items-center gap-1 px-1 text-[10px] font-black uppercase tracking-[0.16em] text-white/50"
          >
            <span>Your hand</span>
            <span class="ml-auto flex gap-1">
              <button
                class="rounded-lg bg-white/10 px-2 py-1 text-[11px] font-bold normal-case tracking-normal hover:bg-white/20"
                onclick={() => (showRules = !showRules)}
                ><BookOpen class="mr-1 inline size-3" />Cards</button
              >
              <button
                class="rounded-lg bg-emerald-500 px-3 py-1 text-[11px] font-bold normal-case tracking-normal text-slate-900 disabled:opacity-30"
                disabled={!canDraw}
                onclick={() => act({ type: 'draw' })}>Draw</button
              >
              <button
                class="rounded-lg bg-amber-400 px-3 py-1 text-[11px] font-bold normal-case tracking-normal text-slate-900 disabled:opacity-30"
                disabled={!canEnd}
                onclick={() => act({ type: 'end_turn' })}>End turn</button
              >
            </span>
          </div>
          <div class="mt-1 flex gap-1.5 overflow-x-auto pb-1">
            {#each view.hand ?? [] as card, i (i)}
              <button
                class="flex h-24 w-[4.6rem] shrink-0 flex-col rounded-lg border px-1.5 py-1 text-left transition {cardClass(
                  card,
                )} {selected === i ? '-translate-y-2 ring-2 ring-amber-300' : ''} {canPlay
                  ? 'hover:-translate-y-1'
                  : 'opacity-70'}"
                style="background:{COLOR_HEX[CARDS[card].color]}"
                disabled={!canPlay}
                onclick={() => clickCard(i)}
                title={CARDS[card].text}
              >
                <span class="text-[11px] font-black leading-tight">{CARDS[card].title}</span>
                <span class="mt-auto line-clamp-3 text-[9px] leading-tight text-white/80"
                  >{CARDS[card].text}</span
                >
              </button>
            {/each}
            {#if !(view.hand ?? []).length}<div class="px-2 py-6 text-xs text-white/50">
                No cards in hand. Draw when it is your turn.
              </div>{/if}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if showRules}
    <div
      class="pointer-events-auto absolute inset-x-3 bottom-36 top-16 overflow-y-auto {panel} p-4 text-xs"
    >
      <div class="flex items-center justify-between">
        <span class="text-sm font-black">How Coven plays</span><button
          aria-label="Close"
          onclick={() => (showRules = false)}><X class="size-4" /></button
        >
      </div>
      <p class="mt-2 text-white/75">
        Everyone has face-down verdict cards. Anyone who holds, or ever held, a Witch is a witch. On
        your turn draw two cards or play cards on others. Seven accusations on a player lets the
        accuser turn one of their verdict cards over. When Nightfall is drawn the witches kill and
        the constable guards; anyone may confess by revealing a card to be spared. The town wins
        when every Witch card is face up. The witches win when only witches are left.
      </p>
      <div class="mt-3 grid gap-1 sm:grid-cols-2">
        {#each Object.entries(CARDS) as [key, card] (key)}
          <div class="flex gap-2 rounded-lg bg-white/5 p-1.5">
            <span
              class="mt-0.5 size-3 shrink-0 rounded-sm"
              style="background:{COLOR_HEX[card.color]}"
            ></span><span><b>{card.title}.</b> {card.text}</span>
          </div>
        {/each}
      </div>
      <div class="mt-3 grid gap-1 sm:grid-cols-2">
        {#each Object.entries(CHARACTERS) as [key, c] (key)}
          <div class="rounded-lg bg-white/5 p-1.5"><b>{c.title}.</b> {c.text}</div>
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

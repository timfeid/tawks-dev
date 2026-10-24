<script lang="ts">
  import { TrainFront, Pause, Play, Maximize2, Minimize2, Scan, Ticket, X, Check, Users, Trophy, Layers, Map } from '@lucide/svelte';
  import type { Card, Game, TicketView } from './types';

  let { game, colors, labels, playerColor, paused, fullscreen, togglePause, toggleFullscreen, fitMap, submit,
    selectedId, closeRoute, claim, offerKeep, toggleOffer, keepTickets, previewTicket, clearTicket, mobilePanel = $bindable('map') }: {
    mobilePanel?: 'map' | 'cards' | 'tickets' | 'crews';
    game: Game;
    colors: Record<string, string>;
    labels: Record<string, string>;
    playerColor: (id: string) => string;
    paused: boolean;
    fullscreen: boolean;
    togglePause: () => void;
    toggleFullscreen: () => void;
    fitMap: () => void;
    submit: (action: string, payload?: Record<string, any>) => void;
    selectedId: number | null;
    closeRoute: () => void;
    claim: (id: number, card: Card) => void;
    offerKeep: Set<number>;
    toggleOffer: (id: number) => void;
    keepTickets: () => void;
    previewTicket: (ticket: TicketView) => void;
    clearTicket: () => void;
  } = $props();
  const viewer = $derived(game.viewer);
  const selected = $derived(game.routes.find((route) => route.id === selectedId));
  const options = $derived(viewer?.claimable.find((route) => route.route === selectedId)?.options ?? []);
  const activeName = $derived(game.players.find((player) => player.id === game.active_player_id)?.name ?? '');
  const cityName = (id: string) => game.cities.find((city) => city.id === id)?.name ?? id;
  // A touch preview stays on the board when the player closes the drawer.
  const clearHover = () => { if (window.matchMedia('(hover: hover)').matches) clearTicket(); };
  const offerKey = $derived(viewer?.ticket_offer.map(ticket => ticket.id).join(',') ?? '');
  $effect(() => { mobilePanel = game.stage !== 'playing' ? 'cards' : offerKey ? 'tickets' : 'map'; });
  $effect(() => { if (selectedId !== null) mobilePanel = 'tickets'; });
</script>

<div class="hud" data-panel={mobilePanel}>
  <header class="topbar">
    <div class="identity"><TrainFront size={25} strokeWidth={1.5} /><div><h1>TRACKLINES</h1><span>NORTH AMERICA / 26 STATIONS</span></div></div>
    <div class="status" class:active={viewer?.is_active}>
      <span class="status-dot"></span>
      {#if game.stage === 'lobby'}Boarding open{:else if game.stage === 'finished'}Journey complete{:else if viewer?.ticket_offer.length}Choose your destinations{:else if viewer?.is_active}Your turn{:else}{activeName}'s turn{/if}
      {#if game.stage === 'playing'}<small>{game.final_round ? 'FINAL ROUND' : `ROUND ${game.round}`}</small>{/if}
    </div>
    <nav class="map-controls" aria-label="Map controls">
      <button class="icon-button" onclick={togglePause} aria-label={paused ? 'Resume trains' : 'Pause trains'} title={paused ? 'Resume trains' : 'Pause trains'}>{#if paused}<Play size={17} />{:else}<Pause size={17} />{/if}</button>
      <button class="icon-button" onclick={fitMap} aria-label="Fit map" title="Fit map"><Scan size={18} /></button>
      <button class="icon-button" onclick={toggleFullscreen} aria-label={fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'} title={fullscreen ? 'Exit fullscreen (Esc)' : 'Fullscreen'}>{#if fullscreen}<Minimize2 size={18} />{:else}<Maximize2 size={18} />{/if}</button>
    </nav>
  </header>

  <aside class="roster" aria-label="Players">
    <button class="mobile-close" onclick={() => mobilePanel = 'map'} aria-label="Close crews"><span>CREWS</span><X size={18} /></button>
    <div class="section-label"><Users size={12} /> CREWS <span>{game.players.length}/5</span></div>
    {#each game.players as player (player.id)}
      <div class="player" class:current={player.is_active} style="--player:{playerColor(player.id)}">
        <div class="player-name"><i></i><strong>{player.name}</strong>{#if player.is_ai}<small>AI</small>{/if}<b>{player.score}</b></div>
        <div class="player-stats"><span>{player.trains} trains</span><span>{player.hand_size} cards</span></div>
      </div>
    {/each}
    {#if game.stage === 'lobby'}<p class="roster-note">{5 - game.players.length} open seats<br />Invite friends or add a practice AI.</p>{/if}
  </aside>

  {#if game.stage === 'playing' && viewer}
    <aside class="inspector" aria-label="Route planning">
      <button class="mobile-close" onclick={() => mobilePanel = 'map'} aria-label="Close planning"><span>BACK TO MAP</span><X size={18} /></button>
      {#if viewer.ticket_offer.length}
        <div class="section-label"><Ticket size={14} /> DESTINATIONS <span>{offerKeep.size}/{viewer.must_keep}+</span></div>
        <h2>Choose tickets</h2>
        <p>Keep at least {viewer.must_keep}. Tap or hover a ticket to trace it on the map.</p>
        <div class="ticket-list">
          {#each viewer.ticket_offer as ticket (ticket.id)}
            <label class="ticket-row" class:checked={offerKeep.has(ticket.id)} onmouseenter={() => previewTicket(ticket)} onmouseleave={clearHover}>
              <input type="checkbox" checked={offerKeep.has(ticket.id)} onchange={() => { toggleOffer(ticket.id); previewTicket(ticket); }} onfocus={() => previewTicket(ticket)} onblur={clearHover} />
              <span>{ticket.from_name}<small>to {ticket.to_name}</small></span><b>{ticket.points}<small>PTS</small></b>
            </label>
          {/each}
        </div>
        <button class="primary wide" disabled={offerKeep.size < viewer.must_keep} onclick={keepTickets}>Keep {offerKeep.size} tickets</button>
        <p class="fine-print">Unfinished tickets cost their points at the end.</p>
      {:else if selected}
        <div class="section-label"><TrainFront size={14} /> ROUTE DETAILS <button class="close" onclick={closeRoute} aria-label="Close route"><X size={16} /></button></div>
        <h2>{cityName(selected.from)}<small>to {cityName(selected.to)}</small></h2>
        <div class="route-facts"><span style="--card:{colors[selected.color]}"><i></i>{labels[selected.color]}</span><span>{selected.length} cards + trains</span><b>{selected.points} pts</b></div>
        {#if selected.owner}
          <p>Claimed by {game.players.find((player) => player.id === selected.owner)?.name}.</p>
        {:else if options.length}
          {#each options as option (option.card)}
            <button class="primary wide claim" onclick={() => claim(selected!.id, option.card)}>Claim with {selected.length - option.locomotives} {labels[option.card]}{#if option.locomotives} + {option.locomotives} locomotive{option.locomotives === 1 ? '' : 's'}{/if}</button>
          {/each}
        {:else}<p>{viewer.is_active && game.phase === 'start' ? 'You do not hold the cards for this route.' : 'Open route. Claim it on your turn.'}</p>{/if}
      {:else}
        <div class="section-label"><Ticket size={14} /> YOUR DESTINATIONS <span>{viewer.tickets.filter((ticket) => ticket.completed).length}/{viewer.tickets.length}</span></div>
        {#each viewer.tickets as ticket (ticket.id)}
          <button class="ticket-row saved" class:checked={ticket.completed} onmouseenter={() => previewTicket(ticket)} onmouseleave={clearHover} onfocus={() => previewTicket(ticket)} onblur={clearHover} onclick={() => previewTicket(ticket)}>
            {#if ticket.completed}<Check size={14} />{:else}<Ticket size={14} />{/if}
            <span>{ticket.from_name}<small>to {ticket.to_name}</small></span><b>{ticket.points}<small>PTS</small></b>
          </button>
        {/each}
        <p class="fine-print">Select a line on the map to see its cost and claim it.</p>
      {/if}
    </aside>
  {/if}

  <footer class="command-dock" class:playing={game.stage === 'playing'} aria-label="Game controls">
    <button class="mobile-close" onclick={() => mobilePanel = 'map'} aria-label="Close game controls"><span>{game.stage === 'playing' ? 'CARDS' : 'BOARDING'}</span><X size={18} /></button>
    {#if game.stage === 'lobby'}
      <div class="boarding-copy"><div class="section-label">YOUR NEXT JOURNEY</div><h2>Build a line across the continent.</h2><p>Claim tracks. Connect your destinations. Two to five players.</p></div>
      <div class="boarding-actions">
        {#if !viewer?.seated}<button class="primary" onclick={() => submit('join')} disabled={game.players.length >= 5}><TrainFront size={17} /> {game.players.length >= 5 ? 'Table full' : 'Take a seat'}</button>{/if}
        {#if viewer?.is_host}
          <button class="secondary" onclick={() => submit('add_ai')} disabled={game.players.length >= 5}>Add a practice AI</button>
          <button class="primary" onclick={() => submit('start')} disabled={game.players.length < 2}><Play size={16} /> Start game</button>
        {:else if viewer?.seated}<span class="waiting">Seat reserved. Waiting for the host.</span>{/if}
      </div>
    {:else if game.stage === 'finished'}
      <div class="boarding-copy"><div class="section-label"><Trophy size={14} /> FINAL STANDINGS</div><h2>{game.players.find((player) => player.id === game.winner_id)?.name ?? 'Nobody'} wins</h2>
        <div class="standings">{#each game.standings as row, i}<span title="{row.tickets_done} completed, {row.tickets_missed} missed{row.longest_path ? ', longest line' : ''}">{i + 1}. {row.name} <b>{row.score}</b></span>{/each}</div>
      </div>
      <button class="primary" onclick={() => submit('reset')}>Play again</button>
    {:else if viewer}
      <section class="hand" aria-label="Your hand"><div class="section-label"><Layers size={13} /> YOUR HAND <span>{viewer.hand.reduce((sum, item) => sum + item.count, 0)} CARDS</span></div>
        <div class="hand-cards">{#each viewer.hand as item (item.card)}<div class="hand-card" style="--card:{colors[item.card]}" title={labels[item.card]}><span>{labels[item.card]}</span><b>{item.count}</b></div>{/each}{#if !viewer.hand.length}<p>No cards yet</p>{/if}</div>
      </section>
      <section class="market" aria-label="Train cards"><div class="section-label">TRAIN CARDS <span>{viewer.can_draw_deck ? 'CHOOSE A CARD' : 'WAITING'}</span></div>
        <div class="market-cards">{#each game.face_up as card, slot (slot + card)}<button class="market-card" style="--card:{colors[card]}" aria-label="Draw {labels[card]} card" disabled={!viewer.can_draw_deck || (card === 'locomotive' && game.phase === 'drew_one')} onclick={() => submit('draw_card', { face_up: slot })}><i></i><span>{labels[card]}</span></button>{/each}</div>
      </section>
      <div class="draw-actions"><button class="secondary" disabled={!viewer.can_draw_deck} onclick={() => submit('draw_card', { face_up: null })}>Draw from deck <small>{game.deck_count}</small></button><button class="secondary" disabled={!viewer.can_draw_tickets} onclick={() => submit('draw_tickets')}>Draw tickets <small>{game.ticket_deck_count}</small></button></div>
    {/if}
  </footer>
  <nav class="mobile-tabs" aria-label="Game panels">
    <button aria-pressed={mobilePanel === 'map'} onclick={() => mobilePanel = 'map'}><Map size={19} /><span>Map</span></button>
    <button aria-pressed={mobilePanel === 'cards'} onclick={() => mobilePanel = mobilePanel === 'cards' ? 'map' : 'cards'}><Layers size={19} /><span>{game.stage === 'playing' ? 'Cards' : 'Boarding'}</span></button>
    {#if game.stage === 'playing' && viewer}<button aria-pressed={mobilePanel === 'tickets'} onclick={() => mobilePanel = mobilePanel === 'tickets' ? 'map' : 'tickets'}><Ticket size={19} /><span>Tickets{viewer.ticket_offer.length ? ` (${viewer.ticket_offer.length})` : ''}</span></button>{/if}
    <button aria-pressed={mobilePanel === 'crews'} onclick={() => mobilePanel = mobilePanel === 'crews' ? 'map' : 'crews'}><Users size={19} /><span>Crews</span></button>
  </nav>
  <div class="navigation-hint">DRAG TO PAN <span>·</span> SCROLL TO ZOOM</div>
</div>

<style>
  .hud { position:absolute; inset:0; pointer-events:none; color:#e7ebdf; font:13px/1.4 system-ui,sans-serif; container-type:size; }
  .mobile-tabs,.mobile-close { display:none; }
  .topbar,.roster,.inspector,.command-dock { pointer-events:auto; }
  .topbar { position:absolute; inset:0 0 auto; height:72px; padding:0 24px; display:flex; align-items:center; justify-content:space-between; gap:20px; background:linear-gradient(#102024f5,#102024bb); border-bottom:1px solid #a7b5ab20; }
  .identity { display:flex; gap:12px; align-items:center; color:#deb97b; }
  h1 { font-size:16px; letter-spacing:0.2em; font-weight:750; margin:0; color:#f6e9cf; }
  .identity span { font-size:9px; letter-spacing:0.15em; color:#96a79e; }
  .status { display:flex; align-items:center; gap:8px; color:#a9b9ad; font-size:12px; }
  .status small { font-size:9px; border-left:1px solid #ffffff20; padding-left:10px; margin-left:6px; color:#91a294; letter-spacing:0.08em; }
  .status-dot { width:6px; height:6px; border-radius:50%; background:#97a497; }
  .status.active { color:#f2d59a; }.active .status-dot { background:#f2c778; box-shadow:0 0 12px #eab95a70; }
  .map-controls { display:flex; gap:5px; }.icon-button { width:34px; height:34px; display:grid; place-items:center; border:1px solid #c2cec51b; border-radius:6px; background:#263a3b80; }
  button { cursor:pointer; transition:background 120ms,border-color 120ms; }button:hover:not(:disabled) { filter:brightness(1.15); }button:disabled { opacity:0.4; cursor:not-allowed; }button:focus-visible,input:focus-visible { outline:2px solid #e5c182; outline-offset:3px; }
  .roster { position:absolute; left:20px; top:92px; width:168px; }.section-label { display:flex; align-items:center; gap:7px; font-size:9px; letter-spacing:0.12em; color:#a4b5a8; font-weight:650; margin-bottom:12px; }.section-label>span { margin-left:auto; font-size:9px; }
  .player { padding:9px 10px; background:#122729dd; border-left:2px solid var(--player); margin-bottom:5px; border-radius:0 5px 5px 0; }.player.current { background:#304442e8; }
  .player-name { display:flex; align-items:center; gap:6px; }.player-name strong { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-size:12px; }.player-name b { margin-left:auto; font-size:15px; font-variant-numeric:tabular-nums; }.player-name small { font-size:9px; color:#97aaa0; }.player-stats { display:flex; gap:12px; color:#a3b4ab; font-size:10px; margin-top:3px; }.roster-note { font-size:11px; color:#829a90; line-height:1.8; }
  .inspector { position:absolute; top:92px; right:20px; width:264px; padding:17px; background:#13292dee; border:1px solid #bac9b226; border-top:2px solid #bb985c; border-radius:6px; max-height:calc(100% - 300px); overflow:auto; box-shadow:0 12px 30px #00000025; }
  h2 { font-size:17px; line-height:1.3; font-weight:600; margin:0 0 8px; color:#f1e8d5; }h2 small { display:block; color:#aebfb3; font-size:13px; margin-top:4px; }p { color:#9eb1a6; font-size:11px; margin:6px 0 12px; }
  .ticket-list { margin:14px 0; }.ticket-row { display:flex; align-items:center; width:100%; gap:10px; padding:10px 8px; background:#ffffff04; border:1px solid #ffffff0c; border-radius:4px; text-align:left; margin-bottom:7px; cursor:pointer; }.ticket-row:hover { border-color:#adcea873; background:#e2e6cc0b; }.ticket-row span { flex:1; font-size:12px; }.ticket-row small { display:block; font-size:10px; color:#a1b1a8; margin-top:2px; }.ticket-row b { text-align:right; font-size:17px; color:#ebce99; }.ticket-row b small { font-size:8px; letter-spacing:0.1em; }.ticket-row.checked { border-color:#82b79480; background:#55875d19; }.ticket-row input { accent-color:#d6b575; }.fine-print { font-size:10px; color:#8fa49a; margin-bottom:0; }.close { margin-left:auto; }.route-facts { display:flex; flex-wrap:wrap; gap:10px; font-size:11px; margin:14px 0; color:#c2cdbf; }.route-facts span { display:flex; align-items:center; gap:5px; }.route-facts i { width:10px; height:10px; background:var(--card); border-radius:2px; }
  .command-dock { position:absolute; bottom:20px; left:20px; right:20px; display:flex; align-items:center; justify-content:space-between; gap:28px; padding:20px 24px; background:#10252af2; border:1px solid #b2c3b326; border-top:2px solid #a3834e; border-radius:8px; box-shadow:0 12px 40px #00000040; }.boarding-copy .section-label { color:#caad75; margin-bottom:6px; }.boarding-copy p { margin:0; }.boarding-actions { display:flex; gap:10px; align-items:center; }.primary,.secondary { display:flex; align-items:center; justify-content:center; gap:8px; padding:11px 18px; border-radius:4px; white-space:nowrap; font-size:12px; font-weight:650; }.primary { background:#dcbd80; color:#182a29; border:1px solid #ebd09b; }.secondary { background:#263b3e; color:#d1dcd0; border:1px solid #a7bcab30; }.wide { width:100%; }.claim { white-space:normal; margin-top:8px; }.waiting { color:#c2cdbb; font-size:12px; }
  .playing { align-items:stretch; gap:20px; padding:15px 20px; }.hand { flex:1; min-width:0; }.hand-cards { display:flex; gap:5px; flex-wrap:wrap; }.hand-card { min-width:48px; border-top:3px solid var(--card); border-radius:4px; background:#23373b; padding:6px 9px; }.hand-card span { display:block; font-size:9px; color:#b6c4bb; }.hand-card b { font-size:20px; font-variant-numeric:tabular-nums; color:#f0e9d7; }.market { flex:1.3; min-width:0; }.market-cards { display:flex; gap:6px; }.market-card { flex:1; background:#21373b; border:1px solid #b2c3b324; border-radius:5px; padding:8px 5px; min-width:0; }.market-card i { display:block; height:28px; width:20px; background:var(--card); border:1px solid #ffffff40; border-radius:3px; margin:0 auto 6px; box-shadow:3px 3px 0 #00000030; }.market-card span { font-size:9px; }.draw-actions { display:flex; flex-direction:column; justify-content:flex-end; gap:7px; }.draw-actions .secondary { justify-content:space-between; padding:9px 12px; font-size:11px; }.draw-actions small { color:#b4c4b9; font-weight:400; }.standings { display:flex; gap:16px; flex-wrap:wrap; font-size:12px; color:#a7b9ae; }.standings b { color:#efcf94; margin-left:5px; }
  .navigation-hint { position:absolute; bottom:158px; left:24px; font-size:8px; letter-spacing:0.15em; color:#8fa9a6; }.navigation-hint span { margin:0 8px; }
  @container (max-width: 900px) { .topbar { padding:0 14px; gap:10px; }.status small,.identity span { display:none; }.roster { width:138px; left:12px; }.inspector { right:12px; width:228px; padding:13px; }.command-dock { left:12px; right:12px; bottom:12px; padding:14px; gap:12px; }.boarding-copy h2 { font-size:15px; }.boarding-copy p { max-width:250px; }.boarding-actions { flex-wrap:wrap; justify-content:flex-end; }.primary,.secondary { padding:9px 12px; }.hand-card { min-width:36px; padding:5px; }.market-cards { gap:3px; }.market-card span { font-size:8px; }.draw-actions { max-width:124px; } }
  @container (max-width:760px) or ((max-height:500px) and (max-width:1000px)) {
    .topbar { height:calc(60px + env(safe-area-inset-top)); padding:env(safe-area-inset-top) max(10px,env(safe-area-inset-right)) 0 max(10px,env(safe-area-inset-left)); gap:6px; }
    .identity { gap:7px; }.identity :global(svg) { width:19px; }h1 { font-size:12px; }.identity span { display:none; }
    .status { position:absolute; top:calc(60px + env(safe-area-inset-top)); left:0; right:0; height:32px; padding:0 max(12px,env(safe-area-inset-left)); background:#102024e8; font-size:11px; }.status small { display:block; margin-left:auto; }
    .map-controls { gap:2px; }.icon-button { width:44px; height:44px; }
    .roster,.inspector,.command-dock { display:none; z-index:3; top:auto; bottom:calc(72px + env(safe-area-inset-bottom)); left:max(10px,env(safe-area-inset-left)); right:max(10px,env(safe-area-inset-right)); width:auto; max-height:min(46%,360px); overflow:auto; overscroll-behavior:contain; padding:12px; background:#10252afa; border:1px solid #b2c3b340; border-top:2px solid #b89a64; border-radius:8px; }
    [data-panel='crews'] .roster,[data-panel='tickets'] .inspector { display:block; }
    [data-panel='cards'] .command-dock { display:flex; flex-wrap:wrap; gap:12px; }
    .mobile-close { display:flex; align-items:center; justify-content:space-between; width:100%; min-height:44px; margin:-8px 0 8px; color:#b9c8b6; font-size:10px; letter-spacing:0.12em; flex:none; position:sticky; top:-12px; z-index:1; background:#10252a; }
    .roster .section-label { display:none; }.player { padding:10px; }.player-stats { display:flex; }
    .boarding-copy { width:100%; }.boarding-copy h2 { font-size:16px; }.boarding-copy p { max-width:none; }.boarding-actions { width:100%; justify-content:flex-start; }
    .primary,.secondary { min-height:44px; white-space:normal; }.close { display:grid; place-items:center; min-width:44px; min-height:44px; }
    [data-panel='cards'] .playing { display:grid; grid-template-columns:minmax(0,1fr); gap:12px; }
    .hand-cards { flex-wrap:nowrap; overflow:auto; padding-bottom:4px; }.hand-card { display:flex; align-items:center; gap:6px; min-width:max-content; padding:6px 9px; }.hand-card b { font-size:17px; }.hand-card span { font-size:10px; }
    .section-label { margin-bottom:7px; }.market-card { min-height:60px; }.market-card span { font-size:9px; }.market-card i { height:24px; }.market-cards { gap:5px; }
    .draw-actions { flex-direction:row; max-width:none; }.draw-actions button { flex:1; }.draw-actions .secondary { font-size:11px; }
    .ticket-row { min-height:54px; }.ticket-row input { width:20px; height:20px; flex:none; }.inspector h2 { font-size:16px; }
    .mobile-tabs { pointer-events:auto; display:flex; position:absolute; z-index:4; bottom:0; left:0; right:0; padding:4px max(6px,env(safe-area-inset-right)) env(safe-area-inset-bottom) max(6px,env(safe-area-inset-left)); background:#102024; border-top:1px solid #a9beab30; }
    .mobile-tabs button { display:flex; flex:1; flex-direction:column; align-items:center; justify-content:center; gap:4px; min-height:60px; color:#9db2a8; font-size:10px; border-top:2px solid transparent; }.mobile-tabs button[aria-pressed='true'] { color:#edca87; border-color:#edca87; background:#dfc0850b; }
    .navigation-hint { display:none; }
  }
  @container (max-height:500px) and (min-width:601px) {
    .roster,.inspector,.command-dock { top:calc(102px + env(safe-area-inset-top)); bottom:calc(72px + env(safe-area-inset-bottom)); left:auto; width:290px; max-height:none; }
    .mobile-tabs { justify-content:center; }.mobile-tabs button { flex:0 1 130px; }
  }
</style>

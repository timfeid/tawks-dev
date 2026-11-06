<script lang="ts">
  import { Shield, Siren, Users } from '@lucide/svelte';
  import type { DriveConnection } from '../../lib/net.svelte';
  import type { Difficulty, HeistView, Role } from '../../heist/protocol';

  let { conn, view }: { conn: DriveConnection; view: HeistView } = $props();

  const host = $derived(conn.host);
  const myRole = $derived(view.me?.role ?? null);
  const thieves = $derived(view.players.filter((p) => p.role === 'thief' && p.online));
  const cops = $derived(view.players.filter((p) => p.role === 'cop' && p.online));
  const undecided = $derived(view.players.filter((p) => !p.role && p.online));
  const loading = $derived(view.phase.phase === 'loading');

  function pick(role: Role | null) {
    conn.heistSend({ op: 'role', role });
  }

  function set(patch: Partial<HeistView['settings']>) {
    conn.heistSend({ op: 'settings', settings: { ...view.settings, ...patch } });
  }
</script>

<div class="backdrop">
  <section class="panel setup">
    <header>
      <h1>Port Run</h1>
      <p>A crew slips out of Port Newark in one car to take a car the buyer wants, and brings it back before the night shift catches them. It is always night.</p>
    </header>

    {#if loading}
      <p class="loading">Loading the streets for the run...</p>
    {:else}
      <div class="sides">
        <button class="side thief" class:on={myRole === 'thief'} onclick={() => pick(myRole === 'thief' ? null : 'thief')}>
          <Users size={22} />
          <strong>Crew</strong>
          <span>{thieves.length} of {view.settings.max_thieves}. Share one car, walk the blocks, take the car, bring it in.</span>
          <ul>{#each thieves as p (p.id)}<li>{p.name}</li>{/each}</ul>
        </button>
        <button class="side cop" class:on={myRole === 'cop'} onclick={() => pick(myRole === 'cop' ? null : 'cop')}>
          <Shield size={22} />
          <strong>Police</strong>
          <span>{cops.length} of {view.settings.max_cops}. Your own cruiser, a radio, spikes and a spotlight.</span>
          <ul>{#each cops as p (p.id)}<li>{p.name}</li>{/each}</ul>
        </button>
      </div>
      {#if undecided.length}<p class="note">Still picking: {undecided.map((p) => p.name).join(', ')}</p>{/if}

      {#if host}
        <div class="settings">
          <label>Crew up to <select value={view.settings.max_thieves} onchange={(e) => set({ max_thieves: Number((e.currentTarget as HTMLSelectElement).value) })}>{#each [1, 2, 3, 4] as n}<option value={n}>{n}</option>{/each}</select></label>
          <label>Player cops up to <select value={view.settings.max_cops} onchange={(e) => set({ max_cops: Number((e.currentTarget as HTMLSelectElement).value) })}>{#each [0, 1, 2, 3, 4, 5, 6] as n}<option value={n}>{n}</option>{/each}</select></label>
          <label>AI police <select value={view.settings.difficulty} onchange={(e) => set({ difficulty: (e.currentTarget as HTMLSelectElement).value as Difficulty })}><option value="easy">Easy, 2 cars</option><option value="normal">Normal, 4 cars</option><option value="hard">Hard, 6 cars</option></select></label>
          <label>Minutes <input type="range" min="10" max="20" value={view.settings.minutes} onchange={(e) => set({ minutes: Number((e.currentTarget as HTMLInputElement).value) })} /> <em>{view.settings.minutes}</em></label>
        </div>
        <div class="row">
          <button class="primary" disabled={!thieves.length} onclick={() => conn.heistSend({ op: 'start' })}><Siren size={16} /> Start the run</button>
          <button class="ghost" onclick={() => conn.heistOpen(false)}>Back to free roam</button>
        </div>
      {:else}
        <p class="note">The host starts the run when everyone has picked a side.</p>
      {/if}
    {/if}
  </section>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    background: radial-gradient(ellipse at center, rgba(5, 8, 14, 0.6), rgba(5, 8, 14, 0.92));
    z-index: 20;
  }
  .setup {
    width: min(720px, 100%);
    max-height: calc(100dvh - 32px);
    overflow-y: auto;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-sign);
    font-size: 32px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  p {
    margin: 6px 0 0;
    color: var(--text-dim);
    line-height: 1.45;
  }
  .loading {
    padding: 30px 0;
    text-align: center;
  }
  .sides {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .side {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 14px;
    border-radius: var(--radius-small);
    border: 1px solid var(--line-strong);
    background: rgba(255, 255, 255, 0.03);
    text-align: left;
  }
  .side span {
    color: var(--text-dim);
    font-size: 13px;
  }
  .side ul {
    margin: 4px 0 0;
    padding-left: 16px;
    font-size: 13px;
  }
  .thief.on {
    border-color: var(--signal);
    background: rgba(255, 181, 71, 0.08);
  }
  .cop.on {
    border-color: #5a8cff;
    background: rgba(90, 140, 255, 0.1);
  }
  .note {
    font-size: 13px;
    color: var(--text-faint);
  }
  .settings {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 16px;
  }
  .settings label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 14px;
  }
  select {
    background: var(--panel-raised);
    color: var(--text);
    border: 1px solid var(--line-strong);
    border-radius: 6px;
    padding: 4px 6px;
  }
  input[type='range'] {
    accent-color: var(--signal);
    flex: 1;
  }
  em {
    font-style: normal;
    color: var(--signal);
    min-width: 2ch;
  }
  .row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .row button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  @media (max-width: 560px) {
    .sides,
    .settings {
      grid-template-columns: 1fr;
    }
  }
</style>

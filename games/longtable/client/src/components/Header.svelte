<script lang="ts">
  import { BookOpen, Crown, Maximize2, Minimize2, Users } from '@lucide/svelte';
  import { nameOf } from '../lib/format';
  import type { TableConnection } from '../lib/table.svelte';

  let {
    conn,
    lobbyOwner,
    fullscreen,
    onFullscreen,
    onCredits,
  }: {
    conn: TableConnection;
    lobbyOwner: boolean;
    fullscreen: boolean | null;
    onFullscreen: () => void;
    onCredits: () => void;
  } = $props();

  const table = $derived(conn.table!);
  let seatsOpen = $state(false);
  let handTo = $state('');

  const others = $derived(table.people.filter((p) => p.user_id !== table.me));
</script>

<header class="bar">
  <div class="brand">
    <span class="mark" aria-hidden="true"></span>
    <span class="word">Longtable</span>
    <button class="compat" title="Credits and rules content" onclick={onCredits}>{conn.compatibility}</button>
  </div>

  <div class="gm" title="Game master">
    <Crown size={16} />
    <span>{table.gm === table.me ? 'You run this table' : `${nameOf(table, table.gm)} runs this table`}</span>
  </div>

  <div class="tools">
    <button class="people btn quiet" aria-expanded={seatsOpen} onclick={() => (seatsOpen = !seatsOpen)}>
      <Users size={16} />
      <span class="faces">
        {#each table.people.slice(0, 6) as person (person.user_id)}
          <span class="face" class:away={!person.online} title={`${person.name}${person.online ? '' : ', away'}`}>
            {#if person.avatar_url}<img src={person.avatar_url} alt="" />{:else}{person.name.slice(0, 1)}{/if}
          </span>
        {/each}
      </span>
    </button>
    <button class="btn quiet icon" aria-label="Credits" title="Credits" onclick={onCredits}><BookOpen size={18} /></button>
    {#if fullscreen !== null}
      <button class="btn quiet icon" aria-label={fullscreen ? 'Leave full screen' : 'Full screen'} onclick={onFullscreen}>
        {#if fullscreen}<Minimize2 size={18} />{:else}<Maximize2 size={18} />{/if}
      </button>
    {/if}
  </div>

  {#if seatsOpen}
    <div class="seats" role="dialog" aria-label="Seats">
      <ul>
        {#each table.people as person (person.user_id)}
          <li>
            <span class="dot" class:on={person.online}></span>
            <span class="who">{person.name}{person.user_id === table.me ? ' (you)' : ''}</span>
            {#if person.user_id === table.gm}<span class="role"><Crown size={12} />GM</span>{/if}
          </li>
        {/each}
      </ul>
      {#if conn.isGm && others.length}
        <div class="hand">
          <select class="field" bind:value={handTo} aria-label="Hand the GM seat to">
            <option value="">Hand the GM seat to...</option>
            {#each others as p (p.user_id)}<option value={p.user_id}>{p.name}</option>{/each}
          </select>
          <button class="btn" disabled={!handTo} onclick={() => { conn.act({ type: 'set_gm', user_id: handTo }); handTo = ''; seatsOpen = false; }}>Hand over</button>
        </div>
      {:else if !conn.isGm && lobbyOwner}
        <button class="btn" onclick={() => { conn.act({ type: 'take_gm' }); seatsOpen = false; }}><Crown size={16} />Take the GM seat</button>
      {/if}
    </div>
  {/if}
</header>

<style>
  .bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 14px;
    border-bottom: 1px solid rgba(214, 165, 79, 0.22);
    background: linear-gradient(180deg, rgba(20, 13, 8, 0.85), rgba(20, 13, 8, 0.55));
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .mark {
    width: 22px;
    height: 14px;
    border-radius: 3px;
    background: linear-gradient(180deg, var(--brass-bright), var(--brass-dim));
    box-shadow: 0 0 0 2px var(--wood-600);
  }

  .word {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 700;
    letter-spacing: 0.01em;
  }

  .compat {
    background: transparent;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-faint);
    border: 1px solid rgba(214, 165, 79, 0.3);
    border-radius: 999px;
    padding: 2px 7px;
  }

  .gm {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--brass-bright);
    font-weight: 700;
    font-size: 13px;
    min-width: 0;
  }

  .gm span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tools {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .faces {
    display: flex;
  }

  .face {
    width: 24px;
    height: 24px;
    margin-left: -6px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    overflow: hidden;
    font-size: 11px;
    font-weight: 700;
    color: var(--wood-950);
    background: var(--parchment-deep);
    border: 2px solid var(--wood-900);
  }

  .face img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .face.away {
    opacity: 0.4;
  }

  .seats {
    position: absolute;
    right: 12px;
    top: calc(100% + 6px);
    z-index: 20;
    width: min(320px, calc(100vw - 24px));
    padding: 12px;
    border-radius: var(--radius);
    border: 1px solid rgba(214, 165, 79, 0.35);
    background: var(--wood-800);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .dot.on {
    background: var(--moss);
  }

  .who {
    flex: 1;
  }

  .role {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 700;
    color: var(--brass-bright);
  }

  .hand {
    display: flex;
    gap: 6px;
  }

  .hand .field {
    flex: 1;
  }

  @media (max-width: 720px) {
    .compat {
      display: none;
    }
  }
</style>

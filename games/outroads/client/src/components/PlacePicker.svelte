<script lang="ts">
  import { Check, Locate, MapPin, Search, Vote, X } from '@lucide/svelte';
  import type { DriveConnection } from '../lib/net.svelte';
  import { PRESETS, roughly } from '../lib/places';
  import type { Place } from '../lib/types';

  let {
    conn,
    here,
    onclose,
  }: { conn: DriveConnection; here: () => [number, number] | null; onclose: (() => void) | null } = $props();

  let query = $state('');
  let timer: ReturnType<typeof setTimeout> | null = null;
  let locating = $state(false);
  let nearMe = $state<Place | null>(null);
  let locateProblem = $state<string | null>(null);

  const room = $derived(conn.room);
  const host = $derived(conn.host);
  const online = $derived(room?.drivers.length ?? 1);
  const myName = $derived(room?.drivers.find((d) => d.user_id === conn.me)?.name ?? 'a driver');
  const hostName = $derived(room?.drivers.find((d) => d.user_id === room?.host)?.name ?? 'The host');

  function typed() {
    if (timer) clearTimeout(timer);
    const q = query.trim();
    if (q.length < 3) {
      conn.results = [];
      return;
    }
    timer = setTimeout(() => conn.search(q, here()), 450);
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (timer) clearTimeout(timer);
    if (query.trim().length >= 2) conn.search(query.trim(), here());
  }

  function choose(place: Place) {
    const clean = { name: place.name, lat: place.lat, lon: place.lon };
    if (host) conn.act({ kind: 'set_place', place: clean });
    else conn.act({ kind: 'nominate', place: clean });
    if (host) onclose?.();
  }

  function locate() {
    locateProblem = null;
    if (!('geolocation' in navigator)) {
      locateProblem = 'This browser has no location. Search for your town instead.';
      return;
    }
    locating = true;
    try {
      navigator.geolocation.getCurrentPosition(
        (pos) => {
          locating = false;
          nearMe = { name: `Near ${myName}`, ...roughly(pos.coords.latitude, pos.coords.longitude) };
        },
        () => {
          locating = false;
          locateProblem = 'Location is blocked here. Search for your town instead.';
        },
        { enableHighAccuracy: false, timeout: 10_000, maximumAge: 600_000 },
      );
    } catch {
      locating = false;
      locateProblem = 'Location is blocked here. Search for your town instead.';
    }
  }

  function voted(id: number) {
    return !!conn.me && !!room?.nominations.find((n) => n.id === id)?.votes.includes(conn.me);
  }
</script>

<div class="backdrop">
  <section class="panel picker" aria-label="Pick a place">
    <header>
      <div>
        <h1>Where to?</h1>
        <p>
          {#if host}
            You are hosting. Pick a place and everyone drives there.
          {:else}
            Pick a place to put it to a vote. {hostName} can also move everyone.
          {/if}
        </p>
      </div>
      {#if onclose}
        <button class="chip icon" aria-label="Close" onclick={onclose}><X size={18} /></button>
      {/if}
    </header>

    <form class="search" onsubmit={submit}>
      <Search size={18} />
      <input placeholder="Search a city, street or landmark" bind:value={query} oninput={typed} aria-label="Search places" />
      {#if conn.searching}<span class="spinner" aria-label="Searching"></span>{/if}
    </form>

    {#if conn.results.length && query.trim().length >= 2}
      <ul class="results">
        {#each conn.results as found (found.lat + ',' + found.lon)}
          <li>
            <MapPin size={16} />
            <div class="what">
              <strong>{found.name}</strong>
              <span>{found.label}</span>
            </div>
            <button class={host ? 'primary' : 'ghost'} onclick={() => choose(found)}>{host ? 'Go' : 'Put to vote'}</button>
          </li>
        {/each}
      </ul>
    {/if}

    <div class="near">
      <button class="ghost" onclick={locate} disabled={locating}>
        <Locate size={16} />
        {locating ? 'Finding you...' : 'Near me'}
      </button>
      {#if locateProblem}<span class="note">{locateProblem}</span>{/if}
    </div>
    {#if nearMe}
      <div class="confirm">
        <p>Use a spot about 100 m from you? Everyone in the lobby will see it as the spawn point. Your exact location stays on this device.</p>
        <div class="row">
          <button class="primary" onclick={() => { if (nearMe) choose(nearMe); nearMe = null; }}>{host ? 'Share and go' : 'Share and put to vote'}</button>
          <button class="ghost" onclick={() => (nearMe = null)}>Keep it private</button>
        </div>
      </div>
    {/if}

    {#if room?.nominations.length}
      <h2>On the ballot</h2>
      <ul class="ballot">
        {#each room.nominations as n (n.id)}
          <li>
            <div class="what">
              <strong>{n.place.name}</strong>
              <span>{n.votes.length} of {online} {online === 1 ? 'vote' : 'votes'}, more than half moves everyone</span>
            </div>
            {#if voted(n.id)}
              <button class="ghost voted" onclick={() => conn.act({ kind: 'unvote' })}><Check size={16} /> Voted</button>
            {:else}
              <button class="ghost" onclick={() => conn.act({ kind: 'vote', id: n.id })}><Vote size={16} /> Vote</button>
            {/if}
            {#if host}
              <button class="primary" onclick={() => { conn.act({ kind: 'set_place', place: n.place }); onclose?.(); }}>Go now</button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    <h2>Well known spots</h2>
    <div class="presets">
      {#each PRESETS as p (p.name)}
        <button class="preset" onclick={() => choose(p)}>
          <span class="kind">{p.kind === 'town' ? 'Small town' : 'City'}</span>
          <strong>{p.name}</strong>
          <span>{p.blurb}</span>
        </button>
      {/each}
    </div>
  </section>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    background: radial-gradient(ellipse at center, rgba(8, 12, 18, 0.55), rgba(8, 12, 18, 0.85));
    z-index: 20;
  }
  .picker {
    width: min(680px, 100%);
    max-height: min(760px, calc(100dvh - 32px));
    overflow-y: auto;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-sign);
    font-size: 30px;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  h2 {
    margin: 6px 0 0;
    font-size: 13px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  p {
    margin: 4px 0 0;
    color: var(--text-dim);
    line-height: 1.4;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 48px;
    padding: 0 14px;
    border-radius: var(--radius-small);
    background: var(--panel-raised);
    border: 1px solid var(--line-strong);
    color: var(--text-dim);
  }
  .search:focus-within {
    border-color: var(--signal);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: 16px;
  }
  .spinner {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--line-strong);
    border-top-color: var(--signal);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-small);
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--line);
  }
  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .what span {
    color: var(--text-faint);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .near {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .near button,
  li button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .note {
    color: var(--text-faint);
    font-size: 13px;
  }
  .confirm {
    padding: 12px;
    border-radius: var(--radius-small);
    border: 1px solid var(--signal-deep);
    background: rgba(255, 181, 71, 0.07);
  }
  .confirm p {
    margin: 0 0 10px;
  }
  .row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .voted {
    border-color: var(--go);
    color: var(--go);
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 8px;
  }
  .preset {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 10px 12px;
    border-radius: var(--radius-small);
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.03);
    text-align: left;
  }
  .preset:hover {
    border-color: var(--signal);
    background: rgba(255, 181, 71, 0.06);
  }
  .preset span {
    color: var(--text-faint);
    font-size: 13px;
  }
  .preset .kind {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--signal);
  }
  @media (max-width: 520px) {
    .picker {
      padding: 16px;
    }
    h1 {
      font-size: 24px;
    }
  }
</style>

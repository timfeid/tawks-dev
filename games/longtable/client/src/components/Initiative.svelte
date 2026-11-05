<script lang="ts">
  import { ArrowDownWideNarrow, ChevronLeft, ChevronRight, Eye, EyeOff, Trash2, UserPlus, X } from '@lucide/svelte';
  import { clampInt, nameOf } from '../lib/format';
  import type { TableConnection } from '../lib/table.svelte';

  let { conn }: { conn: TableConnection } = $props();

  const table = $derived(conn.table!);
  const order = $derived(table.initiative);
  const isGm = $derived(conn.isGm);
  const unplaced = $derived(table.sheets.filter((s) => !order.entries.some((c) => c.sheet_id === s.id)));

  let npcName = $state('');
  let npcScore = $state('');
  let npcHidden = $state(false);
  let sheetPick = $state('');
  let sheetScore = $state('');
  let confirmClear = $state(false);
  let adding = $state<boolean | null>(null);
  const addOpen = $derived(adding ?? order.entries.length === 0);

  /** A plain number is the initiative; a signed one is a bonus to roll with. */
  function readScore(text: string): { init: number | null; bonus: number } {
    const t = text.trim();
    if (!t) return { init: null, bonus: 0 };
    if (/^[+-]\d+$/.test(t)) return { init: null, bonus: clampInt(t, -30, 30, 0) };
    return { init: clampInt(t, -99, 99, 0), bonus: 0 };
  }

  function addNpc() {
    const name = npcName.trim();
    if (!name) return;
    const { init, bonus } = readScore(npcScore);
    conn.act({ type: 'init_add_npc', name, init, bonus, hidden: npcHidden });
    adding = true;
    npcName = '';
    npcScore = '';
    npcHidden = false;
  }

  function addSheet() {
    const id = sheetPick || unplaced[0]?.id;
    if (!id) return;
    const t = sheetScore.trim();
    conn.act({ type: 'init_add_sheet', sheet_id: id, init: t ? clampInt(t, -99, 99, 0) : null });
    adding = true;
    sheetPick = '';
    sheetScore = '';
  }

  function setInit(id: string, value: string, before: number) {
    const init = clampInt(value, -99, 99, before);
    if (init !== before) conn.act({ type: 'init_set', id, init });
  }
</script>

<section class="ledger" aria-label="Initiative">
  <header>
    <div class="round">
      <span class="label">Round</span>
      <span class="number big">{order.round || '-'}</span>
    </div>
    {#if isGm}
      <div class="turns">
        <button class="btn icon" aria-label="Previous turn" onclick={() => conn.act({ type: 'init_prev' })} disabled={!order.current}><ChevronLeft size={18} /></button>
        <button class="btn primary" onclick={() => conn.act({ type: 'init_next' })} disabled={order.entries.length === 0}>
          {order.current ? 'Next turn' : 'Start'}<ChevronRight size={18} />
        </button>
      </div>
    {/if}
  </header>

  {#if order.hidden_turn}
    <p class="unseen">Something unseen is acting.</p>
  {/if}

  <ol>
    {#each order.entries as c (c.id)}
      {@const up = order.current === c.id}
      <li class:up class:hidden={c.hidden}>
        <span class="marker" aria-hidden="true"></span>
        {#if isGm}
          <input
            class="field score number"
            type="number"
            value={c.init}
            aria-label={`Initiative for ${c.name}`}
            onchange={(e) => setInit(c.id, e.currentTarget.value, c.init)}
          />
        {:else}
          <span class="score number">{c.init}</span>
        {/if}
        <span class="who">
          <span class="name">{c.name}</span>
          <span class="sub">{c.owner ? nameOf(table, c.owner) : 'GM'}{c.hidden ? ', hidden' : ''}{up ? ', up now' : ''}</span>
        </span>
        {#if isGm}
          <span class="row-tools">
            <button class="btn quiet icon" aria-label={c.hidden ? `Reveal ${c.name}` : `Hide ${c.name}`} onclick={() => conn.act({ type: 'init_hide', id: c.id, hidden: !c.hidden })}>
              {#if c.hidden}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
            </button>
            <button class="btn quiet icon" aria-label={`Remove ${c.name}`} onclick={() => conn.act({ type: 'init_remove', id: c.id })}><X size={16} /></button>
          </span>
        {/if}
      </li>
    {:else}
      <li class="empty">{isGm ? 'Add the party and their foes to start.' : 'The GM has not started the order.'}</li>
    {/each}
  </ol>

  {#if isGm}
    <div class="gm-tools">
      <div class="tool-row">
        <button class="btn" onclick={() => conn.act({ type: 'init_sort' })} disabled={order.entries.length < 2}><ArrowDownWideNarrow size={16} />Sort</button>
        {#if confirmClear}
          <button class="btn" onclick={() => { conn.act({ type: 'init_clear' }); confirmClear = false; }}><Trash2 size={16} />Clear the order</button>
          <button class="btn quiet" onclick={() => (confirmClear = false)}>Keep it</button>
        {:else}
          <button class="btn quiet" onclick={() => (confirmClear = true)} disabled={order.entries.length === 0}>End combat</button>
        {/if}
      </div>

      <button class="btn quiet toggle" aria-expanded={addOpen} onclick={() => (adding = !addOpen)}>
        <UserPlus size={16} />{addOpen ? 'Done adding' : 'Add to the order'}
      </button>
      {#if addOpen}
      <div class="add">
        <span class="label">Add a character</span>
        <div class="tool-row">
          <select class="field grow" bind:value={sheetPick} aria-label="Character sheet" disabled={unplaced.length === 0}>
            <option value="">{unplaced.length === 0 ? 'Everyone is in' : `Next up: ${unplaced[0].name}`}</option>
            {#each unplaced as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
          </select>
          <input class="field small number" bind:value={sheetScore} placeholder="roll" aria-label="Initiative, or leave empty to roll" />
          <button class="btn" onclick={addSheet} disabled={unplaced.length === 0}><UserPlus size={16} />Add</button>
        </div>
      </div>

      <div class="add">
        <span class="label">Add a foe or ally</span>
        <div class="tool-row">
          <input class="field grow" bind:value={npcName} placeholder="Name" aria-label="Name" onkeydown={(e) => e.key === 'Enter' && addNpc()} />
          <input class="field small number" bind:value={npcScore} placeholder="+2" aria-label="Initiative, or a bonus like +2 to roll" onkeydown={(e) => e.key === 'Enter' && addNpc()} />
          <label class="hide-box"><input type="checkbox" bind:checked={npcHidden} />Hidden</label>
          <button class="btn" onclick={addNpc} disabled={!npcName.trim()}><UserPlus size={16} />Add</button>
        </div>
      </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .ledger {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .round {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .big {
    font-family: var(--font-display);
    font-size: 34px;
    font-weight: 700;
    color: var(--brass-bright);
  }

  .turns {
    display: flex;
    gap: 6px;
  }

  .unseen {
    margin: 0;
    color: var(--frost);
    font-style: italic;
  }

  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    min-height: 120px;
    flex: 1;
  }

  li {
    position: relative;
    display: grid;
    grid-template-columns: 6px 58px 1fr auto;
    align-items: center;
    gap: 10px;
    min-height: 54px;
    padding: 6px 8px 6px 0;
    border-radius: 10px;
    background: linear-gradient(90deg, rgba(59, 39, 24, 0.85), rgba(42, 27, 17, 0.85));
    border: 1px solid rgba(214, 165, 79, 0.16);
  }

  li.hidden {
    border-style: dashed;
    background: rgba(42, 27, 17, 0.5);
  }

  li.up {
    border-color: var(--brass);
    background: linear-gradient(90deg, rgba(214, 165, 79, 0.24), rgba(59, 39, 24, 0.9));
    box-shadow: 0 0 0 1px rgba(240, 196, 109, 0.35), 0 4px 14px rgba(0, 0, 0, 0.35);
  }

  .marker {
    align-self: stretch;
    border-radius: 10px 0 0 10px;
  }

  li.up .marker {
    background: var(--brass-bright);
    box-shadow: 0 0 10px rgba(240, 196, 109, 0.8);
  }

  .score {
    width: 58px;
    text-align: center;
    font-size: 18px;
    font-weight: 700;
  }

  span.score {
    display: grid;
    place-items: center;
    height: var(--control);
  }

  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-weight: 700;
    font-size: 16px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    font-size: 12px;
    color: var(--text-faint);
  }

  .row-tools {
    display: flex;
  }

  .empty {
    display: block;
    padding: 16px;
    color: var(--text-faint);
    font-style: italic;
    text-align: center;
  }

  .gm-tools {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 10px;
    border-top: 1px solid rgba(214, 165, 79, 0.2);
  }

  .toggle {
    align-self: flex-start;
  }

  .add {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tool-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .grow {
    flex: 1 1 140px;
  }

  .small {
    width: 70px;
  }

  .hide-box {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-dim);
  }

  select.field {
    appearance: auto;
  }

  select.field option {
    background: var(--wood-800);
  }
</style>

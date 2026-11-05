<script lang="ts">
  import { ChevronDown, ChevronUp, Minus, Plus, Shield, Trash2, X } from '@lucide/svelte';
  import { ABILITIES, clampInt, hpTone, modifier, nameOf, signed } from '../lib/format';
  import type { TableConnection } from '../lib/table.svelte';
  import type { FullSheet, SheetPatch, SheetView, Slots } from '../lib/types';

  let { conn, sheet, open = $bindable(true) }: { conn: TableConnection; sheet: SheetView; open?: boolean } = $props();

  const table = $derived(conn.table!);
  const full = $derived(sheet.access === 'full' ? (sheet as FullSheet) : null);
  const mine = $derived(sheet.owner === table.me);
  const tone = $derived(hpTone(sheet.hp, sheet.hp_max));
  const hpShare = $derived(Math.max(0, Math.min(1, sheet.hp / Math.max(1, sheet.hp_max))));
  const conditionsLeft = $derived(conn.conditions.filter((c) => !sheet.conditions.includes(c.name)));
  const summaries = $derived(new Map(conn.conditions.map((c) => [c.name, c.summary])));

  let editingSlots = $state(false);
  let confirmDelete = $state(false);
  let amount = $state('');

  function patch(p: SheetPatch) {
    conn.act({ type: 'sheet_update', id: sheet.id, patch: p });
  }

  function adjustHp(sign: 1 | -1) {
    const n = clampInt(amount || '1', 1, 9999, 1);
    patch({ hp: Math.max(-9999, Math.min(9999, sheet.hp + sign * n)) });
    amount = '';
  }

  function setAbility(i: number, value: string) {
    if (!full) return;
    const abilities = [...full.abilities];
    abilities[i] = clampInt(value, 1, 30, abilities[i]);
    patch({ abilities });
  }

  function rollCheck(i: number) {
    if (!full) return;
    const mod = modifier(full.abilities[i]);
    conn.act({ type: 'roll', formula: `1d20${mod ? signed(mod) : ''} ${ABILITIES[i]} check`, visibility: 'public' });
  }

  function setSlots(level: number, change: Partial<Slots>) {
    if (!full) return;
    const slots = full.slots.map((s) => ({ ...s }));
    const next = { ...slots[level], ...change };
    next.max = Math.max(0, Math.min(9, next.max));
    next.used = Math.max(0, Math.min(next.max, next.used));
    slots[level] = next;
    patch({ slots });
  }

  function togglePip(level: number, pip: number) {
    if (!full) return;
    const used = full.slots[level].used;
    setSlots(level, { used: pip < used ? pip : pip + 1 });
  }

  function longRest() {
    if (!full) return;
    patch({ slots: full.slots.map((s) => ({ ...s, used: 0 })), hp: full.hp_max });
  }
</script>

<article class="sheet" class:mine class:strip={!full}>
  <header>
    <div class="title">
      {#if full}
        <input class="name" value={sheet.name} aria-label="Character name" maxlength="60" onchange={(e) => patch({ name: e.currentTarget.value })} />
        <input class="class" value={sheet.class_level} placeholder="Class and level, like Ranger 3" aria-label="Class and level" maxlength="60" onchange={(e) => patch({ class_level: e.currentTarget.value })} />
      {:else}
        <span class="name">{sheet.name}</span>
        <span class="class">{sheet.class_level || 'Adventurer'}</span>
      {/if}
      <span class="player">{mine ? 'Your character' : `Played by ${nameOf(table, sheet.owner)}`}</span>
    </div>
    <div class="ac" title="Armor class">
      <Shield size={40} strokeWidth={1.5} />
      {#if full}
        <input class="number" type="number" value={sheet.ac} aria-label="Armor class" onchange={(e) => patch({ ac: clampInt(e.currentTarget.value, 0, 50, sheet.ac) })} />
      {:else}
        <span class="number">{sheet.ac}</span>
      {/if}
    </div>
    {#if full && !mine}
      <button class="fold" aria-label={open ? 'Fold the sheet' : 'Open the sheet'} onclick={() => (open = !open)}>
        {#if open}<ChevronUp size={18} />{:else}<ChevronDown size={18} />{/if}
      </button>
    {/if}
  </header>

  <div class="vitals">
    <div class="hp {tone}">
      <div class="hp-numbers">
        <span class="label">Hit points</span>
        <span class="number hp-value">
          {#if full}
            <input type="number" value={sheet.hp} aria-label="Hit points" onchange={(e) => patch({ hp: clampInt(e.currentTarget.value, -9999, 9999, sheet.hp) })} />
            <span class="of">/</span>
            <input type="number" value={sheet.hp_max} aria-label="Maximum hit points" onchange={(e) => patch({ hp_max: clampInt(e.currentTarget.value, 1, 9999, sheet.hp_max) })} />
          {:else}
            {sheet.hp}<span class="of">/</span>{sheet.hp_max}
          {/if}
        </span>
      </div>
      <div class="bar" role="meter" aria-valuemin="0" aria-valuemax={sheet.hp_max} aria-valuenow={sheet.hp} aria-label="Hit points"><span style="width: {hpShare * 100}%"></span></div>
      {#if full}
        <div class="hp-tools">
          <button class="chip-btn" aria-label="Take damage" onclick={() => adjustHp(-1)}><Minus size={14} />Hurt</button>
          <input class="amount number" bind:value={amount} placeholder="1" aria-label="Amount" inputmode="numeric" />
          <button class="chip-btn" aria-label="Heal" onclick={() => adjustHp(1)}><Plus size={14} />Heal</button>
        </div>
      {/if}
    </div>
  </div>

  <div class="conditions">
    {#each sheet.conditions as condition (condition)}
      <span class="condition" title={summaries.get(condition)}>
        {condition}
        {#if full}<button aria-label={`Clear ${condition}`} onclick={() => patch({ conditions: sheet.conditions.filter((c) => c !== condition) })}><X size={12} /></button>{/if}
      </span>
    {/each}
    {#if sheet.exhaustion > 0 && !full}<span class="condition">Exhaustion {sheet.exhaustion}</span>{/if}
    {#if full}
      <select
        class="add-condition"
        aria-label="Add a condition"
        value=""
        onchange={(e) => {
          const name = e.currentTarget.value;
          e.currentTarget.value = '';
          if (name) patch({ conditions: [...sheet.conditions, name] });
        }}
      >
        <option value="">Add condition</option>
        {#each conditionsLeft as c (c.name)}<option value={c.name}>{c.name}</option>{/each}
      </select>
    {/if}
  </div>

  {#if full && open}
    <div class="abilities">
      {#each ABILITIES as ability, i (ability)}
        <div class="ability">
          <span class="label">{ability}</span>
          <button class="mod number" title={`Roll a ${ability} check`} onclick={() => rollCheck(i)}>{signed(modifier(full.abilities[i]))}</button>
          <input class="score number" type="number" value={full.abilities[i]} aria-label={`${ability} score`} onchange={(e) => setAbility(i, e.currentTarget.value)} />
        </div>
      {/each}
    </div>

    <div class="slots">
      <div class="slots-head">
        <span class="label">Spell slots</span>
        <span class="slot-tools">
          <span class="exhaustion">
            Exhaustion
            <button class="tiny" aria-label="Less exhaustion" onclick={() => patch({ exhaustion: Math.max(0, full.exhaustion - 1) })}><Minus size={12} /></button>
            <span class="number">{full.exhaustion}</span>
            <button class="tiny" aria-label="More exhaustion" onclick={() => patch({ exhaustion: Math.min(6, full.exhaustion + 1) })}><Plus size={12} /></button>
          </span>
          <button class="link" onclick={() => (editingSlots = !editingSlots)}>{editingSlots ? 'Done' : 'Edit slots'}</button>
          <button class="link" onclick={longRest}>Long rest</button>
        </span>
      </div>
      {#each full.slots as slot, level (level)}
        {#if editingSlots || slot.max > 0}
          <div class="slot-row">
            <span class="level number">{level + 1}</span>
            <span class="pips">
              {#each Array(slot.max) as _, pip (pip)}
                <button class="pip" class:used={pip < slot.used} aria-label={`Level ${level + 1} slot ${pip + 1}, ${pip < slot.used ? 'used' : 'ready'}`} onclick={() => togglePip(level, pip)}></button>
              {/each}
            </span>
            {#if editingSlots}
              <span class="slot-edit">
                <button class="tiny" aria-label={`Fewer level ${level + 1} slots`} onclick={() => setSlots(level, { max: slot.max - 1 })}><Minus size={12} /></button>
                <button class="tiny" aria-label={`More level ${level + 1} slots`} onclick={() => setSlots(level, { max: slot.max + 1 })}><Plus size={12} /></button>
              </span>
            {/if}
          </div>
        {/if}
      {/each}
      {#if !editingSlots && full.slots.every((s) => s.max === 0)}
        <p class="hint">No spell slots. Edit slots to add some.</p>
      {/if}
    </div>

    <label class="notes">
      <span class="label">Notes</span>
      <textarea rows="3" value={full.notes} placeholder="Gear, spells, bonds, the name of that innkeeper" maxlength="4000" onchange={(e) => patch({ notes: e.currentTarget.value })}></textarea>
    </label>

    <footer>
      {#if confirmDelete}
        <span>Tear up this sheet?</span>
        <button class="link danger" onclick={() => conn.act({ type: 'sheet_delete', id: sheet.id })}><Trash2 size={14} />Tear it up</button>
        <button class="link" onclick={() => (confirmDelete = false)}>Keep it</button>
      {:else}
        <button class="link" onclick={() => (confirmDelete = true)}>Remove sheet</button>
      {/if}
    </footer>
  {/if}
</article>

<style>
  .sheet {
    color: var(--ink);
    background:
      radial-gradient(ellipse at 20% 0%, rgba(255, 255, 255, 0.5), transparent 60%),
      linear-gradient(180deg, var(--parchment), var(--parchment-deep));
    border-radius: var(--radius);
    border: 1px solid var(--parchment-edge);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45), inset 0 0 0 4px rgba(255, 255, 255, 0.18);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sheet.strip {
    padding: 10px 12px;
    gap: 8px;
  }

  .sheet .label {
    color: var(--ink-soft);
  }

  header {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }

  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 700;
    color: var(--ink);
    line-height: 1.15;
  }

  input.name,
  input.class {
    border: none;
    border-bottom: 1px dashed transparent;
    background: transparent;
    padding: 0;
    width: 100%;
  }

  input.name:hover,
  input.class:hover,
  input.name:focus,
  input.class:focus {
    border-bottom-color: var(--ink-soft);
    outline: none;
  }

  .class {
    font-size: 14px;
    color: var(--ink-soft);
    font-style: italic;
  }

  .player {
    font-size: 11px;
    color: var(--ink-soft);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin-top: 2px;
  }

  .sheet.mine .player {
    color: #8a5a12;
    font-weight: 700;
  }

  .ac {
    position: relative;
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    color: #7c6342;
  }

  .ac :global(svg) {
    position: absolute;
    inset: 4px;
  }

  .ac input,
  .ac span {
    position: relative;
    width: 34px;
    text-align: center;
    font-weight: 700;
    font-size: 16px;
    color: var(--ink);
    background: transparent;
    border: none;
    padding: 0;
    margin-top: -2px;
  }

  .fold {
    border: none;
    background: transparent;
    color: var(--ink-soft);
    padding: 4px;
  }

  .hp {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .hp-numbers {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .hp-value {
    display: inline-flex;
    align-items: baseline;
    gap: 2px;
    font-size: 20px;
    font-weight: 700;
  }

  .hp-value input {
    width: 58px;
    text-align: right;
    font-weight: 700;
    border: 1px solid transparent;
    background: transparent;
    border-radius: 6px;
    padding: 0 2px;
  }

  .hp-value input + .of + input {
    text-align: left;
  }

  .hp-value input:hover,
  .hp-value input:focus {
    border-color: var(--parchment-edge);
    background: rgba(255, 255, 255, 0.4);
    outline: none;
  }

  .of {
    color: var(--ink-soft);
    padding: 0 2px;
  }

  .bar {
    height: 10px;
    border-radius: 999px;
    background: rgba(43, 32, 23, 0.15);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.25);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: #4f8a45;
    transition: width 300ms;
  }

  .hurt .bar span {
    background: #b8882b;
  }

  .low .bar span,
  .down .bar span {
    background: #b5462a;
  }

  .hp-tools {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .chip-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 30px;
    padding: 0 10px;
    border-radius: 999px;
    border: 1px solid var(--parchment-edge);
    background: rgba(255, 255, 255, 0.45);
    color: var(--ink);
    font-weight: 700;
    font-size: 13px;
  }

  .amount {
    width: 56px;
    height: 30px;
    text-align: center;
    border-radius: 999px;
    border: 1px solid var(--parchment-edge);
    background: rgba(255, 255, 255, 0.6);
    color: var(--ink);
  }

  .conditions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .condition {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 10px;
    border-radius: 999px;
    background: #5b2f1f;
    color: #fbe9d6;
    font-size: 12px;
    font-weight: 700;
  }

  .condition button {
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: inherit;
    padding: 0;
  }

  .add-condition {
    height: 26px;
    border-radius: 999px;
    border: 1px dashed var(--ink-soft);
    background: transparent;
    color: var(--ink-soft);
    font-size: 12px;
    font-weight: 700;
    padding: 0 8px;
  }

  .abilities {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 6px;
  }

  .ability {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 6px 2px;
    border-radius: 10px;
    border: 1px solid var(--parchment-edge);
    background: rgba(255, 255, 255, 0.35);
  }

  .ability .mod {
    border: none;
    background: transparent;
    font-size: 20px;
    font-weight: 700;
    color: var(--ink);
    padding: 0;
    border-radius: 6px;
  }

  .ability .mod:hover {
    color: #8a5a12;
  }

  .ability .score {
    width: 44px;
    text-align: center;
    font-size: 12px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--ink-soft);
    border-radius: 6px;
  }

  .ability .score:hover,
  .ability .score:focus {
    border-color: var(--parchment-edge);
    outline: none;
  }

  .slots {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .slots-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .slot-tools {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .exhaustion {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--ink-soft);
  }

  .slot-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .level {
    width: 18px;
    font-weight: 700;
    color: var(--ink-soft);
  }

  .pips {
    display: flex;
    gap: 5px;
    flex: 1;
    flex-wrap: wrap;
  }

  .pip {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid #6c4f2a;
    background: radial-gradient(circle at 35% 35%, #fff6dc, #e2c88f);
    padding: 0;
  }

  .pip.used {
    background: transparent;
    border-style: dashed;
    opacity: 0.55;
  }

  .slot-edit {
    display: inline-flex;
    gap: 4px;
  }

  .tiny {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    border: 1px solid var(--parchment-edge);
    background: rgba(255, 255, 255, 0.5);
    color: var(--ink);
    padding: 0;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: none;
    background: transparent;
    color: #7a4f12;
    font-weight: 700;
    font-size: 13px;
    padding: 2px 0;
  }

  .link.danger {
    color: #a3381c;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--ink-soft);
    font-style: italic;
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  textarea {
    resize: vertical;
    min-height: 64px;
    border-radius: 8px;
    border: 1px solid var(--parchment-edge);
    background: rgba(255, 255, 255, 0.5);
    color: var(--ink);
    padding: 8px;
  }

  footer {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: flex-end;
    font-size: 13px;
  }
</style>

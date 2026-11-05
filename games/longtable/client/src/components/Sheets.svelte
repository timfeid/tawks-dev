<script lang="ts">
  import { ScrollText } from '@lucide/svelte';
  import SheetCard from './SheetCard.svelte';
  import { orderSheets } from '../lib/format';
  import type { TableConnection } from '../lib/table.svelte';

  let { conn }: { conn: TableConnection } = $props();

  const table = $derived(conn.table!);
  const sheets = $derived(orderSheets(table.sheets, table.me));
  const hasOwn = $derived(table.sheets.some((s) => s.owner === table.me));
  let folded = $state<Record<string, boolean>>({});
</script>

<section class="sheets" aria-label="Character sheets">
  <div class="head">
    <span class="label">{conn.isGm ? 'Every sheet at the table' : 'The party'}</span>
    <button class="btn" onclick={() => conn.act({ type: 'sheet_create', name: '' })}>
      <ScrollText size={16} />{hasOwn || conn.isGm ? 'New sheet' : 'Make your character'}
    </button>
  </div>
  <div class="list">
    {#each sheets as sheet (sheet.id)}
      <SheetCard {conn} {sheet} bind:open={() => !folded[sheet.id], (v) => (folded[sheet.id] = !v)} />
    {:else}
      <p class="empty">No sheets yet. Make your character to bring them to the table.</p>
    {/each}
  </div>
</section>

<style>
  .sheets {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
    padding: 2px 2px 12px;
  }

  .empty {
    color: var(--text-faint);
    font-style: italic;
    text-align: center;
    padding: 20px;
  }
</style>

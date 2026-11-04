<script lang="ts">
  import RowScope from './row-scope.svelte';
  import TableImage from './table-image.svelte';
  import { getContext } from 'svelte';
  import { holdsInputs, rowKeys } from '../../utils';
  import { shown } from '../../member-names.svelte';
  /** A length such as 120px, 30% or 8rem; anything else could smuggle in more CSS. */
  const columnWidth = (width: string | null | undefined) =>
    width && /^\d{1,4}(\.\d+)?(px|%|rem|em|ch)$/.test(width.trim()) ? width.trim() : 'auto';

  import { getNestedValue, interpolate, resolveInputValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import * as look from '../../look';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let items = $derived.by(() => {
    if ('Table' in display.type) {
      const key = display.type.Table.items_key;
      const response = getNestedValue(inputs, key);
      if (!Array.isArray(response)) {
        console.warn('Expected array for Table items but got:', response);
        return [];
      }
      return response;
    }
    return [];
  });

  let headers = $derived.by(() => {
    if ('Table' in display.type) {
      return display.type.Table.headers ?? [];
    }
    return [];
  });

  const inForm = !!getContext('form-state');
  let keys = $derived(rowKeys(items, inForm || holdsInputs(display.type)));
  let columnCount = $derived.by(() => headers.length || 1);
</script>

{#if 'Table' in display.type}
  {@const table = display.type.Table}
  <div class="@container w-full overflow-auto rounded-2xl border border-foreground/10">
    <table class="w-full text-left">
      <thead>
        <tr class="border-b border-foreground/10 bg-foreground/[0.03]">
          {#each headers as header}
            <th
              class={[
                'truncate',
                'px-3 py-2 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase',
                header.numeric && 'text-right',
                header.wide_only && 'hidden @xl:table-cell',
              ]}
              style={`width: ${columnWidth(header.width)}`}
            >
              {header.label}
            </th>
          {/each}
        </tr>
      </thead>

      <tbody>
        {#each items as item, index (keys[index])}
          {@const scope = { ...inputs, [table.as]: item }}
          {#if table.section_key && shown(scope, table.section_key)}
            <tr class="border-b border-foreground/10 bg-foreground/[0.03] last:border-b-0">
              <td colspan={columnCount} class="px-3 py-2">
                {#each table.section ?? [] as sectionItem}
                  <WorkflowDisplay from="table" inputs={scope} {workflow} display={sectionItem} />
                {/each}
              </td>
            </tr>
          {:else}
            {@const state = look.stateOf(table.row_state, scope)}
            {@const accent = look.accentOf(table.row_accent, scope)}
            <tr
              data-workflow-row
              class={['border-b border-foreground/10 last:border-b-0', state !== 'Default' && look.rowState[state]]}
              style:background-color={accent ? look.accentFill(accent) : undefined}
            >
              <RowScope>
              {#each table.row as displayItem, index}
                {@const header = headers[index]}
                <td
                  class={['px-3 py-2 text-sm [&_*]:[overflow-wrap:normal]', header?.numeric && 'whitespace-nowrap text-right tabular-nums', header?.wide_only && 'hidden @xl:table-cell']}
                  style:box-shadow={accent && index === 0 ? look.accentEdge(accent) : undefined}
                >
                  {#if header?.image}
                    <div class={look.tableImageCell}>
                      <TableImage template={header.image} inputs={scope} />
                      <WorkflowDisplay from="table" inputs={scope} {workflow} display={displayItem} />
                    </div>
                  {:else}
                    <WorkflowDisplay from="table" inputs={scope} {workflow} display={displayItem} />
                  {/if}
                </td>
              {/each}
              </RowScope>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<script lang="ts">
  /** A length such as 120px, 30% or 8rem; anything else could smuggle in more CSS. */
  const columnWidth = (width: string | null | undefined) =>
    width && /^\d{1,4}(\.\d+)?(px|%|rem|em|ch)$/.test(width.trim()) ? width.trim() : 'auto';

  import { getNestedValue, interpolate, resolveInputValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
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

  let columnCount = $derived.by(() => headers.length || 1);
</script>

{#if 'Table' in display.type}
  <div class="w-full overflow-auto {display.class}">
    <table class="w-full text-left">
      <thead>
        <tr>
          {#each headers as header}
            <th class="truncate px-4 py-2" style={`width: ${columnWidth(header.width)}`}>
              {header.label}
            </th>
          {/each}
        </tr>
      </thead>

      <tbody>
        {#each items as item}
          <tr class="border-b last:border-b-0">
            {#each display.type.Table.row as displayItem}
              <td class="px-4 py-2">
                <WorkflowDisplay
                  inputs={{ ...inputs, [display.type.Table.as]: item }}
                  {workflow}
                  display={displayItem}
                />
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

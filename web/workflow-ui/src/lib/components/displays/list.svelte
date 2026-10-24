<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();
  let listDisplay = $derived(
    typeof display.type === 'string' || !('List' in display.type)
      ? null
      : display.type.List,
  );

  let items = $derived.by(() => {
    if (listDisplay) {
      const key = listDisplay.items_key;
      const response = getNestedValue(inputs, key);
      if (!Array.isArray(response)) {
        console.warn('Found non-array for the list items');
        return [];
      }

      return response;
    }

    return [];
  });
</script>

{#if listDisplay}
  <div class={display.class}>
    {#each items as item}
      {#each listDisplay.content as displayItem}
        <WorkflowDisplay
          from="wf-list"
          inputs={{ ...inputs, [listDisplay.as]: item }}
          {workflow}
          display={displayItem}
        />
      {/each}
    {/each}
  </div>
{/if}

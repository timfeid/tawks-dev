<script lang="ts">
  import { getNestedValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();
  let pageDisplay = $derived(
    typeof display.type === 'string' || !('Page' in display.type)
      ? null
      : display.type.Page,
  );
</script>

{#if pageDisplay}
  <div class="flex max-w-7xl flex-col gap-3 py-3">
    <h1 class="text-2xl">{getNestedValue(inputs, pageDisplay.title_key)}</h1>

    {#each pageDisplay.content as displayItem}
      <WorkflowDisplay from="wf-page" {inputs} {workflow} display={displayItem} />
    {/each}
  </div>
{/if}

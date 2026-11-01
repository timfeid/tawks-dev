<script lang="ts">
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { workflow, display, inputs, ...rest }: Props = $props();
  let divDisplay = $derived(
    typeof display.type === 'string' || !('Div' in display.type)
      ? null
      : display.type.Div,
  );
</script>

{#if divDisplay}
  <div>
    {#each divDisplay.content as displayItem}
      <WorkflowDisplay {...rest} {inputs} {workflow} display={displayItem} />
    {/each}
  </div>
{/if}

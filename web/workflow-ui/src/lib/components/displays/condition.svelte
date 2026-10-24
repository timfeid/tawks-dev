<script lang="ts">
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { evaluateCondition as evalCond } from '../../utils';

  export const evaluateCondition = evalCond;

  let { workflow, display, inputs }: Props = $props();
</script>

{#if 'Condition' in display.type}
  {#if evaluateCondition(display.type.Condition.condition, inputs)}
    {#if display.class && display.class.trim().length > 0}
      <div class={display.class}>
        {#each display.type.Condition.content as displayItem}
          <WorkflowDisplay from="wf-condition" {inputs} {workflow} display={displayItem} />
        {/each}
      </div>
    {:else}
      {#each display.type.Condition.content as displayItem}
        <WorkflowDisplay from="wf-condition" {inputs} {workflow} display={displayItem} />
      {/each}
    {/if}
  {/if}
{/if}
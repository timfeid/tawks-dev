<script lang="ts">
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let panel = $derived(typeof display.type !== 'string' && 'Panel' in display.type ? display.type.Panel : null);
</script>

{#if panel}
  <div class={['flex min-w-0 flex-col', look.gap.Normal, look.panel(look.toneOf(panel.tone, inputs))]}>
    {#each panel.content as child}
      <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
    {/each}
  </div>
{/if}

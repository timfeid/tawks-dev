<script lang="ts">
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let bar = $derived(typeof display.type !== 'string' && 'ActionBar' in display.type ? display.type.ActionBar : null);
</script>

{#if bar}
  <div class="sticky bottom-0 z-20 -mx-1 flex flex-wrap items-center justify-end gap-3 border-t border-foreground/10 bg-background/95 px-1 py-3 backdrop-blur [&>*]:flex-1 sm:[&>*]:flex-none">
    {#each bar.content as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
  </div>
{/if}

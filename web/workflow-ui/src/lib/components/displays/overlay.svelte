<script lang="ts">
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let overlay = $derived(typeof display.type !== 'string' && 'Overlay' in display.type ? display.type.Overlay : null);
</script>

<!-- Pinned to the tile that holds the canvas; the canvas stays playable around it. -->
{#if overlay}
  <div
    class={[
      'pointer-events-auto fixed inset-x-2 bottom-2 z-30 flex flex-col gap-3 rounded-2xl border border-foreground/10 bg-background/95 p-4 shadow-2xl backdrop-blur sm:absolute sm:inset-x-auto sm:w-80',
      look.corner[overlay.corner],
    ]}
  >
    {#each overlay.content as child}
      <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
    {/each}
  </div>
{/if}

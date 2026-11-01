<script lang="ts">
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let grid = $derived(typeof display.type !== 'string' && 'Grid' in display.type ? display.type.Grid : null);
</script>

{#if grid}
  <!-- Columns follow the room this grid has, not the window, since the sidebars take a varying share of it. -->
  <div class="@container min-w-0">
    <div class={['grid min-w-0', look.columns(grid.columns, !!grid.fixed), look.gap[grid.gap]]}>
      {#each grid.content as child}
        <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
      {/each}
    </div>
  </div>
{/if}

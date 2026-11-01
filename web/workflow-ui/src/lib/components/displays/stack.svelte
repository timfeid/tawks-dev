<script lang="ts">
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let stack = $derived(typeof display.type !== 'string' && 'Stack' in display.type ? display.type.Stack : null);
</script>

{#if stack}
  <div
    class={[
      'flex min-w-0',
      stack.direction === 'Row' ? 'flex-row' : 'flex-col',
      look.gap[stack.gap],
      look.align[stack.align ?? 'Stretch'],
      look.justify[stack.justify ?? 'Start'],
      stack.wrap && 'flex-wrap',
    ]}
  >
    {#each stack.content as child}
      <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
    {/each}
  </div>
{/if}

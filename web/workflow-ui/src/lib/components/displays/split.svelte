<script lang="ts">
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let split = $derived(typeof display.type !== 'string' && 'Split' in display.type ? display.type.Split : null);
</script>

{#if split?.rail}
  <div class="@container min-w-0">
    <div class="flex items-start gap-6">
      <div class="flex min-w-0 flex-1 flex-col gap-5">
        {#each split.content as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </div>
      <aside class="sticky top-4 hidden w-56 shrink-0 flex-col gap-5 @2xl:flex">
        {#each split.media as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </aside>
    </div>
  </div>
{:else if split}
  <div class="@container min-w-0">
    <div class="flex flex-col gap-5 @xl:flex-row">
      <div class="w-full shrink-0 @xl:w-56">
        {#each split.media as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </div>
      <div class="flex min-w-0 flex-1 flex-col gap-3">
        {#each split.content as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </div>
    </div>
  </div>
{/if}

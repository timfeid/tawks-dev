<script lang="ts">
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let empty = $derived(typeof display.type !== 'string' && 'Empty' in display.type ? display.type.Empty : null);
  let Icon = $derived(empty?.icon ? LUCIDE_REGISTRY[shown(inputs, empty.icon)] : undefined);
</script>

{#if empty}
  {#if empty.loading}
    <div class={['animate-pulse py-10', look.text('Body', 'Default', { numeric: false, clamp: 0, align: 'Center', struck: false, lines: false, themed: false })]}>
      {shown(inputs, empty.title)}
    </div>
  {:else}
    <div class="flex flex-col items-center gap-2 rounded-2xl border border-dashed border-foreground/10 px-6 py-10 text-center">
      {#if Icon}<Icon class="mb-1 size-6 text-muted-foreground" />{/if}
      <div class={look.text('Subheading', 'Default')}>{shown(inputs, empty.title)}</div>
      {#if empty.body}<div class={['max-w-sm', look.text('Body', 'Default')]}>{shown(inputs, empty.body)}</div>{/if}
      {#if empty.actions.length}
        <div class="mt-2 flex flex-wrap justify-center gap-3">
          {#each empty.actions as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
        </div>
      {/if}
    </div>
  {/if}
{/if}

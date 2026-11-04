<script lang="ts">
  import { shown } from '../../member-names.svelte';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let divider = $derived(typeof display.type !== 'string' && 'Divider' in display.type ? display.type.Divider : null);
</script>

{#if divider}
  {#if divider.label}
    <div class="flex items-center gap-3 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
      <div class="h-px flex-1 border-t border-dashed border-foreground/15"></div>
      {shown(inputs, divider.label)}
      <div class="h-px flex-1 border-t border-dashed border-foreground/15"></div>
    </div>
  {:else}
    <div class="h-px w-full bg-foreground/10"></div>
  {/if}
{/if}

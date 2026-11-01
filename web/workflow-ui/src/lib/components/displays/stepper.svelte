<script lang="ts">
  import { ChevronLeft, ChevronRight } from '@lucide/svelte';
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import { pressable } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs, workflow }: Props = $props();
  let stepper = $derived(typeof display.type !== 'string' && 'Stepper' in display.type ? display.type.Stepper : null);
  const press = pressable(() => ({ workflow, inputs }));
</script>

{#if stepper}
  <div class="flex items-center justify-between gap-3">
    <button type="button" class={[look.action.Icon, !stepper.previous && 'invisible']} aria-label="Previous" disabled={!stepper.previous || press.disabled(stepper.previous)} onclick={() => press.run(stepper.previous)}>
      <ChevronLeft class="size-4" />
    </button>
    <div class="flex min-w-0 flex-col items-center text-center">
      <span class="truncate text-sm font-semibold text-foreground">{interpolate(inputs, stepper.title)}</span>
      {#if stepper.subtitle}<span class="truncate text-xs text-muted-foreground">{interpolate(inputs, stepper.subtitle)}</span>{/if}
    </div>
    <button type="button" class={[look.action.Icon, !stepper.next && 'invisible', press.disabled(stepper.next) && 'opacity-30']} aria-label="Next" disabled={!stepper.next || press.disabled(stepper.next)} onclick={() => press.run(stepper.next)}>
      <ChevronRight class="size-4" />
    </button>
  </div>
{/if}

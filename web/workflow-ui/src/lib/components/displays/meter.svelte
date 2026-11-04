<script lang="ts">
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let meter = $derived(typeof display.type !== 'string' && 'Meter' in display.type ? display.type.Meter : null);
  let tone = $derived(look.toneOf(meter?.tone, inputs));
  let percent = $derived.by(() => {
    const n = Number(meter ? shown(inputs, meter.percent) : 0);
    return Number.isFinite(n) ? Math.min(100, Math.max(0, n)) : 0;
  });
</script>

{#if meter}
  <div class="flex min-w-0 flex-col gap-1.5">
    {#if meter.label || meter.value}
      <div class="flex items-baseline justify-between gap-3 text-xs">
        <span class="min-w-0 truncate text-muted-foreground">{meter.label ? shown(inputs, meter.label) : ''}</span>
        {#if meter.value}<span class="shrink-0 font-semibold whitespace-nowrap text-foreground tabular-nums">{shown(inputs, meter.value)}</span>{/if}
      </div>
    {/if}
    <div class="h-1.5 overflow-hidden rounded-full bg-foreground/10">
      <div class={['h-full rounded-full transition-[width] duration-500', look.toneSolid(tone)]} style:width="{percent}%"></div>
    </div>
  </div>
{/if}

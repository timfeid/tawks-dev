<script lang="ts">
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let stat = $derived(typeof display.type !== 'string' && 'Stat' in display.type ? display.type.Stat : null);
  let tone = $derived(look.toneOf(stat?.tone, inputs));
</script>

{#if stat}
  <div class="flex min-w-0 flex-1 flex-col items-center gap-0.5 rounded-xl border border-foreground/10 bg-foreground/[0.03] px-3 py-2.5 text-center">
    <div class={['text-xl font-bold tabular-nums', look.toneText(tone)]}>{shown(inputs, stat.value)}</div>
    <div class="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">{shown(inputs, stat.label)}</div>
  </div>
{/if}

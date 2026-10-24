<script lang="ts">
  import { getNestedValue, cn } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();

  let chart = $derived.by(() => {
    if (typeof display.type === 'string' || !('Chart' in display.type)) return;
    return display.type.Chart;
  });

  let data = $derived(getNestedValue(inputs, chart?.chart_data_key || ''));
</script>

{#if chart}
  <div class={cn(display.class, "flex items-center justify-center bg-muted/20 border border-dashed rounded-lg p-4")}>
    <div class="text-center">
        <div class="text-sm font-medium">{chart.type} Chart</div>
        <div class="text-xs text-muted-foreground">Data: {chart.chart_data_key} ({Array.isArray(data) ? data.length : 0} items)</div>
    </div>
  </div>
{/if}
<script lang="ts">
  import { onMount, type Component } from 'svelte';
  import { getNestedValue } from '../../utils.js';
  import * as look from '../../look';
  import { chartSummary, fitChart, hasValues, readChart, type ChartInput, type FittedChart } from '../../chart-data';
  import type { Props } from '../workflow-display.svelte';
  import Empty from './empty.svelte';

  const LOADING_GIVES_UP_MS = 8_000;
  const DEFAULT_WIDTH = 640;

  let { display, inputs, ...rest }: Props = $props();

  let chart = $derived(typeof display.type !== 'string' && 'Chart' in display.type ? display.type.Chart : null);
  let raw = $derived(chart ? getNestedValue(inputs, chart.chart_data_key) : undefined);
  let settled = $state(false);
  let waiting = $derived(raw === undefined && !settled);

  // The page's values are rebuilt on any update, so a chart only redraws when its own rows changed.
  let lastRows: string | undefined;
  let lastRead: ReturnType<typeof readChart> | undefined;
  let read = $derived.by(() => {
    if (!chart) return undefined;
    const input: ChartInput = { type: chart.type, x: chart.x, y: chart.y, axis: chart.axis, config: chart.config };
    const key = JSON.stringify([raw, input]);
    if (key !== lastRows || !lastRead) {
      lastRows = key;
      lastRead = readChart(raw, input);
    }
    return lastRead;
  });

  let width = $state(0);
  let drawnWidth = $derived(Math.round((width || DEFAULT_WIDTH) / 50) * 50);
  let fitted = $derived<FittedChart | undefined>(read && chart ? fitChart(read, chart.type, drawnWidth) : undefined);
  let hasData = $derived(Boolean(fitted && hasValues(fitted)));
  let xName = $derived(chart?.config?.x_label ?? '');
  let yName = $derived(chart?.config?.y_label ?? chart?.axis ?? '');
  let summary = $derived(fitted && hasData ? chartSummary(fitted, xName) : '');

  let mounted = $state(false);
  let Plot = $state<Component<{ chart: FittedChart; type: 'Bar' | 'Line' | 'Area'; width: number; xName: string }> | null>(null);
  let failed = $state(false);

  onMount(() => {
    mounted = true;
    const timer = setTimeout(() => (settled = true), LOADING_GIVES_UP_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (!mounted || !hasData || Plot || failed) return;
    import('./chart-plot.svelte').then(
      (module) => (Plot = module.default as typeof Plot),
      () => (failed = true),
    );
  });

  const noData = {
    type: { Empty: { icon: null, title: 'No data yet', body: null, loading: false, actions: [] } },
    mark: null,
  };
</script>

{#if chart}
  {#if !mounted || waiting}
    <div class="aspect-video w-full animate-pulse rounded-2xl bg-foreground/[0.04]" role="status" aria-label="Loading chart"></div>
  {:else if !hasData || !fitted}
    <Empty {...rest} {inputs} display={noData} />
  {:else}
    <figure class="m-0 flex min-w-0 flex-col gap-2" bind:clientWidth={width} data-chart-type={chart.type}>
      {#if yName}<div class={look.text('Fine', 'Default')}>{yName}</div>{/if}
      {#if failed}
        <div class={look.text('Body', 'Default')}>This chart could not load. Reload the page to try again.</div>
      {:else if Plot}
        <div role="img" aria-label={summary}>
          <Plot chart={fitted} type={chart.type} width={drawnWidth} {xName} />
        </div>
      {:else}
        <div class="aspect-video w-full animate-pulse rounded-2xl bg-foreground/[0.04]"></div>
      {/if}
      {#if xName}<div class={['text-center', look.text('Fine', 'Default')]}>{xName}</div>{/if}
      {#if fitted.series.length > 1}
        <ul class="m-0 flex list-none flex-wrap gap-x-4 gap-y-1 p-0">
          {#each fitted.series as series (series.key)}
            <li class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
              <span class="size-2.5 shrink-0 rounded-[2px]" style:background={series.color}></span>
              <span class="min-w-0 truncate">{series.label}</span>
            </li>
          {/each}
        </ul>
      {/if}
      <figcaption class="sr-only">{summary}</figcaption>
    </figure>
  {/if}
{/if}

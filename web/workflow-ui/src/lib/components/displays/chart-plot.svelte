<script lang="ts">
  import { AreaChart, BarChart, ChartContainer, ChartTooltip, LineChart, type ChartConfig } from '@tawks/ui/chart';
  import { formatX, formatY, formatYTick, shorten, xTickStep, type FittedChart, type Point } from '../../chart-data';

  const POINTS_UP_TO = 40;
  const WIDEST_BAR = 56;

  let { chart, type, width, xName }: { chart: FittedChart; type: 'Bar' | 'Line' | 'Area'; width: number; xName: string } = $props();

  let names = $derived<ChartConfig>(Object.fromEntries(chart.series.map((s) => [s.key, { label: s.label }])));
  let values = $derived(chart.points.flatMap((p) => chart.series.map((s) => p[s.key]).filter((v): v is number => v !== null)));
  let largest = $derived(values.reduce((most, v) => Math.max(most, Math.abs(v)), 0));
  let whole = $derived(values.every(Number.isInteger));

  let series = $derived(
    chart.series.map((s) => ({
      key: s.key,
      label: s.label,
      color: s.color,
      props: type === 'Bar' ? {} : { defined: (p: Point) => p[s.key] !== null },
    })),
  );

  let tickLabels = $derived(chart.points.map((p) => shorten(formatX(chart, p.x, 'tick'))));
  let step = $derived(xTickStep(tickLabels, width));

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function xTicks(scale: any) {
    if (type === 'Bar' || chart.xKind === 'text') {
      const domain = type === 'Bar' ? scale.domain() : chart.points.map((p) => p.x);
      return domain.filter((_: unknown, index: number) => index % step === 0);
    }
    const found = scale.ticks(Math.max(2, Math.ceil(chart.points.length / step)));
    return chart.xKind === 'number' && chart.points.every((p) => Number.isInteger(p.x)) ? found.filter(Number.isInteger) : found;
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function yTicks(scale: any) {
    const found = scale.ticks(5);
    return whole ? found.filter(Number.isInteger) : found;
  }

  let leftPad = $derived.by(() => {
    const widest = [largest, -largest, largest / 2].reduce((most, v) => Math.max(most, formatYTick(v, chart.yFormat, largest).length), 1);
    return Math.ceil(widest * 6.5) + 10;
  });
  // A line's first and last labels sit centred on the chart's edges, where they would be cut off.
  let edge = (label: string | undefined) => (type === 'Bar' || !label ? 0 : Math.ceil((label.length * 6.5) / 2));
  let padding = $derived({
    top: 8,
    right: Math.max(12, edge(tickLabels.at(-1))),
    bottom: 24,
    left: Math.max(leftPad, edge(tickLabels[0])),
  });

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let axisProps = $derived({
    xAxis: { ticks: xTicks, format: (value: any) => shorten(formatX(chart, value, 'tick')) },
    yAxis: { ticks: yTicks, format: (value: any) => formatYTick(value, chart.yFormat, largest) },
    grid: { x: false, y: true },
  });

  function tooltipLabel(_: unknown, payload: { payload?: Point }[]) {
    const point = payload[0]?.payload;
    if (!point) return '';
    const from = formatX(chart, point.x);
    if (point.end !== undefined) return `${from} to ${formatX(chart, point.end)}`;
    return chart.xKind === 'number' && xName ? `${xName} ${from}` : from;
  }

  let bandPadding = $derived.by(() => {
    const band = (width - padding.left - padding.right) / Math.max(1, chart.points.length);
    return Math.min(0.9, Math.max(0.25, 1 - (WIDEST_BAR * Math.max(1, chart.series.length)) / band));
  });

  let dots = $derived(chart.points.length <= POINTS_UP_TO ? { r: 3 } : false);
</script>

{#snippet tooltip()}
  <ChartTooltip labelFormatter={tooltipLabel} label="x">
    {#snippet formatter({ value, name, item })}
      <div class="size-2.5 shrink-0 rounded-[2px]" style:background={item.color}></div>
      <div class="flex flex-1 shrink-0 items-center justify-between gap-3 leading-none">
        <span class="text-muted-foreground">{name}</span>
        <span class="font-mono font-medium text-foreground tabular-nums">{formatY(value as number | null, chart.yFormat)}</span>
      </div>
    {/snippet}
  </ChartTooltip>
{/snippet}

<ChartContainer config={names}>
  {#if type === 'Bar'}
    <BarChart
      data={chart.points}
      x="x"
      {series}
      {padding}
      seriesLayout={series.length > 1 ? 'group' : 'overlap'}
      {bandPadding}
      legend={false}
      rule={false}
      props={{ ...axisProps, bars: { rounded: 'none', strokeWidth: 0 }, highlight: { area: { class: 'fill-foreground/5' } } }}
      {tooltip}
    />
  {:else if type === 'Area'}
    <AreaChart
      data={chart.points}
      x="x"
      {series}
      {padding}
      legend={false}
      rule={false}
      points={dots}
      props={{ ...axisProps, area: { fillOpacity: 0.2, line: { class: 'stroke-2' } } }}
      {tooltip}
    />
  {:else}
    <LineChart
      data={chart.points}
      x="x"
      {series}
      {padding}
      legend={false}
      rule={false}
      points={dots}
      props={{ ...axisProps, spline: { class: 'stroke-2' } }}
      {tooltip}
    />
  {/if}
</ChartContainer>

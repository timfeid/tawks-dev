import type { ChartConfig, ChartType } from '@tawks/workflow-types';

/** Reads rows into what a chart draws. Nothing here touches the page: every value from the data stays plain text or a number. */

export const MAX_SERIES = 8;
const MAX_ROWS = 20_000;
const MAX_LABEL_CHARS = 60;

export type ChartInput = {
  type: ChartType;
  x: string;
  y: string;
  axis?: string | null;
  config?: Partial<ChartConfig> | null;
};

type XKind = 'text' | 'number' | 'date' | 'datetime';
type XValue = number | Date;

export type Point = { x: XValue; end?: XValue; [series: `s${number}`]: number | null };

export type Series = { key: `s${number}`; label: string; color: string };

export type ReadChart = {
  points: Point[];
  series: Series[];
  xKind: XKind;
  /** The label of each x when x is text; points hold its index. */
  xLabels: string[];
  yFormat: NonNullable<ChartConfig['y_format']>;
  /** Values that were there and could not be read. */
  unreadable: number;
  /** Series left out for being the smallest. */
  seriesHidden: number;
  /** Rows past the most a chart reads. */
  rowsCut: number;
};

export type FittedChart = ReadChart & { allPoints: Point[]; averagedOver: number };

function field(row: unknown, path: string): unknown {
  let at = row;
  for (const part of path.split('.')) {
    if (at === null || typeof at !== 'object') return undefined;
    if (!Object.prototype.hasOwnProperty.call(at, part)) return undefined;
    at = (at as Record<string, unknown>)[part];
  }
  return at;
}

export function toNumber(value: unknown): number | null {
  if (typeof value === 'number') return Number.isFinite(value) ? value : null;
  if (typeof value !== 'string') return null;
  const text = value.replace(/,/g, '').trim();
  if (!/^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/.test(text)) return null;
  const parsed = Number(text);
  return Number.isFinite(parsed) ? parsed : null;
}

const DATE_ONLY = /^(\d{4})-(\d{2})-(\d{2})$/;
const INSTANT = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}/;

/** A bare date stays on that day wherever the viewer is, rather than moving with UTC. */
function toDate(value: unknown): Date | null {
  if (value instanceof Date) return Number.isNaN(+value) ? null : value;
  if (typeof value !== 'string') return null;
  const day = DATE_ONLY.exec(value);
  if (day) {
    const date = new Date(+day[1], +day[2] - 1, +day[3]);
    return date.getMonth() === +day[2] - 1 ? date : null;
  }
  if (!INSTANT.test(value)) return null;
  const date = new Date(value);
  return Number.isNaN(+date) ? null : date;
}

function toText(value: unknown): string | null {
  if (typeof value === 'string') return value.trim().slice(0, MAX_LABEL_CHARS) || null;
  if (typeof value === 'number' && Number.isFinite(value)) return String(value);
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  return null;
}

function xKindOf(values: unknown[], format: ChartConfig['x_format'] | undefined): XKind {
  if (format === 'Text') return 'text';
  if (format === 'Number') return 'number';
  if (format === 'Date') return 'date';
  if (format === 'DateTime') return 'datetime';
  const present = values.filter((v) => v !== null && v !== undefined);
  if (present.length && present.every((v) => toNumber(v) !== null)) return 'number';
  if (present.length && present.every((v) => toDate(v) !== null)) {
    return present.some((v) => typeof v === 'string' && INSTANT.test(v)) ? 'datetime' : 'date';
  }
  return 'text';
}

export function humanize(name: string): string {
  const last = name.split('.').pop() ?? name;
  const words = last.replace(/_/g, ' ').trim();
  return words ? words[0].toUpperCase() + words.slice(1) : name;
}

/** One series takes the theme's accent; several take the chart tokens in a fixed order. */
function colorOf(index: number, count: number): string {
  return count === 1 ? 'var(--color-primary)' : `var(--chart-${index + 1})`;
}

export function readChart(rows: unknown, input: ChartInput): ReadChart {
  const config = input.config ?? {};
  const list = Array.isArray(rows) ? rows.slice(0, MAX_ROWS) : [];
  const rowsCut = Array.isArray(rows) ? Math.max(0, rows.length - MAX_ROWS) : 0;
  const objects = list.filter((row) => row !== null && typeof row === 'object' && !Array.isArray(row));
  let unreadable = list.length - objects.length;

  const xKind = xKindOf(objects.map((row) => field(row, input.x)), config.x_format);
  const splitBy = config.split_by || null;
  const named = config.series ?? [];
  const yFields: { read: string; label: string }[] = splitBy
    ? []
    : named.length
      ? named.slice(0, MAX_SERIES).map((s) => ({ read: s.key, label: s.label || humanize(s.key) }))
      : [{ read: input.y, label: config.y_label || input.axis || humanize(input.y) }];

  type Bucket = { x: XValue; values: Map<string, number | null> };
  const buckets = new Map<string | number, Bucket>();
  const xLabels: string[] = [];
  const totals = new Map<string, number>();

  for (const row of objects) {
    const rawX = field(row, input.x);
    let x: XValue;
    let key: string | number;
    if (xKind === 'text') {
      const text = toText(rawX);
      if (text === null) {
        unreadable += 1;
        continue;
      }
      key = text;
      x = buckets.get(key)?.x ?? xLabels.length;
      if (!buckets.has(key)) xLabels.push(text);
    } else if (xKind === 'number') {
      const number = toNumber(rawX);
      if (number === null) {
        unreadable += 1;
        continue;
      }
      x = key = number;
    } else {
      const date = toDate(rawX);
      if (date === null) {
        unreadable += 1;
        continue;
      }
      x = date;
      key = +date;
    }
    let bucket = buckets.get(key);
    if (!bucket) buckets.set(key, (bucket = { x, values: new Map() }));

    const cells: [string, unknown][] = splitBy
      ? [[toText(field(row, splitBy)) ?? '', field(row, input.y)]]
      : yFields.map((f) => [f.read, field(row, f.read)]);
    for (const [name, raw] of cells) {
      if (name === '') {
        unreadable += 1;
        continue;
      }
      const value = toNumber(raw);
      if (value === null) {
        if (raw !== null && raw !== undefined) unreadable += 1;
        if (!bucket.values.has(name)) bucket.values.set(name, null);
        continue;
      }
      // Rows sharing an x and a series add up, so a view can hand over raw rows.
      bucket.values.set(name, (bucket.values.get(name) ?? 0) + value);
      totals.set(name, (totals.get(name) ?? 0) + Math.abs(value));
    }
  }

  const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });
  let seriesHidden = 0;
  let shown: { read: string; label: string }[] = yFields;
  if (splitBy) {
    const names = [...totals.keys()];
    const kept = [...names].sort((a, b) => totals.get(b)! - totals.get(a)!).slice(0, MAX_SERIES);
    seriesHidden = names.length - kept.length;
    // Sorted by name so a series keeps its colour as the numbers change.
    shown = kept.sort(collator.compare).map((name) => ({ read: name, label: name }));
  }

  const series: Series[] = shown.map((s, index) => ({
    key: `s${index}`,
    label: s.label,
    color: colorOf(index, shown.length),
  }));

  const points: Point[] = [...buckets.values()].map((bucket) => {
    const point: Point = { x: bucket.x };
    shown.forEach((s, index) => {
      point[`s${index}`] = bucket.values.get(s.read) ?? null;
    });
    return point;
  });
  if (xKind !== 'text') points.sort((a, b) => +a.x - +b.x);

  return {
    // A row with nothing readable stays: it is the gap a line has to show.
    points: series.length ? points : [],
    series,
    xKind,
    xLabels,
    yFormat: config.y_format ?? 'Number',
    unreadable,
    seriesHidden,
    rowsCut,
  };
}

/** Largest-triangle-three-buckets: picks the points that keep the line's shape. */
export function downsampleIndices(values: (number | null)[], target: number): number[] {
  const defined: number[] = [];
  values.forEach((value, index) => {
    if (value !== null) defined.push(index);
  });
  if (defined.length <= target || target < 3) return defined;
  const kept = [defined[0]];
  const size = (defined.length - 2) / (target - 2);
  let last = 0;
  for (let bucket = 0; bucket < target - 2; bucket += 1) {
    const from = Math.floor(bucket * size) + 1;
    const to = Math.min(Math.floor((bucket + 1) * size) + 1, defined.length - 1);
    const nextTo = Math.min(Math.floor((bucket + 2) * size) + 1, defined.length);
    let avgX = 0;
    let avgY = 0;
    const count = Math.max(1, nextTo - to);
    for (let i = to; i < nextTo; i += 1) {
      avgX += defined[i];
      avgY += values[defined[i]] as number;
    }
    avgX /= count;
    avgY /= count;
    let best = from;
    let bestArea = -1;
    const lastX = defined[last];
    const lastY = values[lastX] as number;
    for (let i = from; i < to; i += 1) {
      const px = defined[i];
      const area = Math.abs((lastX - avgX) * ((values[px] as number) - lastY) - (lastX - px) * (avgY - lastY));
      if (area > bestArea) {
        bestArea = area;
        best = i;
      }
    }
    kept.push(defined[best]);
    last = best;
  }
  kept.push(defined[defined.length - 1]);
  return kept;
}

function thinLine(read: ReadChart, width: number): Point[] {
  const budget = Math.max(30, Math.floor(Math.min(400, Math.max(60, width / 3)) / Math.max(1, read.series.length)));
  if (read.points.length <= budget) return read.points;
  const keep = new Set<number>();
  for (const s of read.series) {
    const values = read.points.map((point) => point[s.key]);
    downsampleIndices(values, budget).forEach((index) => keep.add(index));
    // A gap stays a gap: keep the first missing point after a run.
    values.forEach((value, index) => {
      if (value === null && index > 0 && values[index - 1] !== null) keep.add(index);
    });
  }
  return [...keep].sort((a, b) => a - b).map((index) => read.points[index]);
}

function thinBars(read: ReadChart, width: number): { points: Point[]; averagedOver: number } {
  const most = Math.min(200, Math.max(6, Math.floor(width / (8 * Math.max(1, read.series.length)))));
  if (read.points.length <= most) return { points: read.points, averagedOver: 1 };
  const size = Math.ceil(read.points.length / most);
  const points: Point[] = [];
  for (let start = 0; start < read.points.length; start += size) {
    const group = read.points.slice(start, start + size);
    const point: Point = { x: group[0].x, end: group[group.length - 1].x };
    for (const s of read.series) {
      const present = group.map((p) => p[s.key]).filter((v): v is number => v !== null);
      point[s.key] = present.length ? present.reduce((sum, v) => sum + v, 0) / present.length : null;
    }
    points.push(point);
  }
  return { points, averagedOver: size };
}

export function hasValues(chart: ReadChart): boolean {
  return chart.points.some((point) => chart.series.some((s) => point[s.key] !== null));
}

export function fitChart(read: ReadChart, type: ChartType, width: number): FittedChart {
  if (type === 'Bar') {
    const { points, averagedOver } = thinBars(read, width);
    return { ...read, points, allPoints: read.points, averagedOver };
  }
  return { ...read, points: thinLine(read, width), allPoints: read.points, averagedOver: 1 };
}

const plainNumber = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });
const compactNumber = new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 });

export function formatY(value: number | null | undefined, format: ReadChart['yFormat']): string {
  if (value === null || value === undefined) return 'No value';
  if (format === 'Percent') return `${plainNumber.format(value)}%`;
  if (format === 'Compact') return compactNumber.format(value);
  return plainNumber.format(value);
}

/** Tick labels may round big numbers that a tooltip still spells out. */
export function formatYTick(value: number, format: ReadChart['yFormat'], largest: number): string {
  return format === 'Number' && largest >= 100_000 ? compactNumber.format(value) : formatY(value, format);
}

function spanDays(points: Point[]): number {
  if (points.length < 2) return 0;
  return (+points[points.length - 1].x - +points[0].x) / 86_400_000;
}

export function formatX(chart: ReadChart, value: XValue, place: 'tick' | 'full' = 'full'): string {
  if (chart.xKind === 'text') return chart.xLabels[value as number] ?? '';
  if (chart.xKind === 'number') {
    return new Intl.NumberFormat(undefined, { maximumFractionDigits: 2, useGrouping: false }).format(+value);
  }
  const date = value instanceof Date ? value : new Date(value);
  if (place === 'full') {
    return date.toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      ...(chart.xKind === 'datetime' ? { hour: 'numeric', minute: '2-digit' } : {}),
    });
  }
  const days = spanDays(chart.points);
  if (days > 300) return date.toLocaleDateString(undefined, { month: 'short', year: '2-digit' });
  if (chart.xKind === 'datetime' && days < 2) return date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

/** How many x labels fit across `width`, and which of them to write. */
export function xTickStep(labels: string[], width: number): number {
  const longest = labels.reduce((most, label) => Math.max(most, label.length), 1);
  const fit = Math.max(2, Math.floor(width / (Math.min(longest, MAX_TICK_CHARS) * 6.5 + 16)));
  return Math.max(1, Math.ceil(labels.length / fit));
}

export const MAX_TICK_CHARS = 14;

export function shorten(label: string): string {
  return label.length > MAX_TICK_CHARS ? `${label.slice(0, MAX_TICK_CHARS - 1).trimEnd()}\u2026` : label;
}

function count(n: number, one: string, many: string = `${one}s`): string {
  return `${plainNumber.format(n)} ${n === 1 ? one : many}`;
}

/** The chart in words, for screen readers and for anyone who cannot make out the marks. */
export function chartSummary(chart: FittedChart, xName: string): string {
  const parts: string[] = [];
  const { allPoints: points, series } = chart;
  const first = points[0];
  const last = points[points.length - 1];
  if (!first) return '';
  const inOrder = chart.xKind !== 'text';

  if (series.length === 1) {
    const s = series[0];
    const drawn = points.filter((p) => p[s.key] !== null).length;
    const lead = drawn === 1 ? 'One point' : count(drawn, 'point');
    const range = drawn > 1 && inOrder ? `, ${xName ? `${xName.toLowerCase()} ` : ''}${formatX(chart, first.x)} to ${formatX(chart, last.x)}` : '';
    parts.push(`${s.label}: ${lead}${range}.`);
    const present = points.filter((p) => p[s.key] !== null);
    if (present.length > 1) {
      const high = present.reduce((a, b) => ((b[s.key] as number) > (a[s.key] as number) ? b : a));
      const low = present.reduce((a, b) => ((b[s.key] as number) < (a[s.key] as number) ? b : a));
      const latest = present[present.length - 1];
      if (inOrder) parts.push(`Latest ${formatY(latest[s.key], chart.yFormat)}.`);
      parts.push(
        `Highest ${formatY(high[s.key], chart.yFormat)} at ${formatX(chart, high.x)}, lowest ${formatY(low[s.key], chart.yFormat)} at ${formatX(chart, low.x)}.`,
      );
    } else if (present[0]) {
      parts.push(`${formatY(present[0][s.key], chart.yFormat)} at ${formatX(chart, present[0].x)}.`);
    }
  } else {
    const names = series.slice(0, 4).map((s) => s.label).join(', ');
    const more = series.length > 4 ? ` and ${series.length - 4} more` : '';
    parts.push(`${series.length} series (${names}${more}) over ${count(points.length, 'point')}.`);
    if (inOrder) {
      const latest = series
        .map((s) => {
          const at = [...points].reverse().find((p) => p[s.key] !== null);
          return at ? { s, value: at[s.key] as number } : null;
        })
        .filter((entry) => entry !== null)
        .sort((a, b) => b.value - a.value)
        .slice(0, 3)
        .map((entry) => `${entry.s.label} ${formatY(entry.value, chart.yFormat)}`);
      if (latest.length) parts.push(`Latest: ${latest.join(', ')}.`);
    }
  }
  if (chart.seriesHidden) parts.push(`Showing the largest ${series.length} of ${series.length + chart.seriesHidden}.`);
  if (chart.averagedOver > 1) parts.push(`Each bar averages about ${chart.averagedOver} points.`);
  else if (chart.points.length < points.length) parts.push(`Drawn with ${count(chart.points.length, 'point')} that keep its shape.`);
  if (chart.rowsCut) parts.push(`${count(chart.rowsCut, 'row')} past the limit left out.`);
  if (chart.unreadable) parts.push(`${count(chart.unreadable, 'value')} could not be read.`);
  return parts.join(' ');
}

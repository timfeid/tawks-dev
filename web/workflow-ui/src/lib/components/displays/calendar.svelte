<script lang="ts">
  import { mediaUrl } from '../../media.js';
  import { getNestedValue } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  type Sample = {
    outside?: boolean;
    today?: boolean;
    href?: string;
    image?: string;
    label?: string;
    detail?: string;
    tone?: string;
    highlight?: boolean | string;
  };
  type Day = Sample & { day: string | number };
  type Key = Sample & { name: string };

  const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

  let { display, inputs }: Props = $props();
  let calendar = $derived(typeof display.type !== 'string' && 'Calendar' in display.type ? display.type.Calendar : null);
  let accent = $derived(look.accentOf(calendar?.accent, inputs));
  const tint = (day: Sample) =>
    typeof day.highlight === 'string' && /^#[0-9a-f]{3,8}$/i.test(day.highlight)
      ? `color-mix(in srgb, ${day.highlight} 28%, transparent)`
      : undefined;
  function list<T>(key: string | undefined): T[] {
    const value = key ? getNestedValue(inputs, key) : null;
    return Array.isArray(value) ? value : [];
  }
  let days = $derived(list<Day>(calendar?.items_key));
  let legend = $derived(list<Key>(calendar?.legend_key));
  const toneClass = (tone?: string) =>
    accent && tone === 'Accent' ? 'text-(--accent-ink)' : look.toneText(look.toneOf(tone, inputs), 'Muted');
  const fill = (day: Sample) =>
    tint(day) ? '' : day.highlight ? 'bg-primary/15' : day.image ? 'bg-foreground/[0.06]' : 'bg-foreground/[0.02]';
  const edge = (day: Sample) =>
    day.today ? (accent ? 'border-(--accent-ink)/70' : 'border-primary/60') : 'border-foreground/5';
  const dot = () => (accent ? 'bg-(--accent-ink) text-(--accent-on) font-bold' : 'bg-primary text-primary-foreground font-bold');
</script>

{#snippet cell(day: Day)}
  <span class="flex items-center justify-center @md:justify-between">
    <span class={['flex size-6 items-center justify-center rounded-full text-xs tabular-nums', day.today ? dot() : 'text-muted-foreground']}
      >{day.day}</span
    >
    {#if day.label}
      <span class="hidden text-[10px] font-semibold tracking-wide text-muted-foreground uppercase @md:inline">{day.label}</span>
    {/if}
  </span>
  {#if day.image}
    <img src={mediaUrl(day.image)} alt="" class="mx-auto size-7 object-contain drop-shadow @md:size-10" loading="lazy" />
  {/if}
  {#if day.detail}
    <span class={['max-w-full truncate text-center text-[10px] font-semibold tabular-nums @md:text-xs', toneClass(day.tone)]}>{day.detail}</span>
  {/if}
{/snippet}

{#if calendar}
  <div class={['@container w-full min-w-0', accent && look.accentInk]} style={accent ? look.accentVars(accent) : undefined}>
    <div class="grid grid-cols-7 gap-1 @md:gap-1.5">
      {#each WEEKDAYS as weekday}
        <span class="pb-1 text-center text-[10px] font-semibold tracking-wider text-muted-foreground/70 uppercase">{weekday}</span>
      {/each}
      {#each days as day}
        {@const custom = tint(day)}
        {@const box = [
          'flex min-h-16 min-w-0 flex-col items-center gap-1 rounded-lg border p-1 @md:min-h-24 @md:items-stretch @md:p-1.5',
          edge(day),
          fill(day),
          day.outside && 'opacity-35',
        ]}
        {#if day.href}
          <a
            href={day.href}
            class={[
              box,
              'transition-colors',
              accent ? 'hover:border-(--accent-ink)/40 focus-visible:outline-2 focus-visible:outline-(--accent-ink)' : 'hover:border-foreground/20',
              !custom && 'hover:bg-foreground/10',
            ]}
            style:background-color={custom}>{@render cell(day)}</a
          >
        {:else}
          <div class={box} style:background-color={custom}>{@render cell(day)}</div>
        {/if}
      {/each}
    </div>
    {#if legend.length}
      <ul class="flex flex-wrap items-center gap-x-4 gap-y-2 pt-3">
        {#each legend as key}
          <li class="flex items-center gap-1.5 text-xs text-muted-foreground">
            {#if key.detail}
              <span class={['min-w-6 text-center text-xs font-semibold tabular-nums', toneClass(key.tone)]}>{key.detail}</span>
            {:else}
              <span
                class={['flex h-5 min-w-6 items-center justify-center rounded border px-1', edge(key), fill(key)]}
                style:background-color={tint(key)}
              >
                {#if key.today}
                  <span class={['size-2.5 rounded-full', dot()]}></span>
                {:else if key.label}
                  <span class="text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">{key.label}</span>
                {/if}
              </span>
            {/if}
            <span>{key.name}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<script lang="ts">
  import { mediaUrl } from '../../media.js';
  import { getNestedValue } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  type Day = {
    day: string | number;
    outside?: boolean;
    today?: boolean;
    href?: string;
    image?: string;
    label?: string;
    detail?: string;
    tone?: string;
    highlight?: boolean;
  };

  const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

  let { display, inputs }: Props = $props();
  let calendar = $derived(typeof display.type !== 'string' && 'Calendar' in display.type ? display.type.Calendar : null);
  let days = $derived.by(() => {
    const value = calendar ? getNestedValue(inputs, calendar.items_key) : null;
    return (Array.isArray(value) ? value : []) as Day[];
  });
</script>

{#snippet cell(day: Day)}
  <span class="flex items-center justify-center @md:justify-between">
    <span
      class={[
        'flex size-6 items-center justify-center rounded-full text-xs tabular-nums',
        day.today ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground',
      ]}>{day.day}</span
    >
    {#if day.label}
      <span class="hidden text-[10px] font-semibold tracking-wide text-muted-foreground uppercase @md:inline">{day.label}</span>
    {/if}
  </span>
  {#if day.image}
    <img src={mediaUrl(day.image)} alt="" class="mx-auto size-7 object-contain drop-shadow @md:size-10" loading="lazy" />
  {/if}
  {#if day.detail}
    <span class={['max-w-full truncate text-center text-[10px] font-semibold tabular-nums @md:text-xs', look.toneText(look.toneOf(day.tone, inputs), 'Muted')]}>{day.detail}</span>
  {/if}
{/snippet}

{#if calendar}
  <div class="@container w-full min-w-0">
    <div class="grid grid-cols-7 gap-1 @md:gap-1.5">
      {#each WEEKDAYS as weekday}
        <span class="pb-1 text-center text-[10px] font-semibold tracking-wider text-muted-foreground/70 uppercase">{weekday}</span>
      {/each}
      {#each days as day}
        {@const box = [
          'flex min-h-16 min-w-0 flex-col items-center gap-1 rounded-lg border p-1 @md:min-h-24 @md:items-stretch @md:p-1.5',
          day.today ? 'border-primary/60' : 'border-foreground/5',
          day.highlight ? 'bg-primary/15' : day.image ? 'bg-foreground/[0.06]' : 'bg-foreground/[0.02]',
          day.outside && 'opacity-35',
        ]}
        {#if day.href}
          <a href={day.href} class={[box, 'transition-colors hover:border-foreground/20 hover:bg-foreground/10']}>{@render cell(day)}</a>
        {:else}
          <div class={box}>{@render cell(day)}</div>
        {/if}
      {/each}
    </div>
  </div>
{/if}

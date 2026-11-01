<script lang="ts">
  import { interpolate, safeHref } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  let { display, inputs }: Props = $props();
  let feature = $derived(typeof display.type !== 'string' && 'Feature' in display.type ? display.type.Feature : null);
  let Icon = $derived(feature?.icon ? LUCIDE_REGISTRY[interpolate(inputs, feature.icon)] : undefined);
  let href = $derived(feature?.href ? safeHref(interpolate(inputs, feature.href)) : null);
  let external = $derived(!!href && /^https?:\/\//i.test(href));
</script>

{#snippet inner()}
  {#if Icon}
    <div class={look.iconTile}><Icon class="size-5" /></div>
  {/if}
  <div class="flex min-w-0 flex-1 flex-col gap-1">
    <div class="flex items-baseline justify-between gap-2">
      <div class={look.text('Subheading', 'Default')}>{interpolate(inputs, feature!.title)}</div>
      {#if href}<div class={['shrink-0', look.text('Fine', 'Primary')]}>Open</div>{/if}
    </div>
    <div class={look.text('Body', 'Default')}>{interpolate(inputs, feature!.body)}</div>
  </div>
{/snippet}

{#if feature}
  {#if href}
    <a
      {href}
      target={external ? '_blank' : undefined}
      rel={external ? 'noopener noreferrer' : undefined}
      class="flex gap-4 rounded-2xl border border-foreground/10 bg-foreground/[0.03] p-4 transition-colors hover:border-primary/40 hover:bg-foreground/[0.04]"
    >
      {@render inner()}
    </a>
  {:else}
    <div class="flex gap-4">{@render inner()}</div>
  {/if}
{/if}

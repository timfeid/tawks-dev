<script lang="ts">
  import { interpolate } from '../../utils.js';
  import { Badge } from '@tawks/ui/badge/index.js';
  import { LUCIDE_REGISTRY } from './icon/registry.js';
  import type { Props } from '../workflow-display.svelte';
  import * as look from '../../look';

  let { workflow, display, inputs }: Props = $props();

  let badge = $derived.by(() => {
    if (!('Badge' in display.type)) return;
    return display.type.Badge;
  });
</script>

{#if badge?.tone}
  {@const tone = look.toneOf(badge.tone, inputs)}
  {@const text = interpolate(inputs, badge.text)}
  {@const Icon = badge.icon ? LUCIDE_REGISTRY[interpolate(inputs, badge.icon)] : undefined}
  {#if text}
    <span class={['inline-flex shrink-0 items-center gap-1 rounded-md border px-1.5 py-0.5 text-[11px] leading-none font-semibold', look.toneSoft(tone), look.toneText(tone, 'Muted')]}
      >{#if Icon}<Icon class="size-3" />{/if}{text}</span
    >
  {/if}
{:else if badge}
  <Badge variant={badge.variant as 'default'}>{interpolate(inputs, badge.text)}</Badge>
{/if}

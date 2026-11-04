<script lang="ts">
  import { shown } from '../../member-names.svelte';
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
  {@const text = shown(inputs, badge.text)}
  {@const Icon = badge.icon ? LUCIDE_REGISTRY[shown(inputs, badge.icon)] : undefined}
  {#if text}
    <Badge variant={look.badgeVariant(look.toneOf(badge.tone, inputs))}
      >{#if Icon}<Icon class="mr-1 size-3" />{/if}{text}</Badge
    >
  {/if}
{:else if badge}
  <Badge variant={badge.variant as 'default'}>{shown(inputs, badge.text)}</Badge>
{/if}

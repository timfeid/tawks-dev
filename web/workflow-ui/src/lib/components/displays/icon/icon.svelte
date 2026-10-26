<script lang="ts">
  import { mediaUrl } from '../../../media.js';
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../../utils';
  import type { Props } from '../../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './registry.js';

  let { workflow, display, inputs }: Props = $props();

  let icon = $derived.by(() => {
    if (!('Icon' in display.type)) return;
    return display.type.Icon;
  });

  let iconName = $derived(icon ? interpolate(inputs, icon.name) : '');
  let Comp = $derived(icon?.lib === 'Lucide' ? LUCIDE_REGISTRY[iconName] : undefined);
</script>

{#if icon}
  {#if icon.lib === 'Lucide' && Comp}
    <Comp class="shrink-0 {display.class}" aria-label={icon.aria_label ?? undefined} />
  {:else if icon.lib === 'Emoji'}
    <span class={display.class} aria-label={icon.aria_label ?? iconName}>{iconName}</span>
  {:else if icon.lib === 'Url' && icon.url}
    <img class={display.class} src={mediaUrl(icon.url)} alt={icon.aria_label ?? iconName} />
  {:else}
    <span class="inline-block align-middle {display.class}" aria-hidden="true">□</span>
  {/if}
{/if}

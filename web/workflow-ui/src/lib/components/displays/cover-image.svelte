<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();
  let coverImageDisplay = $derived(
    typeof display.type === 'string' || !('CoverImage' in display.type)
      ? null
      : display.type.CoverImage,
  );

  let style = $derived.by(() => {
    if (!coverImageDisplay) {
      return '';
    }

    const styles: string[] = [];
    // Plain lengths only, so a size can't carry more CSS with it.
    const length = (value: string) =>
      /^\d{1,4}$/.test(value) ? `${value}px` : /^\d{1,4}(\.\d+)?(px|%|rem|em|vh|vw)$/.test(value) ? value : null;
    const width = coverImageDisplay.display_width ? length(coverImageDisplay.display_width.trim()) : null;
    const height = coverImageDisplay.display_height ? length(coverImageDisplay.display_height.trim()) : null;
    if (width) styles.push(`width: ${width};`);
    if (height) styles.push(`height: ${height};`);

    return styles.join(' ');
  });
</script>

{#if coverImageDisplay}
  <img
    {style}
    class="object-cover"
    src={getNestedValue(inputs, coverImageDisplay.src_key)}
    alt={getNestedValue(inputs, coverImageDisplay.alt_key)}
  />
{/if}

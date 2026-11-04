<script lang="ts">
  import * as look from '../../look';
  import { mediaUrl } from '../../media.js';
  import { interpolate } from '../../utils.js';

  let { template, inputs }: { template: string; inputs: Record<string, unknown> } = $props();

  let src = $derived.by(() => {
    const value = interpolate(inputs, template);
    return /^https?:\/\/[^\s{}]+$/i.test(value) ? value : '';
  });
  let failedSrc = $state('');
</script>

<span class={look.tableImageSlot} aria-hidden="true">
  {#if src && src !== failedSrc}
    <img
      class={look.tableImage}
      src={mediaUrl(src)}
      alt=""
      loading="lazy"
      decoding="async"
      referrerpolicy="no-referrer"
      onerror={() => (failedSrc = src)}
    />
  {/if}
</span>

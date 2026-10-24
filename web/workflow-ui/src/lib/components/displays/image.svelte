<script lang="ts">
  import { interpolate } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();

  // interpolate() returns the literal template (e.g. "{post.media_urls.0}")
  // when the path doesn't resolve. Treat unresolved templates as missing.
  function safeInterpolate(template: string | undefined | null): string {
    if (!template) return '';
    const result = interpolate(inputs, template);
    if (/\{[^{}]+\}/.test(result)) return '';
    return result;
  }

  let resolved = $derived.by(() => {
    if (typeof display.type === 'string' || !('Image' in display.type)) return null;
    return {
      src: safeInterpolate(display.type.Image.src),
      alt: safeInterpolate(display.type.Image.alt_text),
    };
  });
</script>

{#if resolved && resolved.src}
  <img class={display.class} src={resolved.src} alt={resolved.alt} />
{/if}

<script lang="ts">
  import { getNestedValue, safeHref } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  type Word = { text?: string; weight?: number; href?: string; detail?: string };

  let { display, inputs }: Props = $props();
  let cloud = $derived(
    typeof display.type !== 'string' && 'WordCloud' in display.type ? display.type.WordCloud : null,
  );
  let words = $derived.by(() => {
    const value = cloud ? getNestedValue(inputs, cloud.items_key) : null;
    return (Array.isArray(value) ? value : []).filter(
      (w): w is Word => typeof w?.text === 'string' && w.text.trim() !== '',
    );
  });

  function openExternal(event: MouseEvent) {
    const href = (event.currentTarget as HTMLAnchorElement | null)?.href;
    const opener = (window as unknown as { tawksOpenExternal?: (url: string) => void })
      .tawksOpenExternal;
    if (href && opener) {
      event.preventDefault();
      opener(href);
    }
  }
</script>

{#if words.length}
  <div class={look.wordCloud.box}>
    {#each words as word}
      {@const href = safeHref(word.href)}
      {@const step = look.wordStep(word.weight)}
      {#if href}
        {@const external = /^https?:\/\//i.test(href)}
        <a
          {href}
          class={[look.wordCloud.word, step]}
          title={word.detail || undefined}
          target={external ? '_blank' : undefined}
          rel={external ? 'noopener noreferrer' : undefined}
          onclick={external ? openExternal : undefined}>{word.text}</a
        >
      {:else}
        <span class={step} title={word.detail || undefined}>{word.text}</span>
      {/if}
    {/each}
  </div>
{/if}

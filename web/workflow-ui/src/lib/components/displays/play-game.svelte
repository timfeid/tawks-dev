<script lang="ts">
  import { interpolate, safeHref } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();

  let play = $derived(typeof display.type !== 'string' && 'PlayGame' in display.type ? display.type.PlayGame : null);
  let href = $derived(play ? safeHref(interpolate(inputs, play.fallback_url)) : null);

  function openExternal(event: MouseEvent) {
    const opener = (window as unknown as { tawksOpenExternal?: (url: string) => void }).tawksOpenExternal;
    if (href && opener) {
      event.preventDefault();
      opener(href);
    }
  }
</script>

<!-- Hosts that can start installed games draw this themselves; anywhere else it opens the game's site. -->
{#if play && href}
  <a {href} class={look.playGame} target="_blank" rel="noopener noreferrer" onclick={openExternal}>
    {interpolate(inputs, play.label)}
  </a>
{/if}

<script lang="ts">
  import { interpolate } from '../../utils';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let log = $derived(typeof display.type !== 'string' && 'Log' in display.type ? display.type.Log : null);
  let value = $derived(log ? interpolate(inputs, log.text) : '');
  let box = $state<HTMLPreElement>();

  // New output keeps the end in view, unless someone has scrolled up to read.
  $effect(() => {
    value;
    if (!box) return;
    const atEnd = box.scrollHeight - box.scrollTop - box.clientHeight < 48;
    if (atEnd) queueMicrotask(() => box && (box.scrollTop = box.scrollHeight));
  });
</script>

{#if log}
  <pre bind:this={box} class="max-h-80 overflow-auto rounded-xl border border-foreground/10 bg-black/40 p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-foreground/90">{value}</pre>
{/if}

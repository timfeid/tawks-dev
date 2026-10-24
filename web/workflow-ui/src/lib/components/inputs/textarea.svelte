<script lang="ts">
  import { Textarea } from '@tawks/ui/textarea/index.js';
  import type { Props } from '../workflow-input.svelte';

  let { input, readonly, changed, value, class: classes }: Props & { class?: string } = $props();

  let textArea = $derived.by(() => {
    if (typeof input.input.type === 'string' || !('TextArea' in input.input.type)) return;
    return input.input.type.TextArea;
  });

  let displayValue = $derived.by(() => (value === null || value === undefined ? '' : value));
</script>

{#if textArea}
  <Textarea
    class={classes}
    {readonly}
    rows={textArea.rows ?? undefined}
    placeholder={textArea.placeholder ?? undefined}
    required={input.input.required}
    value={displayValue}
    oninput={(e) => changed(e.currentTarget.value)}
  />
{/if}

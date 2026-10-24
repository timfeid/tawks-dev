<script lang="ts">
  import { Input } from '@tawks/ui/input/index.js';
  import type { Props } from '../workflow-input.svelte';

  let { input, readonly, changed, value, class: classes }: Props & { class?: string } = $props();

  let number = $derived.by(() => {
    if (typeof input.input.type === 'string' || !('Number' in input.input.type)) return;
    return input.input.type.Number;
  });

  let displayValue = $derived.by(() => (value === null || value === undefined ? '' : value));
</script>

{#if number}
  <Input
    class={classes}
    {readonly}
    type="number"
    min={number.min ?? undefined}
    max={number.max ?? undefined}
    required={input.input.required}
    value={displayValue}
    oninput={(e) => {
      const raw = (e.currentTarget as HTMLInputElement).value;
      if (raw === '') {
        changed(input.input.required ? 0 : null);
        return;
      }
      const parsed = Number(raw);
      changed(Number.isNaN(parsed) ? (input.input.required ? 0 : null) : parsed);
    }}
  />
{/if}

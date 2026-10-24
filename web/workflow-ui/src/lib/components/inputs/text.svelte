<script lang="ts">
  import { Input } from '@tawks/ui/input/index.js';
  import type { Props } from '../workflow-input.svelte';

  let { input, readonly, changed, value, class: classes }: Props & { class?: string } = $props();

  let text = $derived.by(() => {
    if (typeof input.input.type === 'string' || !('Text' in input.input.type)) return;
    return input.input.type.Text;
  });
  let inputRef = $state<HTMLElement | null>(null);
</script>

{#if text}
  <Input
    bind:ref={inputRef}
    class={classes}
    {readonly}
    type="text"
    placeholder={text.placeholder}
    required={input.input.required}
    {value}
    onkeydown={(e) => {
      if (e.key === ' ') {
        e.stopPropagation();
        return;
      }

      if (e.key === 'Down' || e.key === 'ArrowDown') {
        e.preventDefault();
        const currentEl = e.currentTarget as HTMLElement;

        const targetSelector = currentEl.dataset.focusDown;

        let target = document.querySelector(`[data-command-root]`);

        if (!target && targetSelector) {
          target = document.querySelector(targetSelector);
        }

        if (target instanceof HTMLElement) {
          // Mark where we came from so we can go back up later
          inputRef?.setAttribute('data-last-focused', '');

          target.focus();
          return;
        }
      }
    }}
    oninput={(e) => changed(e.currentTarget.value)}
  />
{/if}

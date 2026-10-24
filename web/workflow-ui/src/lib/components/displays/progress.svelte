<script lang="ts">
  import { Progress } from '@tawks/ui/progress';
  import { getNestedValue, interpolate, resolveInputValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let progress = $derived.by(() => {
    if (typeof display.type === 'string' || !('Progress' in display.type)) {
      return;
    }

    return display.type.Progress;
  });
  let percent = $derived(
    progress?.progress_percentage_key
      ? getNestedValue(inputs, progress.progress_percentage_key)
      : null,
  );
</script>

{#if progress && percent !== null}
  <Progress
    class={display.class}
    progressClass={interpolate(inputs, progress.percentage_class)}
    value={percent}
  />
{/if}

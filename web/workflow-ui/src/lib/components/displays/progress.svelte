<script lang="ts">
  import { Progress } from '@tawks/ui/progress';
  import { getNestedValue, interpolate, resolveInputValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import * as look from '../../look';

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
    progressClass={look.toneSolid(look.toneOf(progress.tone, inputs))}
    value={percent}
  />
{/if}

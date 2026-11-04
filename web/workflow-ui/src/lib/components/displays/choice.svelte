<script lang="ts">
  import * as look from '../../look';
  import { pressable } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let choice = $derived(typeof display.type !== 'string' && 'Choice' in display.type ? display.type.Choice : null);
  let state = $derived(look.stateOf(choice?.state, inputs));
  const press = pressable(() => ({ workflow, inputs }));
  let disabled = $derived(state === 'Disabled' || press.disabled(choice?.act));
</script>

{#if choice}
  <button
    type="button"
    class={[
      'flex min-w-0 rounded-xl border px-3 py-2.5 transition-colors',
      choice.direction === 'Row' ? 'flex-row flex-wrap items-center gap-x-3 gap-y-1 text-left' : 'flex-col items-stretch justify-center gap-1.5 text-center',
      look.choiceState[state],
      press.pending && 'cursor-progress opacity-60',
    ]}
    aria-pressed={state === 'Selected'}
    {disabled}
    onclick={() => press.run(choice.act)}
  >
    {#each choice.content as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
  </button>
{/if}

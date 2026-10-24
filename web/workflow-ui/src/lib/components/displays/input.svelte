<script lang="ts">
  import type { Props } from '../workflow-display.svelte';
  import WorkflowInput from '../workflow-input.svelte';

  let { workflow, display, inputs, ...rest }: Props = $props();
  console.log(rest.inputsChanged);

  let input = $derived.by(() => {
    if (typeof display.type === 'string' || !('Input' in display.type)) return;
    return display.type.Input;
  });
</script>

{#if input}
  <WorkflowInput class={display.class} {...rest} {workflow} {input} {inputs} />
{/if}

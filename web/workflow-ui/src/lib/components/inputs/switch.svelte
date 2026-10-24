<script lang="ts">
  import { evaluateCondition } from '../../utils.js';
  import { cn, getNestedValue } from '../../utils.js';
  import { Switch } from '@tawks/ui/switch/index.js';
  import type { Props } from '../workflow-input.svelte';
  import { onChange } from '../workflow.js';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../workflow';

  let { workflow, input, inputs }: Props = $props();

  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  function setChecked(value: boolean) {
    onChange(provider, workflow.instance_id, inputs, { [input.id]: value }, input.input.on_change_action);
  }

  function getChecked() {
    let val = getNestedValue(inputs, input.id) as unknown as boolean | undefined;

    return val || false;
  }

  const condition = $derived.by(() => {
    return input.input.readonly === null || evaluateCondition(input.input.readonly, inputs);
  });

  const classes = $derived.by(() => {
    const classes = cn(condition ? 'pointer-events-none opacity-50' : '');

    return classes;
  });
</script>

<Switch class={classes} checked={getChecked()} onCheckedChange={setChecked}></Switch>

<script lang="ts">
  import { deepEqual, getNestedValue, interpolate, objectInterpolate } from '../../utils';
  import type { Props } from '../workflow-input.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import Button from '@tawks/ui/button/button.svelte';
  import { onChange } from '../workflow';
  import { evaluateCondition } from '../../utils.js';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../workflow';

  let { workflow, input, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let selectFromList = $derived(
    typeof input.input.type !== 'string' && 'MultiSelectFromList' in input.input.type
      ? (input.input.type as any).MultiSelectFromList
      : undefined,
  );
  let items = $derived.by(() => {
    if (selectFromList) {
      const key = selectFromList.items_key;
      const response = getNestedValue(inputs, key);
      if (!Array.isArray(response)) {
        console.warn('Found non-array for the list items');
        return [];
      }
      return response;
    }
    return [];
  });

  function toggleValue(val: any) {
    let updatedVals = Array.isArray(selectedVals) ? [...selectedVals] : [];
    const idx = updatedVals.findIndex((v) => deepEqual(v, val));
    if (idx === -1) {
      updatedVals.push(val);
    } else {
      updatedVals.splice(idx, 1);
    }
    selectedVals = updatedVals;
    onChange(
      provider,
      workflow.instance_id,
      inputs,
      { [input.id]: updatedVals },
      input.input.on_change_action as any,
    );
  }

  let currentVal = $derived(getNestedValue(inputs, input.id));
  let selectedVals = $state(Array.isArray(currentVal) ? [...currentVal] : []);
  $effect(() => {
    selectedVals = currentVal;
  });
</script>

{#if selectFromList}
  {#each items as item}
    {@const newInputs = { ...inputs, [selectFromList.as]: item }}
    {@const itemVal = objectInterpolate(newInputs, selectFromList.value)}
    {@const readonly = !input.input.readonly
      ? false
      : evaluateCondition(input.input.readonly, newInputs)}
    <Button
      variant={selectedVals && selectedVals.some((v) => deepEqual(v, itemVal))
        ? 'outline'
        : 'ghost'}
      onclick={() => toggleValue(itemVal)}
      class="h-auto {selectFromList.class} {readonly ? 'pointer-events-none' : ''}"
    >
      {#each selectFromList.content as displayItem}
        <WorkflowDisplay from="wf-multi-select-from-list" inputs={newInputs} {workflow} display={displayItem} />
      {/each}
    </Button>
  {/each}
{/if}
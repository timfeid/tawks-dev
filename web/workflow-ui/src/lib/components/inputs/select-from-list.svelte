<script lang="ts">
  import { deepEqual, getNestedValue, interpolate, objectInterpolate } from '../../utils';
  import type { Props } from '../workflow-input.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import * as look from '../../look';

  let { workflow, input, inputs, changed }: Props = $props();

  let selectFromList = $derived(
    typeof input.input.type !== 'string' && 'SelectFromList' in input.input.type
      ? input.input.type.SelectFromList
      : undefined,
  );

  let items = $derived.by(() => {
    if (selectFromList) {
      const key = selectFromList.items_key;
      const response = getNestedValue(inputs, key);
      return Array.isArray(response) ? response : [];
    }
    return [];
  });

  let currentVal = $derived(getNestedValue(inputs, input.id));
  let selectedVal = $state();

  function onclick(val: any) {
    selectedVal = val;
    changed(val);
  }
</script>

{#if selectFromList}
  <div class={selectFromList.columns ? ['grid gap-2', look.columns(selectFromList.columns, true)] : 'flex flex-col gap-2'}>
    {#each items as item}
      {@const newInputs = { ...inputs, [selectFromList.as]: item }}
      {@const itemVal = objectInterpolate(newInputs, selectFromList.value)}

      {@const selected = deepEqual(selectedVal, itemVal) || deepEqual(itemVal, currentVal)}

      <button
        type="button"
        onclick={() => onclick(itemVal)}
        aria-pressed={selected}
        class={['flex min-w-0 items-center gap-3 rounded-xl border px-3 py-2.5 text-left transition-colors', look.choiceState[selected ? 'Selected' : 'Default']]}
      >
        {#each selectFromList.content as displayItem}
          <WorkflowDisplay from="wf-select-from-list" inputs={newInputs} {workflow} display={displayItem} />
        {/each}
      </button>
    {/each}
  </div>
{/if}

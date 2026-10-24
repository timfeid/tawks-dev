<script lang="ts">
  import { onChange } from '../workflow';
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import * as Select from '@tawks/ui/select/index.js';

  import type { Props } from '../workflow-input.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../workflow';

  let { workflow, input, inputs, changed, value }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let select = $derived.by(() => {
    if (typeof input.input.type === 'string' || !('Select' in input.input.type)) return;
    return (input.input.type as any).Select;
  });

  let items = $derived.by(() => {
    if (typeof input.input.type === 'string' || !('Select' in input.input.type)) return [];
    const select = (input.input.type as any).Select;
    if (select) {
      const key = select.items_key;
      const response = getNestedValue(inputs, key);
      if (!Array.isArray(response)) {
        console.warn('Found non-array for the list items');
        return [];
      }
      return response;
    }
    return [];
  });

  let selectedValue = $state(value ?? '');
  let selectItems = $derived.by(() => {
    if (!select) {
      return [];
    }
    return items.map((i) => {
      const newInputs = { ...inputs, [select.as]: i };
      const value = objectInterpolate(newInputs, select.value);
      const show = !select.filter || evaluateCondition(select.filter, newInputs);

      return { value, show, inputs: newInputs };
    });
  });
  $effect(() => {
    selectedValue = value ?? '';
  });

  let selected = $derived(
    selectedValue ? selectItems.find((i) => String(i.value) === String(selectedValue)) : undefined,
  );
</script>

{#if select}
  <Select.Root
    bind:value={selectedValue}
    onValueChange={(value) => {
      changed?.(value);
      if (!input.input.on_change_action) {
        return;
      }
      const updates = { [input.id]: value };
      onChange(provider, workflow.instance_id, inputs, updates, input.input.on_change_action as any);
    }}
    type="single"
    disabled={input.input.readonly === null
      ? false
      : evaluateCondition(input.input.readonly, inputs)}
  >
    <Select.Trigger class="h-full w-full {select.class}">
      {#if selected}
        {#each select.selected_content || select.content as display}
          <WorkflowDisplay from="wf-select-selected" {workflow} {display} inputs={selected.inputs} />
        {/each}
      {:else if select.unselected_content}
        {#each select.unselected_content as display}
          <WorkflowDisplay from="wf-select-unselected" {workflow} {display} {inputs} />
        {/each}
      {/if}
    </Select.Trigger>
    <Select.Content>
      {#each selectItems as item}
        {#if item.show}
          <Select.Item value={String(item.value)}>
            {#each select.content as display}
              <WorkflowDisplay from="wf-select-item" {workflow} {display} inputs={item.inputs} />
            {/each}
          </Select.Item>
        {/if}
      {/each}
    </Select.Content>
  </Select.Root>
{/if}
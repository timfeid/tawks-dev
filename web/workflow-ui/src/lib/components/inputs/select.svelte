<script lang="ts" module>
  let lastId = 0;
</script>

<script lang="ts">
  import { onChange } from '../workflow';
  import { getNestedValue, interpolate, evaluateCondition, objectInterpolate } from '../../utils.js';
  import * as Select from '@tawks/ui/select/index.js';
  import { Label } from '@tawks/ui/label/index.js';
  import * as look from '../../look';

  import type { Props } from '../workflow-input.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, input, inputs, changed, value }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let select = $derived.by(() => {
    if (typeof input.input.type === 'string' || !('Select' in input.input.type)) return;
    return input.input.type.Select;
  });

  let fixed = $derived(select?.options ?? []);
  let labelled = $derived(fixed.length > 0 || !!select?.option_label);

  let items = $derived.by(() => {
    if (!select || fixed.length) return [];
    const response = getNestedValue(inputs, select.items_key ?? '');
    if (!Array.isArray(response)) {
      console.warn('Found non-array for the list items');
      return [];
    }
    return response;
  });

  let selectedValue = $state(value ?? '');
  let selectItems = $derived.by(() => {
    if (!select) return [];
    if (fixed.length) {
      return fixed.map((option) => ({ value: option.value, label: option.label, show: true, inputs }));
    }
    const current = select;
    const name = current.as ?? '';
    return items.map((i) => {
      const newInputs = { ...inputs, [name]: i };
      const value = objectInterpolate(newInputs, current.value ?? '');
      const show = !current.filter || evaluateCondition(current.filter, newInputs);
      const label = current.option_label ? interpolate(newInputs, current.option_label) : String(value);
      return { value, label, show, inputs: newInputs };
    });
  });
  $effect(() => {
    selectedValue = value === null || value === undefined ? '' : String(value);
  });

  let selected = $derived(
    selectedValue ? selectItems.find((i) => (i.show || !labelled) && String(i.value) === String(selectedValue)) : undefined,
  );
  const triggerId = `workflow-select-${++lastId}`;

  function picked(next: string) {
    changed?.(next);
    if (!input.input.on_change_action) return;
    onChange(provider, workflow.instance_id, inputs, { [input.id]: next }, input.input.on_change_action as any);
  }

  let disabled = $derived(input.input.readonly === null ? false : evaluateCondition(input.input.readonly, inputs));
</script>

{#if select && labelled}
  <div class={look.field}>
    {#if select.label}<Label for={triggerId}>{interpolate(inputs, select.label)}</Label>{/if}
    <!-- The form reads values from the DOM, and a default nobody touched has to reach it too. -->
    <input type="hidden" data-input-clears value={selected ? String(selected.value) : ''} oninput={() => changed?.('')} />
    <Select.Root type="single" bind:value={selectedValue} onValueChange={picked} {disabled}>
      <Select.Trigger chevron role="combobox" id={triggerId} aria-required={input.input.required} aria-label={select.label ? undefined : select.placeholder ?? input.id}>
        {selected ? selected.label : (select.placeholder ?? '')}
      </Select.Trigger>
      <Select.Content class={look.dropdownList}>
        {#each selectItems as item}
          {#if item.show}
            <Select.Item value={String(item.value)} label={item.label} />
          {/if}
        {/each}
      </Select.Content>
    </Select.Root>
  </div>
{:else if select}
  <Select.Root bind:value={selectedValue} onValueChange={picked} type="single" {disabled}>
    <Select.Trigger class="h-full w-full">
      {#if selected}
        {#each select.selected_content || select.content || [] as display}
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
            {#each select.content ?? [] as display}
              <WorkflowDisplay from="wf-select-item" {workflow} {display} inputs={item.inputs} />
            {/each}
          </Select.Item>
        {/if}
      {/each}
    </Select.Content>
  </Select.Root>
{/if}

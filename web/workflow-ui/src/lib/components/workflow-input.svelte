<script lang="ts">
  import { getContext } from 'svelte';
  import type { JsonValue, WorkflowInput, WorkflowResource } from '@tawks/workflow-types';
  import Checkbox from './inputs/checkbox.svelte';
  import Switch from './inputs/switch.svelte';
  import Text from './inputs/text.svelte';
  import NumberInput from './inputs/number.svelte';
  import TextAreaInput from './inputs/textarea.svelte';
  import SelectFromList from './inputs/select-from-list.svelte';
  import MultiSelectFromList from './inputs/multi-select-from-list.svelte';
  import ServerActionLoader from './inputs/server-action-loader.svelte';
  import Select from './inputs/select.svelte';
  import Wysiwyg from './inputs/wysiwyg.svelte';
  import { getNestedValue, setNestedValue, evaluateCondition } from '../utils';

  type IncomingProps = {
    workflow: WorkflowResource;
    input: { id: string; input: WorkflowInput };
    inputs: Record<string, JsonValue>;
    inputsChanged?: (key: string, value: any) => void;
    class: string;
  };
  export type Props = IncomingProps & {
    readonly: boolean;
    changed: (value: any) => void;
    value: any;
  };

  let originalProps: IncomingProps = $props();

  function resolveValue(
    inputs: Record<string, JsonValue>,
    id: string,
  ): JsonValue | '' {
    const direct = inputs[id];
    if (typeof direct !== 'undefined') {
      return direct;
    }
    const nested = getNestedValue(inputs, id) as JsonValue | undefined;
    return typeof nested === 'undefined' ? '' : nested;
  }

  // 1. The "Shield" state: This is the source of truth for the UI
  let localValue = $state(resolveValue(originalProps.inputs, originalProps.input.id));

  const formState = getContext<{ version: number } | undefined>('form-state');
  let seenVersion = formState?.version ?? 0;

  // 2. The Sync Guard: Only update localValue from props if the user ISN'T typing
  $effect(() => {
    const serverValue = resolveValue(originalProps.inputs, originalProps.input.id);
    const version = formState?.version ?? 0;
    if (version !== seenVersion) {
      seenVersion = version;
      localValue = serverValue;
      return;
    }

    // Check if THIS specific input container has focus
    const isFocused = document.activeElement?.closest(
      `[data-input-id="${originalProps.input.id}"]`,
    );

    // Only overwrite if the user isn't currently interacting with this field
    if (!isFocused && serverValue !== localValue) {
      localValue = serverValue;
    }
  });

  // 3. Stable derived props to pass down to children
  let finalProps = $derived.by(() => {
    const currentInputs = {
      ...originalProps.inputs,
      $user_id: originalProps.workflow.user_id,
      [originalProps.input.id]: localValue,
    };

    return {
      ...originalProps,
      readonly: readonly(originalProps.input.input, currentInputs, originalProps.workflow.waiting),
      value: localValue,

      changed: (newValue: any) => {
        localValue = newValue; // Instant UI feedback

        // Bubble up to Form.svelte or other parent for debounced submission
        if (originalProps.inputsChanged) {
          originalProps.inputsChanged(originalProps.input.id, newValue);
        }
      },
    };
  });

  function readonly(input: WorkflowInput, inputs: Record<string, JsonValue>, waiting: boolean) {
    if (waiting) return true;
    if (!input.readonly) return false;
    return evaluateCondition(input.readonly, inputs);
  }
</script>

<div data-input-id={originalProps.input.id} class="contents">
  {#if 'Switch' === finalProps.input.input.type}
    <Switch {...finalProps}></Switch>
  {:else if 'Wysiwyg' === finalProps.input.input.type}
    <Wysiwyg {...finalProps}></Wysiwyg>
  {:else if 'Select' in finalProps.input.input.type}
    <Select {...finalProps}></Select>
  {:else if 'ServerActionLoader' in finalProps.input.input.type}
    <ServerActionLoader {...finalProps}></ServerActionLoader>
  {:else if 'MultiSelectFromList' in finalProps.input.input.type}
    <MultiSelectFromList {...finalProps}></MultiSelectFromList>
  {:else if 'SelectFromList' in finalProps.input.input.type}
    <SelectFromList {...finalProps}></SelectFromList>
  {:else if 'Number' in finalProps.input.input.type}
    <NumberInput {...finalProps}></NumberInput>
  {:else if 'TextArea' in finalProps.input.input.type}
    <TextAreaInput {...finalProps}></TextAreaInput>
  {:else if 'Text' in finalProps.input.input.type}
    <Text {...finalProps}></Text>
  {:else if 'Checkbox' in finalProps.input.input.type}
    <Checkbox {...finalProps}></Checkbox>
  {:else}
    <div class="text-xs text-red-500">
      Unsupported input type: {JSON.stringify(finalProps.input.input.type)}
    </div>
  {/if}
</div>
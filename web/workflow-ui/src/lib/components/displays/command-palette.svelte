<script lang="ts">
  import type { JsonValue } from '@tawks/workflow-types';
  import * as Command from '@tawks/ui/command/index.js';
  import { getNestedValue, interpolate } from '../../utils';
  import { submitServerAction } from '../workflow';
  import type { Props } from '../workflow-display.svelte';
  import type { WorkflowStateProvider } from '../../workflow';
  import Icon from './icon/icon.svelte';
  import { getContext } from 'svelte';

  let { workflow, display, inputs, buttonClicked }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  // 1. Create a reference for the root element
  let rootRef = $state<HTMLElement | null>(null);

  let commandPalette = $derived.by(() => {
    if (typeof display.type === 'string') return;
    if ('CommandPalette' in display.type) return display.type.CommandPalette;
  });

  let commands = $derived.by(() => {
    if (!commandPalette) return null;
    return getNestedValue(inputs, commandPalette.key);
  });

  function handleSelect(item: any) {
    console.log('selected item', item);
    buttonClicked?.();

    // 2. Dispatch a bubbling event to tell any parent Popover to close
    rootRef?.dispatchEvent(new CustomEvent('close-popover', { bubbles: true }));

    if (!item.action_id) return;

    const requestInputs: Record<string, JsonValue> = {};
    if (item.action_input_map) {
      for (const [key, val] of Object.entries(item.action_input_map)) {
        requestInputs[key] = interpolate(item.data, val || '');
      }
    }
    submitServerAction(provider, workflow.instance_id, requestInputs, item.action_id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowUp') {
      const root = e.currentTarget as HTMLElement;
      const selected = root.querySelector('[data-command-item][data-selected]');
      const visibleItems = Array.from(
        root.querySelectorAll('[data-command-item]'),
      ).filter((item) => item.getAttribute('data-hidden') !== 'true');

      console.log('selected', selected, visibleItems[0]);
      if (selected && visibleItems[0] === selected) {
        e.preventDefault();
        e.stopPropagation();

        const lastFocused = document.querySelector('[data-last-focused]');

        if (lastFocused instanceof HTMLElement) {
          setTimeout(() => {
            if (document.activeElement instanceof HTMLElement) {
              document.activeElement.blur();
            }
            lastFocused.focus();
          }, 10);
        }
      }
    }
  }
</script>

{#if commands}
  <Command.Root
    bind:ref={rootRef}
    class={display.class}
    onkeydown={handleKeydown}
  >
    {#if commands.allow_search}
      <Command.Input
        placeholder={commands.placeholder ?? 'Type a command...'}
      />
    {/if}
    <Command.List>
      <Command.Empty>No results found.</Command.Empty>

      {#each commands.groups as group}
        <Command.Group heading={group.heading}>
          {#each group.items as item}
            <Command.Item
              value={item.label}
              onSelect={() => handleSelect(item)}
              class="flex cursor-pointer items-center gap-2"
            >
              {#if item.icon}
                <div class="mr-2 h-4 w-4">
                  <Icon
                    {workflow}
                    display={{
                      class: 'h-4 w-4',
                      type: {
                        Icon: { ...item.icon },
                      },
                    }}
                    {inputs}
                  ></Icon>
                </div>
              {/if}

              <span class="truncate">{item.label}</span>

              {#if item.keywords && item.keywords.length > 0}
                <span class="hidden">{item.keywords.join(' ')}</span>
              {/if}

              {#if item.shortcut}
                <Command.Shortcut>{item.shortcut}</Command.Shortcut>
              {/if}
            </Command.Item>
          {/each}
        </Command.Group>
        <Command.Separator />
      {/each}
    </Command.List>
  </Command.Root>
{/if}

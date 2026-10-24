<script lang="ts">
  import * as Popover from '@tawks/ui/popover/index.js';
  import * as Command from '@tawks/ui/command/index.js';
  import { Input } from '@tawks/ui/input/index.js';
  import { getNestedValue, interpolate, cn, debounce } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { submitServerAction } from '../workflow.js';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let query = $state('');
  let results = $state<any[]>([]);
  let loading = $state(false);
  let open = $state(false);

  let searchable = $derived.by(() => {
    if (typeof display.type === 'string' || !('Searchable' in display.type)) return;
    return display.type.Searchable;
  });

  const search = debounce(async (q: string) => {
    if (!searchable || !q) {
      results = [];
      return;
    }
    loading = true;
    try {
      // This is a bit of a hack, but we need to get the results from somewhere
      // In the real app, this would be a server call
      if (provider.processAction) {
           await provider.processAction(workflow.instance_id, searchable.server_action_id, { query: q });
      }
    } finally {
      loading = false;
    }
  }, 300);

  $effect(() => {
    search(query);
  });

  function select(item: any) {
    if (!searchable) return;
    submitServerAction(provider, workflow.instance_id, { [searchable.as]: item }, searchable.on_select_action_id);
    open = false;
  }
</script>

{#if searchable}
  <Popover.Root bind:open>
    <Popover.Trigger class={display.class}>
        <div class="flex items-center gap-2 px-3 py-2 border rounded-md bg-background">
            <span class="text-muted-foreground italic text-sm">Search...</span>
        </div>
    </Popover.Trigger>
    <Popover.Content class="w-[400px] p-0" align="start">
      <Command.Root>
        <div class="flex items-center border-b px-3">
          <div class="mr-2 h-4 w-4 shrink-0 opacity-50">🔍</div>
          <Input
            placeholder={searchable.placeholder}
            bind:value={query}
            class="flex h-11 w-full rounded-md bg-transparent py-3 text-sm outline-none border-none focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-50"
          />
          {#if loading}
            <div class="ml-2 h-4 w-4 animate-spin text-muted-foreground">⏳</div>
          {/if}
        </div>
        <Command.List>
          <Command.Empty>No results found.</Command.Empty>
          <Command.Group>
            {#each results as item}
              <Command.Item
                onSelect={() => select(item)}
                class="flex items-center gap-2 p-2 cursor-pointer hover:bg-accent rounded-sm"
              >
                {#each searchable.content as content}
                  <WorkflowDisplay from="searchable-item" {workflow} display={content} inputs={{ ...inputs, [searchable.as]: item }} />
                {/each}
              </Command.Item>
            {/each}
          </Command.Group>
        </Command.List>
      </Command.Root>
    </Popover.Content>
  </Popover.Root>
{/if}
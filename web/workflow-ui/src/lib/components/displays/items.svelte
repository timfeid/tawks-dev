<script lang="ts">
  import RowScope from './row-scope.svelte';
  import { getContext } from 'svelte';
  import { holdsInputs, rowKeys } from '../../utils';
  import { getNestedValue } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let items = $derived(typeof display.type !== 'string' && 'Items' in display.type ? display.type.Items : null);
  let list = $derived.by(() => {
    const value = items ? getNestedValue(inputs, items.items_key) : null;
    return Array.isArray(value) ? value : [];
  });
  let layout = $derived.by(() => {
    const l = items?.layout;
    if (!l || typeof l === 'string') return 'flex flex-col divide-y divide-foreground/10';
    if ('Stack' in l) return `flex flex-col ${look.gap[l.Stack.gap]}`;
    if ('Row' in l) return `flex flex-wrap items-center ${look.gap[l.Row.gap]}`;
    if ('Scroll' in l) return `flex flex-col overflow-y-auto pr-1 ${look.gap[l.Scroll.gap]}`;
    return `grid ${look.columns(l.Grid.columns, l.Grid.fixed ?? false)} ${look.gap[l.Grid.gap]}`;
  });
  // Roughly a row's height each, so the list shows about that many before it scrolls.
  let maxHeight = $derived.by(() => {
    const l = items?.layout;
    return l && typeof l === 'object' && 'Scroll' in l ? `${Math.max(l.Scroll.rows, 1) * 3.75}rem` : undefined;
  });
  const inForm = !!getContext('form-state');
  let keys = $derived(rowKeys(list, inForm || holdsInputs(items?.content)));
  let grid = $derived(!!items?.layout && typeof items.layout === 'object' && 'Grid' in items.layout);
</script>

{#if items && list.length}
  <div class={grid ? '@container w-full min-w-0' : 'contents'}>
    <div class={['min-w-0', layout]} style:max-height={maxHeight}>
      {#each list as item, index (keys[index])}
        <div class="contents" data-workflow-row>
          <RowScope>
            {#each items.content as child}
              <WorkflowDisplay {...rest} {workflow} inputs={{ ...inputs, [items.as]: item }} display={child} />
            {/each}
          </RowScope>
        </div>
      {/each}
    </div>
  </div>
{/if}

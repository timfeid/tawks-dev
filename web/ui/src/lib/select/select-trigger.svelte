<script lang="ts">
  import { Select as SelectPrimitive, type WithoutChild } from 'bits-ui';
  import { ChevronDown } from '@lucide/svelte';
  import { cn } from '../utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    chevron = false,
    ...restProps
  }: WithoutChild<SelectPrimitive.TriggerProps> & { chevron?: boolean } = $props();
</script>

<SelectPrimitive.Trigger
  bind:ref
  class={cn(
    'border-input bg-background ring-offset-background data-[placeholder]:text-muted-foreground focus:ring-ring flex h-10 w-full items-center justify-between rounded-md border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
  {...restProps}
>
  <!-- One line however long the choice is; the full text is in the list. -->
  <span class="min-w-0 flex-1 truncate text-left">{@render children?.()}</span>
  {#if chevron}<ChevronDown class="ml-2 size-4 shrink-0 opacity-50" />{/if}
</SelectPrimitive.Trigger>

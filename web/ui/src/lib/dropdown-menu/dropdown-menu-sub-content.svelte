<script lang="ts">
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';
  import { MediaQuery } from 'svelte/reactivity';
  import { cn } from '../utils.js';

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  }: DropdownMenuPrimitive.SubContentProps = $props();

  const narrow = new MediaQuery('(max-width: 639px)');
</script>

<DropdownMenuPrimitive.SubContent
  bind:ref
  side={narrow.current ? 'top' : 'right'}
  class={cn(
    'bg-popover text-popover-foreground z-50 min-w-[8rem] rounded-md border p-1 shadow-lg focus:outline-none',
    className,
  )}
  {...restProps}
/>

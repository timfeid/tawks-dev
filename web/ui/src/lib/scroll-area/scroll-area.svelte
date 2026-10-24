<script lang="ts">
  import { ScrollArea as ScrollAreaPrimitive, type WithoutChild } from 'bits-ui';
  import { Scrollbar } from './index.js';
  import { cn } from '../utils.js';

  let {
    ref = $bindable(null),
    class: className,
    orientation = 'vertical',
    scrollbarXClasses = '',
    scrollbarYClasses = '',
    children,
    onscroll,
    ...restProps
  }: WithoutChild<ScrollAreaPrimitive.RootProps> & {
    orientation?: 'vertical' | 'horizontal' | 'both' | undefined;
    scrollbarXClasses?: string | undefined;
    scrollbarYClasses?: string | undefined;
  } = $props();
</script>

<ScrollAreaPrimitive.Root bind:ref {...restProps} class={cn('relative overflow-hidden', className)}>
  <ScrollAreaPrimitive.Viewport {onscroll} class="h-full w-full rounded-[inherit]">
    {@render children?.()}
  </ScrollAreaPrimitive.Viewport>
  {#if orientation === 'vertical' || orientation === 'both'}
    <Scrollbar orientation="vertical" class={scrollbarYClasses} />
  {/if}
  {#if orientation === 'horizontal' || orientation === 'both'}
    <Scrollbar orientation="horizontal" class={scrollbarXClasses} />
  {/if}
  <ScrollAreaPrimitive.Corner />
</ScrollAreaPrimitive.Root>

<style lang="postcss">
  :global(.scrollarea:h-full) {
    :global(& [data-scroll-area-content]) {
      height: 100%;
    }
  }
  :global(.flex-scrollarea) {
    :global(& [data-scroll-area-content]) {
      display: flex;
      flex-direction: column;
      min-height: 100%;
    }
  }
</style>

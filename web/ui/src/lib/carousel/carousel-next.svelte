<script lang="ts">
  import { ArrowRight } from '@lucide/svelte';
  import type { WithoutChildren } from 'bits-ui';
  import { getEmblaContext } from './context.js';
  import { cn } from '../utils.js';
  import { Button, type Props } from '../button/index.js';

  let {
    ref = $bindable(null),
    class: className,
    variant = 'outline',
    size = 'icon',
    ...restProps
  }: WithoutChildren<Props> = $props();

  const emblaCtx = getEmblaContext('<Carousel.Next/>');
</script>

<Button
  {variant}
  {size}
  class={cn(
    'to-background via-background absolute h-full touch-manipulation rounded-none border-0 bg-transparent bg-gradient-to-r from-transparent via-50% hover:bg-transparent disabled:hidden',
    emblaCtx.orientation === 'horizontal'
      ? 'right-0 top-1/2 -translate-y-1/2'
      : '-bottom-12 left-1/2 -translate-x-1/2 rotate-90',
    className,
  )}
  disabled={!emblaCtx.canScrollNext}
  onclick={emblaCtx.scrollNext}
  onkeydown={emblaCtx.handleKeyDown}
  bind:ref
  {...restProps}
>
  <ArrowRight class="size-4" />
  <span class="sr-only">Next slide</span>
</Button>

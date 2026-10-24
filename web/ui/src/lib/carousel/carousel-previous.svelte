<script lang="ts">
  import { ArrowLeft } from '@lucide/svelte';
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

  const emblaCtx = getEmblaContext('<Carousel.Previous/>');
</script>

<Button
  {variant}
  {size}
  class={cn(
    'from-background to-transprent via-background absolute h-full touch-manipulation rounded-none border-0 bg-transparent bg-gradient-to-r via-50% hover:bg-transparent disabled:hidden',
    emblaCtx.orientation === 'horizontal'
      ? 'bottom-0 left-0 top-0 '
      : '-top-12 left-1/2 -translate-x-1/2 rotate-90',
    className,
  )}
  disabled={!emblaCtx.canScrollPrev}
  onclick={emblaCtx.scrollPrev}
  onkeydown={emblaCtx.handleKeyDown}
  {...restProps}
  bind:ref
>
  <ArrowLeft class="size-4" />
  <span class="sr-only">Previous slide</span>
</Button>

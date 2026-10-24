<script lang="ts">
  import { getNestedValue, resolveInputValue } from '../../utils.js';
  import * as Carousel from '@tawks/ui/carousel/index.js';
  import type { CarouselAPI } from '@tawks/ui/carousel/context.js';
  import type { JsonValue } from '@tawks/workflow-types';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { submitServerAction } from '../workflow';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let carouselDisplay = $derived(
    typeof display.type === 'string' || !('Carousel' in display.type)
      ? null
      : (display.type.Carousel as {
          items_key: string;
          as: string;
          content: any[];
          on_select_action?: string | null;
          input_map?: Record<string, JsonValue> | null;
          active_key?: string | null;
        }),
  );

  let cards = $derived.by(() => {
    if (carouselDisplay) {
      const key = carouselDisplay.items_key;
      const response = getNestedValue(inputs, key);
      if (!Array.isArray(response)) {
        console.warn('Found non-array for the carousel items');
        return [];
      }

      return response;
    }

    return [];
  });

  let activeIndex = $derived.by(() => {
    const key = carouselDisplay?.active_key;
    if (!key) return 0;
    const idx = cards.findIndex((card) => Boolean(getNestedValue(card, key)));
    return idx < 0 ? 0 : idx;
  });

  let api = $state<CarouselAPI | undefined>();
  let lastSubmitted = $state<number | null>(null);

  function selectFor(index: number) {
    if (!carouselDisplay?.on_select_action) return;
    const card = cards[index];
    if (card === undefined) return;
    const scoped = { ...inputs, [carouselDisplay.as]: card };
    const requestInputs: Record<string, JsonValue> = {};
    for (const [key, val] of Object.entries(carouselDisplay.input_map ?? {})) {
      requestInputs[key] = resolveInputValue(scoped, val);
    }
    submitServerAction(provider, workflow.instance_id, requestInputs, carouselDisplay.on_select_action);
  }

  function onSelect(embla: CarouselAPI) {
    const index = embla.selectedScrollSnap();
    const current = lastSubmitted ?? activeIndex;
    if (index === current) return;
    lastSubmitted = index;
    selectFor(index);
  }

  // Fire once the snap animation has finished so the page does not re-render
  // under a finger that is still dragging.
  $effect(() => {
    if (!api) return;
    api.on('settle', onSelect);
    return () => {
      api?.off('settle', onSelect);
    };
  });

  // Follow the server's selected slide. A swipe we submitted is confirmed
  // when the server lands on that index, which clears the in-flight marker;
  // any other selection change (a chip tap, a tab change) scrolls the
  // carousel to match. While a swipe is still in flight, stay put.
  $effect(() => {
    const target = activeIndex;
    if (!api) return;
    if (lastSubmitted !== null) {
      if (lastSubmitted === target) {
        lastSubmitted = null;
      }
      return;
    }
    if (api.selectedScrollSnap() !== target) {
      api.scrollTo(target, true);
    }
  });
</script>

{#if carouselDisplay}
  <Carousel.Root
    opts={{ startIndex: activeIndex, align: 'center', containScroll: 'trimSnaps' }}
    setApi={(embla) => (api = embla)}
  >
    <Carousel.Content>
      {#each cards as card}
        <Carousel.Item class={display.class}>
          {#each carouselDisplay.content as item}
            <WorkflowDisplay
              from="wf-carousel"
              inputs={{ ...inputs, [carouselDisplay.as]: card }}
              {workflow}
              display={item}
            />
          {/each}
        </Carousel.Item>
      {/each}
    </Carousel.Content>
  </Carousel.Root>
{/if}

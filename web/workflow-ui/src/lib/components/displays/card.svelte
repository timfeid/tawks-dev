<script lang="ts">
  import { mediaUrl } from '../../media.js';
  import * as Card from '@tawks/ui/card/index.js';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getNestedValue } from '../../utils.js';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');
  let cardDisplay = $derived(
    typeof display.type === 'string' || !('Card' in display.type)
      ? null
      : display.type.Card,
  );

  async function actionClicked() {
    if (!action) {
      console.error('no action??');
      return;
    }
    console.log('action clicked', action);
    const value = getNestedValue(inputs, action.from_input_key);
    console.log('from', value);

    if (provider.processAction) {
        await provider.processAction(
            workflow.instance_id,
            action.action,
            { [action.update_responses_key]: value }
        );
    }
  }

  let action = $derived.by(() => {
    return cardDisplay &&
      typeof cardDisplay.layout === 'object' &&
      'ActionImage' in cardDisplay.layout
      ? (cardDisplay.layout as any).ActionImage
      : undefined;
  });

  let card = $derived.by(() => {
    if (!cardDisplay) {
      return;
    }

    const layout = cardDisplay.layout;

    return {
      image:
        typeof layout === 'object'
          ? getNestedValue(
              inputs,
              'Image' in layout
                ? layout.Image.image_key
                : (layout as any)?.ActionImage?.image_key,
            )
          : undefined,
      description: '', //getNestedValue(inputs, display.Card.description_key),
      children: cardDisplay.content,
    };
  });
</script>

{#snippet renderCard(renderCard: NonNullable<typeof card>)}
  <Card.Root class="overflow-hidden rounded-lg">
    {#if renderCard.image}
      <div class="relative aspect-[16/9] w-full overflow-hidden">
        <img
          src={mediaUrl(renderCard.image) || '/placeholder.svg'}
          alt="Card image"
          class="object-cover transition-transform"
          sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
        />
      </div>
    {/if}
    <Card.Header>
      {#if renderCard.description}
        <Card.Description class="line-clamp-3">{renderCard.description}</Card.Description>
      {/if}
    </Card.Header>
    <Card.Content class="flex flex-col">
      {#each renderCard.children as child}
        <WorkflowDisplay from="card" display={child} {inputs} {workflow} />
      {/each}
    </Card.Content>
    <Card.Footer class="flex items-center justify-between p-2 pt-0">
    </Card.Footer>
  </Card.Root>
{/snippet}

{#if card}
  {#if action}
    <button class="block w-full" onclick={() => actionClicked()}>{@render renderCard(card)}</button>
  {:else}
    {@render renderCard(card)}
  {/if}
{/if}

<style lang="postcss">
  :global(.card-content) {
    & :global(.wf-display-text) {
      @apply line-clamp-3;
    }
  }
</style>

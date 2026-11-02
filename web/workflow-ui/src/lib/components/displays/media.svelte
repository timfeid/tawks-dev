<script lang="ts">
  import { ImageOff } from '@lucide/svelte';
  import { mediaUrl } from '../../media.js';
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let media = $derived(typeof display.type !== 'string' && 'Media' in display.type ? display.type.Media : null);
  let src = $derived(media ? interpolate(inputs, media.src) : '');
  let usable = $derived(!!src && !/\{[^{}]+\}/.test(src));
  let failed = $state(false);
  $effect(() => {
    src;
    failed = false;
  });
  let below = $derived(!!media?.caption.length && media.caption_placement === 'Below');
  // A fixed size is a square box; anything filling its width keeps its aspect.
  let box = $derived.by(() => {
    if (!media) return '';
    if (media.size === 'Fill') return `w-full ${look.aspect[media.aspect]}`;
    return media.aspect === 'Square' || media.aspect === 'Auto' ? look.mediaSize[media.size] : `${look.width[media.size]} ${look.aspect[media.aspect]}`;
  });
  let tint = $derived(media?.tint ? look.tint(look.toneOf(media.tint, inputs)) : '');
  // Small pictures keep their blank box, where an icon would only be clutter.
  let placeholder = $derived(!!media && (media.size === 'Fill' || media.size === 'Small' || media.size === 'Medium' || media.size === 'Large' || media.size === 'ExtraLarge'));
  let shape = $derived(media?.shape === 'Round' ? 'rounded-full' : media?.shape === 'Square' ? 'rounded-none' : media?.size === 'ExtraSmall' || media?.size === 'Small' ? 'rounded-md' : 'rounded-xl');
</script>

{#snippet picture()}
  {#if media}
    {#if usable && !failed}
      <img src={mediaUrl(src)} alt={interpolate(inputs, media.alt)} loading="lazy" class={['size-full', media.fit === 'Contain' ? 'object-contain' : 'object-cover']} onerror={() => (failed = true)} />
    {:else if placeholder}
      <div class="flex size-full items-center justify-center text-muted-foreground/70" aria-hidden="true"><ImageOff class="size-6" /></div>
    {/if}
    {#if tint}
      <div class={['pointer-events-none absolute inset-0', tint]}></div>
    {/if}
    {#if media.badge.length}
      <!-- The backing keeps a badge readable on any picture. -->
      <div class="absolute top-2 right-2 flex gap-1 rounded-full bg-background/85">
        {#each media.badge as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </div>
    {/if}
  {/if}
{/snippet}

{#if media && below}
  <div class={['@container flex shrink-0 flex-col overflow-hidden border border-foreground/10 bg-foreground/[0.03]', look.width[media.size], shape]}>
    <div class={['relative w-full overflow-hidden', look.aspect[media.aspect] || 'aspect-square']}>
      {@render picture()}
      <div class="pointer-events-none absolute inset-x-0 bottom-0 h-3 bg-gradient-to-t from-foreground/[0.06] to-transparent"></div>
    </div>
    <div class="flex flex-1 flex-col gap-0.5 bg-foreground/[0.06] px-2 pt-1 pb-2 @[10rem]:gap-1.5 @[10rem]:p-3">
      {#each media.caption as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
    </div>
  </div>
{:else if media}
  <div class={['relative shrink-0 overflow-hidden', box, shape, media.fit === 'Cover' && 'bg-foreground/5']}>
    {@render picture()}
    {#if media.caption.length}
      <div class="absolute inset-x-0 bottom-0 flex flex-col gap-0.5 bg-gradient-to-t from-black/85 via-black/50 to-transparent p-3 pt-8">
        {#each media.caption as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </div>
    {/if}
  </div>
{/if}

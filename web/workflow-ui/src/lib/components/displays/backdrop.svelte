<script lang="ts">
  import { mediaUrl } from '../../media.js';
  import { interpolate } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let backdrop = $derived(typeof display.type !== 'string' && 'Backdrop' in display.type ? display.type.Backdrop : null);
  let color = $derived.by(() => {
    const value = backdrop ? interpolate(inputs, backdrop.color) : '';
    return /^#[0-9a-f]{3,8}$/i.test(value) ? value : '#27272a';
  });
  let image = $derived(backdrop ? interpolate(inputs, backdrop.image) : '');
  let usable = $derived(!!image && !/\{[^{}]+\}/.test(image));
</script>

{#if backdrop}
  <div class="relative isolate overflow-hidden rounded-2xl border border-white/10" style:background-color={color}>
    {#if usable}
      <img
        src={mediaUrl(image)}
        alt=""
        aria-hidden="true"
        class="pointer-events-none absolute top-1/2 -right-[8%] -z-10 h-[190%] max-w-none -translate-y-1/2 -rotate-12 opacity-40 select-none"
      />
    {/if}
    <div class="absolute inset-0 -z-10 bg-gradient-to-r from-black/85 via-black/65 to-black/35"></div>
    <div class="flex min-w-0 flex-col gap-3 p-5 text-white">
      {#each backdrop.content as child}
        <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
      {/each}
    </div>
  </div>
{/if}

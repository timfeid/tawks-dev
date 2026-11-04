<script lang="ts" module>
  import * as look from '../../look';

  let layer: HTMLElement | undefined;

  /** Page overlays sit in a layer under everything the app itself opens over a page. */
  function overlayLayer(): HTMLElement | undefined {
    if (typeof document === 'undefined') return undefined;
    if (!layer || !layer.isConnected) {
      layer = document.createElement('div');
      layer.className = look.dialogLayer;
      document.body.append(layer);
    }
    return layer;
  }
</script>

<script lang="ts">
  import { setContext, tick, untrack } from 'svelte';
  import { X } from '@lucide/svelte';
  import * as Dialog from '@tawks/ui/dialog/index.js';
  import * as Sheet from '@tawks/ui/sheet/index.js';
  import * as Drawer from '@tawks/ui/drawer/index.js';
  import { IsMobile } from '@tawks/ui/hooks/is-mobile.svelte.js';
  import { interpolate } from '../../utils';
  import { LUCIDE_REGISTRY } from './icon/registry.js';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow }: Props = $props();
  let dialog = $derived(typeof display.type !== 'string' && 'Dialog' in display.type ? display.type.Dialog : null);
  let label = $derived(dialog ? interpolate(inputs, dialog.label) : '');
  let title = $derived(dialog ? interpolate(inputs, dialog.title) : '');
  let description = $derived(dialog?.description ? interpolate(inputs, dialog.description) : '');
  let Icon = $derived(dialog?.icon ? LUCIDE_REGISTRY[interpolate(inputs, dialog.icon)] : undefined);

  const phone = new IsMobile();
  let open = $state(false);
  let opener = $state<HTMLButtonElement | null>(null);

  const portalProps = { to: overlayLayer() };
  let surface = $state<HTMLElement | null>(null);

  // Into the first field, or the surface itself: keyboard users stay in it without a button lighting up.
  function focusInside(event: Event) {
    event.preventDefault();
    const field = surface?.querySelector<HTMLElement>(
      'input:not([type=hidden]):not([type=checkbox]):not([type=radio]):not([disabled]):not([readonly]), textarea:not([disabled]):not([readonly])',
    );
    if (field) field.focus();
    else surface?.focus();
  }

  setContext('dialog-close', () => {
    if (dialog?.close_on_submit) open = false;
  });

  // The three surfaces share one plain button, so none of them knows to go back to it.
  let wasOpen = false;
  $effect(() => {
    const now = open;
    untrack(() => {
      if (wasOpen && !now) tick().then(() => opener?.focus());
      wasOpen = now;
    });
  });
</script>

{#snippet body(extra = '')}
  <div class={[look.dialogBody, extra]}>
    {#each dialog?.content ?? [] as child}
      <WorkflowDisplay from="wf-dialog" {workflow} display={child} {inputs} />
    {/each}
  </div>
{/snippet}

{#if dialog}
  <button bind:this={opener} type="button" class={[look.action[dialog.look ?? 'Primary'], 'max-w-full shrink-0']} data-workflow-dialog aria-haspopup="dialog" aria-expanded={open} onclick={() => (open = true)}>
    {#if Icon}<Icon class="size-4 shrink-0" />{/if}
    <span>{label}</span>
  </button>

  {#if phone.current}
    <Drawer.Root bind:open>
      <Drawer.Content {portalProps}>
        <div class={look.dialogSheet}>
          <Drawer.Header class="px-0 text-left">
            <Drawer.Title>{title}</Drawer.Title>
            {#if description}<Drawer.Description>{description}</Drawer.Description>{/if}
          </Drawer.Header>
          {@render body()}
        </div>
        <Drawer.Close class={look.dialogSheetClose}>
          <X class="size-5" />
          <span class="sr-only">Close</span>
        </Drawer.Close>
      </Drawer.Content>
    </Drawer.Root>
  {:else if dialog.style === 'Drawer'}
    <Sheet.Root bind:open>
      <Sheet.Content {portalProps} bind:ref={surface} class={look.dialogDrawer} tabindex={-1} onOpenAutoFocus={focusInside}>
        <Sheet.Header>
          <Sheet.Title>{title}</Sheet.Title>
          {#if description}<Sheet.Description>{description}</Sheet.Description>{/if}
        </Sheet.Header>
        {@render body()}
      </Sheet.Content>
    </Sheet.Root>
  {:else}
    <Dialog.Root bind:open>
      <Dialog.Content {portalProps} bind:ref={surface} class={look.dialogModal} closeButton={false} tabindex={-1} onOpenAutoFocus={focusInside}>
        <Dialog.Header>
          <Dialog.Title>{title}</Dialog.Title>
          {#if description}<Dialog.Description>{description}</Dialog.Description>{/if}
        </Dialog.Header>
        {@render body(look.dialogModalBody)}
        <Dialog.Close class={look.dialogSheetClose}>
          <X class="size-5" />
          <span class="sr-only">Close</span>
        </Dialog.Close>
      </Dialog.Content>
    </Dialog.Root>
  {/if}
{/if}

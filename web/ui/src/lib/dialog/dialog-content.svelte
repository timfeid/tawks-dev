<script lang="ts">
  import { Dialog as DialogPrimitive, type WithoutChildrenOrChild } from 'bits-ui';
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import * as Dialog from './index.js';
  import { cn } from '../utils.js';

  let {
    ref = $bindable(null),
    class: className,
    portalProps,
    closeButton = true,
    children,
    ...restProps
  }: WithoutChildrenOrChild<DialogPrimitive.ContentProps> & {
    portalProps?: DialogPrimitive.PortalProps;
    /** A close X on phones, where the dialog fills the screen and leaves no backdrop to tap. Off for dialogs with their own. */
    closeButton?: boolean;
    children: Snippet;
  } = $props();
  const classes = [
    'fixed z-50 flex flex-col gap-4 bg-background shadow-lg duration-200 focus:outline-0',
    'data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0',
    'left-0 top-[env(safe-area-inset-top)] w-full max-h-3/4 overflow-y-auto rounded-lg',
    'max-md:!max-h-none max-md:!max-w-none max-md:!w-full max-md:!rounded-none',
    'md:left-[50%] md:top-[50%] md:!h-auto md:max-w-3xl md:translate-x-[-50%] md:translate-y-[-50%] md:overflow-hidden',
    'md:data-[state=closed]:zoom-out-95 md:data-[state=open]:zoom-in-95 md:data-[state=closed]:slide-out-to-left-1/2 md:data-[state=closed]:slide-out-to-top-[48%] md:data-[state=open]:slide-in-from-left-1/2 md:data-[state=open]:slide-in-from-top-[48%]',
  ].join(' ');

  /** Opening lands in the first text field, if there is one; otherwise nothing is focused, so no button lights up. */
  function focusTextField(event: Event) {
    event.preventDefault();
    ref
      ?.querySelector<HTMLElement>(
        'input:not([type=hidden]):not([type=checkbox]):not([type=radio]):not([disabled]):not([readonly]), textarea:not([disabled]):not([readonly]), [contenteditable=true]',
      )
      ?.focus();
  }
</script>

<Dialog.Portal {...portalProps}>
  <Dialog.Overlay />
  <DialogPrimitive.Content
    bind:ref
    class={cn(classes, className)}
    {...restProps}
    onOpenAutoFocus={restProps.onOpenAutoFocus ?? focusTextField}
    style="height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom))"
  >
    {@render children?.()}
    {#if closeButton}
      <DialogPrimitive.Close
        class="text-muted-foreground hover:text-foreground focus-visible:ring-ring absolute top-2 right-2 z-10 flex size-11 items-center justify-center rounded-md transition-colors focus-visible:ring-2 focus-visible:outline-none md:hidden"
      >
        <X class="size-5" />
        <span class="sr-only">Close</span>
      </DialogPrimitive.Close>
    {/if}
  </DialogPrimitive.Content>
</Dialog.Portal>

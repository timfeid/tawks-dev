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
    children,
    ...restProps
  }: WithoutChildrenOrChild<DialogPrimitive.ContentProps> & {
    portalProps?: DialogPrimitive.PortalProps;
    children: Snippet;
  } = $props();
  const classes =
    ' overflow-y-auto md:overflow-y-hidden bg-background max-h-3/4 md:!h-auto w-screen data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] fixed top-[env(safe-area-inset-top)] md:left-[50%] md:top-[50%] z-50 flex flex-col  w-full md:max-w-3xl md:translate-x-[-50%] md:translate-y-[-50%] gap-4 md:overflow-hidden   shadow-lg duration-200 focus:outline-0 sm:rounded-lg';
</script>

<Dialog.Portal {...portalProps}>
  <Dialog.Overlay />
  <DialogPrimitive.Content
    bind:ref
    class={cn(classes, className)}
    {...restProps}
    style="height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom))"
  >
    {@render children?.()}
    <!-- <DialogPrimitive.Close
      class="ring-offset-background focus:ring-ring absolute right-2 top-2 flex size-8 items-center justify-center rounded-sm opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:pointer-events-none"
    >
      <X class="size-4" />
      <span class="sr-only">Close</span>
    </DialogPrimitive.Close> -->
  </DialogPrimitive.Content>
</Dialog.Portal>

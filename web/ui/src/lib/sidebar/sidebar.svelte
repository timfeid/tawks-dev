<script lang="ts">
  import * as Sheet from '../sheet/index.js';
  import { cn } from '../utils.js';
  import type { WithElementRef } from 'bits-ui';
  import type { HTMLAttributes } from 'svelte/elements';
  import { SIDEBAR_WIDTH_MOBILE } from './constants.js';
  import { useSidebar } from './context.svelte.js';
  import { onDestroy, onMount } from 'svelte';

  let {
    ref = $bindable(null),
    side = 'left',
    variant = 'sidebar',
    collapsible = 'offcanvas',
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
    side?: 'left' | 'right';
    variant?: 'sidebar' | 'floating' | 'inset';
    collapsible?: 'offcanvas' | 'icon' | 'none';
  } = $props();

  // Tune these if needed
  const TAP_TIME_MS = 350; // max press duration
  const TAP_MOVE_PX = 8; // max move distance
  const SUPPRESS_MS = 220; // window to ignore the synthetic click

  let sidebarEl: HTMLElement; // bind this to the sidebar root element

  let downX = 0,
    downY = 0,
    downAt = 0;
  let lastSynthAt = 0,
    lastSynthX = 0,
    lastSynthY = 0;

  function isTap(upX: number, upY: number, upAt: number) {
    const dt = upAt - downAt;
    const dx = upX - downX,
      dy = upY - downY;
    return dt <= TAP_TIME_MS && dx * dx + dy * dy <= TAP_MOVE_PX * TAP_MOVE_PX;
  }

  function activate(target: Element) {
    const el = target.closest('a,button,[role="button"],[data-tap]') as HTMLElement | null;
    if (!el) return;

    // Remember this activation to suppress the ghost click only for it
    lastSynthAt = performance.now();
    const rect = el.getBoundingClientRect();
    lastSynthX = rect.left + rect.width / 2;
    lastSynthY = rect.top + rect.height / 2;

    // Prefer native navigation for links
    if (el instanceof HTMLAnchorElement && el.href) {
      // Only web links navigate; anything else (such as a script URL) is ignored.
      if (!/^https?:$/.test(el.protocol)) return;
      if (el.target && el.target !== '_self') {
        window.open(el.href, el.target, 'noopener,noreferrer');
      } else {
        window.location.href = el.href;
      }
    } else {
      el.click(); // fires the element’s click handlers immediately
    }
  }

  function onPointerDown(e: PointerEvent) {
    // Ignore multi-touch
    if (e.pointerType === 'touch' && (e as any).isPrimary === false) return;
    downX = e.clientX;
    downY = e.clientY;
    downAt = performance.now();
  }
  function onPointerUp(e: PointerEvent) {
    if (!sidebar.isMobile) return;
    if (isTap(e.clientX, e.clientY, performance.now())) {
      activate(e.target as Element);
      // prevent iOS from also doing a late synthetic click
      console.log('stopping stuff pionterup');
      e.preventDefault?.();
    }
  }

  // Fallback for older iOS: use touch events as well (non-passive to allow preventDefault)
  function onTouchStart(e: TouchEvent) {
    const t = e.changedTouches[0];
    downX = t.clientX;
    downY = t.clientY;
    downAt = performance.now();
  }
  function onTouchEnd(e: TouchEvent) {
    if (!sidebar.isMobile) return;
    const t = e.changedTouches[0];
    if (isTap(t.clientX, t.clientY, performance.now())) {
      activate(e.target as Element);
      console.log('stopping stuff touchend');
      e.preventDefault(); // stop the synthesised click for THIS tap
    }
  }

  // Suppress ONLY the one ghost click that follows our manual activation
  function onWindowClickCapture(e: MouseEvent) {
    if (!sidebar.isMobile) return;
    const dt = performance.now() - lastSynthAt;
    if (dt > SUPPRESS_MS) return;
    const dx = e.clientX - lastSynthX;
    const dy = e.clientY - lastSynthY;
    if (dx * dx + dy * dy <= 24 * 24) {
      console.log('stopping stuff windowclick');
      e.stopPropagation();
      e.preventDefault();
    }
  }
  onMount(() => {
    // Bind to the sidebar surface so only sidebar taps are affected
    if (ref) {
      // ref.addEventListener('pointerdown', onPointerDown, { capture: true });
      // ref.addEventListener('pointerup', onPointerUp, { capture: true });
      // ref.addEventListener('touchstart', onTouchStart as any, { capture: true, passive: false });
      // ref.addEventListener('touchend', onTouchEnd as any, { capture: true, passive: false });
    }
    // window.addEventListener('click', onWindowClickCapture, true);
  });

  onDestroy(() => {
    if (ref) {
      ref.removeEventListener('pointerdown', onPointerDown, { capture: true } as any);
      ref.removeEventListener('pointerup', onPointerUp, { capture: true } as any);
      ref.removeEventListener('touchstart', onTouchStart as any, { capture: true } as any);
      ref.removeEventListener('touchend', onTouchEnd as any, { capture: true } as any);
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('click', onWindowClickCapture, true);
    }
  });

  const sidebar = useSidebar();
</script>

{#if collapsible === 'none'}
  <div
    class={cn(
      'bg-sidebar text-sidebar-foreground flex h-full w-[var(--sidebar-width)] flex-col',
      className,
    )}
    bind:this={ref}
    {...restProps}
  >
    {@render children?.()}
  </div>
{:else if sidebar.isMobile}
  <Sheet.Root bind:open={sidebar.openMobile} onOpenChange={sidebar.setOpenMobile} {...restProps}>
    <Sheet.Content
      data-sidebar="sidebar"
      data-mobile="true"
      class="bg-sidebar text-sidebar-foreground w-[var(--sidebar-width)] p-0 [&>button]:hidden"
      style="--sidebar-width: {SIDEBAR_WIDTH_MOBILE}"
      side={side}
    >
      <div class="flex h-full w-full flex-col">
        {@render children?.()}
      </div>
    </Sheet.Content>
  </Sheet.Root>
{:else}
  <div
    bind:this={ref}
    class="text-sidebar-foreground group peer hidden h-full w-[var(--sidebar-width)] md:block"
    data-state={sidebar.state}
    data-collapsible={sidebar.state === 'collapsed' ? collapsible : ''}
    data-variant={variant}
    data-side={side}
  >
    <!-- This is what handles the sidebar gap on desktop -->
    <div
      class={cn(
        'relative h-full w-[var(--sidebar-width)] bg-transparent transition-[width] duration-200 ease-linear',
        'group-data-[collapsible=offcanvas]:w-0',
        'group-data-[side=right]:rotate-180',
        variant === 'floating' || variant === 'inset'
          ? 'group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)_+_theme(spacing.4))]'
          : 'group-data-[collapsible=icon]:w-[var(--sidebar-width-icon)]',
      )}
    ></div>
    <div
      class={cn(
        'fixed inset-y-0 bottom-0 top-0 z-10 hidden h-full w-[var(--sidebar-width)] transition-[left,right,width] duration-200 ease-linear md:flex',
        side === 'left'
          ? 'left-0 group-data-[collapsible=offcanvas]:left-[calc(var(--sidebar-width)*-1)]'
          : 'right-0 group-data-[collapsible=offcanvas]:right-[calc(var(--sidebar-width)*-1)]',
        // Adjust the padding for floating and inset variants.
        variant === 'floating' || variant === 'inset'
          ? 'p-2 group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)_+_theme(spacing.4)_+2px)]'
          : 'border-sidebar-border group-data-[collapsible=icon]:w-[var(--sidebar-width-icon)] group-data-[side=left]:border-r group-data-[side=right]:border-l',
        className,
      )}
      {...restProps}
    >
      <div
        data-sidebar="sidebar"
        class="group-data-[variant=floating]:border-sidebar-border flex h-full w-full flex-col group-data-[variant=floating]:rounded-lg group-data-[variant=floating]:border group-data-[variant=floating]:shadow"
      >
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

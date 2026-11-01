<script lang="ts">
	import { getContext } from 'svelte';
	import { LinkPreview as HoverCardPrimitive } from 'bits-ui';
	import { cn } from '../utils.js';
	import { POPOVER_PORTAL } from '../popover/portal.js';

	let {
		ref = $bindable(null),
		class: className,
		sideOffset = 4,
		align = 'center',
		children,
		...restProps
	}: HoverCardPrimitive.ContentProps = $props();

	const portal = getContext<HTMLElement | undefined>(POPOVER_PORTAL);
</script>

<HoverCardPrimitive.Portal to={portal}>
	<HoverCardPrimitive.Content
		bind:ref
		data-slot="hover-card-content"
		{sideOffset}
		{align}
		class={cn(
			'bg-popover text-popover-foreground border-primary/10 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 outline-hidden z-50 w-72 rounded-md border p-3 text-xs shadow-md',
			className,
		)}
		{...restProps}
	>
		{@render children?.()}
	</HoverCardPrimitive.Content>
</HoverCardPrimitive.Portal>

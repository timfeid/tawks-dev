import { LinkPreview as HoverCardPrimitive } from 'bits-ui';
import Content from './hover-card-content.svelte';

/** A card that opens on hover or keyboard focus and can hold buttons, unlike a tooltip. */
const Root = HoverCardPrimitive.Root;
const Trigger = HoverCardPrimitive.Trigger;

export {
	Root,
	Trigger,
	Content,
	//
	Root as HoverCard,
	Trigger as HoverCardTrigger,
	Content as HoverCardContent,
};

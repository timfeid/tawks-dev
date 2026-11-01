/**
 * Context key for the element popovers open into. A page drawn in another
 * window (a video pop-out) sets it so its popovers open in that window
 * instead of the one that owns the code. Unset, they open in the page body.
 */
export const POPOVER_PORTAL = Symbol('popover-portal');

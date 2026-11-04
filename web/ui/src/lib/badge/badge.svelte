<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const badgeVariants = tv({
    base: 'focus:ring-ring inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2',
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground hover:bg-primary/80 border-transparent',
        secondary:
          'bg-secondary text-secondary-foreground hover:bg-secondary/80 border-transparent',
        destructive:
          'bg-destructive text-destructive-foreground hover:bg-destructive/80 border-transparent',
        outline: 'text-foreground',
        success:
          'border-transparent bg-emerald-500/15 text-emerald-700 hover:bg-emerald-500/25 dark:bg-emerald-400/15 dark:text-emerald-300',
        warning:
          'border-transparent bg-amber-500/15 text-amber-700 hover:bg-amber-500/25 dark:bg-amber-400/15 dark:text-amber-300',
        danger:
          'border-transparent bg-red-500/15 text-red-700 hover:bg-red-500/25 dark:bg-red-400/15 dark:text-red-300',
        info: 'border-transparent bg-sky-500/15 text-sky-700 hover:bg-sky-500/25 dark:bg-sky-400/15 dark:text-sky-300',
        violet:
          'border-transparent bg-violet-500/15 text-violet-700 hover:bg-violet-500/25 dark:bg-violet-400/15 dark:text-violet-300',
        glass:
          'bg-white/5 border-white/10 text-white/70 backdrop-blur-md hover:bg-white/10 hover:text-white hover:border-white/20',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  });

  export type BadgeVariant = VariantProps<typeof badgeVariants>['variant'];

  /** A left edge in a badge's tone, for showing the same meaning without the tag. */
  export const badgeStrip = {
    success: 'border-emerald-700 dark:border-emerald-300',
    warning: 'border-amber-700 dark:border-amber-300',
    danger: 'border-red-700 dark:border-red-300',
    info: 'border-sky-700 dark:border-sky-300',
    violet: 'border-violet-700 dark:border-violet-300',
    secondary: 'border-muted-foreground/60',
  } as const;
</script>

<script lang="ts">
  import type { WithElementRef } from 'bits-ui';
  import type { HTMLAnchorAttributes } from 'svelte/elements';
  import { cn } from '../utils.js';

  let {
    ref = $bindable(null),
    href,
    class: className,
    variant = 'default',
    children,
    ...restProps
  }: WithElementRef<HTMLAnchorAttributes> & {
    variant?: BadgeVariant;
  } = $props();
</script>

<svelte:element
  this={href ? 'a' : 'span'}
  bind:this={ref}
  {href}
  class={cn(badgeVariants({ variant }), className)}
  {...restProps}
>
  {@render children?.()}
</svelte:element>

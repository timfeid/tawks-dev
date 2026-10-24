<script lang="ts" module>
  import type { WithElementRef } from 'bits-ui';
  import type {
    HTMLAnchorAttributes,
    HTMLButtonAttributes,
  } from 'svelte/elements';
  import { type VariantProps, tv } from 'tailwind-variants';

  export const buttonVariants = tv({
    base: 'cursor-pointer ring-offset-background focus-visible:ring-ring inline-flex items-center justify-center gap-1 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none  [&_svg]:shrink-0',
    variants: {
      variant: {
        default: 'bg-primary/80 text-primary-foreground hover:bg-primary/90',
        destructive:
          'bg-destructive text-destructive-foreground hover:bg-destructive/90',
        outline:
          'border-input bg-background hover:bg-accent hover:text-accent-foreground border',
        secondary:
          'bg-secondary text-secondary-foreground hover:bg-secondary/80',
        ghost:
          'hover:bg-accent text-muted-foreground hover:text-accent-foreground',
        hoveronly: 'hover:text-primary text-primary/50',
        link: 'text-primary underline-offset-4 hover:underline',
        iconButton:
          'rounded-full bg-neutral-400 p-0 text-black hover:bg-neutral-100 hover:text-neutral-900 group-hover:bg-neutral-100 hover:text-neutral-900',
      },
      size: {
        default: 'h-9 px-3 py-1.5 [&_svg]:size-4',
        sm: 'h-8 rounded-md px-2.5',
        md: 'h-8 rounded-md px-2.5',
        lg: 'h-10 rounded-md px-8',
        icon: 'h-8 w-8 [&_svg]:size-4',
        'icon-lgsvg': 'h-8 w-8 [&_svg]:size-5',
        'icon-lg': 'h-10 w-10 [&_svg]:size-5',
        'icon-sm': 'h-6 w-6 [&_svg]:size-4',
        'icon-xs': 'h-3.5 w-3.5 [&_svg]:size-3 p-0',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  });

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
  export type ButtonSize = VariantProps<typeof buttonVariants>['size'];

  export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
    WithElementRef<HTMLAnchorAttributes> & {
      variant?: ButtonVariant;
      size?: ButtonSize;
    };
</script>

<script lang="ts">
  import { cn } from '../utils.js';

  let {
    class: className,
    variant = 'default',
    size = 'default',
    ref = $bindable(null),
    href = undefined,
    type = 'button',
    children,
    ...restProps
  }: ButtonProps = $props();
</script>

{#if href}
  <a
    bind:this={ref}
    class={cn(buttonVariants({ variant, size }), className)}
    {href}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    bind:this={ref}
    class={cn(buttonVariants({ variant, size }), className)}
    {type}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}

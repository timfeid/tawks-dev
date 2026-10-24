<script lang="ts">
  import { Label as LabelPrimitive } from 'bits-ui';
  import { cn } from '../utils.js';

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  }: LabelPrimitive.RootProps = $props();
</script>

<LabelPrimitive.Root
  bind:ref
  class={cn(
    'lbl block text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70',
    className,
  )}
  {...restProps}
/>

<style lang="postcss">
  :global(.lbl .required) {
    color: var(--color-red-300);
    font-size: 10px;
    vertical-align: bottom;
  }
</style>

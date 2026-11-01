<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import { cn } from '../utils.js';
  import { tabsLook } from './look.js';

  let { ref = $bindable(null), class: className, ...restProps }: TabsPrimitive.ListProps = $props();
</script>

<TabsPrimitive.List bind:ref class={cn(tabsLook.bar, className)} {...restProps} />

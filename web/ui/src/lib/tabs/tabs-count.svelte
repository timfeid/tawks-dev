<script lang="ts">
  import { tabsLook } from './look.js';

  let { value, label }: { value: number; label?: string } = $props();
</script>

<span class={tabsLook.count} aria-label={label}>{value}</span>

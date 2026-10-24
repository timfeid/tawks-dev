<script lang="ts">
  let { children, class: className } = $props();
</script>

<div class="flex shrink-0 items-center justify-center rounded-full px-2 font-mono {className}">
  {@render children()}
</div>

<script lang="ts">
  let { title = 'Full combo' }: { title?: string } = $props();
</script>

<span class="rounded bg-primary/15 px-1 py-px font-mono text-[10px] font-bold text-primary" {title}>FC</span>

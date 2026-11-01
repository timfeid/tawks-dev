<script lang="ts">
  import { cn } from '../utils.js';

  let { label = 'Loading', class: className }: { label?: string; class?: string } = $props();
</script>

<div class={cn('flex items-center justify-center', className)} role="status" aria-label={label}>
  <span class="page-loader" aria-hidden="true">
    <span></span><span></span><span></span>
  </span>
</div>

<style>
  .page-loader {
    display: flex;
    gap: 0.375rem;
  }
  .page-loader span {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 9999px;
    background: var(--color-primary);
    opacity: 0.25;
    animation: page-loader-wave 1.2s ease-in-out infinite;
  }
  .page-loader span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .page-loader span:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes page-loader-wave {
    0%,
    60%,
    100% {
      opacity: 0.25;
      transform: translateY(0) scale(0.85);
    }
    30% {
      opacity: 1;
      transform: translateY(-0.25rem) scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .page-loader span {
      animation: none;
      opacity: 0.6;
    }
  }
</style>

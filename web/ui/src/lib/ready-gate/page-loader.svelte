<script lang="ts">
  import { cn } from '../utils.js';
  import { SILHOUETTE } from '../ghost/paths.js';

  let { label = 'Loading', class: className }: { label?: string; class?: string } = $props();
</script>

<div class={cn('flex items-center justify-center', className)} role="status" aria-label={label}>
  <span class="ghost-loader" aria-hidden="true">
    <span class="sway">
      <svg viewBox="0 0 128 128" class="ghost"><path d={SILHOUETTE} /></svg>
    </span>
  </span>
</div>

<style>
  .ghost-loader {
    display: block;
    width: 2.75rem;
    height: 2.75rem;
    animation: ghost-loader-float 3.6s ease-in-out infinite;
  }

  .sway {
    display: block;
    animation: ghost-loader-sway 7.2s infinite;
  }

  .ghost {
    display: block;
    width: 2.75rem;
    height: 2.75rem;
    fill: color-mix(in oklab, var(--color-foreground) 30%, transparent);
    animation: ghost-loader-breathe 3.6s ease-in-out infinite;
  }

  @keyframes ghost-loader-float {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-0.3125rem);
    }
  }

  /* Eases out of the centre and back in, so the side-to-side drift never pauses mid-swing. */
  @keyframes ghost-loader-sway {
    0% {
      transform: translateX(0) rotate(0);
      animation-timing-function: cubic-bezier(0.33, 0.66, 0.66, 1);
    }
    25% {
      transform: translateX(0.1875rem) rotate(3deg);
      animation-timing-function: ease-in-out;
    }
    75% {
      transform: translateX(-0.1875rem) rotate(-3deg);
      animation-timing-function: cubic-bezier(0.33, 0, 0.66, 0.34);
    }
    100% {
      transform: translateX(0) rotate(0);
    }
  }

  @keyframes ghost-loader-breathe {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ghost-loader,
    .sway,
    .ghost {
      animation: none;
    }
  }
</style>

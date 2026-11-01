<script lang="ts">
  import { cn } from '../utils.js';
  import {
    BODY,
    FACE_BAND,
    GHOST_COLORS,
    GHOST_LENSES,
    GLASSES_BRIDGE,
    HEAD,
    LEFT_ARM,
    LEFT_CUP,
    LEFT_LENS,
    MOUTH,
    RIGHT_ARM,
    RIGHT_CUP,
    RIGHT_LENS,
    SILHOUETTE,
  } from '../ghost/paths.js';

  let { label = 'Loading', class: className }: { label?: string; class?: string } = $props();

  const id = $props.id();
</script>

<div class={cn('flex items-center justify-center', className)} role="status" aria-label={label}>
  <span class="ghost-loader" aria-hidden="true">
    <svg viewBox="0 0 128 128" class="ghost">
      <defs>
        <clipPath id="{id}-inside"><path d={SILHOUETTE} /></clipPath>
        <clipPath id="{id}-lenses"><path d={LEFT_LENS} /><path d={RIGHT_LENS} /></clipPath>
      </defs>
      <path class="halo" d={SILHOUETTE} />
      <path d={SILHOUETTE} fill={GHOST_COLORS.outline} />
      <path d={HEAD} fill={GHOST_COLORS.face} />
      <path d={BODY} fill={GHOST_COLORS.face} />
      <path d={FACE_BAND} fill={GHOST_COLORS.face} clip-path="url(#{id}-inside)" />
      <path class="arm left" d={LEFT_ARM} fill={GHOST_COLORS.face} />
      <path class="arm right" d={RIGHT_ARM} fill={GHOST_COLORS.face} />
      <path d={LEFT_CUP} fill={GHOST_COLORS.leftCup} />
      <path d={RIGHT_CUP} fill={GHOST_COLORS.rightCup} />
      <path
        d={MOUTH}
        fill={GHOST_COLORS.mouth}
        stroke={GHOST_COLORS.mouth}
        stroke-width="3"
        stroke-linejoin="round"
      />
      <path
        d={GLASSES_BRIDGE}
        fill="none"
        stroke={GHOST_COLORS.outline}
        stroke-width="8"
        stroke-linecap="round"
      />
      {#each GHOST_LENSES as lens (lens.rim)}
        <path
          d={lens.rim}
          fill={GHOST_COLORS.outline}
          stroke={GHOST_COLORS.outline}
          stroke-width="9"
          stroke-linejoin="round"
        />
        {#each lens.layers as [d, fill] (d)}<path {d} {fill} />{/each}
      {/each}
      <g clip-path="url(#{id}-lenses)">
        <path class="glint" d="M6 36 L20 36 L4 64 L-10 64 Z" />
      </g>
    </svg>
    <span class="shadow"></span>
  </span>
</div>

<style>
  .ghost-loader {
    position: relative;
    display: block;
    width: 2.75rem;
    height: 3.25rem;
  }

  .ghost {
    display: block;
    width: 2.75rem;
    height: 2.75rem;
    overflow: visible;
    animation: ghost-loader-bob 2.8s ease-in-out infinite;
  }

  .halo {
    fill: none;
    stroke: color-mix(in oklab, var(--color-foreground) 22%, transparent);
    stroke-width: 6;
    stroke-linejoin: round;
  }

  .arm {
    transform-box: fill-box;
    animation: ghost-loader-wave 2.8s ease-in-out infinite;
  }

  .arm.left {
    transform-origin: 100% 0%;
  }

  .arm.right {
    transform-origin: 0% 0%;
    animation-name: ghost-loader-wave-right;
  }

  .glint {
    fill: #fff;
    opacity: 0.85;
    animation: ghost-loader-glint 2.8s ease-in-out infinite;
  }

  .shadow {
    position: absolute;
    left: 50%;
    bottom: 0;
    width: 1.75rem;
    height: 0.3125rem;
    margin-left: -0.875rem;
    border-radius: 50%;
    background: radial-gradient(
      closest-side,
      color-mix(in oklab, var(--color-foreground) 30%, transparent),
      transparent
    );
    animation: ghost-loader-shadow 2.8s ease-in-out infinite;
  }

  @keyframes ghost-loader-bob {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-0.1875rem);
    }
  }

  @keyframes ghost-loader-shadow {
    0%,
    100% {
      transform: scaleX(1);
      opacity: 1;
    }
    50% {
      transform: scaleX(0.72);
      opacity: 0.55;
    }
  }

  @keyframes ghost-loader-wave {
    0%,
    100% {
      transform: rotate(0);
    }
    50% {
      transform: rotate(14deg);
    }
  }

  @keyframes ghost-loader-wave-right {
    0%,
    100% {
      transform: rotate(0);
    }
    50% {
      transform: rotate(-14deg);
    }
  }

  @keyframes ghost-loader-glint {
    0%,
    30% {
      transform: translateX(0);
    }
    70%,
    100% {
      transform: translateX(110px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ghost,
    .arm,
    .glint,
    .shadow {
      animation: none;
    }
  }
</style>

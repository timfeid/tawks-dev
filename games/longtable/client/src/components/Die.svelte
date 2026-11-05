<script lang="ts">
  import { onDestroy } from 'svelte';

  let {
    sides,
    value = null,
    size = 44,
    kept = true,
    tumble = false,
    tone = 'bone',
  }: {
    sides: number;
    value?: number | null;
    size?: number;
    kept?: boolean;
    tumble?: boolean;
    tone?: 'bone' | 'brass' | 'ember' | 'plain';
  } = $props();

  const SHAPES: Record<number, string> = {
    4: '50,7 95,89 5,89',
    6: '14,14 86,14 86,86 14,86',
    8: '50,3 96,50 50,97 4,50',
    10: '50,3 93,40 50,97 7,40',
    12: '50,4 96,37 79,93 21,93 4,37',
    20: '50,3 93,27 93,73 50,97 7,73 7,27',
  };
  const FACETS: Record<number, string> = {
    20: 'M50 3 L28 62 L72 62 Z M7 27 L28 62 M93 27 L72 62 M50 97 L28 62 M50 97 L72 62',
    8: 'M4 50 L96 50',
    10: 'M7 40 L50 58 L93 40 M50 58 L50 97',
    12: 'M50 22 L72 40 L64 66 L36 66 L28 40 Z',
  };

  const shape = $derived(SHAPES[sides] ?? SHAPES[10]);
  const facets = $derived(FACETS[sides === 100 ? 10 : sides] ?? '');
  const label = $derived(value === null ? (sides === 100 ? '%' : `d${sides}`) : String(value));
  const textY = $derived(sides === 4 ? 70 : sides === 20 ? 54 : 52);
  const fontSize = $derived(label.length >= 3 ? 26 : label.length === 2 ? 32 : 38);

  let shown = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  $effect(() => {
    if (!tumble || value === null) return;
    let ticks = 0;
    timer = setInterval(() => {
      ticks += 1;
      shown = ticks >= 9 ? null : String(1 + Math.floor(Math.random() * sides));
      if (ticks >= 9 && timer) clearInterval(timer);
    }, 55);
    return () => {
      if (timer) clearInterval(timer);
      shown = null;
    };
  });

  onDestroy(() => timer && clearInterval(timer));
</script>

<svg
  class="die {tone}"
  class:dropped={!kept}
  class:tumble
  width={size}
  height={size}
  viewBox="0 0 100 100"
  role="img"
  aria-label={value === null ? `d${sides}` : `d${sides} showing ${value}`}
>
  <polygon class="body" points={shape} />
  {#if facets}<path class="facet" d={facets} />{/if}
  <text x="50" y={textY} font-size={fontSize} text-anchor="middle" dominant-baseline="middle">{shown ?? label}</text>
</svg>

<style>
  .die {
    flex: none;
    overflow: visible;
    filter: drop-shadow(0 2px 2px rgba(0, 0, 0, 0.55));
  }

  .body {
    stroke-width: 4;
    stroke-linejoin: round;
  }

  .facet {
    fill: none;
    stroke-width: 2;
    opacity: 0.35;
  }

  text {
    font-family: var(--font-number);
    font-weight: 700;
  }

  .bone .body {
    fill: #efe3c8;
    stroke: #b59c6d;
  }
  .bone .facet {
    stroke: #8f7a52;
  }
  .bone text {
    fill: var(--ink);
  }

  .brass .body {
    fill: var(--brass-bright);
    stroke: #fff0c2;
  }
  .brass .facet {
    stroke: #7a5a24;
  }
  .brass text {
    fill: var(--wood-950);
  }
  .brass {
    filter: drop-shadow(0 0 8px rgba(240, 196, 109, 0.75));
  }

  .ember .body {
    fill: #f0a184;
    stroke: var(--ember);
  }
  .ember .facet {
    stroke: #8a3018;
  }
  .ember text {
    fill: #3a1206;
  }

  .plain .body {
    fill: transparent;
    stroke: var(--brass);
  }
  .plain .facet {
    stroke: var(--brass);
  }
  .plain text {
    fill: var(--text);
    font-family: var(--font-body);
    font-size: 26px;
  }

  .dropped {
    opacity: 0.38;
    filter: grayscale(0.6);
  }

  .tumble {
    animation: tumble 560ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }

  @keyframes tumble {
    0% {
      transform: translate(-30px, -14px) rotate(-200deg) scale(0.7);
      opacity: 0;
    }
    60% {
      transform: translate(4px, 2px) rotate(18deg) scale(1.05);
      opacity: 1;
    }
    100% {
      transform: none;
    }
  }
</style>

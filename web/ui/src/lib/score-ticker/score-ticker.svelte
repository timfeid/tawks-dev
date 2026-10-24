<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { spring } from 'svelte/motion';
  import { cn } from '../utils.js';

  interface Props {
    value: number;
    icon: (className: string) => ReturnType<Snippet>;
    class?: string;
    child?: (isIncreasing: boolean, increaseAmount: number) => ReturnType<Snippet>;
    iconClass?: string;
    containerClass?: string;
    id?: string;
    delay?: number;
    popTextClass?: string;
    popContainerClass?: string;
    onIncrease?: (amount: number) => void;
  }

  let {
    delay,
    value = 0,
    class: className = '',
    child,
    icon,
    id,
    iconClass = 'text-primary/55',
    containerClass = 'bg-primary/5',
    popTextClass = 'text-indigo-100 drop-shadow-[0_0_8px_rgba(99,102,241,0.5)]',
    popContainerClass = 'border-indigo-500/50 bg-indigo-500/20',
    onIncrease,
  }: Props = $props();

  let previousValue = $state(value);
  let isIncreasing = $state(false);
  let increaseAmount = $state(0);
  let animationTimeout: ReturnType<typeof setTimeout>;
  let springTimeout: ReturnType<typeof setTimeout>;

  const displayedScore = spring(value, {
    stiffness: 0.01,
    damping: 0.2,
    precision: 1,
  });

  $effect(() => {
    clearTimeout(springTimeout);
    springTimeout = setTimeout(() => {
      displayedScore.set(value);
    }, delay);

    if (value > previousValue) {
      increaseAmount = value - previousValue;
      triggerIncreaseAnimation();
    }

    previousValue = value;
  });

  function triggerIncreaseAnimation() {
    clearTimeout(animationTimeout);
    isIncreasing = true;
    onIncrease?.(increaseAmount);
    animationTimeout = setTimeout(() => {
      isIncreasing = false;
    }, 600);
  }
</script>

<div {id} class="relative inline-flex {className}">
  <div
    class={cn(
      'relative flex items-center gap-1.5 rounded-full px-2 py-0.5 transition-colors duration-300',
      isIncreasing ? `${popContainerClass} animate-score-pop-container` : '',
      containerClass,
    )}
  >
    {@render icon(`size-4 ${iconClass}`)}
    <div class="flex items-baseline gap-1 leading-none">
      {@render child?.(isIncreasing, increaseAmount)}
      <span
        class="font-mono font-bold tabular-nums transition-all duration-200 {isIncreasing
          ? `${popTextClass} animate-score-pop-text`
          : ''}"
      >
        {Math.round($displayedScore).toLocaleString()}
      </span>
    </div>
  </div>
</div>

<style>
  @keyframes score-pop-text {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.15);
    }
    100% {
      transform: scale(1);
    }
  }

  @keyframes score-pop-container {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.05);
    }
    100% {
      transform: scale(1);
    }
  }

  .animate-score-pop-text {
    animation: score-pop-text 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .animate-score-pop-container {
    animation: score-pop-container 0.6s ease-out;
  }
</style>

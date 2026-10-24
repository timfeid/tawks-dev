<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils';
  import { onMount, onDestroy } from 'svelte';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();
  let countdown = $derived('Countdown' in display.type ? display.type.Countdown : undefined);
  let str = $derived(countdown ? interpolate(inputs, countdown.until) : '');
  let until = $derived(str ? new Date(str) : undefined);
  console.log(str, until);
  let remainingMs = $state(0);
  let shake = $derived(
    countdown?.shake_under_ms_left && remainingMs < countdown.shake_under_ms_left,
  );
  let interval = $state<ReturnType<typeof setInterval>>();

  let shouldUpdateFast = $derived(remainingMs < (countdown?.shake_under_ms_left ?? 5000));

  function updateRemaining() {
    const now = new Date();
    if (until) {
      const diff = until.getTime() - now.getTime();
      remainingMs = Math.max(0, diff);
    }

    const delay = shouldUpdateFast ? 50 : 1000;
    interval = setTimeout(updateRemaining, delay);
  }

  onMount(() => {
    updateRemaining();
  });

  onDestroy(() => {
    clearInterval(interval);
  });

  function updateTime(ms: number) {
    const formatter = new Intl.NumberFormat(navigator.language, { maximumFractionDigits: 0 });
    const decimalFormatter = new Intl.NumberFormat(navigator.language, {
      maximumFractionDigits: 1,
      minimumFractionDigits: 1,
    });
    const totalSeconds = ms / 1000;
    const days = Math.floor(totalSeconds / (24 * 3600));
    const hours = Math.floor((totalSeconds % (24 * 3600)) / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = Math.floor(totalSeconds % 60);

    return {
      days: formatter.format(days),
      hours: formatter.format(hours),
      minutes: formatter.format(minutes),
      seconds: formatter.format(seconds),
      ms: formatter.format(ms % 1000),
      total_seconds: shouldUpdateFast
        ? decimalFormatter.format(totalSeconds)
        : formatter.format(totalSeconds),
    };
  }
</script>

{#if countdown}
  <div class={` flex justify-center`}>
    <span
      class={`${display.class} transition-all duration-200 ${shake ? 'shake animate-pulse' : ''}`}
      aria-live="polite"
    >
      {interpolate({ ...inputs, [countdown.as]: updateTime(remainingMs) }, countdown.text)}
    </span>
  </div>
{/if}

<style>
  @keyframes shake {
    0% {
      transform: translateX(0);
    }
    25% {
      transform: translateX(-2px);
    }
    50% {
      transform: translateX(2px);
    }
    75% {
      transform: translateX(-1px);
    }
    100% {
      transform: translateX(0);
    }
  }

  .shake {
    animation: shake 0.4s ease-in-out infinite;
  }
</style>

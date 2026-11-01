<script lang="ts" module>
  let formatters: { whole: Intl.NumberFormat; tenths: Intl.NumberFormat } | undefined;

  function numberFormats() {
    formatters ??= {
      whole: new Intl.NumberFormat(navigator.language, { maximumFractionDigits: 0 }),
      tenths: new Intl.NumberFormat(navigator.language, { maximumFractionDigits: 1, minimumFractionDigits: 1 }),
    };
    return formatters;
  }
</script>

<script lang="ts">
  import { interpolate } from '../../utils';
  import { untrack } from 'svelte';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();
  let countdown = $derived('Countdown' in display.type ? display.type.Countdown : undefined);
  let str = $derived(countdown ? interpolate(inputs, countdown.until) : '');
  let until = $derived(str ? new Date(str) : undefined);
  let remainingMs = $state(0);
  let shake = $derived(
    countdown?.shake_under_ms_left && remainingMs < countdown.shake_under_ms_left,
  );
  let timer: ReturnType<typeof setTimeout> | undefined;

  let shouldUpdateFast = $derived(remainingMs < (countdown?.shake_under_ms_left ?? 5000));

  function tick(target: number) {
    remainingMs = Math.max(0, target - Date.now());
    if (remainingMs === 0) return;
    timer = setTimeout(() => tick(target), shouldUpdateFast ? 50 : 1000);
  }

  $effect(() => {
    const target = until?.getTime();
    if (target === undefined || Number.isNaN(target)) {
      remainingMs = 0;
      return;
    }
    untrack(() => tick(target));
    return () => clearTimeout(timer);
  });

  function updateTime(ms: number) {
    const { whole: formatter, tenths: decimalFormatter } = numberFormats();
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
  <div class="flex justify-center">
    <span
      class={[look.text(countdown.role ?? 'Body', look.toneOf(countdown.tone, inputs)), 'tabular-nums transition-all duration-200', shake && 'shake animate-pulse']}
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

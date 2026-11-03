<script lang="ts">
  import type { VacuumView } from './protocol';
  let { vacuum, seat, button }: { vacuum: VacuumView; seat: number; button: string } = $props();
  let seenOut = $state(-1);
  let seenAt = $state(performance.now());
  let now = $state(performance.now());
  $effect(() => {
    if (vacuum.out !== seenOut) {
      seenOut = vacuum.out;
      seenAt = performance.now();
    }
  });
  $effect(() => {
    let frame = 0;
    const loop = () => {
      now = performance.now();
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  });
  const period = $derived(vacuum.brace_period ?? 0.8);
  const window = $derived(vacuum.brace_window ?? 0.15);
  const phase = $derived(((vacuum.out + (vacuum.on ? (now - seenAt) / 1000 : 0)) / period) % 1);
  const braced = $derived(vacuum.braced.includes(seat));
  const gripped = $derived(!!vacuum.gripped?.includes(seat));
  const slipping = $derived(!!vacuum.slipping?.includes(seat));
</script>

<div
  class="pointer-events-none absolute top-[18%] left-1/2 z-30 w-[min(360px,calc(100vw-2rem))] -translate-x-1/2 rounded-2xl border px-4 py-2 text-sm font-bold backdrop-blur {gripped
    ? 'border-emerald-300/50 bg-emerald-950/85 text-emerald-100'
    : slipping
      ? 'border-rose-300/50 bg-rose-950/85 text-rose-100'
      : 'border-white/15 bg-slate-950/85 text-white'}"
  aria-live="polite"
>
  <div>
    {gripped
      ? 'Tight grip: you keep almost all your coins'
      : slipping
        ? 'Slipped: grab on the beat, not in between'
        : braced
          ? `Holding on. Let go and grab ${button} again as the ring hits the band for a tight grip`
          : `Your coins are being sucked away. Hold ${button} to brace`}
  </div>
  <div class="relative mt-2 h-4 rounded-full bg-white/10" aria-label="Motor beat">
    <div class="absolute inset-y-0.5 rounded-full border border-emerald-300 bg-emerald-300/30" style="left:{(0.5 - window) * 100}%;width:{window * 200}%"></div>
    <div class="absolute top-1/2 h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white" style="left:{phase * 100}%"></div>
  </div>
</div>

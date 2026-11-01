export const SPIKE_SWEEP_SECONDS = 0.45;
export const SPIKE_SLOW_SECONDS = 1.0;
export const SPIKE_RECOVER_SECONDS = 0.3;
export const SPIKE_IMPACT_SECONDS = SPIKE_SWEEP_SECONDS + SPIKE_SLOW_SECONDS + SPIKE_RECOVER_SECONDS;

/** Local character-animation clock; authoritative room time keeps running. */
export function spikeImpactClock(age: number) {
  const elapsed=Math.max(0,age);
  return Math.min(elapsed,SPIKE_SWEEP_SECONDS)
    +Math.max(0,Math.min(elapsed-SPIKE_SWEEP_SECONDS,SPIKE_SLOW_SECONDS))*0.2
    +Math.max(0,elapsed-SPIKE_SWEEP_SECONDS-SPIKE_SLOW_SECONDS);
}

/** A smooth camera sweep avoids cutting abruptly into the landed character. */
export function impactSweep(age: number) {
  const t=Math.max(0,Math.min(1,age/SPIKE_SWEEP_SECONDS));
  return t*t*(3-2*t);
}

/** Eases an AudioParam toward `value`; skips the step when a value is NaN or infinite, which the Web Audio API rejects with a TypeError. */
export function glide(
  param: AudioParam,
  value: number,
  at: number,
  timeConstant: number,
) {
  if (Number.isFinite(value) && Number.isFinite(at))
    param.setTargetAtTime(value, at, timeConstant);
}

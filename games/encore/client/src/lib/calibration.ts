/** Taps further than this many scaled deviations from the median are slips, not timing. */
const OUTLIER_MADS = 3;
/** Taps this close to the median always count, so a very steady player keeps them all. */
const OUTLIER_FLOOR_MS = 15;
/** A calibration run needs at least this many taps that count. */
export const MIN_CALIBRATION_TAPS = 8;
/** Learning from play waits for this many hits in a song. */
export const MIN_PLAY_HITS = 50;
/** Hits that average closer to on time than this need no change. */
export const PLAY_THRESHOLD_MS = 8;
/** Auto adjust moves the audio offset by at most this much per song. */
export const MAX_AUTO_STEP_MS = 10;
/** Auto adjust takes this share of the measured error each song. */
export const AUTO_SMOOTHING = 0.5;

function middle(sorted: number[]) {
  const n = sorted.length;
  return n % 2 ? sorted[(n - 1) / 2] : (sorted[n / 2 - 1] + sorted[n / 2]) / 2;
}

/** The median after dropping outliers by median absolute deviation, with how many samples it kept. */
export function robustMedian(samples: number[]): { median: number; kept: number } | null {
  const finite = samples.filter(Number.isFinite);
  if (!finite.length) return null;
  const first = middle([...finite].sort((a, b) => a - b));
  const mad = middle(finite.map((x) => Math.abs(x - first)).sort((a, b) => a - b)) * 1.4826;
  const limit = Math.max(OUTLIER_FLOOR_MS, OUTLIER_MADS * mad);
  const kept = finite.filter((x) => Math.abs(x - first) <= limit).sort((a, b) => a - b);
  return { median: middle(kept), kept: kept.length };
}

/** The offset a calibration run measured, rounded to a millisecond, or null with too few good taps. */
export function calibrationResult(deviations: number[]) {
  const result = robustMedian(deviations);
  if (!result || result.kept < MIN_CALIBRATION_TAPS) return null;
  return Math.round(result.median);
}

/**
 * How far a song's hits landed from on time, when there were enough of them
 * and they were off by more than the threshold; otherwise null.
 */
export function playSuggestion(errors: number[]) {
  if (errors.length < MIN_PLAY_HITS) return null;
  const result = robustMedian(errors);
  if (!result || result.kept < MIN_PLAY_HITS) return null;
  const late = Math.round(result.median);
  return Math.abs(late) > PLAY_THRESHOLD_MS ? late : null;
}

/** One auto-adjust step toward the measured error, smoothed and capped. */
export function autoStep(offset: number, late: number) {
  const step = Math.max(-MAX_AUTO_STEP_MS, Math.min(MAX_AUTO_STEP_MS, Math.round(late * AUTO_SMOOTHING)));
  return offset + step;
}

/** The video offset that makes a visual run's taps line up, from the offset it ran with. */
export function videoResult(current: number, deviations: number[]) {
  const measured = calibrationResult(deviations);
  return measured === null ? null : current - measured;
}

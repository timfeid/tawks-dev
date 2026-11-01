import type { Activity, MicrophoneState } from '@tawks/activity-sdk';
import { PitchTracker } from '@tawks/activity-sdk/pitch';

/** The tawks page this game is framed in, once connected; null when run on its own. */
export const host: { activity: Activity | null } = { activity: null };

/** A pitch reading with its time on this page's `performance.now()` clock. */
export type HeardSample = { at: number; hz: number | null; level: number };

/**
 * Starts listening for singing. Inside tawks the page around the frame owns
 * the microphone and sends pitch only; run on its own the game listens itself.
 */
export async function listen(onSamples: (samples: HeardSample[]) => void): Promise<{ state: MicrophoneState; stop: () => void }> {
  const convert = (samples: { t: number; hz: number | null; level: number }[]) =>
    onSamples(samples.map((s) => ({ at: s.t - performance.timeOrigin, hz: s.hz, level: s.level })));
  const activity = host.activity;
  if (activity) {
    const off = activity.onPitch(convert);
    const state = await activity.microphone(true);
    return {
      state,
      stop: () => {
        off();
        void activity.microphone(false);
      },
    };
  }
  const tracker = new PitchTracker(convert);
  try {
    await tracker.start();
    return { state: 'on', stop: () => tracker.stop() };
  } catch {
    return { state: 'denied', stop: () => {} };
  }
}

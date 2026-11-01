export type Hand = 'left' | 'right' | 'both';

type Pad = { axes: readonly number[]; buttons: readonly { pressed: boolean }[] };

const PRESS_AT = 0.5;
const RELEASE_AT = 0.3;

/** The hands a controller holds down: the left stick, the d-pad or a bumper
 * for each side, and the action button for both. The stick lets go a little
 * inside where it caught, so a wobble at the edge is not a second press. */
export function padHands(pad: Pad, action: number, was: ReadonlySet<Hand>): Set<Hand> {
  const x = pad.axes[0] ?? 0;
  const button = (index: number) => !!pad.buttons[index]?.pressed;
  const leaning = (sign: number, hand: Hand) => sign * x > (was.has(hand) ? RELEASE_AT : PRESS_AT);
  const held = new Set<Hand>();
  if (leaning(-1, 'left') || button(14) || button(4)) held.add('left');
  if (leaning(1, 'right') || button(15) || button(5)) held.add('right');
  if (button(action)) held.add('both');
  return held;
}

/**
 * One message per new press, sent the moment it happens, for bouts that count
 * presses. The stick is sampled and throttled, so quick taps would merge.
 */
export class PressSource {
  active = false;
  private pad = new Set<Hand>();
  private send: (hand: Hand) => void;

  constructor(send: (hand: Hand) => void) {
    this.send = send;
  }

  key(hand: Hand | null, repeat: boolean) {
    if (!this.active || !hand || repeat) return false;
    this.send(hand);
    return true;
  }

  poll(pad: Pad | null, action: number) {
    const held = pad && this.active ? padHands(pad, action, this.pad) : new Set<Hand>();
    for (const hand of held) if (!this.pad.has(hand)) this.send(hand);
    this.pad = held;
  }
}

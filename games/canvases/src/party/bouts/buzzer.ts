import { lerp, type BoutFrame, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import { BuzzerPanel, type PanelPlayer } from '../buzzer/panel';
import { BuzzerPodiums } from '../buzzer/podiums';
import type { BuzzerEvent, BuzzerSetup, BuzzerSnap } from '../buzzer/types';

type View = BoutFrame<BuzzerSnap, BuzzerSetup>;

const NUMBER_KEYS: Record<string, number> = {
  Digit1: 0, Digit2: 1, Digit3: 2, Digit4: 3,
  Numpad1: 0, Numpad2: 1, Numpad3: 2, Numpad4: 3,
};

const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

function joinNames(names: string[]) {
  if (names.length <= 1) return names[0] ?? '';
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}

/** Buzzer Round: the question and answers are a HUD over a row of lecterns. */
const buzzer: BoutModule<BuzzerSnap, BuzzerSetup, BuzzerEvent> = {
  create(host): BoutRenderer<BuzzerSnap, BuzzerSetup, BuzzerEvent> {
    const podiums = new BuzzerPodiums(host);
    let last: View | null = null;
    let picked: number | null = null;
    let pickedRound = -1;
    let highlight = 0;
    let stickRested = true;
    let padHeld = false;

    const canAnswer = (view: View | null): view is View & { snapshot: BuzzerSnap } =>
      !!view && view.playing && view.phase === 'playing' && view.snapshot?.stage === 'asking';

    const pick = (choice: number) => {
      if (!canAnswer(last) || picked !== null) return;
      picked = choice;
      pickedRound = last.snapshot.round;
      host.send({ type: 'fire', yaw: choice, pitch: 0 });
    };

    const panel = new BuzzerPanel(host.canvas.parentElement ?? document.body, (choice) => {
      (document.activeElement as HTMLElement | null)?.blur?.();
      pick(choice);
    });

    const onKey = (event: KeyboardEvent) => {
      if (event.repeat || isTyping(event.target) || !canAnswer(last)) return;
      const numbered = NUMBER_KEYS[event.code];
      if (numbered !== undefined) {
        event.preventDefault();
        pick(numbered);
      } else if (event.code === 'Space' || event.code === 'Enter') {
        // A focused answer button answers through its own click.
        if (event.target instanceof HTMLElement && event.target.closest('.buzzer-option')) return;
        event.preventDefault();
        pick(highlight);
      }
    };
    window.addEventListener('keydown', onKey);

    /** Arrows, the stick and the touch pad move the highlight one step at a time. */
    const steer = (view: View) => {
      const { x, z } = view.stick;
      const reach = Math.max(Math.abs(x), Math.abs(z));
      if (reach < 0.35) {
        stickRested = true;
        return;
      }
      if (!stickRested || reach < 0.6) return;
      stickRested = false;
      let column = highlight % 2;
      let row = Math.floor(highlight / 2);
      if (Math.abs(x) > Math.abs(z)) column = x > 0 ? 1 : 0;
      else row = z > 0 ? 1 : 0;
      highlight = row * 2 + column;
    };

    const readPad = () => {
      const pads = typeof navigator.getGamepads === 'function' ? Array.from(navigator.getGamepads()) : [];
      const pressed = !!pads.find(Boolean)?.buttons[0]?.pressed;
      if (pressed && !padHeld) pick(highlight);
      padHeld = pressed;
    };

    const note = (view: View, snap: BuzzerSnap, name: (seat: number) => string) => {
      if (view.phase === 'countdown') return 'First right answer scores. One guess each.';
      if (snap.stage === 'reveal') {
        if (!snap.scorers.length) return 'Nobody got that one.';
        const who = joinNames(snap.scorers.map(name));
        return snap.scorers.length > 1 ? `${who} got it at the same moment.` : `${who} got it first.`;
      }
      if (!view.playing) return 'Everyone is answering.';
      const mine = snap.players.find((player) => player.seat === view.mySeat);
      if (mine?.state === 'wrong') return 'Not that one. Wait for the next question.';
      if (picked !== null) return 'Locked in.';
      return 'One guess. First right answer scores.';
    };

    return {
      frame(view) {
        last = view;
        const snap = view.snapshot;
        if (view.phase !== 'playing' || !snap || snap.round !== pickedRound) {
          picked = null;
          pickedRound = -1;
        }
        podiums.sync(view.seats, view.setup?.stage);
        podiums.update(snap, view.winners, view.time);
        if (canAnswer(view) && picked === null) {
          steer(view);
          readPad();
        } else {
          stickRested = false;
          padHeld = true;
        }

        const seats = new Map(view.seats.map((seat) => [seat.seat, seat]));
        const name = (seat: number) => seats.get(seat)?.name ?? 'Someone';
        const players: PanelPlayer[] = (snap?.players ?? []).map((player) => {
          const seat = seats.get(player.seat);
          return {
            seat: player.seat,
            name: name(player.seat),
            color: (seat && host.character(seat.character)?.body) || '#94a3b8',
            points: player.points,
            state: player.state,
            me: player.seat === view.mySeat,
          };
        });
        const stage = view.phase === 'playing' ? 'playing' : view.phase === 'countdown' ? 'countdown' : 'hidden';
        panel.render({
          stage,
          snapshot: snap,
          answering: view.playing,
          picked,
          highlight,
          players,
          note: snap ? note(view, snap, name) : '',
        });
      },
      effect(event, view) {
        if (event.kind !== 'buzz') return;
        if (event.right) host.sounds.count(true);
        else if (event.seat === view.mySeat) host.sounds.bump(0.5);
      },
      shot: () => ({ kind: 'frame', half: [podiums.halfWidth, 3.2], pitch: 14, portraitPitch: 20, lookHeight: 2.4 }),
      hud: (view) => ({
        objective: 'First right answer scores',
        hint: 'Press 1 to 4, or pick with the arrows and press Space.',
        touch: 'none',
        scores: view.snapshot?.players.map((player) => ({ seat: player.seat, hits: player.points, up: true })),
      }),
      dispose() {
        window.removeEventListener('keydown', onKey);
        panel.dispose();
        podiums.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game || a.round !== b.round || a.stage !== b.stage) return b;
    return { ...b, time: lerp(a.time, b.time, t) };
  },
};

export default buzzer;

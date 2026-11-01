import { lerp, type BoutFrame, type BoutHud, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { ArenaSetup, ArenaSnap, GameEvent } from '../protocol';
import { PartyArena } from '../arena/scene';
import { asMembers, rosterKeeper } from './common';
import { PressSource, type Hand } from './presses';
import { gamepad, settings } from '../settings';
import { isGameplayTyping } from '../typing';

const HINTS: Record<string, string> = {
  crumble: 'Move with WASD. Standing still cracks the tile under you.',
  hotpotato: 'Move with WASD. Run into someone to hand the bomb on.',
  balloon: 'Move with WASD. Click or press space to dash and burst a balloon.',
  sumo: 'Move with WASD. Build up speed, then run through a rival to shove them off.',
  coins: 'Move with WASD. Click or press space to charge and shake coins loose.',
  tug: 'Alternate A, D or space to heave. The same press twice does nothing.',
};

const OBJECTIVES: Record<string, string> = {
  coins: 'Grab the coins · most coins wins',
};

/** What the clock warns about: gold is worth five and anyone can reach it. */
function goldAlert(snap: ArenaSnap | null) {
  if (snap?.game !== 'coins') return null;
  if ((snap.coins ?? []).some((coin) => coin.value > 1 && coin.y > 0.5)) return 'Gold coin falling';
  const due = snap.next_gold;
  return due !== null && due !== undefined && due <= 4 ? 'Gold coin coming' : null;
}


const SIDES: [string, string] = ['RED', 'BLUE'];

function keyHand(code: string): Hand | null {
  const { keys } = settings();
  if (keys.left.includes(code)) return 'left';
  if (keys.right.includes(code)) return 'right';
  if (keys.action.includes(code)) return 'both';
  return null;
}

/**
 * The knot's position is the whole of Tug of War, so the HUD carries it: the
 * two sides, who is on yours, and how much of the rope each has.
 */
function tugHud(view: BoutFrame<ArenaSnap, ArenaSetup>): BoutHud {
  const snap = view.snapshot?.game === 'tug' ? view.snapshot : null;
  const me = snap?.players.find((player) => player.seat === view.mySeat);
  const brink = snap?.brink ?? (view.setup?.game === 'tug' ? view.setup.brink : 4.5);
  const rope = snap?.rope ?? 0;
  const side = me?.team ?? 0;
  // Red hauls the knot to -x and blue to +x, so the bar fills toward your post.
  const ours = rope * (side === 0 ? -1 : 1);
  const lead = Math.abs(rope) < 0.3 ? 'dead even' : `${Math.abs(rope).toFixed(1)}m toward ${SIDES[rope < 0 ? 0 : 1]}`;
  return {
    objective: me ? `${SIDES[side]} · heave the knot back to your post` : 'Heave the knot back to your post',
    hint: HINTS.tug,
    touch: 'pump',
    teams: SIDES,
    scores: (snap?.players ?? []).map((player) => ({
      seat: player.seat,
      hits: player.heaves ?? 0,
      up: !player.out,
      team: player.team,
    })),
    meter: { label: 'ROPE', progress: Math.min(1, Math.max(0, 0.5 + ours / (brink * 2))), text: lead },
    alert: Math.abs(rope) > brink * 0.7 ? `${SIDES[rope < 0 ? 1 : 0]} are on the edge` : null,
  };
}

/** Crumble Run, Hot Potato Panic, Balloon Brawl, Sumo, Coin Scramble and Tug of War
 * share one floor. */
const floor: BoutModule<ArenaSnap, ArenaSetup, GameEvent> = {
  create(host): BoutRenderer<ArenaSnap, ArenaSetup, GameEvent> {
    const arena = new PartyArena(host.scene);
    const roster = rosterKeeper(host);
    const presses = new PressSource((hand) => host.send({ type: 'press', hand }));
    const onKey = (event: KeyboardEvent) => {
      if (isGameplayTyping(event.target)) return;
      if (presses.key(keyHand(event.code), event.repeat)) event.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return {
      frame(view) {
        presses.active = view.key === 'tug' && view.phase === 'playing' && view.playing;
        presses.poll(gamepad(), settings().pad.action);
        arena.visible = true;
        arena.sync(view.snapshot, view.setup, asMembers(view.seats), roster(view.seats));
        arena.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        arena.effect(event, asMembers(view.seats));
        const { sounds } = host;
        // Four players hauling at once is a lot of thuds, so only yours is heard.
        if (event.kind === 'heave' && event.seat === view.mySeat) sounds.step(0.8);
        else if (event.kind === 'splash') sounds.splash();
        else if (event.kind === 'dunked') sounds.fall();
        else if (event.kind === 'boom') sounds.blast();
        else if (event.kind === 'grab') sounds.beep(event.value > 1 ? 2.4 : 0.4);
        else if (event.kind === 'spill') sounds.splat();
        else if (event.kind === 'gold') sounds.count(true);
        else if (event.kind === 'pop') sounds.splat();
        else if (event.kind === 'over' || event.kind === 'drop') sounds.fall();
        else if (event.kind === 'pass') sounds.bump(0.5);
        else if (event.kind === 'bump') sounds.bump(event.strength);
        else if (event.kind === 'tile_fall') sounds.step(0.5);
      },
      shot(view) {
        const setup = view.setup;
        // The rope runs across x, so this one is framed wide and from lower down.
        if (setup?.game === 'tug') {
          return { kind: 'frame', half: [setup.half + 1.5, setup.half_z + 4], pitch: 44, portraitPitch: 58, lookHeight: 0 };
        }
        const snap = view.snapshot;
        // Sumo's platform closes in all round, so the shot closes with it.
        const half =
          snap?.game === 'sumo' && snap.radius
            ? Math.max(snap.radius, 7)
            : setup && 'radius' in setup
              ? setup.radius
              : setup && 'half' in setup
                ? setup.half
                : 14;
        return { kind: 'frame', half: [half + 2, half + 2], pitch: 54, portraitPitch: 64 };
      },
      shake: () => arena.shake,
      hud: (view) =>
        view.key === 'tug'
          ? tugHud(view)
          : {
              objective: OBJECTIVES[view.key] ?? 'Outlast the field',
              hint: HINTS[view.key],
              alert: goldAlert(view.snapshot),
            },
      dispose() {
        window.removeEventListener('keydown', onKey);
        arena.reset();
        arena.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev || prev.out !== next.out) return next;
      return { ...next, x: lerp(prev.x, next.x, t), y: lerp(prev.y ?? 0, next.y ?? 0, t), z: lerp(prev.z, next.z, t) };
    });
    const radius = a.radius === undefined || b.radius === undefined ? b.radius : lerp(a.radius, b.radius, t);
    const rope = a.rope !== undefined && b.rope !== undefined ? lerp(a.rope, b.rope, t) : b.rope;
    const coins = b.coins?.map((next) => {
      const prev = a.coins?.find((coin) => coin.id === next.id);
      if (!prev) return next;
      return { ...next, x: lerp(prev.x, next.x, t), y: lerp(prev.y, next.y, t), z: lerp(prev.z, next.z, t) };
    });
    return { ...b, time: lerp(a.time, b.time, t), radius, rope, players, coins };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) =>
        player.out ? player : { ...player, x: player.x + (player.vx ?? 0) * dt, z: player.z + (player.vz ?? 0) * dt },
      ),
    };
  },
};

export default floor;

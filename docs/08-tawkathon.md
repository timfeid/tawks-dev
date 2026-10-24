# Tawkathon bouts

Tawkathon is the board game people play in a lobby. Players take turns around an island board, and between turns everyone votes on the next bout: a short real-time minigame for two to four players. The winners collect coins, and the board carries on.

Each bout has two halves:

- **The simulation** runs on the tawks game server. It reads every player's stick and buttons sixty times a second, moves everything, and decides who won.
- **The renderer** runs in each player's tawks app. It draws the arena from the snapshots the simulation sends and plays the effects.

Both halves are built against the bout SDK: `tawks-bout` for the Rust simulation and `@tawks/bout-sdk` for the TypeScript renderer. Bumper Balls is the worked example on this page, and its full source sits next to the SDK.

> Status: the SDK and Bumper Balls are in the [tawks-dev repository](https://github.com/timfeid/tawks-dev). To get a bout into Tawkathon today, build it against the SDK and open a pull request there; the tawks team reviews it and ships it with the first-party bouts. Running your own bout on tawks without a review, with the simulation in a sandbox and the renderer in an isolated frame, comes later. The contract on this page is the one both will use.

## How a room runs a bout

1. The bout is picked in the lobby or voted in on the board. The room builds it right away, so players see the arena while they wait.
2. A three second countdown.
3. The room calls `step` 60 times a second with every player's input. Practice bots get their input from the bout's own `bot_input`. A player who drops out sends an empty input.
4. Every third tick (20 times a second) the room sends `snapshot` to every client, with the events the bout pushed since the last one.
5. As soon as `placements` returns the finishing order, the room shows the results and pays out.

The room owns seats, characters, phases and the clock. The bout owns everything inside the arena.

## The simulation

```rust
pub trait Bout: Send {
    fn key(&self) -> &'static str;
    /// The fixed layout the client needs to build the arena.
    fn setup(&self) -> Value { Value::Null }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>);
    fn bot_input(&mut self, seat: u8, dt: f32) -> Input;
    fn snapshot(&self) -> Value;
    fn placements(&self) -> Option<Vec<Vec<u8>>>;
}
```

- **Inputs.** `Input` carries an analog `stick` (`x` to the right, `z` toward the camera, clamped to the unit circle), `yaw` and `pitch` for aiming, and `fire`, `jump` and `crouch`. `fire` is true only on the tick after a press. Call `stick.clamped()` before using it; inputs come from players, so never trust their range.
- **Setup.** Anything that never changes during the bout, such as a track layout or the walls of a maze, goes in `setup`. It is sent once with the room, not twenty times a second.
- **Snapshots.** A snapshot is a JSON object with `game` (your key), `time` and `limit` in seconds (the clock at the top of the screen reads them), and whatever your renderer needs. Round floats to three decimals to keep it small.
- **Events.** Push one JSON object per moment the players should see or hear, such as a bump, a splash or a goal. Give each a `kind`.
- **Placements.** Groups of seats, best first. Seats in the same group tied. Return `None` until the bout is over, and make sure it ends: run a clock, shrink the arena, or both.
- **Bots.** `bot_input` is called for every practice bot each tick. Each `Contestant` has a `skill` from 0 to 1, and `Skill` turns it into the usual knobs: aim error, reaction time, hesitation and pace.
- **Randomness.** `create` receives a seed. Draw every random number from a generator seeded with it so a bout can be replayed in a test.

Every bout is described by a `BoutSpec`:

```rust
pub const SPEC: BoutSpec = BoutSpec {
    key: "bumper",
    name: "Bumper Balls",
    rules: "Roll, bump, and be the last one above the water.",
    controls: Controls::Move,
    format: Format::FreeForAll,
    min_players: 2,
    max_players: 4,
    create: |contestants, seed| Box::new(BumperBalls::new(contestants, seed)),
};
```

`rules` is the line players read on the ballot. `controls` decides which inputs the app collects and sends:

| Controls | The player gets |
| --- | --- |
| `Move` | the stick (WASD, arrow keys, a gamepad or the on-screen joystick) |
| `MoveAction` | the stick and one button (space, a click or the touch button), sent as `fire` |
| `MoveAim` | the stick, the mouse to aim (sent as `yaw`), and a button to fire |
| `FirstPerson` | mouse look, the stick to walk, fire, jump and crouch |

`format` is `FreeForAll` or `Teams`. The vote only offers bouts whose player range fits the room.

### Bumper Balls, step by step

Each ball accelerates toward the stick, drags to a stop when the stick is let go, and cannot be pushed past top speed:

```rust
let stick = input.stick.clamped();
let (ax, az) = (stick.x * ACCEL, stick.z * ACCEL);
let speed = (ball.vx * ball.vx + ball.vz * ball.vz).sqrt();
if speed < MAX_SPEED || ball.vx * ax + ball.vz * az < 0.0 {
    ball.vx += ax * dt;
    ball.vz += az * dt;
}
let pushing = stick.x.abs() + stick.z.abs() > 0.1;
let drag = if pushing { ROLL_DRAG } else { COAST_DRAG };
let keep = (-drag * dt).exp();
ball.vx *= keep;
ball.vz *= keep;
```

Two balls that overlap are pushed apart and bounce with a minimum kick, so even a slow nudge does something. Each bump becomes an event:

```rust
events.push(json!({
    "kind": "bump", "a": a.seat, "b": b.seat,
    "strength": (impulse / MAX_SPEED).min(2.0),
    "x": (a.x + b.x) / 2.0, "z": (a.z + b.z) / 2.0,
}));
```

A ball whose centre leaves the platform starts falling, pushes a `fall` event, and a `splash` when it hits the water. After 25 seconds the platform starts to shrink, and at 60 seconds whoever is still on it shares first place. Everyone else ranks by how long they lasted.

The snapshot is the platform radius and one entry per ball:

```rust
json!({
    "game": "bumper",
    "time": round(self.time),
    "limit": ROUND_SECONDS,
    "radius": round(self.radius),
    "ball_radius": BALL_RADIUS,
    "balls": [{ "seat": 0, "x": 1.2, "z": -3.4, "y": 0.0, "vx": 0.5, "vz": 0.0, "state": "rolling" }],
})
```

### Testing a bout

A bout is plain state, so test it tick by tick without a server:

```rust
#[test]
fn practice_bots_finish_a_round_on_their_own() {
    let mut game = BumperBalls::new(&contestants(4, true), 4);
    let mut events = Vec::new();
    let mut steps = 0;
    while game.placements().is_none() && steps < 5000 {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
    }
    assert!(game.placements().is_some());
    assert!(events.iter().any(|e| e["kind"] == "bump"));
}
```

Every bout should have at least this test: bots alone must finish it.

## The renderer

A renderer module is a TypeScript module whose default export is a `BoutModule`:

```ts
export type BoutModule<Snapshot, Setup, Event> = {
  create(host: BoutHost): BoutRenderer<Snapshot, Setup, Event>;
  blend?(a: Snapshot, b: Snapshot, t: number): Snapshot;
  extrapolate?(snapshot: Snapshot, dt: number): Snapshot;
};
```

tawks loads it the first time someone picks the bout, calls `create` when the bout appears, and `dispose` when it goes away, for example when the room returns to the board.

**The host** is everything tawks lends you: the three.js `scene` and `camera`, the shared sea at `waterY`, `sounds` (bump, fall, splash, shot, splat, blast and a few more), `character(id)` and `rider(character)` to build each player's party character, and `send` for bouts that need to send input themselves. Do not add lights, sky or water; the shared stage already has them.

**Each frame** tawks calls `frame(view)`. The view has the blended `snapshot`, the `setup`, the room `phase` (`lobby`, `countdown`, `playing` or `results`), the `seats` with names and characters, `mySeat`, the `winners` once the bout is over, whether this viewer is `playing`, their `stick` and `look`, and `time` and `dt`.

**The rest is optional:**

| Method | What it does |
| --- | --- |
| `effect(event, view)` | plays an event once the view reaches the moment it happened |
| `shot(view)` | where the camera goes: `{ kind: 'frame', half, pitch }` keeps a box in view from above, `{ kind: 'eye', ... }` is first person |
| `shake()` | camera shake in world units |
| `hud(view)` | the objective line, the keyboard hint, an alert under the clock, and for first-person bouts the health panel and scores |
| `aim(point, view)` | for `MoveAim` bouts, turns the ground point under the mouse into the yaw sent with input |
| `facing(view)` | for `FirstPerson` bouts, where the player looks when they come back in |
| `fog` | mist distances, for arenas bigger than the default |

### Smooth motion

Snapshots arrive 20 times a second, and tawks draws the bout about 100 ms behind the newest one. With `blend`, tawks hands you the two snapshots on either side of that moment and how far between them it is. Without it, the bout moves in steps. `extrapolate` carries the newest snapshot forward when the next one is late.

```ts
blend(a, b, t) {
  const balls = b.balls.map((next) => {
    const prev = a.balls.find((ball) => ball.seat === next.seat);
    if (!prev || (prev.state === 'out') !== (next.state === 'out')) return next;
    return { ...next, x: lerp(prev.x, next.x, t), z: lerp(prev.z, next.z, t), y: lerp(prev.y, next.y, t) };
  });
  return { ...b, time: lerp(a.time, b.time, t), radius: lerp(a.radius, b.radius, t), balls };
},
```

Do not blend through a teleport, such as a respawn; return the newer state instead.

### Bumper Balls on screen

The renderer builds the platform once, then keeps one ball and rider per seat in the snapshot:

```ts
frame(view) {
  this.sync(view);
  this.animate(view);
}

shot(): Shot {
  return { kind: 'frame', half: [11.5, 9.5], pitch: 38, portraitPitch: 52 };
}

hud(view): BoutHud {
  const closing = view.phase === 'playing' && !!view.snapshot && view.snapshot.radius < ARENA_RADIUS - 0.02;
  return {
    objective: 'Outlast the field',
    hint: 'Roll with WASD, the arrow keys or a gamepad. Knock everyone off.',
    alert: closing ? 'The edge is closing in' : null,
  };
}
```

Each ball is rolled by the distance it travelled since the last frame, and its rider turns toward where it is heading. The rider's pose follows the phase: `ride` while playing, `fall` on the way into the water, and `cheer` or `sad` on the results screen.

Events become particles and sounds:

```ts
effect(event: BumperEvent) {
  if (event.kind === 'bump') {
    this.host.sounds.bump(event.strength);
    this.burstStars(event.x, event.z, event.strength);
    this.quake = Math.min(0.5, this.quake + event.strength * 0.18);
  } else if (event.kind === 'splash') {
    this.host.sounds.splash();
    this.splash(event.x, event.z);
  }
}
```

`dispose` frees every geometry, material and texture the renderer made and removes its group from the scene. A bout is created and disposed many times in one Tawkathon, so leaks add up.

## Checklist

- Clamp every input and treat missing input as standing still.
- Make sure every bout ends, even if nobody moves.
- Put fixed layout in `setup`, and only what moves in `snapshot`.
- Seed all randomness from the seed you are given.
- Test that bots alone finish the bout.
- Dispose everything you create in the renderer.
- Keep the arena around the origin, roughly 20 by 20 units, unless your `shot` frames something bigger.

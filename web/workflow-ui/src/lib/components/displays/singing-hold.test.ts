import { test } from "node:test";
import assert from "node:assert/strict";
import { SingingHold } from "./singing-hold.ts";

function clock() {
  let now = 0;
  let next = 1;
  const pending = new Map<number, { at: number; fn: () => void }>();
  return {
    timers: {
      set: (fn: () => void, ms: number) => {
        const id = next++;
        pending.set(id, { at: now + ms, fn });
        return id;
      },
      clear: (id: unknown) => void pending.delete(id as number),
    },
    advance(ms: number) {
      now += ms;
      for (const [id, t] of [...pending]) {
        if (t.at <= now) {
          pending.delete(id);
          t.fn();
        }
      }
    },
  };
}

test("singing mutes once, heartbeats keep it, and stopping lets go", () => {
  const c = clock();
  const seen: boolean[] = [];
  const hold = new SingingHold((on) => seen.push(on), 5000, c.timers);
  hold.beat(true);
  for (let i = 0; i < 5; i++) {
    c.advance(2000);
    hold.beat(true);
  }
  assert.deepEqual(seen, [true]);
  hold.beat(false);
  assert.deepEqual(seen, [true, false]);
  hold.beat(false);
  hold.release();
  assert.deepEqual(seen, [true, false]);
});

test("a game that stops beating is let go after the timeout", () => {
  const c = clock();
  const seen: boolean[] = [];
  const hold = new SingingHold((on) => seen.push(on), 5000, c.timers);
  hold.beat(true);
  c.advance(4999);
  assert.equal(hold.on, true);
  c.advance(1);
  assert.equal(hold.on, false);
  assert.deepEqual(seen, [true, false]);
});

test("the frame closing lets go", () => {
  const c = clock();
  const seen: boolean[] = [];
  const hold = new SingingHold((on) => seen.push(on), 5000, c.timers);
  hold.beat(true);
  hold.release();
  c.advance(10000);
  assert.deepEqual(seen, [true, false]);
});

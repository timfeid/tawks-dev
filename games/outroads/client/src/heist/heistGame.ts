/**
 * One player's Port Run: drives their car, walks their person, rides along
 * in someone else's, and draws the night around them: other cars, cops and
 * people, parked cars, spikes, pings, light bars and sirens.
 */

import * as THREE from 'three';
import { Sound } from '../lib/audio';
import { Input, type Press } from '../lib/input';
import type { DriveConnection } from '../lib/net.svelte';
import { forwardSpeed, newCar, step, type CarState } from '../lib/physics';
import { QUALITY, World, type Quality } from '../lib/world';
import { animate, disposeWalker, makeWalker, type Walker } from './character';
import { Effects } from './effects';
import { stance, stepFoot, type FootState } from './foot';
import { beamGeometry, beamMaterial, Glow, glowMaterial, glowTexture, Lights, torchGeometry } from './night';
import { CLASS_LABEL, COLOURS, SHADE, type Class, type HeistEvent, type HeistPoses, type HeistView, type HeistWorld, type VehicleView } from './protocol';
import { Sirens } from './siren';
import { Track } from './track';
import { Traffic } from './traffic';
import { applyDamage, disposeVehicle, makeVehicle, parkedFleet, SHAPES, type VehicleModel } from './vehicles';

const SEND_EVERY = 0.1;
const BEHIND = 0.12;
const NIGHT_HOUR = 23.6;

type Dyn = { model: VehicleModel; beam: THREE.Mesh; spot: THREE.Mesh | null; track: Track; x: number; y: number; z: number; h: number; s: number; lights: boolean; siren: boolean; spotOn: boolean; kind: string };
type Person = { walker: Walker; torch: THREE.Mesh; track: Track; x: number; y: number; z: number; h: number; s: number; crouch: boolean; flash: boolean; label: HTMLDivElement };

export type HeistHud = {
  mode: 'drive' | 'ride' | 'foot' | 'watch';
  speed: number;
  street: string | null;
  prompt: string | null;
  inspect: { text: string; matches: boolean | null } | null;
  torch: boolean;
  lights: boolean;
  fps: number;
  calls: number;
  triangles: number;
  x: number;
  z: number;
  h: number;
};

export class HeistGame {
  readonly world: World;
  readonly input: Input;
  readonly sound = new Sound();
  readonly sirens = new Sirens();
  readonly effects: Effects;
  readonly lights: Lights;
  glow: Glow | null = null;
  traffic: Traffic;
  hud: HeistHud = { mode: 'watch', speed: 0, street: null, prompt: null, inspect: null, torch: false, lights: true, fps: 0, calls: 0, triangles: 0, x: 0, z: 0, h: 0 };
  onHud: (h: HeistHud) => void = () => {};
  onPress: (p: Press) => void = () => {};
  onNotice: (text: string) => void = () => {};
  cinematic: { x: number; z: number; h: number; t: number } | null = null;

  car: CarState | null = null;
  carId: number | null = null;
  foot: FootState | null = null;
  torchOn = false;
  headlights = true;
  camYaw = 0;
  camPitch = 0.25;
  private dragYawAt = -10;

  private view: HeistView | null = null;
  private worldVersion = -1;
  private dyn = new Map<number, Dyn>();
  private ai = new Map<string, Dyn>();
  private people = new Map<string, Person>();
  private me: Person | null = null;
  private fleets = new Map<Class, THREE.InstancedMesh>();
  private parked: HeistWorld['parked'] = [];
  private parkedSlot = new Map<number, { cls: Class; i: number }>();
  private spikes = new Map<number, THREE.Object3D>();
  private beacons = new Map<number, THREE.Mesh>();
  private readers: THREE.Object3D[] = [];
  private gate: THREE.Group;
  private container: THREE.Mesh | null = null;
  private alarms = new Map<number, number>();
  private porch: { x: number; z: number; until: number }[] = [];
  private porchMesh: THREE.InstancedMesh;
  private beamMat = beamMaterial();
  private torchMat = beamMaterial();
  private sendClock = 0;
  private hornHeld = false;
  private impactAt = 0;
  private epoch = -1;
  private raf = 0;
  private last = 0;
  private frames = 0;
  private hudClock = 0;
  private camPos = new THREE.Vector3();
  private disposed = false;
  private resizeObserver: ResizeObserver;
  private dragging: { x: number; y: number } | null = null;

  constructor(
    private canvas: HTMLCanvasElement,
    private labels: HTMLElement,
    private conn: DriveConnection,
    quality: Quality = matchMedia('(pointer: coarse)').matches ? QUALITY.low : QUALITY.high,
  ) {
    this.world = new World(canvas, quality);
    this.world.terrainUrl = conn.terrainUrl;
    this.world.lampPools = true;
    this.world.setPoolTexture(glowTexture());
    this.world.setHour(NIGHT_HOUR);
    if (quality.antialias) {
      this.glow = new Glow(this.world.renderer, this.world.scene, this.world.camera);
      this.world.glow = this.glow;
    }
    this.input = new Input(window);
    this.effects = new Effects(this.world.scene);
    this.lights = new Lights(this.world.scene);
    this.traffic = new Traffic(this.world.scene, this.world.index, quality.antialias ? 10 : 5, (x, z) => this.world.groundAt(x, z));
    this.gate = this.makeGate();
    this.world.scene.add(this.gate);
    this.porchMesh = new THREE.InstancedMesh(new THREE.PlaneGeometry(9, 9).rotateX(-Math.PI / 2), glowMaterial(0xffe0a0, 0.6), 12);
    this.porchMesh.count = 0;
    this.porchMesh.frustumCulled = false;
    this.world.scene.add(this.porchMesh);
    conn.onHeistPoses = (p) => this.poses(p);
    conn.onHeistEvent = (e) => this.event(e);
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    canvas.addEventListener('pointerdown', this.pointerDown);
    window.addEventListener('pointermove', this.pointerMove);
    window.addEventListener('pointerup', this.pointerUp);
    this.resize();
    this.last = performance.now();
    this.raf = requestAnimationFrame(this.tick);
  }

  private pointerDown = (e: PointerEvent) => {
    this.dragging = { x: e.clientX, y: e.clientY };
  };
  private pointerMove = (e: PointerEvent) => {
    if (!this.dragging) return;
    this.camYaw += (e.clientX - this.dragging.x) * 0.006;
    this.camPitch = Math.max(-0.1, Math.min(1.0, this.camPitch + (e.clientY - this.dragging.y) * 0.004));
    this.dragging = { x: e.clientX, y: e.clientY };
    this.dragYawAt = performance.now() / 1000;
  };
  private pointerUp = () => {
    this.dragging = null;
  };

  private resize() {
    const r = this.canvas.getBoundingClientRect();
    this.world.resize(Math.max(1, r.width), Math.max(1, r.height));
  }

  private makeGate(): THREE.Group {
    const g = new THREE.Group();
    // Tall yard lights around the gate, each a pole, a glowing head and a pool on the ground.
    for (let i = 0; i < 6; i++) {
      const a = (i / 6) * Math.PI * 2 + 0.3;
      const x = Math.cos(a) * 46;
      const z = Math.sin(a) * 46;
      const pole = new THREE.Mesh(new THREE.BoxGeometry(0.4, 18, 0.4).translate(x, 9, z), new THREE.MeshLambertMaterial({ color: 0x55585e }));
      const head = new THREE.Mesh(new THREE.BoxGeometry(2.2, 0.5, 1.2).translate(x, 18, z), new THREE.MeshBasicMaterial({ color: 0xfff0c8 }));
      const pool = new THREE.Mesh(new THREE.PlaneGeometry(46, 46).rotateX(-Math.PI / 2).translate(x * 0.85, 0.2, z * 0.85), glowMaterial(0xffe2a8, 0.5));
      g.add(pole, head, pool);
    }
    const ring = new THREE.Mesh(new THREE.RingGeometry(20, 22, 48).rotateX(-Math.PI / 2), glowMaterial(0xff9d2e, 0.9));
    ring.position.y = 0.3;
    g.add(ring);
    const colours = [0x8a3324, 0x2f5d7c, 0x6b6f2a, 0x7a7d80, 0x9c5a1c];
    for (let i = 0; i < 14; i++) {
      const box = new THREE.Mesh(new THREE.BoxGeometry(12.2, 2.6, 2.45), new THREE.MeshLambertMaterial({ color: colours[i % colours.length] }));
      const a = (i / 14) * Math.PI * 2;
      box.position.set(Math.cos(a) * 34, 1.3 + (i % 3 === 0 ? 2.6 : 0), Math.sin(a) * 34);
      box.rotation.y = -a;
      g.add(box);
    }
    return g;
  }

  /** The server's view of the run changed. */
  sync(view: HeistView) {
    this.view = view;
    const room = this.conn.room;
    if (room?.place && room.epoch !== this.epoch) {
      this.epoch = room.epoch;
      this.world.setOrigin(room.place);
      this.world.setHour(NIGHT_HOUR);
    }
    const world = this.conn.heistWorld;
    if (world && world.version !== this.worldVersion) this.rebuildWorld(world);
    const ground = this.world.groundAt(view.gate.x, view.gate.z);
    this.gate.position.set(view.gate.x, ground, view.gate.z);
    this.syncPlace(view);
    this.syncVehicles(view);
    this.syncSpikes(view);
    this.syncBeacons(view);
  }

  private myId() {
    return this.conn.me;
  }

  private syncPlace(view: HeistView) {
    const place = view.me?.place;
    const running = view.phase.phase === 'running';
    if (!place || !running || view.me?.arrested) {
      this.car = null;
      this.carId = null;
      this.foot = null;
      return;
    }
    if (place.at === 'seat' && place.driver) {
      if (this.carId !== place.vehicle || !this.car) {
        const v = view.vehicles.find((x) => x.id === place.vehicle);
        if (v) {
          this.car = newCar(v.x, v.z, v.h, this.world.groundAt(v.x, v.z));
          this.carId = v.id;
        }
      }
      this.foot = null;
    } else if (place.at === 'seat') {
      this.car = null;
      this.carId = place.vehicle;
      this.foot = null;
    } else {
      if (!this.foot) {
        const anchor = this.carId !== null ? (this.dyn.get(this.carId) ?? null) : null;
        const p = view.players.find((x) => x.id === this.myId());
        void p;
        const x = anchor ? anchor.x + Math.cos(anchor.h) * -2.4 : this.hud.x;
        const z = anchor ? anchor.z + Math.sin(anchor.h) * -2.4 : this.hud.z;
        this.foot = { x, z, y: this.world.groundAt(x, z), h: anchor?.h ?? 0, s: 0 };
        this.camYaw = this.foot.h;
      }
      this.car = null;
      this.carId = null;
    }
  }

  private rebuildWorld(w: HeistWorld) {
    this.worldVersion = w.version;
    for (const m of this.fleets.values()) this.world.scene.remove(m);
    this.fleets.clear();
    this.parkedSlot.clear();
    this.parked = w.parked;
    const byClass = new Map<Class, HeistWorld['parked']>();
    for (const p of w.parked) byClass.set(p.class, [...(byClass.get(p.class) ?? []), p]);
    const dummy = new THREE.Object3D();
    const colour = new THREE.Color();
    for (const [cls, list] of byClass) {
      const fleet = parkedFleet(cls, list.length);
      list.forEach((p, i) => {
        dummy.position.set(p.x, this.world.groundAt(p.x, p.z), p.z);
        dummy.rotation.set(0, -p.h, 0);
        dummy.updateMatrix();
        fleet.setMatrixAt(i, dummy.matrix);
        fleet.setColorAt(i, colour.set(COLOURS[p.colour]));
        this.parkedSlot.set(p.id, { cls, i });
      });
      fleet.instanceMatrix.needsUpdate = true;
      if (fleet.instanceColor) fleet.instanceColor.needsUpdate = true;
      fleet.computeBoundingSphere();
      fleet.frustumCulled = false;
      this.fleets.set(cls, fleet);
      this.world.scene.add(fleet);
    }
    for (const r of this.readers) this.world.scene.remove(r);
    this.readers = w.readers.map((r) => {
      const g = new THREE.Group();
      g.add(new THREE.Mesh(new THREE.BoxGeometry(0.15, 5, 0.15).translate(0, 2.5, 0), new THREE.MeshLambertMaterial({ color: 0x3a3d42 })));
      g.add(new THREE.Mesh(new THREE.BoxGeometry(0.5, 0.3, 0.6).translate(0, 5, 0), new THREE.MeshLambertMaterial({ color: 0x222428 })));
      g.add(new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.08, 0.08).translate(0, 5.05, -0.32), new THREE.MeshBasicMaterial({ color: 0xff2020 })));
      g.position.set(r.x, this.world.groundAt(r.x, r.z), r.z);
      this.world.scene.add(g);
      return g;
    });
  }

  /** Parked cars still sitting there, for bumping into and for checking up close. */
  private parkedNow() {
    return this.parked;
  }

  private syncVehicles(view: HeistView) {
    const live = new Set<number>();
    for (const v of view.vehicles) {
      if (v.kind === 'parked') continue;
      live.add(v.id);
      let d = this.dyn.get(v.id);
      if (!d) {
        d = this.makeDyn(v.class, v.kind === 'cruiser' ? '#f2f2ee' : COLOURS[v.colour], v.kind);
        d.x = v.x;
        d.z = v.z;
        d.h = v.h;
        d.track.push(performance.now() / 1000, v.x, v.z, v.h, 0);
        this.dyn.set(v.id, d);
      }
      applyDamage(d.model, v.damage);
      const slot = this.parkedSlot.get(v.id);
      if (slot) {
        const fleet = this.fleets.get(slot.cls)!;
        fleet.setMatrixAt(slot.i, new THREE.Matrix4().makeScale(0, 0, 0));
        fleet.instanceMatrix.needsUpdate = true;
        this.parkedSlot.delete(v.id);
        this.parked = this.parked.filter((p) => p.id !== v.id);
      }
    }
    for (const [id, d] of this.dyn) {
      if (!live.has(id)) {
        this.world.scene.remove(d.model.group);
        disposeVehicle(d.model);
        this.dyn.delete(id);
      }
    }
  }

  private makeDyn(cls: Class, colour: string, kind: string): Dyn {
    const model = makeVehicle(cls, colour);
    const beam = new THREE.Mesh(beamGeometry(SHAPES[cls].width), this.beamMat);
    beam.position.z = -SHAPES[cls].length / 2;
    model.group.add(beam);
    let spot: THREE.Mesh | null = null;
    if (cls === 'cruiser') {
      spot = new THREE.Mesh(torchGeometry(70, 0.14), this.torchMat);
      spot.position.set(-0.95, 1.4, -0.6);
      spot.visible = false;
      model.group.add(spot);
    }
    this.world.scene.add(model.group);
    return { model, beam, spot, track: new Track(), x: 0, y: 0, z: 0, h: 0, s: 0, lights: true, siren: false, spotOn: false, kind };
  }

  private makePerson(role: string, name: string): Person {
    const walker = role === 'cop' ? makeWalker('#1c2a4a', '#10182c') : makeWalker('#2b2b2f', '#1a1a1d');
    const torch = new THREE.Mesh(torchGeometry(26, 0.3), this.torchMat);
    torch.position.set(0.25, 1.3, -0.2);
    torch.visible = false;
    walker.group.add(torch);
    this.world.scene.add(walker.group);
    const label = document.createElement('div');
    label.className = 'car-label';
    label.textContent = name;
    label.style.setProperty('--car', role === 'cop' ? '#3a7bff' : '#ff9d2e');
    this.labels.appendChild(label);
    return { walker, torch, track: new Track(), x: 0, y: 0, z: 0, h: 0, s: 0, crouch: false, flash: false, label };
  }

  private syncSpikes(view: HeistView) {
    const live = new Set(view.spikes.map((s) => s.id));
    for (const s of view.spikes) {
      if (this.spikes.has(s.id)) continue;
      const len = Math.hypot(s.b[0] - s.a[0], s.b[1] - s.a[1]);
      const g = new THREE.Group();
      g.add(new THREE.Mesh(new THREE.BoxGeometry(len, 0.06, 0.35), new THREE.MeshLambertMaterial({ color: 0x1b1b1e })));
      for (let i = 0; i < Math.floor(len / 0.4); i++) {
        const tooth = new THREE.Mesh(new THREE.ConeGeometry(0.05, 0.14, 4), new THREE.MeshLambertMaterial({ color: 0xc9b23a }));
        tooth.position.set(-len / 2 + 0.2 + i * 0.4, 0.1, 0);
        g.add(tooth);
      }
      const mx = (s.a[0] + s.b[0]) / 2;
      const mz = (s.a[1] + s.b[1]) / 2;
      g.position.set(mx, this.world.groundAt(mx, mz) + 0.25, mz);
      g.rotation.y = -Math.atan2(s.b[1] - s.a[1], s.b[0] - s.a[0]);
      this.world.scene.add(g);
      this.spikes.set(s.id, g);
    }
    for (const [id, g] of this.spikes) {
      if (!live.has(id)) {
        this.world.scene.remove(g);
        this.spikes.delete(id);
      }
    }
  }

  private syncBeacons(view: HeistView) {
    const colour: Record<string, number> = { mark: 0xffffff, pickup: 0x3fd08f, danger: 0xff5b4a, radio: 0x5aa0ff, witness: 0xffd24a, reader: 0xff7a2a, report: 0xff3b3b, flagged: 0xffd24a, seen: 0xff3b3b, alarm: 0xffb547 };
    const live = new Set(view.pings.map((p) => p.id));
    for (const p of view.pings) {
      let m = this.beacons.get(p.id);
      if (!m) {
        m = new THREE.Mesh(new THREE.CylinderGeometry(0.6, 0.6, 60, 8, 1, true), glowMaterial(colour[p.kind] ?? 0xffffff, 0.5));
        m.position.set(p.x, this.world.groundAt(p.x, p.z) + 30, p.z);
        this.world.scene.add(m);
        this.beacons.set(p.id, m);
      }
      (m.material as THREE.MeshBasicMaterial).opacity = Math.max(0.08, 0.55 * (1 - p.age / 75));
    }
    for (const [id, m] of this.beacons) {
      if (!live.has(id)) {
        this.world.scene.remove(m);
        this.beacons.delete(id);
      }
    }
  }

  private poses(p: HeistPoses) {
    const now = performance.now() / 1000;
    for (const c of p.cars) {
      const d = this.dyn.get(c.id);
      if (!d) continue;
      d.lights = c.lights;
      d.siren = c.siren;
      d.spotOn = c.spot;
      if (c.id === this.carId && this.car) continue;
      d.track.push(now, c.x, c.z, c.h, c.s);
    }
    const aiLive = new Set(p.ai.map((a) => a.id));
    for (const a of p.ai) {
      let d = this.ai.get(a.id);
      if (!d) {
        d = this.makeDyn('cruiser', '#f2f2ee', 'ai');
        this.ai.set(a.id, d);
      }
      d.siren = a.siren;
      d.track.push(now, a.x, a.z, a.h, a.s);
    }
    for (const [id, d] of this.ai) {
      if (!aiLive.has(id)) {
        this.world.scene.remove(d.model.group);
        disposeVehicle(d.model);
        this.ai.delete(id);
      }
    }
    const feet = new Set<string>();
    for (const f of p.feet) {
      if (f.id === this.myId()) continue;
      feet.add(f.id);
      let person = this.people.get(f.id);
      if (!person) {
        const name = this.view?.players.find((x) => x.id === f.id)?.name ?? '';
        person = this.makePerson(f.role, name);
        this.people.set(f.id, person);
      }
      person.crouch = f.stance === 'crouch';
      person.flash = f.flash;
      person.track.push(now, f.x, f.z, f.h, f.s);
    }
    for (const [id, person] of this.people) {
      if (!feet.has(id)) {
        this.world.scene.remove(person.walker.group);
        disposeWalker(person.walker);
        person.label.remove();
        this.people.delete(id);
      }
    }
  }

  private event(e: HeistEvent) {
    const listener = { x: this.hud.x, z: this.hud.z };
    switch (e.event) {
      case 'notice':
        this.onNotice(e.text);
        break;
      case 'alarm': {
        this.alarms.set(e.vehicle, performance.now() / 1000 + 30);
        const v = this.parked.find((p) => p.id === e.vehicle) ?? this.dyn.get(e.vehicle);
        if (v) this.sirens.alarm(v.x, v.z, 30, listener);
        break;
      }
      case 'porch_light':
        this.porch.push({ x: e.x, z: e.z, until: performance.now() / 1000 + 25 });
        if (this.porch.length > 12) this.porch.shift();
        break;
      case 'bark':
        this.sirens.bark(e.x, e.z, listener);
        break;
      case 'spin':
        if (e.vehicle === this.carId && this.car) {
          const c = this.car;
          c.h += e.side * 0.9;
          c.vx += Math.cos(c.h) * e.side * 7;
          c.vz += Math.sin(c.h) * e.side * 7;
        }
        break;
      case 'correction':
        if (e.vehicle !== null && e.vehicle === this.carId && this.car) {
          this.car.x = e.x;
          this.car.z = e.z;
          this.car.h = e.h;
          this.car.vx = 0;
          this.car.vz = 0;
        } else if (e.vehicle === null && this.foot) {
          this.foot = { ...this.foot, x: e.x, z: e.z };
        }
        break;
      case 'ended':
        if (e.outcome === 'delivered') {
          const stolen = this.view?.vehicles.find((v) => v.kind === 'stolen');
          if (stolen) this.cinematic = { x: stolen.x, z: stolen.z, h: stolen.h, t: 0 };
        }
        break;
    }
  }

  private handle(press: Press) {
    const view = this.view;
    const role = view?.me?.role;
    this.sound.wake();
    this.sirens.wake(this.sound.context);
    if (press === 'interact') this.interact();
    else if (press === 'torch') {
      if (this.foot) this.torchOn = !this.torchOn;
      else if (role === 'cop' && this.carId !== null) {
        const v = view?.vehicles.find((x) => x.id === this.carId);
        this.conn.heistSend({ op: 'spotlight', on: !v?.spotlight });
      }
    } else if (press === 'siren' && role === 'cop') {
      const v = view?.vehicles.find((x) => x.id === this.carId);
      this.conn.heistSend({ op: 'siren', on: !v?.siren });
    } else if (press === 'spike' && role === 'cop') this.conn.heistSend({ op: 'spike' });
    else if (press === 'lights') this.headlights = !this.headlights;
    else if (press === 'honk') {
      this.sound.horn(1);
      this.hornHeld = true;
      this.conn.honk();
    }
    this.onPress(press);
  }

  /** What E does right now, if anything. */
  private interaction(): { label: string; run: () => void } | null {
    const view = this.view;
    if (!view || view.phase.phase !== 'running' || view.me?.arrested) return null;
    const role = view.me?.role;
    if (!this.foot) {
      return this.carId !== null
        ? {
            label: 'Get out',
            run: () => {
              this.car = null;
              this.conn.heistSend({ op: 'exit' });
            },
          }
        : null;
    }
    const f = this.foot;
    let best: { d: number; v: VehicleView } | null = null;
    for (const v of view.vehicles) {
      const mine = (role === 'thief' && (v.kind === 'crew' || v.kind === 'stolen')) || (role === 'cop' && v.kind === 'cruiser' && v.owner === this.myId());
      if (!mine) continue;
      const d = this.dyn.get(v.id);
      const at = d ? { x: d.x, z: d.z } : v;
      const dist = Math.hypot(at.x - f.x, at.z - f.z);
      if (dist < 4.5 && (!best || dist < best.d)) best = { d: dist, v };
    }
    if (best) {
      const v = best.v;
      const label = v.driver ? 'Ride along' : 'Drive';
      return { label, run: () => this.conn.heistSend({ op: 'enter', vehicle: v.id, driver: !v.driver }) };
    }
    if (role === 'thief' && !view.take) {
      const near = this.parkedNow().find((p) => Math.hypot(p.x - f.x, p.z - f.z) < 3.2);
      if (near) return { label: 'Try this car', run: () => this.conn.heistSend({ op: 'take_start', vehicle: near.id }) };
    }
    return null;
  }

  private interact() {
    this.interaction()?.run();
  }

  /** What the player can tell about the parked car beside them: class up close, colour only in some light. */
  private inspect(): HeistHud['inspect'] {
    const f = this.foot;
    const view = this.view;
    if (!f || !view || view.me?.role !== 'thief') return null;
    const near = this.parkedNow()
      .map((p) => ({ p, d: Math.hypot(p.x - f.x, p.z - f.z) }))
      .filter((x) => x.d < 5)
      .sort((a, b) => a.d - b.d)[0];
    if (!near) return null;
    const lit = this.torchOn || this.world.lampDistance(near.p.x, near.p.z, 30) < 14;
    const cls = CLASS_LABEL[near.p.class];
    if (!lit) return { text: `A ${cls}. Too dark to tell the colour`, matches: null };
    const shade = SHADE[near.p.colour];
    const c = view.contract;
    const matches = c ? c.class === near.p.class && c.shade === shade : null;
    return { text: `A ${near.p.colour} ${cls}${matches ? '. This is the one' : ''}`, matches };
  }

  private obstacles() {
    const out: { x: number; z: number }[] = [];
    for (const [id, d] of this.dyn) if (id !== this.carId) out.push({ x: d.x, z: d.z });
    for (const d of this.ai.values()) out.push({ x: d.x, z: d.z });
    for (const p of this.parked) out.push({ x: p.x, z: p.z });
    return out.concat(this.traffic.obstacles());
  }

  private drive(dt: number) {
    const car = this.car;
    if (!car || this.carId === null) return;
    const v = this.view?.vehicles.find((x) => x.id === this.carId);
    const mods = v ? { grip: v.handling.grip, pull: v.handling.pull, top: v.handling.top_speed } : undefined;
    const controls = this.input.read(dt);
    const index = this.world.index;
    const result = step(car, controls, dt, {
      road: (x, z) => index.nearestRoad(x, z, 120),
      walls: (x, z, r, visit) => index.walls(x, z, r, visit),
      ground: (x, z, y) => index.groundAt(x, z, y),
      cars: this.obstacles(),
      assist: false,
      mods,
    });
    const now = performance.now() / 1000;
    if ((result.impact > 3 || result.bumped) && now - this.impactAt > 0.4) {
      this.impactAt = now;
      const fs = forwardSpeed(car);
      this.conn.heistSend({ op: 'impact', vehicle: this.carId, x: 0, z: fs >= 0 ? 1 : -1, speed: Math.max(result.impact, result.bumped ? 5 : 0) });
    }
    const d = this.dyn.get(this.carId);
    if (d) {
      d.x = car.x;
      d.z = car.z;
      d.y = car.y;
      d.h = car.h;
      d.s = forwardSpeed(car);
      d.lights = this.headlights;
    }
    this.sendClock -= dt;
    if (this.sendClock <= 0) {
      this.sendClock = SEND_EVERY;
      this.conn.heistSend({ op: 'vehicle', id: this.carId, pose: { x: car.x, z: car.z, h: car.h, s: forwardSpeed(car), lights: this.headlights, horn: this.hornHeld } });
      this.hornHeld = false;
    }
    this.sound.setEngine(forwardSpeed(car), controls.throttle);
  }

  private walk(dt: number) {
    const f = this.foot;
    if (!f) return;
    const i = this.input;
    const forward = (i.held('KeyW', 'ArrowUp') || i.touch.gas ? 1 : 0) - (i.held('KeyS', 'ArrowDown') || i.touch.brake ? 1 : 0);
    const right = (i.held('KeyD', 'ArrowRight') || i.touch.right ? 1 : 0) - (i.held('KeyA', 'ArrowLeft') || i.touch.left ? 1 : 0);
    const input = { forward, right, run: i.held('ShiftLeft', 'ShiftRight'), crouch: i.held('ControlLeft', 'KeyZ', 'KeyC'), facing: this.camYaw };
    const index = this.world.index;
    this.foot = stepFoot(f, input, dt, { walls: (x, z, r, visit) => index.walls(x, z, r, visit), ground: (x, z) => this.world.groundAt(x, z) });
    // The camera swings round behind a walker who has not dragged it lately.
    if (performance.now() / 1000 - this.dragYawAt > 1.5 && forward > 0 && right === 0) {
      const dh = Math.atan2(Math.sin(this.foot.h - this.camYaw), Math.cos(this.foot.h - this.camYaw));
      this.camYaw += dh * Math.min(1, dt * 1.5);
    }
    if (!this.me) {
      this.me = this.makePerson(this.view?.me?.role ?? 'thief', '');
      this.me.label.remove();
    }
    this.me.walker.group.position.set(this.foot.x, this.foot.y, this.foot.z);
    this.me.walker.group.rotation.y = -this.foot.h;
    this.me.walker.group.visible = true;
    animate(this.me.walker, dt, this.foot.s, input.crouch);
    this.me.torch.visible = this.torchOn;
    this.sendClock -= dt;
    if (this.sendClock <= 0) {
      this.sendClock = SEND_EVERY;
      this.conn.heistSend({ op: 'foot', foot: { x: this.foot.x, z: this.foot.z, h: this.foot.h, s: this.foot.s, stance: stance(input), flash: this.torchOn } });
    }
  }

  private placeDyn(d: Dyn, at: number, mine: boolean, t: number, sirenAt: { at: THREE.Vector3; d: number } | null, listener: THREE.Vector3) {
    if (!mine) {
      const p = d.track.at(at);
      if (p) {
        d.x = p.x;
        d.z = p.z;
        d.h = p.h;
        d.s = p.s;
      }
      d.y += (this.world.groundAt(d.x, d.z, d.y + 1) - d.y) * 0.5;
    }
    d.model.group.position.set(d.x, d.y, d.z);
    d.model.group.rotation.set(0, -d.h, 0);
    const lit = d.lights;
    d.beam.visible = lit;
    for (const head of d.model.heads) head.material.color.set(lit ? 0xffffff : 0x666666);
    d.model.tails.material.color.set(lit ? 0xcc3333 : 0x553333);
    if (d.model.bar) {
      const on = d.siren;
      const phase = Math.floor(t * 6) % 2 === 0;
      d.model.bar.red.color.set(on ? (phase ? 0xff3030 : 0x501010) : 0x401010);
      d.model.bar.blue.color.set(on ? (phase ? 0x102050 : 0x3060ff) : 0x101a40);
    }
    if (d.spot) d.spot.visible = d.spotOn;
    if (d.siren && sirenAt) {
      const dist = Math.hypot(d.x - listener.x, d.z - listener.z);
      if (dist < sirenAt.d) {
        sirenAt.d = dist;
        sirenAt.at.set(d.x, d.y, d.z);
      }
    }
  }

  private placeCamera(dt: number) {
    const cam = this.world.camera;
    const k = 1 - Math.exp(-dt * 5);
    if (this.cinematic) {
      const c = this.cinematic;
      c.t += dt;
      const a = c.t * 0.4;
      const y = this.world.groundAt(c.x, c.z);
      cam.position.set(c.x + Math.cos(a) * 16, y + 7, c.z + Math.sin(a) * 16);
      cam.lookAt(c.x, y + 1.5, c.z);
      if (!this.container) {
        this.container = new THREE.Mesh(new THREE.BoxGeometry(2.9, 2.9, 12.4), new THREE.MeshLambertMaterial({ color: 0x8a3324, side: THREE.DoubleSide }));
        this.world.scene.add(this.container);
      }
      const slide = Math.min(1, c.t / 3);
      this.container.position.set(c.x - Math.sin(c.h) * (14 - slide * 14), y + 1.45, c.z + Math.cos(c.h) * (14 - slide * 14));
      this.container.rotation.y = -c.h;
      return;
    }
    let target: { x: number; y: number; z: number; h: number } | null = null;
    let back = 8.5;
    let up = 3.4;
    if (this.car) target = { x: this.car.x, y: this.car.y, z: this.car.z, h: this.car.h };
    else if (this.carId !== null) {
      const d = this.dyn.get(this.carId);
      if (d) target = { x: d.x, y: d.y, z: d.z, h: this.camYaw };
      back = 9;
      up = 3.2;
    } else if (this.foot) {
      target = { x: this.foot.x, y: this.foot.y + 1.2, z: this.foot.z, h: this.camYaw };
      back = 4.4;
      up = 1.1;
    }
    if (!target) {
      const g = this.view?.gate ?? { x: 0, z: 0 };
      cam.position.lerp(new THREE.Vector3(g.x + 40, this.world.groundAt(g.x, g.z) + 50, g.z + 60), k);
      cam.lookAt(g.x, 0, g.z);
      return;
    }
    const fx = Math.sin(target.h);
    const fz = -Math.cos(target.h);
    const pitch = this.foot ? this.camPitch : 0.25;
    const want = new THREE.Vector3(target.x - fx * back * Math.cos(pitch), target.y + up + back * Math.sin(pitch) * 0.6, target.z - fz * back * Math.cos(pitch));
    this.camPos.lerp(want, k);
    this.camPos.y = Math.max(this.camPos.y, this.world.groundAt(this.camPos.x, this.camPos.z) + 1.0);
    cam.position.copy(this.camPos);
    cam.lookAt(target.x + fx * (this.foot ? 2 : 6), target.y + (this.foot ? 0.4 : 1.2), target.z + fz * (this.foot ? 2 : 6));
  }

  private tick = (time: number) => {
    if (this.disposed) return;
    this.raf = requestAnimationFrame(this.tick);
    const dt = Math.min(0.1, (time - this.last) / 1000);
    this.last = time;
    const t = time / 1000;
    for (const p of this.input.drain()) this.handle(p);
    this.input.spaceTaken = !!this.view?.take?.mine;
    if (this.car) this.drive(dt);
    else if (this.foot) this.walk(dt);
    if (!this.foot && this.me) this.me.walker.group.visible = false;
    const listener = this.world.camera.position.clone();
    const sirenAt = { at: new THREE.Vector3(), d: 160 };
    const at = performance.now() / 1000 - BEHIND;
    for (const [id, d] of this.dyn) this.placeDyn(d, at, id === this.carId && !!this.car, t, sirenAt, listener);
    for (const d of this.ai.values()) this.placeDyn(d, at, false, t, sirenAt, listener);
    this.lights.flash(sirenAt.d < 160 ? sirenAt.at : null, t);
    this.placePeople(at, dt);
    this.localLights();
    this.vehicleEffects(dt);
    this.porchLights(t);
    this.alarmFlashes(t);
    const focus = this.car ?? this.foot ?? (this.carId !== null ? this.dyn.get(this.carId) : null) ?? { x: this.view?.gate.x ?? 0, z: this.view?.gate.z ?? 0 };
    this.traffic.update(dt, focus.x, focus.z);
    this.effects.update(dt);
    this.placeCamera(dt);
    const sources = [...this.dyn.entries()].filter(([, d]) => d.siren).map(([id, d]) => ({ id: `v${id}`, x: d.x, y: d.y, z: d.z, yelp: this.view?.pursuit === 'pursuit' }));
    for (const [id, d] of this.ai) if (d.siren) sources.push({ id, x: d.x, y: d.y, z: d.z, yelp: this.view?.pursuit === 'pursuit' });
    this.sirens.update(dt, { x: listener.x, y: listener.y, z: listener.z, h: this.car?.h ?? this.camYaw }, sources);
    this.world.render(dt, focus.x, focus.z);
    this.frames++;
    this.hudClock += dt;
    if (this.hudClock >= 0.2) {
      const stats = this.world.stats();
      const f = focus as { x: number; z: number; h?: number };
      this.hud = {
        mode: this.car ? 'drive' : this.foot ? 'foot' : this.carId !== null ? 'ride' : 'watch',
        speed: this.car ? forwardSpeed(this.car) : (this.foot?.s ?? 0),
        street: this.world.index.streetName(f.x, f.z, 30),
        prompt: this.interaction()?.label ?? null,
        inspect: this.inspect(),
        torch: this.torchOn,
        lights: this.headlights,
        fps: Math.round(this.frames / this.hudClock),
        calls: stats.calls,
        triangles: stats.triangles,
        x: f.x,
        z: f.z,
        h: this.car?.h ?? this.foot?.h ?? this.camYaw,
      };
      this.frames = 0;
      this.hudClock = 0;
      this.onHud(this.hud);
    }
  };

  private placePeople(at: number, dt: number) {
    const cam = this.world.camera;
    const v = new THREE.Vector3();
    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    for (const p of this.people.values()) {
      const s = p.track.at(at);
      if (s) {
        p.x = s.x;
        p.z = s.z;
        p.h = s.h;
        p.s = s.s;
      }
      p.y = this.world.groundAt(p.x, p.z);
      p.walker.group.position.set(p.x, p.y, p.z);
      p.walker.group.rotation.y = -p.h;
      animate(p.walker, dt, p.s, p.crouch);
      p.torch.visible = p.flash;
      v.set(p.x, p.y + 2.2, p.z).project(cam);
      const visible = v.z < 1 && Math.abs(v.x) < 1.1 && Math.abs(v.y) < 1.1;
      p.label.style.display = visible ? '' : 'none';
      if (visible) p.label.style.transform = `translate(${((v.x + 1) / 2) * width}px, ${((1 - v.y) / 2) * height}px) translate(-50%, -100%)`;
    }
  }

  /** The player's own headlights and torch get the real spot lights. */
  private localLights() {
    const head = this.lights.headlight;
    if (this.car && this.headlights) {
      const c = this.car;
      const fx = Math.sin(c.h);
      const fz = -Math.cos(c.h);
      head.position.set(c.x + fx * 2.2, c.y + 1.0, c.z + fz * 2.2);
      head.target.position.set(c.x + fx * 26, c.y, c.z + fz * 26);
      head.intensity = 70;
    } else head.intensity = 0;
    const torch = this.lights.torch;
    const cruiser = this.carId !== null ? this.view?.vehicles.find((v) => v.id === this.carId && v.kind === 'cruiser') : null;
    if (this.foot && this.torchOn) {
      const f = this.foot;
      torch.position.set(f.x + Math.cos(f.h) * 0.3, f.y + 1.35, f.z + Math.sin(f.h) * 0.3);
      torch.target.position.set(f.x + Math.sin(this.camYaw) * 14, f.y, f.z - Math.cos(this.camYaw) * 14);
      torch.intensity = 45;
      torch.angle = 0.32;
    } else if (cruiser?.spotlight) {
      const d = this.dyn.get(cruiser.id);
      if (d) {
        torch.position.set(d.x, d.y + 1.5, d.z);
        torch.target.position.set(d.x + Math.sin(this.camYaw) * 60, d.y, d.z - Math.cos(this.camYaw) * 60);
        torch.intensity = 140;
        torch.angle = 0.16;
      }
    } else torch.intensity = 0;
  }

  private vehicleEffects(dt: number) {
    for (const v of this.view?.vehicles ?? []) {
      const d = this.dyn.get(v.id);
      if (!d) continue;
      if (v.handling.sparks && Math.abs(d.s) > 2) {
        v.damage.tyres.forEach((tyre, i) => {
          if (tyre !== 'rim') return;
          const s = SHAPES[v.class];
          const side = i % 2 === 0 ? -1 : 1;
          const along = i < 2 ? -s.wheelbase : s.wheelbase;
          const x = d.x + Math.cos(d.h) * side * s.width * 0.45 + Math.sin(d.h) * -along;
          const z = d.z + Math.sin(d.h) * side * s.width * 0.45 - Math.cos(d.h) * -along;
          this.effects.rim(x, d.y, z, d.h, d.s, dt);
        });
      }
      if (v.handling.smoke) {
        const s = SHAPES[v.class];
        this.effects.engine(d.x + Math.sin(d.h) * s.length * 0.35, d.y + 1.1, d.z - Math.cos(d.h) * s.length * 0.35, dt);
      }
    }
  }

  private porchLights(t: number) {
    this.porch = this.porch.filter((p) => p.until > t);
    const dummy = new THREE.Object3D();
    this.porch.forEach((p, i) => {
      dummy.position.set(p.x, this.world.groundAt(p.x, p.z) + 0.35, p.z);
      dummy.updateMatrix();
      this.porchMesh.setMatrixAt(i, dummy.matrix);
    });
    this.porchMesh.count = this.porch.length;
    this.porchMesh.instanceMatrix.needsUpdate = true;
  }

  /** A car alarm flashes its hazards until it gives up. */
  private alarmFlashes(t: number) {
    for (const [id, until] of this.alarms) {
      if (t > until) {
        this.alarms.delete(id);
        continue;
      }
      const d = this.dyn.get(id);
      if (!d) continue;
      const on = Math.floor(t * 3) % 2 === 0;
      for (const head of d.model.heads) head.material.color.set(on ? 0xffb030 : 0x332211);
      d.model.tails.material.color.set(on ? 0xffb030 : 0x332211);
    }
  }

  /** Rough map data for the overlay: where things are. */
  mapData() {
    return { dyn: this.dyn, ai: this.ai, people: this.people, parked: this.parked };
  }

  dispose() {
    this.disposed = true;
    cancelAnimationFrame(this.raf);
    this.resizeObserver.disconnect();
    this.canvas.removeEventListener('pointerdown', this.pointerDown);
    window.removeEventListener('pointermove', this.pointerMove);
    window.removeEventListener('pointerup', this.pointerUp);
    for (const p of this.people.values()) p.label.remove();
    this.traffic.dispose();
    this.sirens.dispose();
    this.input.dispose();
    this.sound.dispose();
    this.glow?.dispose();
    this.world.dispose();
  }
}

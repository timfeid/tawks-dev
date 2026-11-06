/** One player's drive: their car, everyone else's, the camera and the loop that ties them to the world. */

import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { Sound } from './audio';
import { makeCar, disposeCar, type CarModel } from './car';
import { Input, type Press } from './input';
import type { DriveConnection } from './net.svelte';
import { forwardSpeed, headingOf, newCar, step, wrapAngle, type CarState } from './physics';
import type { Place, Pose } from './types';
import { QUALITY, World, type Quality } from './world';

const SEND_EVERY = 0.1;
const INTERPOLATE_BEHIND = 0.12;

type Sample = { t: number; pose: Pose };
type Remote = { id: string; model: CarModel; samples: Sample[]; label: HTMLDivElement; x: number; y: number; z: number; h: number; color: string };

export type CameraMode = 'chase' | 'far' | 'hood';
export type Hud = {
  speed: number;
  heading: number;
  street: string | null;
  offRoad: boolean;
  spawning: boolean;
  fps: number;
  frameMs: number;
  /** The longest gap between frames in the last stats window. */
  worstMs: number;
  gpuMs: number | null;
  calls: number;
  triangles: number;
  tiles: number;
  loading: number;
  lights: boolean;
};

export class Game {
  readonly world: World;
  readonly input: Input;
  readonly sound = new Sound();
  car: CarState | null = null;
  assist = true;
  lightsMode: 'auto' | 'on' | 'off' = 'auto';
  camera: CameraMode = 'chase';
  photo = false;
  hud: Hud = { speed: 0, heading: 0, street: null, offRoad: false, spawning: true, fps: 0, frameMs: 0, worstMs: 0, gpuMs: null, calls: 0, triangles: 0, tiles: 0, loading: 0, lights: false };
  onHud: (hud: Hud) => void = () => {};
  onPress: (press: Press) => void = () => {};

  private mine: CarModel;
  private spot = new THREE.SpotLight(0xfff1d0, 0, 90, 0.55, 0.5, 1.2);
  private remotes = new Map<string, Remote>();
  private colors = new Map<string, { name: string; color: string }>();
  private epoch = -1;
  private spawnAt: { x: number; z: number; h?: number } | null = null;
  private sendClock = 0;
  private lastSent = '';
  private hudClock = 0;
  private frames = 0;
  private frameTime = 0;
  private worst = 0;
  private raf = 0;
  private last = 0;
  private camPos = new THREE.Vector3();
  private camLook = new THREE.Vector3();
  private orbit: OrbitControls | null = null;
  private resizeObserver: ResizeObserver;
  private disposed = false;
  private hour = -1;
  private myColor = '#e8553d';

  constructor(
    private canvas: HTMLCanvasElement,
    private labels: HTMLElement,
    private conn: DriveConnection,
    quality: Quality = matchMedia('(pointer: coarse)').matches ? QUALITY.low : QUALITY.high,
  ) {
    this.world = new World(canvas, quality);
    this.world.terrainUrl = conn.terrainUrl;
    this.input = new Input(window);
    this.mine = makeCar(this.myColor);
    this.mine.group.add(this.spot, this.spot.target);
    this.spot.position.set(0, 1.1, -2.2);
    this.spot.target.position.set(0, 0, -24);
    this.world.scene.add(this.mine.group);
    this.mine.group.visible = false;
    conn.onPoses = (cars) => this.receivePoses(cars);
    conn.onHonk = (id) => this.remoteHonk(id);
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    this.resize();
    this.last = performance.now();
    this.raf = requestAnimationFrame(this.tick);
  }

  private resize() {
    const rect = this.canvas.getBoundingClientRect();
    this.world.resize(Math.max(1, rect.width), Math.max(1, rect.height));
  }

  /** Called whenever the room changes. */
  sync() {
    const room = this.conn.room;
    if (!room) return;
    this.colors = new Map(room.drivers.map((d) => [d.user_id, { name: d.name, color: d.color }]));
    const me = room.drivers.find((d) => d.user_id === this.conn.me);
    if (me && me.color !== this.myColor) {
      this.myColor = me.color;
      (this.mine.group.children[0] as THREE.Mesh<THREE.BufferGeometry, THREE.MeshLambertMaterial>).material.color.set(me.color);
    }
    for (const [id, remote] of this.remotes) {
      const info = this.colors.get(id);
      if (!info) this.removeRemote(id);
      else remote.label.textContent = info.name;
    }
    if (room.hour !== this.hour) {
      this.hour = room.hour;
      this.world.setHour(room.hour);
    }
    if (room.place && room.epoch !== this.epoch) this.moveTo(room.place, room.epoch);
  }

  private moveTo(place: Place, epoch: number) {
    this.epoch = epoch;
    this.world.setOrigin(place);
    this.world.setHour(this.hour);
    for (const id of [...this.remotes.keys()]) this.removeRemote(id);
    this.car = null;
    this.mine.group.visible = false;
    this.spawnAt = { x: 0, z: 0 };
  }

  respawn() {
    if (this.car) this.spawnAt = { x: this.car.x, z: this.car.z, h: this.car.h };
  }

  teleportTo(id: string) {
    const r = this.remotes.get(id);
    if (!r) return;
    this.spawnAt = { x: r.x - Math.sin(r.h) * 12, z: r.z + Math.cos(r.h) * 12, h: r.h };
    this.car = null;
  }

  /**
   * Places the car on a road near the spot once the ground under it has
   * loaded: a wide road with open street ahead beats the very nearest one.
   */
  private trySpawn() {
    const at = this.spawnAt;
    if (!at || !this.world.ready(at.x, at.z)) return;
    const index = this.world.index;
    let pick: { x: number; z: number; h: number; score: number } | null = null;
    const candidates = index.roadsNear(at.x, at.z, 90).slice(0, 40);
    const fallback = candidates.length ? null : index.nearestRoad(at.x, at.z, 700);
    for (const hit of fallback ? [fallback] : candidates) {
      const along = headingOf(hit.dx, hit.dz);
      for (const h of [along, along + Math.PI]) {
        const fx = Math.sin(h);
        const fz = -Math.cos(h);
        const open = index.clearance(hit.px, hit.pz, fx, fz, 40);
        let score = hit.half * 2 + open * 0.6 - hit.dist * 0.15;
        if (at.h !== undefined) score += Math.cos(h - at.h) * 8;
        if (!pick || score > pick.score) pick = { x: hit.px, z: hit.pz, h, score };
      }
    }
    const h = wrapAngle(pick?.h ?? at.h ?? 0);
    // Everyone spawns on the same road, so each driver lines up behind the one before.
    const place = at.h === undefined ? Math.max(0, this.conn.room?.drivers.findIndex((d) => d.user_id === this.conn.me) ?? 0) : 0;
    const x = (pick?.x ?? at.x) - Math.sin(h) * place * 9;
    const z = (pick?.z ?? at.z) + Math.cos(h) * place * 9;
    this.car = newCar(x, z, h, this.world.groundAt(x, z));
    this.spawnAt = null;
    this.mine.group.visible = true;
    this.camPos.set(x - Math.sin(h) * 10, this.car.y + 5, z + Math.cos(h) * 10);
    this.sendClock = 0;
  }

  setPhoto(on: boolean) {
    this.photo = on;
    this.input.enabled = !on;
    if (on && this.car) {
      this.orbit = new OrbitControls(this.world.camera, this.canvas);
      this.orbit.target.set(this.car.x, this.car.y + 1.2, this.car.z);
      this.orbit.maxPolarAngle = Math.PI * 0.49;
      this.orbit.minDistance = 3;
      this.orbit.maxDistance = 400;
      this.orbit.enableDamping = true;
      this.orbit.update();
    } else {
      this.orbit?.dispose();
      this.orbit = null;
    }
  }

  /** The current view as a PNG data address. */
  capture(): string {
    this.world.render(0, this.car?.x ?? 0, this.car?.z ?? 0);
    return this.canvas.toDataURL('image/png');
  }

  get lightsOn() {
    return this.lightsMode === 'on' || (this.lightsMode === 'auto' && this.world.night > 0.35);
  }

  private handle(press: Press) {
    if (press === 'honk') {
      this.sound.wake();
      this.sound.horn(1);
      this.conn.honk();
    } else if (press === 'lights') {
      const on = this.lightsOn;
      this.lightsMode = on ? 'off' : 'on';
    } else if (press === 'respawn') this.respawn();
    else if (press === 'camera') this.camera = this.camera === 'chase' ? 'far' : this.camera === 'far' ? 'hood' : 'chase';
    this.onPress(press);
  }

  private receivePoses(cars: (Pose & { id: string })[]) {
    const now = performance.now() / 1000;
    for (const car of cars) {
      if (car.id === this.conn.me) continue;
      let r = this.remotes.get(car.id);
      if (!r) {
        const info = this.colors.get(car.id);
        if (!info) continue;
        const model = makeCar(info.color);
        this.world.scene.add(model.group);
        const label = document.createElement('div');
        label.className = 'car-label';
        label.textContent = info.name;
        label.style.setProperty('--car', info.color);
        this.labels.appendChild(label);
        r = { id: car.id, model, samples: [], label, x: car.x, y: this.world.groundAt(car.x, car.z), z: car.z, h: car.h, color: info.color };
        this.remotes.set(car.id, r);
      }
      r.samples.push({ t: now, pose: car });
      if (r.samples.length > 12) r.samples.shift();
    }
  }

  private removeRemote(id: string) {
    const r = this.remotes.get(id);
    if (!r) return;
    this.world.scene.remove(r.model.group);
    disposeCar(r.model);
    r.label.remove();
    this.remotes.delete(id);
  }

  private remoteHonk(id: string) {
    const r = this.remotes.get(id);
    const d = r && this.car ? Math.hypot(r.x - this.car.x, r.z - this.car.z) : 0;
    this.sound.horn(r ? Math.max(0, 1 - d / 300) : 0.4);
    if (r) {
      r.label.classList.remove('honk');
      void r.label.offsetWidth;
      r.label.classList.add('honk');
    }
  }

  /** Remote cars drawn a little in the past, between the two poses around that moment. */
  private placeRemotes(now: number) {
    const at = now - INTERPOLATE_BEHIND;
    const v = new THREE.Vector3();
    const cam = this.world.camera;
    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    for (const r of this.remotes.values()) {
      const s = r.samples;
      if (!s.length) continue;
      let a = s[0];
      let b = s[s.length - 1];
      for (let i = 0; i + 1 < s.length; i++) {
        if (s[i].t <= at && s[i + 1].t >= at) {
          a = s[i];
          b = s[i + 1];
          break;
        }
      }
      let x: number, z: number, h: number;
      if (at >= b.t) {
        const ahead = Math.min(0.4, at - b.t);
        x = b.pose.x + Math.sin(b.pose.h) * b.pose.s * ahead;
        z = b.pose.z - Math.cos(b.pose.h) * b.pose.s * ahead;
        h = b.pose.h;
      } else {
        const k = b.t > a.t ? Math.max(0, Math.min(1, (at - a.t) / (b.t - a.t))) : 1;
        x = a.pose.x + (b.pose.x - a.pose.x) * k;
        z = a.pose.z + (b.pose.z - a.pose.z) * k;
        const dh = Math.atan2(Math.sin(b.pose.h - a.pose.h), Math.cos(b.pose.h - a.pose.h));
        h = a.pose.h + dh * k;
      }
      r.x = x;
      r.z = z;
      r.h = h;
      const floor = this.world.groundAt(x, z, r.y + 1);
      r.y += (floor - r.y) * 0.5;
      this.orient(r.model.group, x, r.y, z, h, false);
      const lit = (b.pose.f & 1) === 1;
      r.model.lamps.color.set(lit ? 0xffffff : (b.pose.f & 2) === 2 ? 0xd0d0d0 : 0x8a8a8a);
      v.set(x, r.y + 2.6, z).project(cam);
      const visible = v.z < 1 && Math.abs(v.x) < 1.1 && Math.abs(v.y) < 1.1;
      r.label.style.display = visible && !this.photo ? '' : 'none';
      if (visible) r.label.style.transform = `translate(${((v.x + 1) / 2) * width}px, ${((1 - v.y) / 2) * height}px) translate(-50%, -100%)`;
    }
  }

  /** Sits a car model on the ground, tipped to its slope, or level-ish in the air. */
  private orient(group: THREE.Object3D, x: number, y: number, z: number, h: number, airborne: boolean, vy = 0, speed = 0) {
    group.position.set(x, y, z);
    group.rotation.order = 'YXZ';
    group.rotation.y = -h;
    if (airborne) {
      group.rotation.x = THREE.MathUtils.clamp(Math.atan2(vy, Math.max(4, Math.abs(speed))), -0.5, 0.5);
      group.rotation.z *= 0.9;
      return;
    }
    const fx = Math.sin(h);
    const fz = -Math.cos(h);
    const front = this.world.groundAt(x + fx * 1.5, z + fz * 1.5, y + 1);
    const back = this.world.groundAt(x - fx * 1.5, z - fz * 1.5, y + 1);
    const right = this.world.groundAt(x - fz * 0.9, z + fx * 0.9, y + 1);
    const left = this.world.groundAt(x + fz * 0.9, z - fx * 0.9, y + 1);
    group.rotation.x = Math.atan2(front - back, 3);
    // Roads are cut level across a slope, so the car leans only a little with the hillside.
    group.rotation.z = THREE.MathUtils.clamp(Math.atan2(right - left, 1.8) * 0.4, -0.15, 0.15);
  }

  private placeCamera(dt: number) {
    const cam = this.world.camera;
    const car = this.car;
    if (this.orbit) {
      this.orbit.update();
      return;
    }
    if (!car) {
      const sx = this.spawnAt?.x ?? 0;
      const sz = this.spawnAt?.z ?? 0;
      const sy = this.world.groundAt(sx, sz);
      cam.position.set(sx, sy + 60, sz + 80);
      cam.lookAt(sx, sy, sz);
      return;
    }
    const fx = Math.sin(car.h);
    const fz = -Math.cos(car.h);
    const speed = forwardSpeed(car);
    if (this.camera === 'hood') {
      cam.position.set(car.x + fx * 0.4, car.y + 1.45, car.z + fz * 0.4);
      cam.lookAt(car.x + fx * 20, this.world.groundAt(car.x + fx * 20, car.z + fz * 20, car.y + 3) + 1.2, car.z + fz * 20);
      this.camPos.copy(cam.position);
      return;
    }
    const back = this.camera === 'far' ? 17 : 8.5 + Math.min(3, Math.abs(speed) * 0.05);
    const up = this.camera === 'far' ? 8 : 3.4;
    const want = new THREE.Vector3(car.x - fx * back, car.y + up, car.z - fz * back);
    const k = 1 - Math.exp(-dt * 6);
    this.camPos.lerp(want, k);
    // Never under a hill behind the car.
    this.camPos.y = Math.max(this.camPos.y, this.world.groundAt(this.camPos.x, this.camPos.z) + 1.5);
    const min = back * 0.6;
    const dx = this.camPos.x - car.x;
    const dz = this.camPos.z - car.z;
    const d = Math.hypot(dx, dz);
    if (d < min && d > 0) {
      this.camPos.x = car.x + (dx / d) * min;
      this.camPos.z = car.z + (dz / d) * min;
    }
    this.camLook.set(car.x + fx * 6, car.y + 1.2, car.z + fz * 6);
    cam.position.copy(this.camPos);
    cam.lookAt(this.camLook);
    const fov = 62 + Math.min(10, Math.abs(speed) * 0.25);
    if (Math.abs(cam.fov - fov) > 0.1) {
      cam.fov = fov;
      cam.updateProjectionMatrix();
    }
  }

  private sendPose(dt: number, braking: boolean) {
    const car = this.car;
    if (!car) return;
    this.sendClock -= dt;
    const pose: Pose = {
      x: Math.round(car.x * 100) / 100,
      z: Math.round(car.z * 100) / 100,
      h: Math.round(car.h * 1000) / 1000,
      s: Math.round(forwardSpeed(car) * 10) / 10,
      st: Math.round(car.steer * 100) / 100,
      f: (this.lightsOn ? 1 : 0) | (braking ? 2 : 0),
    };
    const key = JSON.stringify(pose);
    if (this.sendClock > 0 || key === this.lastSent) return;
    this.sendClock = SEND_EVERY;
    this.lastSent = key;
    this.conn.pose(pose);
  }

  private tick = (time: number) => {
    if (this.disposed) return;
    this.raf = requestAnimationFrame(this.tick);
    const started = performance.now();
    this.worst = Math.max(this.worst, time - this.last);
    const dt = Math.min(0.1, (time - this.last) / 1000);
    this.last = time;
    for (const press of this.input.drain()) this.handle(press);
    const controls = this.input.read(dt);
    if (controls.throttle > 0 || controls.steer !== 0) this.sound.wake();
    if (this.spawnAt) this.trySpawn();
    const car = this.car;
    let offRoad = false;
    if (car && !this.photo) {
      const others = [...this.remotes.values()].map((r) => ({ x: r.x, z: r.z }));
      const index = this.world.index;
      const result = step(car, controls, dt, {
        road: (x, z) => index.nearestRoad(x, z, 120),
        walls: (x, z, r, visit) => index.walls(x, z, r, visit),
        ground: (x, z, y) => index.groundAt(x, z, y),
        cars: others,
        assist: this.assist,
      });
      offRoad = result.offRoad;
      this.orient(this.mine.group, car.x, car.y, car.z, car.h, car.airborne, car.vy, forwardSpeed(car));
      const braking = controls.brake > 0 && forwardSpeed(car) > 0.5;
      this.mine.lamps.color.set(this.lightsOn ? 0xffffff : braking ? 0xd0d0d0 : 0x8a8a8a);
      this.spot.intensity = this.lightsOn ? 60 : 0;
      this.sendPose(dt, braking);
      this.sound.setEngine(forwardSpeed(car), controls.throttle);
    }
    const now = performance.now() / 1000;
    this.placeRemotes(now);
    this.placeCamera(dt);
    const focusX = car?.x ?? this.spawnAt?.x ?? 0;
    const focusZ = car?.z ?? this.spawnAt?.z ?? 0;
    this.world.render(dt, focusX, focusZ);

    this.frames++;
    this.frameTime += performance.now() - started;
    this.hudClock += dt;
    if (this.hudClock >= 0.2) {
      const stats = this.world.stats();
      this.hud = {
        speed: car ? forwardSpeed(car) : 0,
        heading: car?.h ?? 0,
        street: car ? this.world.index.streetName(car.x, car.z, 30) : null,
        offRoad,
        spawning: !car,
        fps: Math.round(this.frames / this.hudClock),
        frameMs: Math.round((this.frameTime / this.frames) * 10) / 10,
        worstMs: Math.round(this.worst),
        gpuMs: this.world.gpuMs,
        calls: stats.calls,
        triangles: stats.triangles,
        tiles: stats.tiles,
        loading: stats.loading,
        lights: this.lightsOn,
      };
      this.frames = 0;
      this.frameTime = 0;
      this.worst = 0;
      this.hudClock = 0;
      this.onHud(this.hud);
    }
  };

  /** Roads, names and cars around the player, heading up, for the minimap. */
  drawMinimap(ctx: CanvasRenderingContext2D, size: number, range: number) {
    const car = this.car;
    const dpr = ctx.canvas.width / size;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, size, size);
    if (!car) return;
    const scale = size / 2 / range;
    ctx.save();
    ctx.beginPath();
    ctx.arc(size / 2, size / 2, size / 2, 0, Math.PI * 2);
    ctx.clip();
    ctx.fillStyle = 'rgba(16, 20, 26, 0.82)';
    ctx.fillRect(0, 0, size, size);
    ctx.translate(size / 2, size / 2);
    ctx.rotate(-car.h);
    ctx.scale(scale, scale);
    ctx.translate(-car.x, -car.z);
    ctx.lineCap = 'round';
    ctx.strokeStyle = 'rgba(214, 220, 228, 0.9)';
    const index = this.world.index;
    index.forEachRoad(car.x, car.z, range * 1.45, (s, o, half) => {
      ctx.lineWidth = Math.max(half * 1.6, 2 / scale);
      ctx.beginPath();
      ctx.moveTo(s[o], s[o + 1]);
      ctx.lineTo(s[o + 2], s[o + 3]);
      ctx.stroke();
    });
    for (const r of this.remotes.values()) {
      ctx.fillStyle = r.color;
      ctx.beginPath();
      ctx.arc(r.x, r.z, 5 / scale, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();

    const best = new Map<string, { len: number; x: number; z: number; a: number }>();
    index.forEachName(car.x, car.z, range, (name, s, o) => {
      const mx = (s[o] + s[o + 2]) / 2;
      const mz = (s[o + 1] + s[o + 3]) / 2;
      if (Math.hypot(mx - car.x, mz - car.z) > range * 0.85) return;
      const len = Math.hypot(s[o + 2] - s[o], s[o + 3] - s[o + 1]);
      const prev = best.get(name);
      if (!prev || len > prev.len) best.set(name, { len, x: mx, z: mz, a: Math.atan2(s[o + 3] - s[o + 1], s[o + 2] - s[o]) });
    });
    ctx.save();
    ctx.font = '600 10px Inter, system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const cos = Math.cos(-car.h);
    const sin = Math.sin(-car.h);
    let drawn = 0;
    for (const [name, l] of [...best.entries()].sort((a, b) => b[1].len - a[1].len)) {
      if (drawn++ >= 7) break;
      const dx = (l.x - car.x) * scale;
      const dz = (l.z - car.z) * scale;
      const sx = size / 2 + dx * cos - dz * sin;
      const sy = size / 2 + dx * sin + dz * cos;
      let a = l.a - car.h;
      a = Math.atan2(Math.sin(a), Math.cos(a));
      if (a > Math.PI / 2) a -= Math.PI;
      if (a < -Math.PI / 2) a += Math.PI;
      ctx.save();
      ctx.translate(sx, sy);
      ctx.rotate(a);
      ctx.lineWidth = 3;
      ctx.strokeStyle = 'rgba(16, 20, 26, 0.9)';
      ctx.strokeText(name, 0, 0);
      ctx.fillStyle = '#f3e9c6';
      ctx.fillText(name, 0, 0);
      ctx.restore();
    }
    ctx.restore();

    ctx.save();
    ctx.translate(size / 2, size / 2);
    ctx.fillStyle = this.myColor;
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(0, -8);
    ctx.lineTo(6, 6);
    ctx.lineTo(0, 3);
    ctx.lineTo(-6, 6);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  /** Driver ids with their latest spot, for the driver list. */
  nearby(): { id: string; distance: number }[] {
    const car = this.car;
    return [...this.remotes.values()].map((r) => ({ id: r.id, distance: car ? Math.hypot(r.x - car.x, r.z - car.z) : 0 }));
  }

  /** Where the player is, for biasing searches toward it. */
  here(): [number, number] | null {
    if (!this.car || !this.world.frame) return null;
    const p = this.world.frame.toLatLon(this.car.x, this.car.z);
    return [p.lat, p.lon];
  }

  dispose() {
    this.disposed = true;
    cancelAnimationFrame(this.raf);
    this.resizeObserver.disconnect();
    this.orbit?.dispose();
    for (const id of [...this.remotes.keys()]) this.removeRemote(id);
    this.input.dispose();
    this.sound.dispose();
    this.world.dispose();
  }
}

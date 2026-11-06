/**
 * Background traffic: a few cars that keep to the right of the road, slow
 * for junctions and turn at random. Each player sees their own, so it costs
 * the server nothing; it gives a stolen car something to blend in with.
 */

import * as THREE from 'three';
import type { RoadIndex } from '../lib/roads';
import { beamGeometry, beamMaterial } from './night';
import { COLOURS, type Class, type Colour } from './protocol';
import { makeVehicle, SHAPES, type VehicleModel } from './vehicles';

type Car = {
  model: VehicleModel;
  beam: THREE.Mesh;
  x: number;
  z: number;
  dx: number;
  dz: number;
  /** End of the current stretch. */
  ex: number;
  ez: number;
  half: number;
  speed: number;
  wait: number;
};

const CLASSES: Class[] = ['sedan', 'sedan', 'suv', 'hatch', 'pickup'];
const PAINTS = Object.keys(COLOURS) as Colour[];

export class Traffic {
  cars: Car[] = [];
  private beamMat = beamMaterial();

  constructor(
    private scene: THREE.Scene,
    private index: RoadIndex,
    private count: number,
    private ground: (x: number, z: number) => number,
  ) {}

  private spawn(cx: number, cz: number): Car | null {
    const a = Math.random() * Math.PI * 2;
    const r = 180 + Math.random() * 260;
    const hits = this.index.roadsNear(cx + Math.cos(a) * r, cz + Math.sin(a) * r, 60);
    const hit = hits.find((h) => h.half >= 3.4);
    if (!hit) return null;
    const dir = Math.random() < 0.5 ? 1 : -1;
    const kind = CLASSES[Math.floor(Math.random() * CLASSES.length)];
    const model = makeVehicle(kind, COLOURS[PAINTS[Math.floor(Math.random() * PAINTS.length)]]);
    model.heads.forEach((h) => h.material.color.set(0xffffff));
    const beam = new THREE.Mesh(beamGeometry(SHAPES[kind].width), this.beamMat);
    model.group.add(beam);
    this.scene.add(model.group);
    const dx = hit.dx * dir;
    const dz = hit.dz * dir;
    return { model, beam, x: hit.px, z: hit.pz, dx, dz, ex: hit.px + dx * 40, ez: hit.pz + dz * 40, half: hit.half, speed: 8 + hit.half, wait: 0 };
  }

  private turn(car: Car) {
    const options = this.index.roadsNear(car.ex, car.ez, 4).filter((h) => h.half >= 3.4);
    const junction = options.length >= 2;
    const pick = options
      .map((h) => {
        const forward = h.dx * car.dx + h.dz * car.dz;
        return { h, dir: forward >= 0 ? 1 : -1, score: Math.abs(forward) + Math.random() * 0.9 };
      })
      .filter((o) => o.h.dx * car.dx * o.dir + o.h.dz * car.dz * o.dir > -0.5)
      .sort((a, b) => b.score - a.score)[0];
    if (!pick) {
      car.dx = -car.dx;
      car.dz = -car.dz;
    } else {
      car.dx = pick.h.dx * pick.dir;
      car.dz = pick.h.dz * pick.dir;
      car.half = pick.h.half;
    }
    car.ex = car.x + car.dx * 35;
    car.ez = car.z + car.dz * 35;
    if (junction && Math.random() < 0.5) car.wait = 0.8 + Math.random() * 1.4;
  }

  update(dt: number, cx: number, cz: number) {
    this.cars = this.cars.filter((c) => {
      if (Math.hypot(c.x - cx, c.z - cz) < 560) return true;
      this.scene.remove(c.model.group);
      return false;
    });
    if (this.cars.length < this.count && Math.random() < dt * 3) {
      const c = this.spawn(cx, cz);
      if (c) this.cars.push(c);
    }
    for (const c of this.cars) {
      if (c.wait > 0) {
        c.wait -= dt;
      } else {
        const step = Math.min(c.speed * dt, Math.hypot(c.ex - c.x, c.ez - c.z));
        c.x += c.dx * step;
        c.z += c.dz * step;
        if (Math.hypot(c.ex - c.x, c.ez - c.z) < 0.5) this.turn(c);
        const road = this.index.nearestRoad(c.x, c.z, 20);
        if (road) {
          // Keep to the right of the centre line.
          const lane = road.half * 0.45;
          const tx = road.px - c.dz * lane;
          const tz = road.pz + c.dx * lane;
          c.x += (tx - c.x) * Math.min(1, dt * 3);
          c.z += (tz - c.z) * Math.min(1, dt * 3);
        }
      }
      const h = Math.atan2(c.dx, -c.dz);
      c.model.group.position.set(c.x, this.ground(c.x, c.z), c.z);
      c.model.group.rotation.y = -h;
    }
  }

  /** Where the traffic is, for bumping into. */
  obstacles(): { x: number; z: number }[] {
    return this.cars.map((c) => ({ x: c.x, z: c.z }));
  }

  dispose() {
    for (const c of this.cars) this.scene.remove(c.model.group);
    this.cars = [];
  }
}

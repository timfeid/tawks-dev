/** A plain low-poly walker with swinging limbs, for thieves and cops on foot. */

import * as THREE from 'three';

export type Walker = {
  group: THREE.Group;
  body: THREE.Group;
  legs: [THREE.Mesh, THREE.Mesh];
  arms: [THREE.Mesh, THREE.Mesh];
  phase: number;
  materials: THREE.Material[];
};

const limb = (w: number, h: number, d: number) => new THREE.BoxGeometry(w, h, d).translate(0, -h / 2, 0);
const shared = {
  torso: new THREE.BoxGeometry(0.46, 0.62, 0.26).translate(0, 1.18, 0),
  head: new THREE.BoxGeometry(0.24, 0.26, 0.24).translate(0, 1.64, 0),
  leg: limb(0.17, 0.84, 0.18),
  arm: limb(0.12, 0.6, 0.13),
};

export function makeWalker(clothes: string, accent: string, skin = '#c99b7b'): Walker {
  const group = new THREE.Group();
  const body = new THREE.Group();
  group.add(body);
  const cloth = new THREE.MeshLambertMaterial({ color: clothes });
  const trim = new THREE.MeshLambertMaterial({ color: accent });
  const face = new THREE.MeshLambertMaterial({ color: skin });
  body.add(new THREE.Mesh(shared.torso, cloth), new THREE.Mesh(shared.head, face));
  const leg = (x: number) => {
    const m = new THREE.Mesh(shared.leg, trim);
    m.position.set(x, 0.86, 0);
    body.add(m);
    return m;
  };
  const arm = (x: number) => {
    const m = new THREE.Mesh(shared.arm, cloth);
    m.position.set(x, 1.46, 0);
    body.add(m);
    return m;
  };
  return { group, body, legs: [leg(-0.11), leg(0.11)], arms: [arm(-0.31), arm(0.31)], phase: 0, materials: [cloth, trim, face] };
}

/** Swings the limbs for the speed and crouches when asked. */
export function animate(w: Walker, dt: number, speed: number, crouch: boolean) {
  const run = speed > 3.2;
  w.phase += dt * (2 + speed * (run ? 2.1 : 2.6));
  const swing = Math.min(1, speed / 2) * (run ? 0.9 : 0.55);
  const a = Math.sin(w.phase) * swing;
  w.legs[0].rotation.x = a;
  w.legs[1].rotation.x = -a;
  w.arms[0].rotation.x = -a * 0.9;
  w.arms[1].rotation.x = a * 0.9;
  const lower = crouch ? 0.38 : 0;
  w.body.position.y += (-lower - w.body.position.y) * Math.min(1, dt * 10);
  w.body.rotation.x = crouch ? 0.35 : run ? 0.12 : 0;
  w.legs[0].position.y = 0.86;
  w.legs[1].position.y = 0.86;
  if (crouch) {
    w.legs[0].rotation.x -= 0.6;
    w.legs[1].rotation.x -= 0.6;
  }
}

export function disposeWalker(w: Walker) {
  for (const m of w.materials) m.dispose();
}

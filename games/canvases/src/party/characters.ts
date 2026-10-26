import * as THREE from 'three';
import type { CharacterDef, Feature } from './protocol';

export type Pose = 'idle' | 'ride' | 'cheer' | 'sad' | 'fall';

export type CharacterRig = {
  group: THREE.Group;
  /** Advance the animation. `speed` is ground speed in units per second. */
  animate: (pose: Pose, time: number, speed: number) => void;
  dispose: () => void;
};

let toonRamp: THREE.DataTexture | null = null;

function ramp() {
  if (!toonRamp) {
    toonRamp = new THREE.DataTexture(new Uint8Array([90, 90, 90, 255, 175, 175, 175, 255, 255, 255, 255, 255]), 3, 1, THREE.RGBAFormat);
    toonRamp.minFilter = THREE.NearestFilter;
    toonRamp.magFilter = THREE.NearestFilter;
    toonRamp.needsUpdate = true;
  }
  return toonRamp;
}

const toon = (color: string) => new THREE.MeshToonMaterial({ color, gradientMap: ramp() });

/**
 * Builds a party character from its roster entry: a round body, big eyes and
 * a signature headpiece. Feet rest at y = 0 and the character faces +z.
 */
export function buildCharacter(def: CharacterDef): CharacterRig {
  const group = new THREE.Group();
  group.name = `character-${def.id}`;
  const materials = {
    body: toon(def.body),
    accent: toon(def.accent),
    belly: toon(def.belly),
    white: toon('#ffffff'),
    dark: new THREE.MeshBasicMaterial({ color: '#1b1b24' }),
    cheek: new THREE.MeshBasicMaterial({ color: '#ff8fa3', transparent: true, opacity: 0.75 }),
  };
  const geometries: THREE.BufferGeometry[] = [];
  const extraMaterials: THREE.Material[] = [];
  const glowMaterial = (color: string) => {
    const material = new THREE.MeshBasicMaterial({ color });
    extraMaterials.push(material);
    return material;
  };
  const mesh = (geometry: THREE.BufferGeometry, material: THREE.Material, parent: THREE.Object3D = group) => {
    geometries.push(geometry);
    const m = new THREE.Mesh(geometry, material);
    m.castShadow = true;
    parent.add(m);
    return m;
  };

  const legs = [-1, 1].map((side) => {
    const leg = mesh(new THREE.CapsuleGeometry(0.1, 0.12, 4, 10), materials.accent);
    leg.position.set(side * 0.17, 0.16, 0);
    return leg;
  });

  const torso = new THREE.Group();
  group.add(torso);
  const body = mesh(new THREE.SphereGeometry(0.42, 24, 18), materials.body, torso);
  body.scale.set(1, 1.08, 0.95);
  body.position.y = 0.55;
  const belly = mesh(new THREE.SphereGeometry(0.3, 20, 14), materials.belly, torso);
  belly.scale.set(1, 1.1, 0.45);
  belly.position.set(0, 0.5, 0.27);

  const head = new THREE.Group();
  head.position.y = 1.08;
  torso.add(head);
  mesh(new THREE.SphereGeometry(0.4, 24, 18), materials.body, head);
  for (const side of [-1, 1]) {
    const eye = mesh(new THREE.SphereGeometry(0.1, 14, 10), materials.white, head);
    eye.scale.set(0.9, 1.25, 0.55);
    eye.position.set(side * 0.14, 0.05, 0.33);
    const pupil = mesh(new THREE.SphereGeometry(0.05, 10, 8), materials.dark, head);
    pupil.position.set(side * 0.135, 0.04, 0.385);
    const cheek = mesh(new THREE.CircleGeometry(0.06, 14), materials.cheek, head);
    cheek.position.set(side * 0.24, -0.1, 0.33);
    cheek.rotation.y = side * 0.55;
    cheek.castShadow = false;
  }
  const smile = mesh(new THREE.TorusGeometry(0.07, 0.016, 6, 16, Math.PI), materials.dark, head);
  smile.position.set(0, -0.08, 0.385);
  smile.rotation.z = Math.PI;

  const arms = [-1, 1].map((side) => {
    const shoulder = new THREE.Group();
    shoulder.position.set(side * 0.38, 0.72, 0);
    torso.add(shoulder);
    const arm = mesh(new THREE.CapsuleGeometry(0.075, 0.2, 4, 10), materials.body, shoulder);
    arm.position.y = -0.14;
    const hand = mesh(new THREE.SphereGeometry(0.085, 12, 10), materials.white, shoulder);
    hand.position.y = -0.3;
    shoulder.rotation.z = side * 0.35;
    return shoulder;
  });

  addFeature(def.feature, head, mesh, materials, glowMaterial);

  const animate = (pose: Pose, time: number, speed: number) => {
    const bounce = Math.sin(time * 6);
    torso.position.y = 0;
    torso.rotation.set(0, 0, 0);
    head.rotation.set(0, 0, 0);
    legs.forEach((leg) => (leg.position.y = 0.16));
    const [left, right] = arms;
    switch (pose) {
      case 'idle':
        torso.position.y = Math.abs(Math.sin(time * 2.2)) * 0.04;
        left.rotation.set(0, 0, -0.35 - Math.sin(time * 2.2) * 0.08);
        right.rotation.set(0, 0, 0.35 + Math.sin(time * 2.2) * 0.08);
        head.rotation.z = Math.sin(time * 1.1) * 0.06;
        break;
      case 'ride': {
        // Balance on the ball: lean into the roll and pinwheel the arms.
        const effort = Math.min(1, speed / 6);
        torso.rotation.x = effort * 0.3;
        torso.position.y = Math.abs(bounce) * 0.03 * effort;
        left.rotation.set(Math.sin(time * 9) * 0.6 * effort, 0, -1.1 + Math.sin(time * 7) * 0.25);
        right.rotation.set(-Math.sin(time * 9) * 0.6 * effort, 0, 1.1 - Math.sin(time * 7) * 0.25);
        legs[0].position.y = 0.16 + Math.max(0, Math.sin(time * 12)) * 0.05 * effort;
        legs[1].position.y = 0.16 + Math.max(0, -Math.sin(time * 12)) * 0.05 * effort;
        break;
      }
      case 'cheer': {
        const hop = Math.abs(Math.sin(time * 7));
        torso.position.y = hop * 0.25;
        left.rotation.set(0, 0, -2.6 + Math.sin(time * 14) * 0.25);
        right.rotation.set(0, 0, 2.6 - Math.sin(time * 14) * 0.25);
        head.rotation.x = -0.15;
        break;
      }
      case 'sad':
        torso.rotation.x = 0.25;
        head.rotation.x = 0.35;
        left.rotation.set(0, 0, -0.12);
        right.rotation.set(0, 0, 0.12);
        break;
      case 'fall':
        torso.rotation.x = -0.4 + Math.sin(time * 18) * 0.1;
        left.rotation.set(0, 0, -2.2 + Math.sin(time * 22) * 0.9);
        right.rotation.set(0, 0, 2.2 - Math.sin(time * 22 + 1) * 0.9);
        legs[0].position.y = 0.16 + Math.sin(time * 20) * 0.08;
        legs[1].position.y = 0.16 - Math.sin(time * 20) * 0.08;
        break;
    }
  };

  const dispose = () => {
    geometries.forEach((g) => g.dispose());
    Object.values(materials).forEach((m) => m.dispose());
    extraMaterials.forEach((m) => m.dispose());
    group.removeFromParent();
  };

  return { group, animate, dispose };
}

type MeshFactory = (geometry: THREE.BufferGeometry, material: THREE.Material, parent?: THREE.Object3D) => THREE.Mesh;
type Materials = Record<'body' | 'accent' | 'belly' | 'white' | 'dark' | 'cheek', THREE.Material>;

function addFeature(feature: Feature, head: THREE.Group, mesh: MeshFactory, m: Materials, glow: (color: string) => THREE.Material) {
  switch (feature) {
    case 'cap': {
      const crown = mesh(new THREE.SphereGeometry(0.41, 24, 12, 0, Math.PI * 2, 0, Math.PI / 2), m.accent, head);
      crown.position.y = 0.07;
      const brim = mesh(new THREE.CylinderGeometry(0.26, 0.26, 0.04, 20, 1, false, -Math.PI / 2, Math.PI), m.accent, head);
      brim.position.set(0, 0.1, 0.24);
      const button = mesh(new THREE.SphereGeometry(0.05, 10, 8), m.white, head);
      button.position.y = 0.48;
      break;
    }
    case 'leaf': {
      const stem = mesh(new THREE.CylinderGeometry(0.025, 0.03, 0.22, 8), m.accent, head);
      stem.position.set(0, 0.47, 0);
      for (const side of [-1, 1]) {
        const leaf = mesh(new THREE.SphereGeometry(0.16, 14, 10), m.accent, head);
        leaf.scale.set(1, 0.3, 0.55);
        leaf.position.set(side * 0.15, 0.58, 0);
        leaf.rotation.z = side * -0.5;
      }
      break;
    }
    case 'antenna':
      for (const side of [-1, 1]) {
        const stalk = mesh(new THREE.CylinderGeometry(0.02, 0.02, 0.3, 8), m.dark, head);
        stalk.position.set(side * 0.14, 0.48, 0);
        stalk.rotation.z = side * -0.35;
        const tip = mesh(new THREE.SphereGeometry(0.07, 12, 10), m.accent, head);
        tip.position.set(side * 0.2, 0.63, 0);
      }
      break;
    case 'long_ears':
      for (const side of [-1, 1]) {
        const ear = mesh(new THREE.CapsuleGeometry(0.09, 0.4, 4, 12), m.body, head);
        ear.position.set(side * 0.17, 0.55, -0.02);
        ear.rotation.z = side * -0.2;
        const inner = mesh(new THREE.CapsuleGeometry(0.05, 0.3, 4, 10), m.accent, head);
        inner.position.set(side * 0.17, 0.56, 0.05);
        inner.rotation.z = side * -0.2;
        inner.castShadow = false;
      }
      break;
    case 'round_ears':
      for (const side of [-1, 1]) {
        const ear = mesh(new THREE.SphereGeometry(0.15, 16, 12), m.body, head);
        ear.scale.z = 0.5;
        ear.position.set(side * 0.3, 0.3, -0.02);
        const inner = mesh(new THREE.SphereGeometry(0.09, 12, 10), m.accent, head);
        inner.scale.z = 0.4;
        inner.position.set(side * 0.3, 0.3, 0.04);
        inner.castShadow = false;
      }
      break;
    case 'horns':
      for (const side of [-1, 1]) {
        const horn = mesh(new THREE.ConeGeometry(0.08, 0.26, 12), m.accent, head);
        horn.position.set(side * 0.2, 0.4, 0);
        horn.rotation.z = side * -0.45;
      }
      break;
    case 'beanie': {
      const cap = mesh(new THREE.SphereGeometry(0.42, 24, 12, 0, Math.PI * 2, 0, Math.PI / 2.1), m.accent, head);
      cap.position.y = 0.05;
      const band = mesh(new THREE.TorusGeometry(0.39, 0.06, 10, 28), m.white, head);
      band.rotation.x = Math.PI / 2;
      band.position.y = 0.12;
      const pom = mesh(new THREE.SphereGeometry(0.1, 12, 10), m.white, head);
      pom.position.y = 0.5;
      break;
    }
    case 'visor': {
      const visor = mesh(new THREE.CylinderGeometry(0.42, 0.42, 0.16, 24, 1, false, -Math.PI * 0.42, Math.PI * 0.84), m.accent, head);
      visor.position.y = 0.06;
      const light = mesh(new THREE.BoxGeometry(0.34, 0.025, 0.02), glow('#7df9ff'), head);
      light.position.set(0, 0.06, 0.425);
      const fin = mesh(new THREE.BoxGeometry(0.05, 0.16, 0.3), m.accent, head);
      fin.position.set(0, 0.42, -0.05);
      break;
    }
  }
}

import * as THREE from 'three';

export const FACE_REACTIONS = ['wink', 'big-laugh', 'tongue-out', 'starry-eyes', 'heart-eyes', 'smug-grin', 'shocked', 'sleepy', 'angry-steam', 'blush'] as const;
export type FaceReaction = (typeof FACE_REACTIONS)[number];
export const isFaceReaction = (value: unknown): value is FaceReaction => FACE_REACTIONS.includes(value as FaceReaction);
export type Mood = 'idle' | 'cheer' | 'sad' | 'fall';

/** Seconds a reaction takes to come in, stay and ease back to the normal face. */
export const REACTION_IN = 0.18, REACTION_HOLD = 1.7, REACTION_OUT = 0.45;
export const REACTION_LENGTH = REACTION_IN + REACTION_HOLD + REACTION_OUT;

/** How far into its reaction the face is, 0 to 1, `elapsed` seconds after it started. A held reaction stays on. */
export function reactionWeight(elapsed: number, hold = false) {
  if (hold) return 1;
  if (!(elapsed >= 0) || elapsed >= REACTION_LENGTH) return 0;
  const t = elapsed < REACTION_IN ? elapsed / REACTION_IN : elapsed < REACTION_IN + REACTION_HOLD ? 1 : 1 - (elapsed - REACTION_IN - REACTION_HOLD) / REACTION_OUT;
  return t * t * (3 - 2 * t);
}

export type MeshFactory = (geometry: THREE.BufferGeometry, material: THREE.Material, parent?: THREE.Object3D) => THREE.Mesh;
export type FaceMaterials = Record<'body' | 'white' | 'dark' | 'ink' | 'cheek' | 'mouth', THREE.Material>;
/** A plain colour for a reaction's parts, released with the character. */
export type Paint = (color: string, opacity?: number) => THREE.MeshBasicMaterial;
export type HeadMotion = { x: number; z: number };
export type Face = {
  /** The face for a pose's mood, with `reaction` drawn over it at `weight` (0 shows the mood alone). Returns a head turn to add. */
  show: (mood: Mood, reaction: FaceReaction | null, weight: number, time: number) => HeadMotion;
};

/** Moves a flat shape to (x, y) on the face and bends it onto the round head, just proud of its faceted surface. */
export function onHead(geometry: THREE.BufferGeometry, x: number, y: number, radius = 0.403) {
  const position = geometry.getAttribute('position');
  for (let i = 0; i < position.count; i++) {
    const px = position.getX(i) + x, py = position.getY(i) + y;
    position.setXYZ(i, px, py, Math.sqrt(radius * radius - px * px - py * py));
  }
  return geometry;
}

const EYE = { x: 0.14, y: 0.05, z: 0.33 }, EYE_SCALE = new THREE.Vector3(0.9, 1.25, 0.55);
const PUPIL = { x: 0.135, y: 0.04, z: 0.385 };
const SIDES = [-1, 1] as const;
type Side = (typeof SIDES)[number];
type Brow = { y: number; tilt: number };
type Built = { group: THREE.Group; animate?: (time: number, weight: number) => void };
type Spec = {
  /** Sides whose open eye the reaction replaces. */
  closes?: readonly Side[];
  /** A mouth the face already has, or none to leave it to the reaction's own parts. */
  mouth?: 'smile' | 'grin' | 'frown' | null;
  brows?: (side: Side) => Brow;
  white?: number;
  pupil?: { scale?: number; x?: number; y?: number };
  hidesCheeks?: boolean;
  head?: (time: number) => HeadMotion;
  build: (kit: Kit) => Built;
};
type Kit = { mesh: MeshFactory; m: FaceMaterials; paint: Paint };

export function buildFace(face: THREE.Group, m: FaceMaterials, mesh: MeshFactory, paint: Paint): Face {
  mesh(new THREE.SphereGeometry(0.4, 24, 18), m.body, face);
  const eyes = SIDES.map((side) => {
    const white = mesh(new THREE.SphereGeometry(0.1, 14, 10), m.white, face);
    white.scale.copy(EYE_SCALE);
    white.position.set(side * EYE.x, EYE.y, EYE.z);
    const pupil = mesh(new THREE.SphereGeometry(0.05, 10, 8), m.dark, face);
    pupil.position.set(side * PUPIL.x, PUPIL.y, PUPIL.z);
    const cheek = mesh(new THREE.CircleGeometry(0.06, 14), m.cheek, face);
    cheek.position.set(side * 0.24, -0.1, 0.33);
    cheek.rotation.y = side * 0.55;
    cheek.castShadow = false;
    return { side, white, pupil, cheek };
  });
  const smile = mesh(new THREE.TorusGeometry(0.07, 0.016, 6, 16, Math.PI), m.ink, face);
  smile.position.set(0, -0.08, 0.385);
  smile.rotation.z = Math.PI;
  const frown = mesh(new THREE.TorusGeometry(0.07, 0.016, 6, 16, Math.PI), m.ink, face);
  frown.position.set(0, -0.15, 0.372);
  frown.rotation.x = 0.3;
  const grin = mesh(onHead(new THREE.CircleGeometry(0.09, 20, Math.PI, Math.PI), 0, -0.06), m.mouth, face);
  const gasp = mesh(onHead(new THREE.CircleGeometry(0.042, 16).scale(1, 1.3, 1), 0, -0.11), m.mouth, face);
  const mouths = { smile, frown, grin, gasp };
  const brows = SIDES.map((side) => {
    const brow = new THREE.Group();
    brow.position.set(side * 0.14, 0.2, 0.33);
    brow.rotation.y = side * 0.45;
    face.add(brow);
    const stroke = mesh(new THREE.CapsuleGeometry(0.016, 0.08, 4, 8), m.ink, brow);
    stroke.rotation.z = Math.PI / 2;
    return { brow, stroke, side };
  });
  for (const part of [smile, frown, grin, gasp, ...brows.map(({ stroke }) => stroke)]) part.castShadow = false;

  const moodBrow = (mood: Mood, side: Side): Brow => ({
    y: mood === 'sad' ? 0.19 : 0.23,
    // Positive tilt lifts the brow's +x end; a sad brow lifts its inner end.
    tilt: mood === 'sad' ? -side * 0.45 : mood === 'cheer' ? side * 0.2 : 0,
  });
  const moodMouth = (mood: Mood) => (mood === 'sad' ? frown : mood === 'cheer' ? grin : mood === 'fall' ? gasp : smile);

  const kit: Kit = { mesh, m, paint };
  const reactions = new Map<FaceReaction, Built>();
  const partsOf = (reaction: FaceReaction) => {
    let parts = reactions.get(reaction);
    if (!parts) {
      parts = SPECS[reaction].build(kit);
      parts.group.traverse((part) => (part.castShadow = false));
      face.add(parts.group);
      reactions.set(reaction, parts);
    }
    return parts;
  };

  let shown: string | null = null;
  const result: Face = {
    show(mood, reaction, weight, time) {
      const s = reaction ? THREE.MathUtils.clamp(weight, 0, 1) : 0;
      const key = s > 0 ? null : mood;
      // A settled mood face needs nothing set until it changes.
      if (key !== null && key === shown) return { x: 0, z: 0 };
      shown = key;
      const spec = reaction && s > 0 ? SPECS[reaction] : null;
      for (const [id, parts] of reactions) parts.group.visible = !!spec && id === reaction;
      const swapped = !!spec && s >= 0.5;
      const mouth = swapped ? (spec.mouth ? mouths[spec.mouth] : null) : moodMouth(mood);
      for (const part of Object.values(mouths)) part.visible = part === mouth;
      for (const { side, white, pupil, cheek } of eyes) {
        const closed = spec?.closes?.includes(side) ? s : 0;
        white.visible = pupil.visible = closed < 0.999;
        white.scale.copy(EYE_SCALE).multiplyScalar((1 - closed) * THREE.MathUtils.lerp(1, spec?.white ?? 1, s));
        pupil.scale.setScalar((1 - closed) * THREE.MathUtils.lerp(1, spec?.pupil?.scale ?? 1, s));
        pupil.position.set(side * PUPIL.x + (spec?.pupil?.x ?? 0) * s, PUPIL.y + (spec?.pupil?.y ?? 0) * s, PUPIL.z);
        const blushed = spec?.hidesCheeks ? s : 0;
        cheek.visible = blushed < 0.999;
        cheek.scale.setScalar(1 - blushed);
      }
      for (const { brow, stroke, side } of brows) {
        const from = moodBrow(mood, side), to = spec?.brows?.(side) ?? from;
        brow.visible = mood !== 'idle' || !!spec?.brows;
        brow.scale.setScalar(mood === 'idle' && spec?.brows ? s : 1);
        brow.position.y = THREE.MathUtils.lerp(from.y, to.y, s);
        stroke.rotation.z = Math.PI / 2 + THREE.MathUtils.lerp(from.tilt, to.tilt, s);
      }
      if (!spec || !reaction) return { x: 0, z: 0 };
      const parts = partsOf(reaction);
      parts.group.visible = true;
      parts.group.traverse((part) => {
        const rest = part.userData.rest as THREE.Vector3 | undefined;
        if (rest) part.scale.copy(rest).multiplyScalar(s);
      });
      parts.animate?.(time, s);
      const head = spec.head?.(time) ?? { x: 0, z: 0 };
      return { x: head.x * s, z: head.z * s };
    },
  };
  result.show('idle', null, 0, 0);
  return result;
}

/** A flat shape laid on the face around (x, y), centred on its own origin so it grows from where it sits. */
function decal(kit: Kit, parent: THREE.Object3D, geometry: THREE.BufferGeometry, material: THREE.Material, x: number, y: number, radius = 0.41) {
  onHead(geometry, x, y, radius);
  geometry.computeBoundingBox();
  const centre = geometry.boundingBox!.getCenter(new THREE.Vector3());
  geometry.translate(-centre.x, -centre.y, -centre.z);
  return part(kit.mesh(geometry, material, parent), centre);
}

function part(mesh: THREE.Mesh, position: THREE.Vector3 | [number, number, number]) {
  if (Array.isArray(position)) mesh.position.set(...position);
  else mesh.position.copy(position);
  mesh.userData.rest = mesh.scale.clone();
  return mesh;
}

/** A happy closed eye, an upturned arc. */
function closedEye(kit: Kit, parent: THREE.Object3D, side: Side) {
  const arc = kit.mesh(new THREE.TorusGeometry(0.062, 0.017, 6, 14, Math.PI), kit.m.ink, parent);
  part(arc, [side * EYE.x, 0.03, 0.383]);
  arc.rotation.y = side * 0.36;
}

/** An eyelid over the top of the eye down to `edge` (radians from the top), with its lash line. */
function lid(kit: Kit, parent: THREE.Object3D, side: Side, edge: number) {
  const cap = kit.mesh(new THREE.SphereGeometry(0.1, 16, 10, 0, Math.PI * 2, 0, edge), kit.m.body, parent);
  cap.scale.set(0.97, 1.3, 1.12);
  part(cap, [side * EYE.x, EYE.y, EYE.z]);
  const line = kit.mesh(new THREE.CapsuleGeometry(0.012, 0.13, 4, 8), kit.m.ink, parent);
  part(line, [side * EYE.x, EYE.y + Math.cos(edge) * 0.13, EYE.z + Math.sin(edge) * 0.112]);
  line.rotation.set(0, side * 0.36, Math.PI / 2);
}

function star(outer: number, inner: number) {
  const shape = new THREE.Shape();
  for (let i = 0; i < 10; i++) {
    const r = i % 2 ? inner : outer, a = Math.PI / 2 + (i * Math.PI) / 5;
    if (i) shape.lineTo(Math.cos(a) * r, Math.sin(a) * r);
    else shape.moveTo(Math.cos(a) * r, Math.sin(a) * r);
  }
  return new THREE.ShapeGeometry(shape);
}

function heart(size: number) {
  const shape = new THREE.Shape();
  shape.moveTo(0, -0.9);
  shape.bezierCurveTo(-0.3, -0.55, -1, -0.2, -1, 0.3);
  shape.bezierCurveTo(-1, 0.85, -0.35, 1.05, 0, 0.55);
  shape.bezierCurveTo(0.35, 1.05, 1, 0.85, 1, 0.3);
  shape.bezierCurveTo(1, -0.2, 0.3, -0.55, 0, -0.9);
  return new THREE.ShapeGeometry(shape, 10).scale(size, size, 1);
}

function zed(kit: Kit, parent: THREE.Object3D, material: THREE.Material) {
  const letter = new THREE.Group();
  parent.add(letter);
  for (const y of [0.05, -0.05]) kit.mesh(new THREE.BoxGeometry(0.1, 0.022, 0.01), material, letter).position.y = y;
  const slash = kit.mesh(new THREE.BoxGeometry(0.13, 0.022, 0.01), material, letter);
  slash.rotation.z = Math.atan2(0.1, 0.1);
  return letter;
}

/** Items that rise and fade on a loop, each `phase` of the way through it. */
function drift<T extends THREE.Object3D>(items: { item: T; material: THREE.MeshBasicMaterial; phase: number }[], speed: number, place: (item: T, age: number, index: number) => void, peak = 1) {
  return (time: number, weight: number) => {
    items.forEach(({ item, material, phase }, index) => {
      const age = (time * speed + phase) % 1;
      place(item, age, index);
      material.opacity = peak * weight * Math.sin(age * Math.PI);
    });
  };
}

const raised = (side: Side): Brow => ({ y: 0.25, tilt: side * 0.2 });

const SPECS: Record<FaceReaction, Spec> = {
  wink: {
    closes: [1],
    mouth: 'smile',
    brows: (side) => (side < 0 ? { y: 0.25, tilt: side * 0.25 } : { y: 0.18, tilt: 0 }),
    head: () => ({ x: 0, z: -0.1 }),
    build: (kit) => {
      const group = new THREE.Group();
      closedEye(kit, group, 1);
      return { group };
    },
  },
  'big-laugh': {
    closes: [-1, 1],
    mouth: null,
    brows: raised,
    head: (time) => ({ x: -0.14 + Math.sin(time * 22) * 0.05, z: 0 }),
    build: (kit) => {
      const group = new THREE.Group();
      for (const side of SIDES) closedEye(kit, group, side);
      decal(kit, group, new THREE.CircleGeometry(0.12, 24, Math.PI, Math.PI), kit.m.mouth, 0, -0.04, 0.404);
      decal(kit, group, new THREE.CircleGeometry(0.05, 16).scale(1.2, 0.6, 1), kit.paint('#ff6f91'), 0, -0.13, 0.408);
      return { group };
    },
  },
  'tongue-out': {
    mouth: null,
    brows: (side) => ({ y: 0.24, tilt: side * 0.12 }),
    pupil: { y: 0.01 },
    head: () => ({ x: 0, z: 0.1 }),
    build: (kit) => {
      const group = new THREE.Group();
      const lip = kit.mesh(new THREE.TorusGeometry(0.065, 0.016, 6, 16, Math.PI * 0.8), kit.m.ink, group);
      part(lip, [0, -0.08, 0.388]);
      lip.rotation.z = Math.PI * 1.1;
      const tongue = kit.mesh(new THREE.SphereGeometry(0.068, 14, 10), kit.paint('#ff6f91'), group);
      tongue.scale.set(1, 1.25, 0.55);
      part(tongue, [0.015, -0.165, 0.375]);
      tongue.rotation.set(0.35, 0, 0.15);
      const groove = kit.mesh(new THREE.CapsuleGeometry(0.007, 0.05, 3, 6), kit.paint('#d94f74'), group);
      part(groove, [0.018, -0.175, 0.41]);
      groove.rotation.set(0.35, 0, 0.15);
      return { group };
    },
  },
  'starry-eyes': {
    closes: [-1, 1],
    mouth: 'grin',
    brows: raised,
    build: (kit) => {
      const group = new THREE.Group();
      const gold = kit.paint('#ffcf33');
      // Outlined, so gold still shows on a yellow skin.
      const stars = SIDES.flatMap((side) => [decal(kit, group, star(0.118, 0.056), kit.m.dark, side * EYE.x, EYE.y, 0.408), decal(kit, group, star(0.1, 0.045), gold, side * EYE.x, EYE.y, 0.411)]);
      return {
        group,
        animate: (time, weight) =>
          stars.forEach((s, i) => {
            // Flat on the face, so the outline behind each star never comes through it.
            const k = weight * (1 + 0.14 * Math.sin(time * 9 + Math.floor(i / 2) * 1.7));
            s.scale.set(k, k, 1);
          }),
      };
    },
  },
  'heart-eyes': {
    closes: [-1, 1],
    mouth: 'smile',
    brows: raised,
    build: (kit) => {
      const group = new THREE.Group();
      const red = kit.paint('#ff3b5c');
      const hearts = SIDES.map((side) => decal(kit, group, heart(0.075), red, side * EYE.x, EYE.y));
      return {
        group,
        animate: (time, weight) =>
          hearts.forEach((h) => {
            const k = weight * (1 + 0.16 * Math.max(0, Math.sin(time * 8)));
            h.scale.set(k, k, 1);
          }),
      };
    },
  },
  'smug-grin': {
    mouth: null,
    pupil: { y: -0.025, x: -0.012 },
    brows: (side) => (side > 0 ? { y: 0.25, tilt: side * 0.22 } : { y: 0.19, tilt: 0 }),
    head: () => ({ x: -0.06, z: 0.12 }),
    build: (kit) => {
      const group = new THREE.Group();
      for (const side of SIDES) lid(kit, group, side, Math.PI * 0.45);
      const smirk = kit.mesh(new THREE.TorusGeometry(0.07, 0.016, 6, 16, Math.PI / 2 + 0.75), kit.m.ink, group);
      part(smirk, [0.02, -0.08, 0.387]);
      smirk.rotation.z = -Math.PI / 2 - 0.35;
      return { group };
    },
  },
  shocked: {
    mouth: null,
    white: 1.28,
    pupil: { scale: 0.6 },
    brows: () => ({ y: 0.29, tilt: 0 }),
    head: () => ({ x: -0.1, z: 0 }),
    build: (kit) => {
      const group = new THREE.Group();
      decal(kit, group, new THREE.CircleGeometry(0.04, 18).scale(1, 1.25, 1), kit.m.mouth, 0, -0.13, 0.405);
      return { group };
    },
  },
  sleepy: {
    mouth: null,
    pupil: { y: -0.03 },
    brows: () => ({ y: 0.18, tilt: 0 }),
    head: (time) => ({ x: 0.12 + Math.sin(time * 1.6) * 0.05, z: 0.08 }),
    build: (kit) => {
      const group = new THREE.Group();
      for (const side of SIDES) lid(kit, group, side, Math.PI * 0.6);
      const yawn = decal(kit, group, new THREE.CircleGeometry(0.028, 14), kit.m.mouth, 0.02, -0.12, 0.405);
      // Above the head the z shows against whatever is behind, not the skin, so it keeps one soft colour.
      const letters = [0, 0.5].map((phase) => {
        const material = kit.paint('#a5c8ff', 0);
        return { item: zed(kit, group, material), material, phase };
      });
      const float = drift(letters, 0.45, (letter, age) => {
        letter.position.set(0.3 + age * 0.14, 0.42 + age * 0.32, 0.1);
        letter.scale.setScalar(0.6 + age * 0.7);
      });
      return {
        group,
        animate: (time, weight) => {
          float(time, weight);
          yawn.scale.setScalar(weight * (1 + 0.25 * Math.sin(time * 1.6)));
        },
      };
    },
  },
  'angry-steam': {
    mouth: 'frown',
    pupil: { scale: 0.85 },
    brows: (side) => ({ y: 0.19, tilt: side * 0.55 }),
    head: (time) => ({ x: 0.05, z: Math.sin(time * 34) * 0.025 }),
    build: (kit) => {
      const group = new THREE.Group();
      const puffs = SIDES.flatMap((side) =>
        [0, 1 / 3, 2 / 3].map((phase) => {
          const material = kit.paint('#d4d4d8', 0);
          const item = kit.mesh(new THREE.SphereGeometry(1, 8, 6), material, group);
          item.userData.side = side;
          return { item, material, phase };
        }),
      );
      return {
        group,
        animate: drift(puffs, 1.1, (item, age) => {
          const side = item.userData.side as Side;
          item.position.set(side * (0.32 + age * 0.16), 0.3 + age * 0.3, 0.02);
          item.scale.setScalar(0.035 + age * 0.06);
        }, 0.85),
      };
    },
  },
  blush: {
    mouth: null,
    hidesCheeks: true,
    pupil: { y: -0.025, x: -0.012 },
    brows: (side) => ({ y: 0.21, tilt: -side * 0.3 }),
    head: () => ({ x: 0.12, z: 0.14 }),
    build: (kit) => {
      const group = new THREE.Group();
      const rose = kit.paint('#ff4f7b', 0.85);
      for (const side of SIDES) decal(kit, group, new THREE.CircleGeometry(0.075, 18).scale(1.25, 0.8, 1), rose, side * 0.23, -0.07, 0.406);
      const shy = kit.mesh(new THREE.TorusGeometry(0.045, 0.014, 6, 14, Math.PI), kit.m.ink, group);
      part(shy, [0, -0.1, 0.39]);
      shy.rotation.z = Math.PI;
      return { group };
    },
  },
};

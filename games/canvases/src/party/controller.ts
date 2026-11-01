import * as THREE from "three";

/** A game controller built from primitives, held in front of a character playing a game. */
export type HeldController = {
  group: THREE.Group;
  /** Wiggles the thumbsticks; `time` in seconds, 0 holds them still. */
  update: (time: number) => void;
  dispose: () => void;
};

export const HOLD_ROTATION = new THREE.Euler(-0.55, 0, 0);
const holdRotation = new THREE.Quaternion().setFromEuler(HOLD_ROTATION);

/** Where the controller sits in the holder's character group: just clear of
 * the belly or a jersey front, which reach further forward on a rounder build. */
export function holdPosition(bodyWidth = 1, target = new THREE.Vector3()) {
  return target.set(0, 0.6, 0.52 * Math.sqrt(bodyWidth));
}
/** The centre of each grip in the controller's own space, left (-x) then right. */
export const GRIPS = [
  new THREE.Vector3(-0.16, -0.07, 0.01),
  new THREE.Vector3(0.16, -0.07, 0.01),
] as const;

const offset = new THREE.Vector3();
const holderWidth = (holder: THREE.Object3D | null) =>
  (holder?.userData.bodyWidth as number | undefined) ?? 1;

/** A point in the controller's space, in the holder's character group. */
export function heldPoint(
  point: THREE.Vector3,
  bodyWidth = 1,
  target = new THREE.Vector3(),
) {
  return target
    .copy(point)
    .applyQuaternion(holdRotation)
    .add(holdPosition(bodyWidth, offset));
}

export function buildController(): HeldController {
  const group = new THREE.Group();
  group.name = "held-controller";
  group.userData.carried = true;
  const geometries: THREE.BufferGeometry[] = [];
  const materials = {
    shell: new THREE.MeshToonMaterial({ color: "#2b3040" }),
    stick: new THREE.MeshToonMaterial({ color: "#11141c" }),
    a: new THREE.MeshToonMaterial({ color: "#4ade80" }),
    b: new THREE.MeshToonMaterial({ color: "#f87171" }),
    x: new THREE.MeshToonMaterial({ color: "#60a5fa" }),
    y: new THREE.MeshToonMaterial({ color: "#facc15" }),
  };
  const mesh = (
    geometry: THREE.BufferGeometry,
    material: THREE.Material,
    parent: THREE.Object3D = group,
  ) => {
    geometries.push(geometry);
    const part = new THREE.Mesh(geometry, material);
    parent.add(part);
    return part;
  };

  const body = mesh(
    new THREE.CapsuleGeometry(0.07, 0.26, 4, 12),
    materials.shell,
  );
  body.rotation.z = Math.PI / 2;
  body.scale.set(1, 1, 0.7);
  for (const at of GRIPS) {
    const grip = mesh(
      new THREE.CapsuleGeometry(0.06, 0.1, 4, 10),
      materials.shell,
    );
    grip.position.copy(at);
    grip.rotation.z = Math.sign(at.x) * 0.45;
  }
  const sticks = [-1, 1].map((side) => {
    const pivot = new THREE.Group();
    pivot.position.set(side * 0.07, side < 0 ? 0.02 : -0.015, 0.05);
    group.add(pivot);
    const cap = mesh(
      new THREE.CylinderGeometry(0.028, 0.024, 0.03, 12),
      materials.stick,
      pivot,
    );
    cap.rotation.x = Math.PI / 2;
    cap.position.z = 0.012;
    return pivot;
  });
  const button = new THREE.SphereGeometry(0.016, 8, 6);
  geometries.push(button);
  (
    [
      [materials.a, 0.13, -0.02],
      [materials.b, 0.16, 0.01],
      [materials.x, 0.1, 0.01],
      [materials.y, 0.13, 0.04],
    ] as const
  ).forEach(([material, x, y]) => {
    const dot = new THREE.Mesh(button, material);
    dot.position.set(x + 0.02, y, 0.045);
    group.add(dot);
  });
  const pad = mesh(new THREE.BoxGeometry(0.05, 0.016, 0.01), materials.stick);
  pad.position.set(-0.15, 0.02, 0.045);
  const padCross = mesh(
    new THREE.BoxGeometry(0.016, 0.05, 0.01),
    materials.stick,
  );
  padCross.position.copy(pad.position);

  const update = (time: number) => {
    // The build slider can change the holder's width while it plays.
    if (group.parent) holdPosition(holderWidth(group.parent), group.position);
    sticks[0].rotation.set(
      Math.sin(time * 7.3) * 0.35,
      Math.cos(time * 5.1) * 0.35,
      0,
    );
    sticks[1].rotation.set(
      Math.sin(time * 4.2 + 1) * 0.3,
      Math.sin(time * 9.4) * 0.3,
      0,
    );
  };
  update(0);
  const dispose = () => {
    geometries.forEach((g) => g.dispose());
    Object.values(materials).forEach((m) => m.dispose());
    group.removeFromParent();
  };
  return { group, update, dispose };
}

/** In both hands, in front of a character facing +z whose arms take the 'play' pose. */
export function holdController(
  parent: THREE.Object3D,
  controller: HeldController,
) {
  holdPosition(holderWidth(parent), controller.group.position);
  controller.group.rotation.copy(HOLD_ROTATION);
  parent.add(controller.group);
}

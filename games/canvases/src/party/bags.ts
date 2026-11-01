import * as THREE from 'three';

/**
 * What a character carries their items in. Each is built at its empty size,
 * about 0.6 tall with its bottom at y = 0, and faces +z; the board scales it
 * up as it fills.
 */
export type CarriedBag = {
  group: THREE.Group;
  /** Height of the opening, where items drop in. */
  mouth: number;
  /** The strap or handle up to the shoulder. */
  strap: THREE.Material;
  dispose: () => void;
};

const toon = (color: string) => new THREE.MeshToonMaterial({ color });

/** Where the strap leaves the right shoulder of a character facing +z. */
export const BAG_SHOULDER = new THREE.Vector3(-0.3, 0.86, 0.04);

/** At the right side, just off the ground, as the board carries it at `size`, with its strap up to the shoulder. */
export function hangBag(parent: THREE.Object3D, bag: CarriedBag, size: number) {
  const width = parent.userData.bodyWidth ?? 1;
  const shoulder = BAG_SHOULDER.clone();shoulder.x *= width;
  bag.group.scale.setScalar(size);
  bag.group.position.set(-(0.42 * width + 0.27 * size), 0.04, -0.05 - 0.05 * size);
  bag.group.rotation.z = -0.08 - 0.03 * size;
  parent.add(bag.group);
  bag.group.updateMatrix();
  const neck = new THREE.Vector3(0, bag.mouth - 0.08, 0).applyMatrix4(bag.group.matrix).sub(shoulder);
  const geometry = new THREE.CylinderGeometry(0.03, 0.03, 1, 6).translate(0, 0.5, 0);
  const strap = new THREE.Mesh(geometry, bag.strap);
  strap.position.copy(shoulder);
  strap.quaternion.setFromUnitVectors(THREE.Object3D.DEFAULT_UP, neck.clone().normalize());
  strap.scale.y = neck.length();
  bag.group.userData.carried = strap.userData.carried = true;
  parent.add(strap);
  return () => { geometry.dispose(); strap.removeFromParent(); bag.dispose(); };
}

export function buildBag(style: string | null | undefined): CarriedBag {
  const group = new THREE.Group();
  group.name = `bag-${style ?? 'sack'}`;
  const geometries: THREE.BufferGeometry[] = [];
  const materials: THREE.Material[] = [];
  const material = (color: string) => {
    const made = toon(color);
    materials.push(made);
    return made;
  };
  const mesh = (geometry: THREE.BufferGeometry, paint: THREE.Material) => {
    geometries.push(geometry);
    const made = new THREE.Mesh(geometry, paint);
    made.castShadow = true;
    group.add(made);
    return made;
  };
  let mouth = 0.62;
  let strap: THREE.Material;

  if (style === 'bag-pillowcase') {
    // A stuffed pillowcase, gathered and twisted shut in the fist.
    const body = new THREE.BoxGeometry(0.44, 0.56, 0.3, 8, 8, 4);
    const at = body.getAttribute('position');
    for (let i = 0; i < at.count; i++) {
      const x = at.getX(i), y = at.getY(i), z = at.getZ(i);
      // Puffed out in the middle, soft at the corners, bunched in at the top.
      const round = Math.cos((y / 0.56) * Math.PI * 0.9);
      const squeeze = 1 - Math.max(0, y / 0.28) ** 2 * 0.6;
      const corner = 1 - 0.25 * Math.abs((2 * x) / 0.44) ** 3;
      at.setXYZ(i, x * (1 + 0.15 * round) * squeeze, y + 0.28, z * (1 + 0.45 * round) * squeeze * corner);
    }
    body.computeVertexNormals();
    const cloth = material('#f1f5f9');
    mesh(body, cloth);
    const stripe = material('#60a5fa');
    for (const y of [0.1, 0.17]) {
      const band = mesh(new THREE.BoxGeometry(0.52, 0.025, 0.36), stripe);
      band.position.y = y;
    }
    const twist = mesh(new THREE.ConeGeometry(0.07, 0.16, 7), cloth);
    twist.position.y = 0.56;
    twist.rotation.x = Math.PI;
    strap = cloth;
  } else if (style === 'bag-stocking') {
    // Red with a white cuff, the toe kicked out to one side.
    const red = material('#dc2626');
    const white = material('#f8fafc');
    const leg = mesh(new THREE.CylinderGeometry(0.16, 0.15, 0.4, 14), red);
    leg.position.y = 0.33;
    const foot = mesh(new THREE.CapsuleGeometry(0.15, 0.16, 6, 14), red);
    foot.rotation.z = Math.PI / 2;
    foot.position.set(-0.08, 0.15, 0);
    const toe = mesh(new THREE.SphereGeometry(0.152, 14, 10, 0, Math.PI * 2, 0, Math.PI / 2), material('#16a34a'));
    toe.rotation.z = Math.PI / 2;
    toe.position.set(-0.16, 0.15, 0);
    const cuff = mesh(new THREE.CylinderGeometry(0.2, 0.19, 0.14, 14), white);
    cuff.position.y = 0.56;
    const loop = mesh(new THREE.TorusGeometry(0.05, 0.014, 6, 12), white);
    loop.position.set(0.17, 0.66, 0);
    strap = white;
  } else if (style === 'bag-pumpkin') {
    // A trick-or-treat pail: open at the top, a grin on the front, a wire handle.
    const orange = material('#f97316');
    const body = new THREE.SphereGeometry(0.28, 18, 12, 0, Math.PI * 2, 0.55, Math.PI - 0.55);
    body.scale(1, 0.85, 1);
    body.translate(0, 0.24, 0);
    const shell = mesh(body, orange);
    (shell.material as THREE.Material).side = THREE.DoubleSide;
    mouth = 0.46;
    const inside = mesh(new THREE.CircleGeometry(0.15, 16), material('#431407'));
    inside.rotation.x = -Math.PI / 2;
    inside.position.y = 0.42;
    const ink = material('#1c1917');
    for (const side of [-1, 1]) {
      const eye = mesh(new THREE.ConeGeometry(0.045, 0.07, 3), ink);
      eye.scale.z = 0.3;
      eye.position.set(side * 0.09, 0.3, 0.255);
    }
    const grin = mesh(new THREE.TorusGeometry(0.09, 0.02, 4, 12, Math.PI), ink);
    grin.rotation.z = Math.PI;
    grin.position.set(0, 0.2, 0.265);
    const handle = mesh(new THREE.TorusGeometry(0.2, 0.012, 6, 20, Math.PI), ink);
    handle.position.y = 0.44;
    strap = ink;
  } else {
    // A lathed sack: round bottom, fat belly, cinched neck and a frill of cloth over the tie.
    const santa = style === 'bag-santa';
    const profile = [[0, 0], [0.14, 0.015], [0.23, 0.08], [0.27, 0.19], [0.26, 0.31], [0.2, 0.42], [0.1, 0.5], [0.075, 0.53], [0.1, 0.58], [0.13, 0.63], [0.08, 0.6], [0, 0.57]].map(([x, y]) => new THREE.Vector2(x, y));
    const sack = new THREE.LatheGeometry(profile, 14);
    sack.computeVertexNormals();
    const body = mesh(sack, material(santa ? '#b91c1c' : '#c9a26a'));
    body.scale.set(1.08, 1, 0.92);
    const rope = material(santa ? '#fef3c7' : '#6b4423');
    const tie = mesh(new THREE.TorusGeometry(0.085, santa ? 0.032 : 0.024, 6, 14), rope);
    tie.rotation.x = Math.PI / 2;
    tie.position.y = 0.53;
    strap = rope;
  }

  return {
    group,
    mouth,
    strap,
    dispose: () => {
      geometries.forEach((geometry) => geometry.dispose());
      materials.forEach((made) => made.dispose());
      group.removeFromParent();
    },
  };
}

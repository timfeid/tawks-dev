import * as THREE from "three";

type Built = { size: THREE.Vector3; top: number };
type Kit = {
  part: (
    geometry: THREE.BufferGeometry,
    material: THREE.Material,
    parent?: THREE.Object3D,
  ) => THREE.Mesh;
  toon: (color: string) => THREE.Material;
  flat: (color: string) => THREE.Material;
};

export const BUILT: Record<string, (kit: Kit, root: THREE.Group) => Built> = {
  "head-turkey": turkey,
  "head-gingerbread": gingerbread,
  "head-snowman": snowman,
  "head-penguin": penguin,
};
/** Builds head `id` at its natural size, shaded by `toon`, or null when it is not one built here. */
export function buildHead(id: string, toon: (color: string) => THREE.Material) {
  const build = BUILT[id];
  if (!build) return null;
  const geometries: THREE.BufferGeometry[] = [],
    materials = new Map<string, THREE.Material>();
  const cached = (key: string, make: () => THREE.Material) => {
    let material = materials.get(key);
    if (!material) materials.set(key, (material = make()));
    return material;
  };
  const group = new THREE.Group();
  group.name = id;
  const kit: Kit = {
    part(geometry, material, parent = group) {
      geometries.push(geometry);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.castShadow = true;
      parent.add(mesh);
      return mesh;
    },
    toon: (color) => cached(`toon${color}`, () => toon(color)),
    flat: (color) =>
      cached(`flat${color}`, () => new THREE.MeshBasicMaterial({ color })),
  };
  const { size, top } = build(kit, group);
  return {
    group,
    size,
    top,
    dispose() {
      group.removeFromParent();
      geometries.forEach((geometry) => geometry.dispose());
      materials.forEach((material) => material.dispose());
    },
  };
}

/** Where a ball of `radius` about the origin, squashed to `depth` front to back, has its front surface at (x, y). */
const front = (radius: number, x: number, y: number, depth = 1) =>
  radius *
  depth *
  Math.sqrt(Math.max(0, 1 - (x * x + y * y) / (radius * radius)));

function eyes(
  kit: Kit,
  parent: THREE.Object3D,
  radius: number,
  x: number,
  y: number,
  depth = 1,
) {
  for (const side of [-1, 1]) {
    const white = kit.part(
      new THREE.SphereGeometry(0.085, 16, 12),
      kit.toon("#ffffff"),
      parent,
    );
    white.scale.set(0.9, 1.15, 0.5);
    white.position.set(side * x, y, front(radius, x, y, depth) - 0.012);
    const pupil = kit.part(
      new THREE.SphereGeometry(0.045, 12, 10),
      kit.flat("#1b1b24"),
      parent,
    );
    pupil.scale.z = 0.5;
    pupil.position.set(side * x * 0.97, y - 0.01, white.position.z + 0.04);
    const shine = kit.part(
      new THREE.SphereGeometry(0.014, 8, 6),
      kit.flat("#ffffff"),
      parent,
    );
    shine.position.set(
      side * x * 0.97 + 0.016,
      y + 0.012,
      pupil.position.z + 0.02,
    );
    shine.castShadow = false;
  }
}

function cheeks(
  kit: Kit,
  parent: THREE.Object3D,
  radius: number,
  x: number,
  y: number,
  depth = 1,
) {
  for (const side of [-1, 1]) {
    const cheek = kit.part(
      new THREE.SphereGeometry(0.05, 12, 8),
      kit.flat("#ff8fa3"),
      parent,
    );
    cheek.scale.set(1.2, 0.8, 0.3);
    cheek.position.set(side * x, y, front(radius, x, y, depth));
    cheek.castShadow = false;
  }
}

function turkey(kit: Kit, root: THREE.Group): Built {
  const radius = 0.42;
  const tail = new THREE.Group();
  tail.position.set(0, -0.06, -0.24);
  root.add(tail);
  const plumage = ["#8a4b22", "#c0502b", "#e8892f", "#f2c14e"];
  // Two rows of tail feathers fanned behind the head, the long dark row at the back.
  for (const [row, length, count, spread] of [
    [0, 0.44, 9, 1.35],
    [1, 0.34, 7, 1.15],
  ] as const) {
    for (let i = 0; i < count; i++) {
      const angle = -spread + (2 * spread * i) / (count - 1);
      const pivot = new THREE.Group();
      pivot.rotation.z = -angle;
      pivot.position.z = row ? 0.03 : -0.03;
      tail.add(pivot);
      const feather = kit.part(
        new THREE.SphereGeometry(1, 14, 10),
        kit.toon(plumage[(i + row * 2) % plumage.length]),
        pivot,
      );
      feather.scale.set(0.105, length / 2, 0.035);
      feather.position.y = radius * 0.62 + length / 2;
      const tip = kit.part(
        new THREE.SphereGeometry(1, 12, 8),
        kit.toon(row ? "#5a2f17" : "#3d2212"),
        pivot,
      );
      tip.scale.set(0.085, 0.035, 0.037);
      tip.position.y = radius * 0.62 + length - 0.07;
    }
  }
  const head = kit.part(
    new THREE.SphereGeometry(radius, 32, 24),
    kit.toon("#9b6236"),
    root,
  );
  head.name = "head-skull";
  const tuft = kit.part(
    new THREE.ConeGeometry(0.05, 0.14, 8),
    kit.toon("#7a4a26"),
    root,
  );
  tuft.position.set(0.02, radius + 0.04, 0.04);
  tuft.rotation.z = -0.3;
  eyes(kit, root, radius, 0.14, 0.11);
  const beak = kit.part(
    new THREE.ConeGeometry(0.075, 0.2, 14),
    kit.toon("#f3a530"),
    root,
  );
  beak.rotation.x = Math.PI / 2;
  beak.scale.set(1.15, 1, 0.85);
  beak.position.set(0, -0.03, radius + 0.07);
  // The wattle hangs from under the beak, down past the chin.
  const wattle = kit.part(
    new THREE.CapsuleGeometry(0.045, 0.16, 6, 12),
    kit.toon("#d8343f"),
    root,
  );
  wattle.position.set(0.045, -0.16, radius - 0.02);
  wattle.rotation.set(0.25, 0, 0.18);
  cheeks(kit, root, radius, 0.26, -0.04);
  return {
    size: new THREE.Vector3(2 * radius, 2 * radius, 2 * radius),
    top: radius * 0.62 + 0.44 - 0.06,
  };
}

function gingerbread(kit: Kit, root: THREE.Group): Built {
  const radius = 0.43,
    depth = 0.72;
  const cookie = kit.part(
    new THREE.SphereGeometry(radius, 36, 24),
    kit.toon("#b5713a"),
    root,
  );
  cookie.scale.z = depth;
  cookie.name = "head-skull";
  const icing = kit.toon("#fbf6ec");
  // A wavy line of piped icing just in from the rim.
  const trim: THREE.Vector3[] = [];
  for (let i = 0; i < 168; i++) {
    const angle = (i / 168) * Math.PI * 2,
      r = 0.33 + 0.014 * Math.sin(angle * 14);
    const x = Math.sin(angle) * r,
      y = Math.cos(angle) * r;
    trim.push(new THREE.Vector3(x, y, front(radius, x, y, depth) + 0.004));
  }
  kit.part(
    new THREE.TubeGeometry(
      new THREE.CatmullRomCurve3(trim, true),
      168,
      0.021,
      6,
      true,
    ),
    icing,
    root,
  );
  for (const side of [-1, 1]) {
    const eye = kit.part(
      new THREE.SphereGeometry(0.052, 14, 10),
      kit.flat("#3a2214"),
      root,
    );
    eye.scale.set(1, 1.15, 0.45);
    eye.position.set(side * 0.13, 0.09, front(radius, 0.13, 0.09, depth));
    const shine = kit.part(
      new THREE.SphereGeometry(0.014, 8, 6),
      kit.flat("#ffffff"),
      root,
    );
    shine.position.set(side * 0.13 + 0.016, 0.105, eye.position.z + 0.022);
    shine.castShadow = false;
  }
  const smile: THREE.Vector3[] = [];
  for (let i = 0; i <= 16; i++) {
    const angle = -0.95 + (1.9 * i) / 16;
    const x = Math.sin(angle) * 0.15,
      y = 0.02 - Math.cos(angle) * 0.15;
    smile.push(new THREE.Vector3(x, y, front(radius, x, y, depth) + 0.004));
  }
  kit.part(
    new THREE.TubeGeometry(new THREE.CatmullRomCurve3(smile), 32, 0.02, 6),
    icing,
    root,
  );
  for (const end of [smile[0], smile[smile.length - 1]]) {
    const dot = kit.part(new THREE.SphereGeometry(0.021, 8, 6), icing, root);
    dot.position.copy(end);
  }
  // Gumdrop cheeks.
  for (const side of [-1, 1]) {
    const gumdrop = kit.part(
      new THREE.SphereGeometry(0.045, 14, 8, 0, Math.PI * 2, 0, Math.PI / 2),
      kit.toon(side < 0 ? "#d93644" : "#3f9e4d"),
      root,
    );
    gumdrop.rotation.x = Math.PI / 2;
    gumdrop.position.set(
      side * 0.24,
      -0.05,
      front(radius, 0.24, -0.05, depth) - 0.006,
    );
  }
  return {
    size: new THREE.Vector3(2 * radius, 2 * radius, 2 * radius * depth),
    top: radius,
  };
}

function snowman(kit: Kit, root: THREE.Group): Built {
  const radius = 0.42;
  const snow = kit.part(
    new THREE.SphereGeometry(radius, 32, 24),
    kit.toon("#f2f6fb"),
    root,
  );
  snow.name = "head-skull";
  const coal = kit.flat("#1c1c22");
  for (const side of [-1, 1]) {
    const eye = kit.part(new THREE.SphereGeometry(0.05, 12, 10), coal, root);
    eye.position.set(side * 0.13, 0.1, front(radius, 0.13, 0.1) - 0.01);
  }
  const carrot = kit.part(
    new THREE.ConeGeometry(0.055, 0.3, 14),
    kit.toon("#ef7d22"),
    root,
  );
  carrot.rotation.x = Math.PI / 2 - 0.12;
  carrot.position.set(0, -0.01, radius + 0.12);
  for (let i = 0; i < 5; i++) {
    const angle = -0.7 + (1.4 * i) / 4;
    const x = Math.sin(angle) * 0.17,
      y = 0.02 - Math.cos(angle) * 0.19;
    const stone = kit.part(new THREE.SphereGeometry(0.027, 10, 8), coal, root);
    stone.position.set(x, y, front(radius, x, y) - 0.005);
  }
  cheeks(kit, root, radius, 0.25, -0.05);
  return {
    size: new THREE.Vector3(2 * radius, 2 * radius, 2 * radius),
    top: radius,
  };
}

function penguin(kit: Kit, root: THREE.Group): Built {
  const radius = 0.42;
  const feathers = kit.part(
    new THREE.SphereGeometry(radius, 32, 24),
    kit.toon("#26303f"),
    root,
  );
  feathers.name = "head-skull";
  // A heart-shaped white face: two lobes over the eyes meeting a rounder chin.
  const white = kit.toon("#f6f5ef");
  for (const side of [-1, 1]) {
    const lobe = kit.part(new THREE.SphereGeometry(0.2, 24, 16), white, root);
    lobe.scale.set(1, 1.05, 0.45);
    lobe.position.set(side * 0.115, 0.03, front(radius, 0.115, 0.03) - 0.075);
  }
  const chin = kit.part(new THREE.SphereGeometry(0.25, 24, 16), white, root);
  chin.scale.set(1.15, 0.85, 0.45);
  chin.position.set(0, -0.1, front(radius, 0, -0.1) - 0.1);
  const patch = (x: number, y: number) => front(radius, x, y) + 0.018;
  for (const side of [-1, 1]) {
    const eye = kit.part(
      new THREE.SphereGeometry(0.052, 14, 10),
      kit.flat("#14161c"),
      root,
    );
    eye.scale.set(0.9, 1.1, 0.5);
    eye.position.set(side * 0.12, 0.07, patch(0.12, 0.07));
    const shine = kit.part(
      new THREE.SphereGeometry(0.015, 8, 6),
      kit.flat("#ffffff"),
      root,
    );
    shine.position.set(side * 0.12 + 0.017, 0.088, eye.position.z + 0.022);
    shine.castShadow = false;
  }
  const beak = kit.part(
    new THREE.ConeGeometry(0.07, 0.17, 14),
    kit.toon("#f39a2b"),
    root,
  );
  beak.rotation.x = Math.PI / 2;
  beak.scale.set(1.2, 1, 0.7);
  beak.position.set(0, -0.05, radius + 0.08);
  cheeks(kit, root, radius + 0.015, 0.23, -0.07);
  return {
    size: new THREE.Vector3(2 * radius, 2 * radius, 2 * radius),
    top: radius,
  };
}

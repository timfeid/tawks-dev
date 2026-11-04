import * as THREE from "three";

/** Each swapped head's height and depth as fractions of its width. */
export const HEAD_PROPORTIONS: Record<string, { y: number; z: number }> = {
  "head-pumpkin": { y: 0.74, z: 0.934 },
  "head-turkey": { y: 1, z: 1 },
  "head-gingerbread": { y: 1, z: 0.72 },
  "head-snowman": { y: 1, z: 1 },
  "head-penguin": { y: 1, z: 1 },
};

/** How wide a swapped head goes on a model whose own head is `size`, so it covers it. */
export function swappedHeadWidth(id: string, size: THREE.Vector3) {
  const { y, z } = HEAD_PROPORTIONS[id] ?? { y: 1, z: 1 };
  // The pumpkin was fitted before the flat gingerbread; only a head much shallower than it is widened to cover a deep model head.
  const depth = id === "head-pumpkin" ? 1 : z;
  return THREE.MathUtils.clamp(
    Math.max(size.x, size.z / depth, size.y / y) * 1.25,
    0.7,
    1.8,
  );
}

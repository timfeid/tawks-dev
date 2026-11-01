type Point = { x: number; y: number; z: number };

/** The stalk is decoration, not part of the skull we attach and fit hats to. */
export function pumpkinFit(points: readonly Point[], width: number) {
  if (!points.length) throw new Error("Pumpkin mesh is empty");
  const min = { x: Infinity, y: Infinity, z: Infinity };
  const max = { x: -Infinity, y: -Infinity, z: -Infinity };
  for (const p of points)
    for (const axis of ["x", "y", "z"] as const) {
      min[axis] = Math.min(min[axis], p[axis]);
      max[axis] = Math.max(max[axis], p[axis]);
    }
  const x = (min.x + max.x) / 2,
    z = (min.z + max.z) / 2;
  const span = Math.max(max.x - min.x, max.z - min.z);
  const crown = Math.max(
    ...points
      .filter((p) => Math.hypot(p.x - x, p.z - z) >= span * 0.1)
      .map((p) => p.y),
  );
  const scale = width / Math.max(0.001, span);
  return {
    scale,
    center: { x, y: (min.y + crown) / 2, z },
    size: {
      x: (max.x - min.x) * scale,
      y: (crown - min.y) * scale,
      z: (max.z - min.z) * scale,
    },
  };
}

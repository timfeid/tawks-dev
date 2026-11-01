import * as THREE from 'three';
import GEAR from './gear.json' with { type: 'json' };

type Vec = [number, number, number];
/** One primitive of a gear style. Hats are built on a base of radius 1 at the origin, pointing up;
 * neckwear around a ring of radius 1 in the xz plane, front at +z. A torus lies in the xz plane.
 * apps/tawkathon-native/src/pets.rs builds the same parts. */
export type GearPart = {
  sphere?: number;
  /** Top radius, bottom radius and height, along y. */
  frustum?: Vec;
  /** Ring radius and tube radius. */
  torus?: [number, number];
  box?: Vec;
  at?: Vec;
  /** Euler angles in radians, applied x then y then z. */
  turn?: Vec;
  scale?: Vec;
  tint: number;
};

export const GEAR_STYLES = GEAR as unknown as Record<string, GearPart[]>;

function geometry(part: GearPart): THREE.BufferGeometry {
  if (part.sphere !== undefined) return new THREE.SphereGeometry(part.sphere, 20, 14);
  if (part.frustum) return new THREE.CylinderGeometry(part.frustum[0], part.frustum[1], part.frustum[2], 24);
  if (part.torus) return new THREE.TorusGeometry(part.torus[0], part.torus[1], 10, 32).rotateX(Math.PI / 2);
  if (part.box) return new THREE.BoxGeometry(...part.box);
  throw new Error('Unknown gear part');
}

/** The pet gear drawn in `style`, tinted from `colors` with `shade`; null for an unknown style. */
export function buildGear(style: string | undefined, colors: string[], shade: (color: string) => THREE.Material) {
  const parts = style ? GEAR_STYLES[style] : undefined;
  if (!parts) return null;
  const group = new THREE.Group();
  group.name = `pet-gear-${style}`;
  const materials = new Map<string, THREE.Material>();
  for (const part of parts) {
    const color = colors[part.tint] ?? '#ffffff';
    let material = materials.get(color);
    if (!material) materials.set(color, (material = shade(color)));
    const mesh = new THREE.Mesh(geometry(part), material);
    mesh.position.set(...(part.at ?? [0, 0, 0]));
    if (part.turn) mesh.rotation.set(...part.turn);
    if (part.scale) mesh.scale.set(...part.scale);
    mesh.castShadow = true;
    group.add(mesh);
  }
  return {
    group,
    dispose() {
      group.removeFromParent();
      group.traverse((object) => {
        if (object instanceof THREE.Mesh) object.geometry.dispose();
      });
      materials.forEach((material) => material.dispose());
    },
  };
}

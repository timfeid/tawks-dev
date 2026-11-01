import * as THREE from "three";

let toonRamp: THREE.DataTexture | null = null;

function ramp() {
  if (!toonRamp) {
    toonRamp = new THREE.DataTexture(
      new Uint8Array([90, 90, 90, 255, 175, 175, 175, 255, 255, 255, 255, 255]),
      3,
      1,
      THREE.RGBAFormat,
    );
    toonRamp.minFilter = THREE.NearestFilter;
    toonRamp.magFilter = THREE.NearestFilter;
    toonRamp.needsUpdate = true;
  }
  return toonRamp;
}

/** The three-step shading every party character is drawn with. */
export const toon = (color: string) =>
  new THREE.MeshToonMaterial({ color, gradientMap: ramp() });

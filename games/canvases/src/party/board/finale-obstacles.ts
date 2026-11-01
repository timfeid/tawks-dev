import * as THREE from "three";
import type { FinaleObstacle } from "../protocol";

/** Shared meshes keep a whole slalom's roadworks to six draw calls. The
 * collider dimensions and transforms come directly from the race server. */
export class FinaleObstacles {
  readonly group = new THREE.Group();
  private box = new THREE.BoxGeometry(1, 1, 1);
  private disc = new THREE.CylinderGeometry(1, 1, 0.025, 32);
  private ring = new THREE.RingGeometry(0.9, 1, 32).rotateX(-Math.PI / 2);
  private orange = new THREE.MeshStandardMaterial({
    color: "#ff7a24",
    roughness: 0.8,
  });
  private white = new THREE.MeshStandardMaterial({
    color: "#fff6de",
    roughness: 0.7,
  });
  private dark = new THREE.MeshStandardMaterial({
    color: "#303b46",
    roughness: 0.9,
  });
  private amber = new THREE.MeshStandardMaterial({
    color: "#ffd84b",
    emissive: "#ffac19",
    emissiveIntensity: 0.9,
  });
  private ice = new THREE.MeshStandardMaterial({
    color: "#6cd0f2",
    roughness: 0.15,
    metalness: 0.15,
    transparent: true,
    opacity: 0.72,
    depthWrite: false,
  });
  private edge = new THREE.MeshBasicMaterial({
    color: "#c4f5ff",
    transparent: true,
    opacity: 0.8,
    depthWrite: false,
  });
  private blocks = new THREE.InstancedMesh(this.box, this.orange, 32);
  private stripes = new THREE.InstancedMesh(this.box, this.white, 256);
  private feet = new THREE.InstancedMesh(this.box, this.dark, 64);
  private beacons = new THREE.InstancedMesh(this.box, this.amber, 64);
  private patches = new THREE.InstancedMesh(this.disc, this.ice, 16);
  private rims = new THREE.InstancedMesh(this.ring, this.edge, 16);
  private transform = new THREE.Object3D();
  private tilt = new THREE.Quaternion().setFromAxisAngle(
    new THREE.Vector3(0, 0, 1),
    -0.45,
  );
  constructor(private floor: (x: number, z: number) => number) {
    this.group.name = "finale-roadworks";
    const meshes = [
      this.blocks,
      this.stripes,
      this.feet,
      this.beacons,
      this.patches,
      this.rims,
    ];
    for (const mesh of meshes) {
      mesh.count = 0;
      mesh.frustumCulled = false;
      this.group.add(mesh);
    }
    this.blocks.name = "finale-collidable-barriers";
    this.patches.name = "finale-slippery-ice";
  }
  sync(obstacles: FinaleObstacle[]) {
    for (const child of this.group.children)
      (child as THREE.InstancedMesh).count = 0;
    const put = (
      mesh: THREE.InstancedMesh,
      obstacle: FinaleObstacle,
      x: number,
      y: number,
      z: number,
      width: number,
      height: number,
      depth: number,
      tilted = false,
    ) => {
      if (mesh.count >= mesh.instanceMatrix.count) return;
      const sin = Math.sin(obstacle.yaw),
        cos = Math.cos(obstacle.yaw);
      this.transform.position.set(
        obstacle.x + x * cos + z * sin,
        this.floor(obstacle.x, obstacle.z) + y,
        obstacle.z - x * sin + z * cos,
      );
      this.transform.quaternion.setFromAxisAngle(
        THREE.Object3D.DEFAULT_UP,
        obstacle.yaw,
      );
      if (tilted) this.transform.quaternion.multiply(this.tilt);
      this.transform.scale.set(width, height, depth);
      this.transform.updateMatrix();
      mesh.setMatrixAt(mesh.count++, this.transform.matrix);
    };
    for (const obstacle of obstacles) {
      if (obstacle.kind === "barrier") {
        put(
          this.blocks,
          obstacle,
          0,
          0.72,
          0,
          obstacle.half_width * 2,
          1.05,
          obstacle.half_depth * 2,
        );
        for (const side of [-1, 1]) {
          put(
            this.feet,
            obstacle,
            side * (obstacle.half_width - 0.25),
            0.18,
            0,
            0.48,
            0.22,
            obstacle.half_depth * 2 + 0.45,
          );
          put(
            this.beacons,
            obstacle,
            side * (obstacle.half_width - 0.2),
            1.36,
            0,
            0.2,
            0.25,
            0.2,
          );
          for (const stripe of [-0.9, -0.3, 0.3, 0.9])
            put(
              this.stripes,
              obstacle,
              stripe,
              0.74,
              side * (obstacle.half_depth + 0.012),
              0.23,
              0.84,
              0.015,
              true,
            );
        }
      } else {
        put(
          this.patches,
          obstacle,
          0,
          0.19,
          0,
          obstacle.half_width,
          1,
          obstacle.half_depth,
        );
        put(
          this.rims,
          obstacle,
          0,
          0.21,
          0,
          obstacle.half_width,
          1,
          obstacle.half_depth,
        );
      }
    }
    for (const child of this.group.children)
      (child as THREE.InstancedMesh).instanceMatrix.needsUpdate = true;
  }
  dispose() {
    for (const child of this.group.children)
      (child as THREE.InstancedMesh).dispose();
    for (const resource of [
      this.box,
      this.disc,
      this.ring,
      this.orange,
      this.white,
      this.dark,
      this.amber,
      this.ice,
      this.edge,
    ])
      resource.dispose();
    this.group.removeFromParent();
  }
}

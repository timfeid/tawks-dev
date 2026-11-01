import * as THREE from "three";

type Part = {
  root: THREE.Object3D;
  mesh: THREE.Mesh;
  matrix: THREE.Matrix4;
  shown: boolean;
};
type Batch = { mesh: THREE.InstancedMesh; parts: Part[] };
/** Batch repeated static buildings in spatial cells. Keep the original roots
 * for camera occlusion and bounding boxes; their shared GLTF resources remain
 * cache-owned. Only instance buffers belong to this class. */
export class StaticProps {
  private roots = new Set<THREE.Object3D>();
  private batches: Batch[] = [];
  private previous = 0;
  private zero = new THREE.Matrix4().makeScale(0, 0, 0);
  private inverse = new THREE.Matrix4();
  private parent: THREE.Group;
  private collection: Iterable<THREE.Object3D>;
  constructor(parent: THREE.Group, collection: Iterable<THREE.Object3D>) {
    this.parent = parent;
    this.collection = collection;
  }
  update(time: number) {
    if (time - this.previous > 0.5) {
      this.previous = time;
      let changed = false;
      for (const root of this.collection)
        if (
          root.parent === this.parent &&
          root.name.startsWith("board-model-town-building-") &&
          !this.roots.has(root)
        ) {
          this.roots.add(root);
          changed = true;
        }
      if (changed) this.rebuild();
    }
    for (const batch of this.batches) {
      let dirty = false;
      for (let i = 0; i < batch.parts.length; i++) {
        const part = batch.parts[i];
        if (part.shown !== part.root.visible) {
          part.shown = part.root.visible;
          batch.mesh.setMatrixAt(i, part.shown ? part.matrix : this.zero);
          dirty = true;
        }
      }
      if (dirty) batch.mesh.instanceMatrix.needsUpdate = true;
    }
  }
  private rebuild() {
    this.clear();
    this.parent.updateWorldMatrix(true, true);
    this.inverse.copy(this.parent.matrixWorld).invert();
    const groups = new Map<
      string,
      {
        material: THREE.Material;
        geometry: THREE.BufferGeometry;
        parts: Part[];
      }
    >();
    for (const root of this.roots) {
      root.traverse((object) => {
        const mesh = object as THREE.Mesh;
        if (
          !mesh.isMesh ||
          !mesh.visible ||
          Array.isArray(mesh.material) ||
          (mesh as THREE.SkinnedMesh).isSkinnedMesh
        )
          return;
        const key = `${mesh.geometry.uuid}:${mesh.material.uuid}:${Math.floor(root.position.x / 32)}:${Math.floor(root.position.z / 32)}`;
        let group = groups.get(key);
        if (!group) {
          group = {
            material: mesh.material,
            geometry: mesh.geometry,
            parts: [],
          };
          groups.set(key, group);
        }
        group.parts.push({
          root,
          mesh,
          matrix: new THREE.Matrix4().multiplyMatrices(
            this.inverse,
            mesh.matrixWorld,
          ),
          shown: root.visible,
        });
      });
    }
    for (const group of groups.values()) {
      if (group.parts.length < 2) continue;
      const mesh = new THREE.InstancedMesh(
        group.geometry,
        group.material,
        group.parts.length,
      );
      mesh.name = "static-building-batch";
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      group.parts.forEach((part, i) => {
        mesh.setMatrixAt(i, part.matrix);
        part.mesh.visible = false;
      });
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
      group.parts.forEach((part, i) => {
        if (!part.shown) mesh.setMatrixAt(i, this.zero);
      });
      this.parent.add(mesh);
      this.batches.push({ mesh, parts: group.parts });
    }
  }
  private clear() {
    for (const batch of this.batches) {
      for (const part of batch.parts) part.mesh.visible = true;
      batch.mesh.removeFromParent();
      batch.mesh.dispose();
    }
    this.batches = [];
  }
  dispose() {
    this.clear();
    this.roots.clear();
  }
}

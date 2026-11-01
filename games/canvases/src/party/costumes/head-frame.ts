import * as THREE from "three";
const frames = new WeakMap<THREE.Object3D, ReturnType<typeof buildFrame>>();
export function headFrame(wrapper: THREE.Group, root: THREE.Object3D) {
  if (!frames.has(root)) frames.set(root, buildFrame(wrapper, root));
  return frames.get(root)!;
}
function buildFrame(wrapper: THREE.Group, root: THREE.Object3D) {
  let head: THREE.Bone | null = null;
  root.traverse((object) => {
    if (
      !head &&
      (object as THREE.Bone).isBone &&
      /(?:^|[:_. -])head(?:$|[:_. -])/i.test(object.name) &&
      !/end|tip/i.test(object.name)
    )
      head = object as THREE.Bone;
  });
  if (!head) return null;
  const bone: THREE.Bone = head;
  // SkinnedMesh.updateMatrixWorld refreshes bindMatrixInverse after the
  // normalized wrapper scale. updateWorldMatrix alone would double-scale
  // the weighted head bounds before the renderer has seen this new rig.
  wrapper.updateMatrixWorld(true);
  const inverse = wrapper.matrixWorld.clone().invert(),
    box = new THREE.Box3();
  const inHead = (candidate: THREE.Bone) => {
    let at: THREE.Object3D | null = candidate;
    while (at) {
      if (at === bone) return true;
      at = at.parent;
    }
    return false;
  };
  root.traverse((object) => {
    const mesh = object as THREE.SkinnedMesh;
    if (!mesh.isSkinnedMesh) return;
    const indices = mesh.geometry.getAttribute("skinIndex"),
      weights = mesh.geometry.getAttribute("skinWeight"),
      positions = mesh.geometry.getAttribute("position");
    if (!indices || !weights || !positions) return;
    const selected = new Set(
      mesh.skeleton.bones.flatMap((candidate, index) =>
        inHead(candidate) ? [index] : [],
      ),
    );
    const matrix = inverse.clone().multiply(mesh.matrixWorld),
      point = new THREE.Vector3();
    for (let i = 0; i < positions.count; i++) {
      let influence = 0;
      for (let component = 0; component < 4; component++)
        if (selected.has(indices.getComponent(i, component)))
          influence += weights.getComponent(i, component);
      if (influence > 0.3) {
        mesh.getVertexPosition(i, point);
        box.expandByPoint(point.applyMatrix4(matrix));
      }
    }
  });
  const center = box.isEmpty()
    ? wrapper
        .worldToLocal(bone.getWorldPosition(new THREE.Vector3()))
        .add(new THREE.Vector3(0, 0.12, 0))
    : box.getCenter(new THREE.Vector3());
  const size = box.isEmpty()
    ? new THREE.Vector3(0.7, 0.65, 0.65)
    : box.getSize(new THREE.Vector3());

  const neutral = wrapper
    .getWorldQuaternion(new THREE.Quaternion())
    .invert()
    .multiply(bone.getWorldQuaternion(new THREE.Quaternion()))
    .invert();
  return {
    size,
    center,
    anchor(delta = new THREE.Vector3()) {
      const group = new THREE.Group();
      wrapper.add(group);
      const offset = bone.worldToLocal(
        wrapper.localToWorld(center.clone().add(delta)),
      );
      const position = new THREE.Vector3(),
        rotation = new THREE.Quaternion(),
        parentRotation = new THREE.Quaternion();
      const update = () => {
        bone.updateWorldMatrix(true, false);
        position.copy(offset).applyMatrix4(bone.matrixWorld);
        wrapper.worldToLocal(position);
        group.position.copy(position);
        parentRotation.copy(wrapper.getWorldQuaternion(rotation)).invert();
        group.quaternion.copy(
          parentRotation
            .multiply(bone.getWorldQuaternion(rotation))
            .multiply(neutral),
        );
      };
      update();
      return {
        group,
        update,
        dispose() {
          group.removeFromParent();
        },
      };
    },
  };
}
export function pumpkinWidth(size: THREE.Vector3) {
  return THREE.MathUtils.clamp(
    Math.max(size.x, size.z, size.y / 0.74) * 1.25,
    0.7,
    1.8,
  );
}
export function attachModelExtras(
  wrapper: THREE.Group,
  root: THREE.Object3D,
  pumpkin: boolean,
) {
  const frame = headFrame(wrapper, root);
  if (!frame) return null;
  const size = pumpkin
    ? new THREE.Vector3(
        pumpkinWidth(frame.size),
        pumpkinWidth(frame.size) * 0.74,
        pumpkinWidth(frame.size) * 0.934,
      )
    : frame.size;
  const head = frame.anchor(
    new THREE.Vector3(0, pumpkin ? 0 : -size.y * 0.08, 0),
  );
  head.group.name = "model-head-extras";
  head.group.scale.set(size.x / 0.8, size.y / 0.8, size.z / 0.8);
  const neck = frame.anchor(
    new THREE.Vector3(0, -size.y * 0.5 - 0.03, size.z * 0.32),
  );
  neck.group.name = "model-neck-extras";
  neck.group.scale.setScalar(size.x / 0.8);
  return {
    head: head.group,
    neck: neck.group,
    update() {
      head.update();
      neck.update();
    },
    dispose() {
      head.dispose();
      neck.dispose();
    },
  };
}

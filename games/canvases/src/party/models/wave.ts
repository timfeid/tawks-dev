import { Bone, Object3D, Quaternion, Vector3 } from 'three';

/** A greeting over the model's idle pose, using joint directions rather than bind axes. */
export function wavePose(root: Object3D) {
  const joints: { bone: Bone; child: Bone; forearm: boolean }[] = [];
  root.traverse(node => {
    if (!(node as Bone).isBone) return;
    const bone = node as Bone;
    const name = bone.name.toLowerCase().replace(/[_ .-]/g, '');
    if (!/(right|r)$|right/.test(name)) return;
    if (!/upperarm|lowerarm|forearm|rightarm$/.test(name)) return;
    const child = bone.children.find(node => (node as Bone).isBone) as Bone | undefined;
    if (child) joints.push({ bone, child, forearm: /lowerarm|forearm/.test(name) });
  });
  const original = new Map<Bone, Quaternion>();
  let started: number | null = null;
  return {
    restore() { for (const [bone, rotation] of original) bone.quaternion.copy(rotation); original.clear(); },
    reset() { started = null; },
    apply(time: number, waveFor = 3) {
      started ??= time;
      const elapsed = time - started;
      const weight = Math.max(0, Math.min(1, elapsed / .2, (waveFor - elapsed) / .3));
      root.updateWorldMatrix(true, true);
      const rootRotation = root.getWorldQuaternion(new Quaternion());
      for (const { bone, child, forearm } of joints) {
        const saved = bone.quaternion.clone();original.set(bone, saved);
        const current = child.getWorldPosition(new Vector3()).sub(bone.getWorldPosition(new Vector3())).normalize();
        const direction = forearm ? new Vector3(Math.sin(time * 8) * .3, 1, .1) : new Vector3(-.65, .75, .12);
        const target = direction.normalize().applyQuaternion(rootRotation);
        const world = new Quaternion().setFromUnitVectors(current, target).multiply(bone.getWorldQuaternion(new Quaternion()));
        const parent = bone.parent?.getWorldQuaternion(new Quaternion()).invert() ?? new Quaternion();
        bone.quaternion.copy(saved).slerp(parent.multiply(world), weight);bone.updateWorldMatrix(false, true);
      }
    },
  };
}

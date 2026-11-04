import * as THREE from "three";
import { buildHead } from "./built-heads";
import { toon } from "../toon";
import { attachPumpkin } from "./pumpkin";
import { headFrame } from "./head-frame";
import { HEAD_PROPORTIONS, swappedHeadWidth } from "./head-sizes";

/** A head worn in place of the round one, sized in the round head's units (a 0.4 ball about the origin). */
export type SwappedHead = {
  /** The pumpkin is an authored hollow mesh; built heads are solid and keep the round head's footing. */
  kind: "pumpkin" | "built";
  group: THREE.Group;
  ready: Promise<boolean>;
  /** The skull hats and extras fit to, without decoration like tail feathers. Filled once ready. */
  size: THREE.Vector3;
  /** How far above the centre the head and its decoration reach. */
  top: () => number;
  dispose(): void;
};

export const isSwappedHead = (id: string) => id in HEAD_PROPORTIONS;
const PUMPKIN_WIDTH = 0.94;

/** Puts head `id` on `parent`, `width` across (its natural width when left out); null for the round head or an unknown id. */
export function attachHead(
  id: string | null | undefined,
  parent: THREE.Object3D,
  width?: number,
): SwappedHead | null {
  if (id === "head-pumpkin") {
    const pumpkin = attachPumpkin(parent, width ?? PUMPKIN_WIDTH);
    return { kind: "pumpkin", ...pumpkin, top: () => pumpkin.size.y / 2 };
  }
  const built = id ? buildHead(id, toon) : null;
  if (!built) return null;
  const scale = width ? width / built.size.x : 1;
  built.group.scale.setScalar(scale);
  parent.add(built.group);
  return {
    kind: "built",
    group: built.group,
    ready: Promise.resolve(true),
    size: built.size.clone().multiplyScalar(scale),
    top: () => built.top * scale,
    dispose: built.dispose,
  };
}

/** Follows the actual evaluated head bone after the animation mixer, keeping
 * its bind orientation aligned with the game's +Z face direction. Unsupported
 * non-humanoid uploads keep their appearance rather than a floating head. */
export function attachAnimatedHead(
  wrapper: THREE.Group,
  root: THREE.Object3D,
  id: string,
) {
  const frame = headFrame(wrapper, root);
  const none = { update() {}, dispose() {}, ready: Promise.resolve() };
  if (!frame) return none;
  const anchor = frame.anchor();
  const head = attachHead(id, anchor.group, swappedHeadWidth(id, frame.size));
  if (!head) {
    anchor.dispose();
    return none;
  }
  return {
    update: anchor.update,
    ready: head.ready.then(() => {}),
    dispose() {
      head.dispose();
      anchor.dispose();
    },
  };
}

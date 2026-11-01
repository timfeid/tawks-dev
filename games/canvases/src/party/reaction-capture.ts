import * as THREE from "three";
import { buildCharacter, type CharacterRig } from "./characters";
import { buildBag, hangBag } from "./bags";
import { characterFor } from "./roster";
import type { Costume } from "./protocol";
import { encodeOneShotPng } from "./apng";

export type AvatarAnimation = "wave" | "celebrate";
export const REACTION_DURATION = 6000;
export const REACTION_SIZE = 320;
const FPS = 20;
const FRAMES = (REACTION_DURATION / 1000) * FPS;

function png(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (blob) =>
        blob
          ? resolve(blob)
          : reject(new Error("Could not capture your avatar")),
      "image/png",
    ),
  );
}

/** Only the sender renders 3D. Recipients receive ordinary image attachments. */
export async function captureAvatarReaction(
  costume: Costume,
  animation: AvatarAnimation,
  onProgress: (fraction: number) => void = () => {},
  signal?: AbortSignal,
): Promise<{ animation: Blob; still: Blob }> {
  signal?.throwIfAborted();
  let renderer: THREE.WebGLRenderer | undefined;
  let rig: CharacterRig | undefined;
  let unhang: (() => void) | undefined;
  try {
    renderer = new THREE.WebGLRenderer({
      alpha: true,
      antialias: true,
      preserveDrawingBuffer: true,
    });
    renderer.setPixelRatio(1);
    renderer.setSize(REACTION_SIZE, REACTION_SIZE, false);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.2;
    const scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight("#ffffff", "#64748b", 2.4));
    const light = new THREE.DirectionalLight("#fff4db", 2.8);
    light.position.set(-3, 5, 4);
    scene.add(light);
    rig = buildCharacter(characterFor(costume));
    await rig.ready;
    signal?.throwIfAborted();
    unhang = hangBag(rig.group, buildBag(costume.bag), 1.1);
    scene.add(rig.group);
    rig.group.rotation.y = -0.18;
    rig.animate("idle", 0, 0);
    scene.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(rig.group);
    const size = box.getSize(new THREE.Vector3());
    const center = box.getCenter(new THREE.Vector3());
    // Leave space for raised hands and a hop without resizing between frames.
    const extent = Math.max(size.y * 1.45, size.x * 1.3);
    const camera = new THREE.OrthographicCamera(
      -extent / 2,
      extent / 2,
      extent / 2,
      -extent / 2,
      0.01,
      30,
    );
    camera.position.set(center.x, center.y + 0.12, 6);
    camera.lookAt(center.x, center.y + 0.12, 0);
    const frames: Uint8Array[] = [];
    let still: Blob | undefined;
    for (let i = 0; i < FRAMES; i++) {
      signal?.throwIfAborted();
      // Finish a celebration on the ground with both hands up.
      const end = animation === "celebrate" ? 2 * Math.PI : 6;
      const time = (end * i) / (FRAMES - 1);
      rig.animate(animation === "wave" ? "wave" : "cheer", time, 0, end);
      renderer.render(scene, camera);
      still = await png(renderer.domElement);
      frames.push(new Uint8Array(await still.arrayBuffer()));
      onProgress((i + 1) / FRAMES);
      if (i % 4 === 0)
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
    }
    signal?.throwIfAborted();
    const bytes = encodeOneShotPng(frames, FPS);
    if (bytes.length > 12 * 1024 * 1024)
      throw new Error("This avatar reaction is too large to send");
    return {
      animation: new Blob([bytes as Uint8Array<ArrayBuffer>], {
        type: "image/png",
      }),
      still: still!,
    };
  } finally {
    unhang?.();
    rig?.dispose();
    renderer?.dispose();
    renderer?.forceContextLoss();
  }
}

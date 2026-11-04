import { Quaternion, Vector3, type PerspectiveCamera } from 'three';

type CameraPose = { position: Vector3; quaternion: Quaternion; fov: number; zoom: number };
const pose = (): CameraPose => ({ position: new Vector3(), quaternion: new Quaternion(), fov: 50, zoom: 1 });
const read = (into: CameraPose, camera: PerspectiveCamera) => {
  into.position.copy(camera.position);
  into.quaternion.copy(camera.quaternion).normalize();
  into.fov = camera.fov;
  into.zoom = camera.zoom;
};
const copy = (into: CameraPose, from: CameraPose) => {
  into.position.copy(from.position);
  into.quaternion.copy(from.quaternion);
  into.fov = from.fov;
  into.zoom = from.zoom;
};

/** Presentation-only camera handoffs. Compute the desired camera normally,
 * then call apply immediately before rendering (and before camera-based HUD
 * layout). `now` is the caller's monotonic render time in SECONDS.
 *
 * A stable mode key keeps mouse look / normal tracking immediate. Change the
 * key only when a view takes or releases camera control: orbit, vacuum, shop,
 * storm, item shot, etc. The caller retains its own orbit/zoom preferences and
 * computes that desired view again when the forced shot ends.
 *
 * Each handoff starts at the last pose actually rendered by this helper, even
 * when another handoff interrupts it. The destination is refreshed every
 * frame, so a moving character remains the target throughout the transition.
 * No server/game clock, input, desired-view state, or device clock is touched.
 */
export class CameraFlow {
  private readonly duration: number;
  private readonly last = pose();
  private readonly origin = pose();
  private readonly desired = pose();
  private key: string | null = null;
  private startedAt: number | null = null;
  private lastTime = 0;
  private requested = false;

  constructor(options: { duration?: number } = {}) {
    const duration = options.duration ?? 0.48;
    this.duration = Number.isFinite(duration) && duration > 0 ? duration : 0.48;
  }

  get active() { return this.startedAt !== null; }

  /** Explicitly blend a same-mode discontinuity, e.g. a teleport or new item
   * cinematic. Do not call this every frame of an ordinary moving target. */
  transition() { this.requested = true; }

  /** Forget the previous rendered camera, e.g. when disposing a whole scene. */
  reset() { this.key = null; this.startedAt = null; this.lastTime = 0; this.requested = false; }

  /** Mutates only the rendered camera pose/projection. Returns true while a
   * handoff is blending; false for initialization or immediate normal input. */
  apply(camera: PerspectiveCamera, key: string, now: number): boolean {
    read(this.desired, camera);
    const time = Number.isFinite(now) ? now : this.lastTime;
    if (this.key === null) {
      this.key = key;
      this.requested = false;
    } else if (key !== this.key || this.requested || (this.active && time < this.lastTime)) {
      copy(this.origin, this.last);
      this.startedAt = time;
      this.key = key;
      this.requested = false;
    }

    if (this.startedAt !== null) {
      const progress = Math.max(0, Math.min(1, (time - this.startedAt) / this.duration));
      // Zero endpoint slope avoids a sharp camera kick when control changes.
      const amount = progress * progress * (3 - 2 * progress);
      camera.position.lerpVectors(this.origin.position, this.desired.position, amount);
      // Three's slerp chooses the shortest arc and treats q and -q as the
      // same orientation, avoiding full spins at equivalent rotations.
      camera.quaternion.slerpQuaternions(this.origin.quaternion, this.desired.quaternion, amount);
      camera.fov = this.origin.fov + (this.desired.fov - this.origin.fov) * amount;
      camera.zoom = this.origin.zoom + (this.desired.zoom - this.origin.zoom) * amount;
      if (progress >= 1) this.startedAt = null;
    }
    camera.updateProjectionMatrix();
    camera.updateMatrixWorld();
    read(this.last, camera);
    this.lastTime = time;
    return this.active;
  }
}

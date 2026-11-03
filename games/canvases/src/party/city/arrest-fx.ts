import * as THREE from "three";
import type { CityArrest, CityImpact, CityView } from "../protocol";

type Anchor = { at: THREE.Vector3; yaw: number; visible: boolean };
type Hold = {
  cuffs: THREE.Group;
  bar: THREE.Sprite;
  canvas: HTMLCanvasElement;
  texture: THREE.CanvasTexture;
  shown: string;
  tether: THREE.Line;
};
type Ring = { mesh: THREE.Mesh; age: number; life: number; grow: number };

const IMPACT_COLORS: Partial<Record<CityImpact["kind"], string>> = {
  ram: "#ff9a52",
  shove: "#ffd36b",
  grab: "#7acfff",
  cuffed: "#7acfff",
  escape: "#baff52",
  jailed: "#7acfff",
  slip: "#ff6b81",
  struggle: "#ffce7b",
  perfect: "#baff52",
  fumble: "#ff6b81",
};

/** Cuffs, drag tether, progress bars and impact shockwaves for City Heist. */
export class ArrestFx {
  readonly group = new THREE.Group();
  private holds = new Map<number, Hold>();
  private rings: Ring[] = [];
  private seen = new Set<number>();
  private arrests: CityArrest[] = [];
  private ringGeometry = new THREE.RingGeometry(0.75, 1, 40);
  private cuffGeometry = new THREE.TorusGeometry(0.13, 0.035, 8, 20);
  private cuffMaterial = new THREE.MeshStandardMaterial({
    color: "#c9d6e2",
    metalness: 0.8,
    roughness: 0.25,
  });
  private tetherMaterial = new THREE.LineBasicMaterial({ color: "#d9e6f2" });

  constructor() {
    this.group.name = "city-arrests";
  }
  role(seat: number) {
    const arrest = this.arrests.find(
      (a) => a.officer === seat || a.robber === seat,
    );
    return arrest ? { arrest, officer: arrest.officer === seat } : null;
  }
  sync(view: CityView | null, ground: (x: number, z: number) => number) {
    this.arrests = view?.arrests ?? [];
    for (const impact of view?.impacts ?? []) {
      if (this.seen.has(impact.id)) continue;
      this.seen.add(impact.id);
      const color = IMPACT_COLORS[impact.kind];
      if (!color || impact.age > 0.5) continue;
      const mesh = new THREE.Mesh(
        this.ringGeometry,
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: 0.9,
          depthWrite: false,
          side: THREE.DoubleSide,
        }),
      );
      mesh.rotation.x = -Math.PI / 2;
      mesh.position.set(impact.x, ground(impact.x, impact.z) + 0.2, impact.z);
      this.group.add(mesh);
      const big = impact.kind === "ram" || impact.kind === "shove";
      this.rings.push({
        mesh,
        age: 0,
        life: big ? 0.7 : 0.5,
        grow: big ? 5 : 2.6,
      });
    }
    if (this.seen.size > 512) this.seen = new Set([...this.seen].slice(-256));
    const live = new Set(this.arrests.map((a) => a.robber));
    for (const [seat, hold] of this.holds)
      if (!live.has(seat)) {
        this.disposeHold(hold);
        this.holds.delete(seat);
      }
    for (const arrest of this.arrests)
      if (!this.holds.has(arrest.robber))
        this.holds.set(arrest.robber, this.makeHold());
  }
  private makeHold(): Hold {
    const cuffs = new THREE.Group();
    for (const x of [-0.12, 0.12]) {
      const ring = new THREE.Mesh(this.cuffGeometry, this.cuffMaterial);
      ring.position.x = x;
      cuffs.add(ring);
    }
    const canvas = document.createElement("canvas");
    canvas.width = 256;
    canvas.height = 48;
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const bar = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: texture,
        transparent: true,
        depthTest: false,
      }),
    );
    bar.scale.set(1.8, 0.34, 1);
    bar.renderOrder = 5;
    const tether = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(),
        new THREE.Vector3(),
      ]),
      this.tetherMaterial,
    );
    tether.frustumCulled = false;
    this.group.add(cuffs, bar, tether);
    return { cuffs, bar, canvas, texture, shown: "", tether };
  }
  private draw(hold: Hold, arrest: CityArrest) {
    const cuffing = arrest.stage === "cuffing";
    const value = cuffing ? arrest.progress : arrest.escape;
    const key = `${arrest.stage}:${Math.round(value * 40)}`;
    if (key === hold.shown) return;
    hold.shown = key;
    const c = hold.canvas.getContext("2d")!;
    c.clearRect(0, 0, 256, 48);
    c.fillStyle = "#08131ee8";
    c.beginPath();
    c.roundRect(0, 4, 256, 40, 12);
    c.fill();
    c.fillStyle = cuffing ? "#7acfff" : "#baff52";
    c.beginPath();
    c.roundRect(8, 12, 240 * Math.max(0.02, value), 24, 8);
    c.fill();
    c.fillStyle = "#f3f8ff";
    c.font = "800 18px sans-serif";
    c.textAlign = "center";
    c.textBaseline = "middle";
    c.fillText(cuffing ? "CUFFING" : "BREAKING FREE", 128, 25);
    hold.texture.needsUpdate = true;
  }
  update(dt: number, time: number, anchor: (seat: number) => Anchor | null) {
    for (const arrest of this.arrests) {
      const hold = this.holds.get(arrest.robber);
      const robber = anchor(arrest.robber);
      const officer = anchor(arrest.officer);
      if (!hold) continue;
      const shown = !!robber?.visible;
      hold.cuffs.visible = hold.bar.visible = shown;
      hold.tether.visible =
        shown && !!officer?.visible && arrest.stage === "escort";
      if (!robber || !shown) continue;
      const wrists = new THREE.Vector3(
        robber.at.x - Math.sin(robber.yaw) * 0.28,
        robber.at.y + 0.85,
        robber.at.z - Math.cos(robber.yaw) * 0.28,
      );
      const closing = arrest.stage === "cuffing" ? arrest.progress : 1;
      hold.cuffs.position.copy(wrists);
      hold.cuffs.rotation.y = robber.yaw;
      hold.cuffs.scale.setScalar(0.4 + 0.6 * closing);
      hold.cuffs.children[0].position.x = -0.12 - 0.2 * (1 - closing);
      hold.cuffs.children[1].position.x = 0.12 + 0.2 * (1 - closing);
      hold.bar.position.set(robber.at.x, robber.at.y + 3.2, robber.at.z);
      this.draw(hold, arrest);
      if (hold.tether.visible && officer) {
        const points = hold.tether.geometry.attributes
          .position as THREE.BufferAttribute;
        const sag = 0.1 + 0.05 * Math.sin(time * 6);
        points.setXYZ(0, officer.at.x, officer.at.y + 0.9, officer.at.z);
        points.setXYZ(1, wrists.x, wrists.y - sag, wrists.z);
        points.needsUpdate = true;
      }
    }
    for (const ring of this.rings) {
      ring.age += dt;
      const k = ring.age / ring.life;
      ring.mesh.scale.setScalar(0.4 + ring.grow * k);
      (ring.mesh.material as THREE.MeshBasicMaterial).opacity = 0.9 * (1 - k);
    }
    this.rings = this.rings.filter((ring) => {
      if (ring.age < ring.life) return true;
      ring.mesh.removeFromParent();
      (ring.mesh.material as THREE.Material).dispose();
      return false;
    });
  }
  private disposeHold(hold: Hold) {
    hold.cuffs.removeFromParent();
    hold.bar.removeFromParent();
    hold.bar.material.dispose();
    hold.texture.dispose();
    hold.tether.removeFromParent();
    hold.tether.geometry.dispose();
  }
  dispose() {
    for (const hold of this.holds.values()) this.disposeHold(hold);
    this.holds.clear();
    for (const ring of this.rings)
      (ring.mesh.material as THREE.Material).dispose();
    this.ringGeometry.dispose();
    this.cuffGeometry.dispose();
    this.cuffMaterial.dispose();
    this.tetherMaterial.dispose();
    this.group.removeFromParent();
  }
}

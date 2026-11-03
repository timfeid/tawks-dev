import * as THREE from "three";
import { PartyBoard } from "../board/scene";
import { buildCharacter, type CharacterRig } from "../characters";
import { characterAppearanceKey } from "../appearance";
import { place, placed } from "../board/models";
import { ArrestFx } from "./arrest-fx";
import type { BoardView, CharacterDef, CityView, Member } from "../protocol";

type Actor = {
  rig: CharacterRig;
  key: string;
  at: THREE.Vector3;
  target: THREE.Vector3;
  yaw: number;
  visible: boolean;
  view: CityView["actors"][number];
  ring: THREE.Mesh;
  tag: THREE.Sprite;
  beam: THREE.Mesh;
  car: THREE.Group | null;
  carAt: THREE.Vector3;
  sirens: THREE.Mesh[];
};
type Resource = THREE.BufferGeometry | THREE.Material | THREE.Texture;

/** Shared city scenery with independent actors; no board tiles, turns or prizes. */
export class CityHeistScene {
  readonly environment: PartyBoard;
  readonly group = new THREE.Group();
  private actors = new Map<number, Actor>();
  private resources: Resource[] = [];
  private markers = new THREE.Group();
  private smoke = new THREE.Group();
  private ready = false;
  private markerKey = "";
  private icons = new Map<string, THREE.CanvasTexture>();
  private lights: THREE.PointLight[] = [];
  private lampPositions: THREE.Vector3[] = [];
  private nearestLamps: THREE.Vector3[] = [];
  private lastLights = 0;
  private oldPosition = new THREE.Vector3();
  private fallbackFocus = new THREE.Vector3(-28, 0, 28);
  private ownSeat: number | null = null;
  private active = false;
  private van: THREE.Object3D | null = null;
  private vanLoading = false;
  private hasEscape = false;
  private lootBag = new THREE.Group();
  private lootAt: [number, number] | null = null;
  private carrier: number | null = null;
  private dead = false;
  private routeMarkers: THREE.InstancedMesh;
  private fx = new ArrestFx();

  constructor(scene: THREE.Scene, mobile: boolean) {
    this.environment = new PartyBoard(scene, {
      detail: mobile ? 0.45 : 1,
      city: true,
    });
    this.group.name = "city-heist";
    const arrow = new THREE.Shape();
    arrow.moveTo(0, 0.6);
    arrow.lineTo(-0.28, -0.3);
    arrow.lineTo(0, -0.12);
    arrow.lineTo(0.28, -0.3);
    arrow.closePath();
    const geometry = this.track(new THREE.ShapeGeometry(arrow));
    geometry.rotateX(Math.PI / 2);
    this.routeMarkers = new THREE.InstancedMesh(
      geometry,
      this.track(
        new THREE.MeshBasicMaterial({
          color: "#7acfff",
          transparent: true,
          opacity: 0.7,
          depthWrite: false,
          side: THREE.DoubleSide,
        }),
      ),
      32,
    );
    this.routeMarkers.name = "police-street-arrows";
    this.routeMarkers.count = 0;
    this.routeMarkers.frustumCulled = false;
    this.group.add(this.routeMarkers);
    this.group.add(this.markers, this.smoke, this.lootBag, this.fx.group);
    scene.add(this.group);
    this.lootBag.name = "stolen-cash-bag";
    this.box(this.lootBag, [0.65, 0.5, 0.4], [0, 0.25, 0], "#8ba16c");
    this.box(this.lootBag, [0.28, 0.12, 0.12], [0, 0.56, 0], "#23332b");
    this.box(this.lootBag, [0.18, 0.34, 0.43], [0, 0.26, 0], "#e1e4c4");
    this.lootBag.visible = false;
    for (let i = 0; i < (mobile ? 1 : 2); i++) {
      const light = new THREE.PointLight("#ffcb78", 16, 18, 2);
      light.castShadow = false;
      this.group.add(light);
      this.lights.push(light);
    }
    this.visible = false;
  }
  set visible(on: boolean) {
    this.active = on;
    this.group.visible = on;
    this.environment.visible = on;
  }
  get visible() {
    return this.active;
  }
  private track<T extends Resource>(value: T): T {
    this.resources.push(value);
    return value;
  }
  private ground(x: number, z: number) {
    return this.environment.groundAt(x, z) + 0.15;
  }
  private box(
    parent: THREE.Object3D,
    size: [number, number, number],
    at: [number, number, number],
    color: string,
    glow = false,
  ) {
    const material = this.track(
      new THREE.MeshStandardMaterial({
        color,
        roughness: 0.7,
        ...(glow ? { emissive: color, emissiveIntensity: 1.4 } : {}),
      }),
    );
    const mesh = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(...size)),
      material,
    );
    mesh.position.set(...at);
    parent.add(mesh);
    return mesh;
  }
  private label(text: string, color: string, width = 4) {
    const canvas = document.createElement("canvas");
    canvas.width = 512;
    canvas.height = 96;
    const c = canvas.getContext("2d")!;
    c.fillStyle = "#030912c0";
    c.fillRect(0, 0, 512, 96);
    c.fillStyle = color;
    c.font = "900 42px sans-serif";
    c.textAlign = "center";
    c.textBaseline = "middle";
    c.fillText(text, 256, 50, 490);
    const texture = this.track(new THREE.CanvasTexture(canvas));
    texture.colorSpace = THREE.SRGBColorSpace;
    const sprite = new THREE.Sprite(
      this.track(
        new THREE.SpriteMaterial({
          map: texture,
          transparent: true,
          depthTest: true,
        }),
      ),
    );
    sprite.scale.set(width, (width * 96) / 512, 1);
    return sprite;
  }
  /** Original lightweight patrol car: no new downloads or realtime lights. */
  private patrolCar() {
    const car = new THREE.Group();
    car.name = "police-patrol-car";
    this.box(car, [1.95, 0.65, 3.7], [0, 0.65, 0], "#e7edf2");
    this.box(car, [1.65, 0.7, 1.7], [0, 1.28, -0.15], "#203d50");
    this.box(car, [1.76, 0.13, 1.8], [0, 1.7, -0.15], "#e7edf2");
    this.box(car, [1.85, 0.1, 1.0], [0, 1.03, 1.28], "#142532");
    for (const x of [-0.99, 0.99])
      this.box(car, [0.04, 0.3, 2.9], [x, 0.8, 0], "#142532");
    this.box(car, [1.5, 0.13, 0.06], [0, 0.88, 1.88], "#ffedbd", true);
    const red = this.box(
      car,
      [0.65, 0.2, 0.3],
      [-0.37, 1.91, -0.15],
      "#ff526e",
      true,
    );
    const blue = this.box(
      car,
      [0.65, 0.2, 0.3],
      [0.37, 1.91, -0.15],
      "#5dabff",
      true,
    );
    const wheels = new THREE.InstancedMesh(
      this.track(new THREE.CylinderGeometry(0.36, 0.36, 0.24, 12)),
      this.track(
        new THREE.MeshStandardMaterial({ color: "#16212b", roughness: 0.9 }),
      ),
      4,
    );
    const transform = new THREE.Object3D();
    let i = 0;
    for (const x of [-1, 1])
      for (const z of [-1.18, 1.18]) {
        transform.position.set(x, 0.36, z);
        transform.rotation.z = Math.PI / 2;
        transform.updateMatrix();
        wheels.setMatrixAt(i++, transform.matrix);
      }
    wheels.instanceMatrix.needsUpdate = true;
    car.add(wheels);
    return { car, sirens: [red, blue] };
  }
  private objectiveIcon(label: string) {
    const key = label.startsWith("POWER")
      ? "power"
      : label.startsWith("BANK VAULT")
        ? "vault"
        : label.startsWith("ESCAPE")
          ? "van"
          : label.startsWith("JAIL")
            ? "jail"
            : "alarm";
    let texture = this.icons.get(key);
    if (!texture) {
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 128;
      const c = canvas.getContext("2d")!;
      c.fillStyle = "#081a25e8";
      c.beginPath();
      c.arc(64, 64, 58, 0, Math.PI * 2);
      c.fill();
      c.strokeStyle = "#baff83";
      c.lineWidth = 4;
      c.stroke();
      c.save();
      c.translate(28, 28);
      c.scale(3, 3);
      c.strokeStyle = "#d6ffb6";
      c.lineWidth = 1.7;
      c.lineCap = "round";
      c.lineJoin = "round";
      // Original line drawings matching the HUD's visual vocabulary.
      const paths: Record<string, string> = {
        power: "M8 2v5M16 2v5M5 7h14v5a7 7 0 0 1-14 0zM12 19v4",
        vault: "M3 3h18v18H3zM8 3v18M12 9h6v6h-6zM15 9v6M12 12h6",
        van: "M2 6h13v12H2zM15 10h4l3 4v4h-7M5 18v3M18 18v3M16 14h5",
        jail: "M4 3v18M9 3v18M15 3v18M20 3v18M2 7h20M2 17h20",
        alarm: "M5 18h14l-2-3V9a5 5 0 0 0-10 0v6zM10 21h4M2 5l2 2M22 5l-2 2",
      };
      c.stroke(new Path2D(paths[key]));
      c.restore();
      texture = this.track(new THREE.CanvasTexture(canvas));
      texture.colorSpace = THREE.SRGBColorSpace;
      this.icons.set(key, texture);
    }
    const sprite = new THREE.Sprite(
      this.track(
        new THREE.SpriteMaterial({
          map: texture,
          transparent: true,
          depthTest: true,
        }),
      ),
    );
    sprite.scale.set(1.6, 1.6, 1);
    return sprite;
  }
  private build(view: CityView) {
    this.ready = true;
    const bank = new THREE.Group();
    bank.position.set(view.bank[0], this.ground(...view.bank), view.bank[1]);
    bank.name = "heist-bank-vault";
    this.group.add(bank);
    this.box(bank, [3.5, 2.5, 0.8], [0, 1.25, 0], "#40515e");
    this.box(bank, [2.6, 1.9, 0.12], [0, 1.2, 0.47], "#85979e");
    for (const x of [-0.8, 0, 0.8])
      this.box(bank, [0.12, 1.6, 0.15], [x, 1.2, 0.57], "#263742");
    this.box(bank, [3.7, 0.18, 1.1], [0, 2.6, 0], "#54d5bd", true);
    const sign = this.label("FROSTLINE BANK", "#afffdd", 5.7);
    sign.position.set(0, 3.3, 0);
    bank.add(sign);
    const power = new THREE.Group();
    power.position.set(-12, this.ground(-12, 28), 28);
    power.name = "heist-power-switch";
    this.group.add(power);
    this.box(power, [0.9, 1.6, 0.6], [0, 0.8, 0], "#38484f");
    this.box(power, [0.15, 0.55, 0.25], [0, 1, 0.37], "#fbbe44", true);
    const jail = new THREE.Group();
    jail.name = "heist-jail";
    jail.position.set(
      view.jail[0],
      this.ground(...view.jail) - 0.15,
      view.jail[1],
    );
    this.group.add(jail);
    this.box(jail, [6, 0.2, 4], [0, 0.12, 0], "#283545");
    for (const x of [-3, -2, -1, 0, 1, 2, 3])
      for (const z of [-2, 2])
        this.box(jail, [0.1, 3, 0.1], [x, 1.6, z], "#8b9ba8");
    for (const y of [0.45, 2.95])
      this.box(jail, [6.2, 0.14, 4.2], [0, y, 0], "#40536a");
    const jailSign = this.label("POLICE · HOLDING", "#87cfff", 5.8);
    jailSign.position.set(0, 3.6, 0);
    jail.add(jailSign);
    this.buildLamps();
  }
  private buildLamps() {
    const pools = new THREE.InstancedMesh(
      this.track(new THREE.CircleGeometry(4.4, 24)),
      this.track(
        new THREE.MeshBasicMaterial({
          color: "#f0b655",
          transparent: true,
          opacity: 0.12,
          depthWrite: false,
        }),
      ),
      36,
    );
    const poles = new THREE.InstancedMesh(
      this.track(new THREE.BoxGeometry(0.14, 4, 0.14)),
      this.track(new THREE.MeshStandardMaterial({ color: "#293a49" })),
      36,
    );
    const lamps = new THREE.InstancedMesh(
      this.track(new THREE.BoxGeometry(0.7, 0.18, 0.7)),
      this.track(
        new THREE.MeshStandardMaterial({
          color: "#ffe0a0",
          emissive: "#ffe0a0",
          emissiveIntensity: 1.4,
        }),
      ),
      36,
    );
    pools.name = "heist-streetlamp-pools";
    poles.name = "streetlamp-poles";
    lamps.name = "streetlamp-fixtures";
    this.group.add(pools, poles, lamps);
    const transform = new THREE.Object3D();
    let i = 0;
    for (const x of [-64, -40, -16, 16, 40, 64])
      for (const z of [-64, -40, -16, 16, 40, 64]) {
        const px = x + 3.6,
          pz = z + 3.6,
          y = this.ground(px, pz) - 0.15;
        transform.rotation.set(0, 0, 0);
        transform.position.set(px, y + 2, pz);
        transform.updateMatrix();
        poles.setMatrixAt(i, transform.matrix);
        transform.position.y = y + 4;
        transform.updateMatrix();
        lamps.setMatrixAt(i, transform.matrix);
        this.lampPositions.push(new THREE.Vector3(px, y + 3.8, pz));
        transform.position.set(px, y + 0.2, pz);
        transform.rotation.set(-Math.PI / 2, 0, 0);
        transform.updateMatrix();
        pools.setMatrixAt(i++, transform.matrix);
      }
    for (const instances of [pools, poles, lamps]) {
      instances.instanceMatrix.needsUpdate = true;
      instances.computeBoundingSphere();
    }
  }
  sync(
    board: BoardView,
    view: CityView | null,
    members: Member[],
    roster: Map<string, CharacterDef>,
    seat: number | null,
  ) {
    this.ownSeat = seat;
    this.environment.sync(
      {
        ...board,
        tokens: [],
        log: [],
        candy_van: null,
        candy_runs: [],
        finale: null,
        driving: [],
        raids: [],
      },
      [],
      roster,
    );
    this.fx.sync(view, (x, z) => this.ground(x, z));
    if (!view) {
      this.hasEscape = false;
      if (this.van) this.van.visible = false;
      for (const actor of this.actors.values()) actor.visible = false;
      this.markers.visible = this.smoke.visible = this.lootBag.visible = false;
      this.routeMarkers.count = 0;
      return;
    }
    this.markers.visible = this.smoke.visible = true;
    if (!this.ready) this.build(view);
    this.hasEscape = !!view.escape;
    if (this.van) this.van.visible = this.hasEscape;
    if (view.escape && !this.van && !this.vanLoading) {
      const escape = view.escape;
      this.vanLoading = true;
      void place(this.group, "candy-van", {
        x: escape[0],
        z: escape[1],
        y: this.ground(...escape) - 0.05,
      }).then((van) => {
        this.vanLoading = false;
        if (!van) return;
        if (this.dead) {
          van.removeFromParent();
          placed.delete(van);
          return;
        }
        van.updateMatrixWorld(true);
        const size = new THREE.Box3()
          .setFromObject(van)
          .getSize(new THREE.Vector3());
        van.scale.multiplyScalar(4.5 / Math.max(size.x, size.z));
        this.van = van;
        van.visible = this.hasEscape;
      });
    }
    const present = new Set<number>();
    for (const actor of view.actors) {
      present.add(actor.seat);
      const member = members.find((m) => m.seat === actor.seat),
        def = member && roster.get(member.character);
      if (!member || !def) continue;
      const key = characterAppearanceKey(def) + actor.side;
      let record = this.actors.get(actor.seat);
      if (record && record.key !== key) {
        this.disposeActor(record);
        this.actors.delete(actor.seat);
        record = undefined;
      }
      if (!record) {
        const rig = buildCharacter(def);
        rig.group.scale.setScalar(0.85);
        rig.group.rotation.order = "YXZ";
        this.group.add(rig.group);
        const color = actor.side === "police" ? "#62c7ff" : "#ffcc65";
        const ring = new THREE.Mesh(
          this.track(new THREE.RingGeometry(0.55, 0.67, 24)),
          this.track(
            new THREE.MeshBasicMaterial({
              color,
              transparent: true,
              opacity: 0.85,
              depthWrite: false,
            }),
          ),
        );
        ring.rotation.x = -Math.PI / 2;
        this.group.add(ring);
        const tag = this.label(member.name, color, 1.8);
        this.group.add(tag);
        const cone = this.track(new THREE.ConeGeometry(0.9, 3, 24, 1, true));
        cone.rotateX(-Math.PI / 2);
        const beam = new THREE.Mesh(
          cone,
          this.track(
            new THREE.MeshBasicMaterial({
              color: "#b8e9ff",
              transparent: true,
              opacity: 0.15,
              depthWrite: false,
              side: THREE.DoubleSide,
            }),
          ),
        );
        this.group.add(beam);
        const at = new THREE.Vector3(
          actor.x ?? 0,
          this.ground(actor.x ?? 0, actor.z ?? 0),
          actor.z ?? 0,
        );
        const vehicle = actor.side === "police" ? this.patrolCar() : null;
        if (vehicle) this.group.add(vehicle.car);
        record = {
          car: vehicle?.car ?? null,
          sirens: vehicle?.sirens ?? [],
          carAt: at.clone(),
          rig,
          key,
          at,
          target: at.clone(),
          yaw: actor.yaw,
          visible: false,
          view: actor,
          ring,
          tag,
          beam,
        };
        this.actors.set(actor.seat, record);
      }
      const hadCar = !!record.view.car;
      record.view = actor;
      if (actor.car && !hadCar)
        record.carAt.set(actor.car[0], this.ground(...actor.car), actor.car[1]);
      const wasVisible = record.visible;
      record.visible = actor.x !== null && actor.z !== null;
      if (record.visible) {
        record.target.set(actor.x!, this.ground(actor.x!, actor.z!) + (actor.height ?? 0), actor.z!);
        // Do not draw a fabricated path from an enemy's last hidden location.
        if (!wasVisible) record.at.copy(record.target);
      }
    }
    for (const [key, actor] of this.actors)
      if (!present.has(key)) {
        this.disposeActor(actor);
        this.actors.delete(key);
      }
    const route = view.patrol_route ?? [];
    this.routeMarkers.count = Math.min(32, route.length);
    const transform = new THREE.Object3D();
    for (let i = 0; i < this.routeMarkers.count; i++) {
      const point = route[i],
        next = route[i + 1] ?? point;
      transform.position.set(point[0], this.ground(...point) + 0.03, point[1]);
      transform.rotation.y = Math.atan2(next[0] - point[0], next[1] - point[1]);
      transform.updateMatrix();
      this.routeMarkers.setMatrixAt(i, transform.matrix);
    }
    this.routeMarkers.instanceMatrix.needsUpdate = true;
    this.lootAt = view.loot;
    this.carrier = view.loot_carrier;
    const markerKey = JSON.stringify(
      view.targets.map((t) => [t.label, t.x, t.z]),
    );
    if (markerKey !== this.markerKey) {
      this.markerKey = markerKey;
      this.markers.clear();
      for (const target of view.targets) {
        const marker = new THREE.Group();
        marker.position.set(
          target.x,
          this.ground(target.x, target.z),
          target.z,
        );
        const ring = new THREE.Mesh(
          this.track(new THREE.RingGeometry(1.8, 2, 32)),
          this.track(
            new THREE.MeshBasicMaterial({
              color: "#baff52",
              transparent: true,
              opacity: 0.75,
              depthWrite: false,
            }),
          ),
        );
        ring.rotation.x = -Math.PI / 2;
        ring.position.y = 0.08;
        marker.add(ring);
        const sign = this.objectiveIcon(target.label);
        sign.position.y = 3.1;
        marker.add(sign);
        this.markers.add(marker);
      }
    }
    while (this.smoke.children.length > view.smokes.length) {
      const object = this.smoke.children.at(-1) as THREE.Mesh;
      object.removeFromParent();
      object.geometry.dispose();
      (object.material as THREE.Material).dispose();
    }
    view.smokes.forEach((s, i) => {
      let cloud = this.smoke.children[i] as THREE.Mesh | undefined;
      if (!cloud) {
        cloud = new THREE.Mesh(
          new THREE.IcosahedronGeometry(1, 2),
          new THREE.MeshBasicMaterial({
            color: "#66768e",
            transparent: true,
            opacity: 0.85,
            depthWrite: false,
          }),
        );
        this.smoke.add(cloud);
      }
      cloud.position.set(s[0], this.ground(s[0], s[1]) + 1.25, s[1]);
      cloud.scale.set(3.3, 2.3, 3.3);
    });
  }
  focus(seat: number | null) {
    return this.actors.get(seat ?? -1)?.at ?? this.fallbackFocus;
  }
  update(time: number, dt: number) {
    if (!this.active) return;
    this.environment.update(time, dt);
    for (const actor of this.actors.values()) {
      actor.rig.group.visible = actor.visible && !actor.view.driving;
      actor.ring.visible = actor.tag.visible = actor.visible;
      if (actor.car) {
        actor.car.visible = !!actor.view.car;
        if (actor.view.car) {
          const [x, z] = actor.view.car;
          actor.carAt.x += (x - actor.carAt.x) * (1 - Math.exp(-dt * 16));
          actor.carAt.z += (z - actor.carAt.z) * (1 - Math.exp(-dt * 16));
          actor.carAt.y = this.ground(actor.carAt.x, actor.carAt.z);
          actor.car.position.copy(actor.carAt);
          if (actor.view.driving) actor.car.rotation.y = actor.yaw;
          for (let i = 0; i < actor.sirens.length; i++)
            (
              actor.sirens[i].material as THREE.MeshStandardMaterial
            ).emissiveIntensity =
              actor.view.driving && Math.floor(time * 3 + i) % 2 === 0
                ? 1.8
                : 0.15;
        }
      }
      actor.beam.visible =
        actor.visible && actor.view.suction && !actor.view.jailed;
      if (!actor.visible) continue;
      this.oldPosition.copy(actor.at);
      actor.at.lerp(actor.target, 1 - Math.exp(-dt * 16));
      actor.rig.group.position.copy(actor.at);
      actor.yaw +=
        Math.atan2(
          Math.sin(actor.view.yaw - actor.yaw),
          Math.cos(actor.view.yaw - actor.yaw),
        ) *
        (1 - Math.exp(-dt * 14));
      const speed = actor.at.distanceTo(this.oldPosition) / Math.max(0.001, dt);
      const role = this.fx.role(actor.view.seat);
      let pose: "fall" | "sad" | "ride" | "idle" | "play" =
        actor.view.stunned > 0
          ? "fall"
          : actor.view.jailed
            ? "sad"
            : speed > 0.1
              ? "ride"
              : "idle";
      let lean = 0,
        wobble = 0;
      if (role?.officer) {
        if (role.arrest.stage === "cuffing") pose = "play";
        else lean = 0.15;
      } else if (role) {
        if (actor.view.stunned <= 0) pose = "sad";
        wobble =
          role.arrest.jolt > 0
            ? Math.sin(time * 45) * 0.28
            : Math.sin(time * 6 + actor.view.seat) * 0.06;
        if (role.arrest.stage === "escort") lean = -0.32;
      }
      actor.rig.group.rotation.set(lean, actor.yaw, wobble);
      actor.rig.animate(pose, time + actor.view.seat, Math.min(7, speed));
      actor.ring.scale.setScalar(actor.view.driving ? 2 : 1);
      actor.ring.position.copy(actor.at);
      actor.ring.position.y += 0.1;
      actor.tag.position.copy(actor.at);
      actor.tag.position.y += 2.6;
      actor.beam.position.set(
        actor.at.x + Math.sin(actor.yaw) * 1.8,
        actor.at.y + 1,
        actor.at.z + Math.cos(actor.yaw) * 1.8,
      );
      actor.beam.rotation.y = actor.yaw;
    }
    this.fx.update(dt, time, (seat) => {
      const actor = this.actors.get(seat);
      return actor
        ? { at: actor.at, yaw: actor.yaw, visible: actor.visible }
        : null;
    });
    const carrier =
      this.carrier === null ? null : this.actors.get(this.carrier);
    this.lootBag.visible = !!this.lootAt || !!carrier?.visible;
    if (carrier?.visible) {
      this.lootBag.position.set(
        carrier.at.x + Math.cos(carrier.yaw) * 0.48,
        carrier.at.y + 0.7,
        carrier.at.z - Math.sin(carrier.yaw) * 0.48,
      );
      this.lootBag.rotation.y = carrier.yaw;
    } else if (this.lootAt)
      this.lootBag.position.set(
        this.lootAt[0],
        this.ground(...this.lootAt),
        this.lootAt[1],
      );
    const focus = this.focus(this.ownSeat);
    if (time - this.lastLights > 0.2) {
      this.lastLights = time;
      this.nearestLamps.length = 0;
      this.nearestLamps.push(...this.lampPositions);
      this.nearestLamps.sort(
        (a, b) => a.distanceToSquared(focus) - b.distanceToSquared(focus),
      );
      for (let i = 0; i < this.lights.length; i++)
        this.lights[i].position.copy(this.nearestLamps[i] ?? focus);
    }
  }
  private disposeActor(actor: Actor) {
    actor.rig.dispose();
    actor.ring.removeFromParent();
    actor.tag.removeFromParent();
    actor.beam.removeFromParent();
    actor.car?.traverse((object) => {
      if ((object as THREE.InstancedMesh).isInstancedMesh)
        (object as THREE.InstancedMesh).dispose();
    });
    actor.car?.removeFromParent();
  }
  dispose() {
    this.dead = true;
    this.fx.dispose();
    for (const actor of this.actors.values()) this.disposeActor(actor);
    this.environment.dispose();
    for (const object of this.smoke.children) {
      (object as THREE.Mesh).geometry.dispose();
      ((object as THREE.Mesh).material as THREE.Material).dispose();
    }
    this.group.traverse((object) => {
      if ((object as THREE.InstancedMesh).isInstancedMesh)
        (object as THREE.InstancedMesh).dispose();
    });
    for (const resource of this.resources) resource.dispose();
    for (const light of this.lights) light.dispose();
    if (this.van) placed.delete(this.van);
    this.group.removeFromParent();
  }
}

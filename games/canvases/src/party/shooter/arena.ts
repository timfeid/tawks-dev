import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { CharacterDef, GameEvent, Member, ShooterBlock, ShooterPlayer, ShooterSetup, ShooterSnap } from '../protocol';
import { CROUCH_EYE, MAX_SPEED, stepLocal, type Court, type LocalInput, type LocalState } from '../fpmove';

const SNAP_ERROR = 2.5;
const NUDGE_ERROR = 0.8;

type FighterView = {
  seat: number;
  characterId: string;
  rig: CharacterRig;
  blaster: THREE.Group;
  position: THREE.Vector3;
  yaw: number;
  alive: boolean;
};

type Tracer = { line: THREE.Mesh; life: number };
type Splat = { sprite: THREE.Sprite; life: number };

function floorTexture(targetRange = false, popgunPit = false, ricochetTable = false, retakeSite = false) {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 512;
  const ctx = canvas.getContext('2d')!;
  const tiles = targetRange ? 12 : 8;
  const size = 512 / tiles;
  for (let x = 0; x < tiles; x++) {
    for (let z = 0; z < tiles; z++) {
      ctx.fillStyle = retakeSite
        ? ((x + z) % 2 ? '#26333b' : '#202b32')
        : ricochetTable
        ? ((x + z) % 2 ? '#153b50' : '#102f43')
        : popgunPit
        ? ((x + z) % 2 ? '#442949' : '#35233d')
        : targetRange
        ? ((x + z) % 2 ? '#293949' : '#23313f')
        : ((x + z) % 2 ? '#f2ead8' : '#e6dcc4');
      ctx.fillRect(x * size, z * size, size, size);
    }
  }
  ctx.strokeStyle = retakeSite ? 'rgba(245,158,11,0.18)' : targetRange ? 'rgba(92, 180, 204, 0.25)' : 'rgba(120,140,120,0.35)';
  ctx.lineWidth = 3;
  for (let i = 0; i <= tiles; i++) {
    ctx.beginPath();
    ctx.moveTo(i * size, 0);
    ctx.lineTo(i * size, 512);
    ctx.moveTo(0, i * size);
    ctx.lineTo(512, i * size);
    ctx.stroke();
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(6, 6);
  return texture;
}

const BLOCK_COLORS = ['#ffd6a5', '#bde0fe', '#ffc8dd', '#caffbf', '#e4d1ff', '#ffe08a'];

function splatTexture() {
  const canvas = document.createElement('canvas');
  canvas.width = 128;
  canvas.height = 128;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#ffffff';
  for (let i = 0; i < 9; i++) {
    const angle = (i / 9) * Math.PI * 2;
    const distance = i === 0 ? 0 : 24 + (i % 3) * 12;
    const radius = i === 0 ? 34 : 16 - (i % 3) * 4;
    ctx.beginPath();
    ctx.arc(64 + Math.cos(angle) * distance, 64 + Math.sin(angle) * distance, radius, 0, Math.PI * 2);
    ctx.fill();
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * A blaster in the character's colours. The held version keeps the paint tank
 * small and well behind the muzzle so it does not fill the screen.
 */
function buildBlaster(def: CharacterDef | undefined, track: (o: THREE.Object3D) => void, held = false) {
  const group = new THREE.Group();
  const body = new THREE.Mesh(
    new THREE.BoxGeometry(0.18, 0.2, 0.62),
    new THREE.MeshStandardMaterial({ color: def?.accent ?? '#3b3b52', roughness: 0.5 }),
  );
  const barrel = new THREE.Mesh(
    new THREE.CylinderGeometry(0.075, 0.09, 0.5, 12),
    new THREE.MeshStandardMaterial({ color: '#f4f4f8', roughness: 0.4 }),
  );
  barrel.rotation.x = Math.PI / 2;
  barrel.position.set(0, 0.02, -0.5);
  const tank = new THREE.Mesh(
    new THREE.SphereGeometry(held ? 0.09 : 0.16, 14, 12),
    new THREE.MeshStandardMaterial({ color: def?.body ?? '#4cc9f0', roughness: 0.25, emissive: def?.body ?? '#4cc9f0', emissiveIntensity: 0.25 }),
  );
  tank.position.set(0, held ? 0.13 : 0.2, held ? 0.3 : 0.18);
  group.add(body, barrel, tank);
  group.traverse((child) => track(child));
  return group;
}

/**
 * Paint Blasters: a walled arena you run around in first person. Your own
 * movement is simulated here and corrected against the server, while everyone
 * else is drawn from the interpolated snapshot.
 */
export class ShooterArena {
  readonly group = new THREE.Group();
  private setup: ShooterSetup | null = null;
  private setupKey = '';
  private built = false;
  private fighters = new Map<number, FighterView>();
  private targets = new Map<number, THREE.Group>();
  private bomb: THREE.Group | null = null;
  private bombLamp: THREE.Mesh<THREE.SphereGeometry, THREE.MeshStandardMaterial> | null = null;
  private bombRing: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial> | null = null;
  private bombFill: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial> | null = null;
  private fuse = 1;
  private bombState: 'ticking' | 'defused' | 'exploded' = 'ticking';
  private tracers: Tracer[] = [];
  private splats: Splat[] = [];
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private splatMap = splatTexture();
  private floorMap: THREE.Texture | null = null;
  private mySeat: number | null = null;
  private local: LocalState | null = null;
  private muzzle = 0;
  private hitAt = 0;
  private view = new THREE.Group();
  private viewBob = 0;
  private bobPower = 0;
  private crouched = false;
  private stepRun = 0;
  private stepped = new Map<number, { x: number; z: number; run: number }>();
  /** Told about every footstep so the canvas can make the noise. */
  onStep: (volume: number, crouched: boolean) => void = () => {};
  private roster = new Map<string, CharacterDef>();

  private court(): Court {
    const setup = this.setup!;
    return { blocks: setup.blocks, radius: setup.radius, halfX: setup.half, halfZ: setup.half_z ?? setup.half };
  }

  /** Set while you are alive and playing, so the camera sits in your head. */
  firstPerson = false;
  shake = 0;

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-shooter';
    this.group.visible = false;
    scene.add(this.group);
  }

  private track<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private trackMaterial<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  private keep = (object: THREE.Object3D) => {
    const mesh = object as THREE.Mesh;
    if (mesh.geometry) this.geometries.push(mesh.geometry);
    if (mesh.material) this.materials.push(mesh.material as THREE.Material);
  };

  private clearBuild() {
    this.fighters.forEach((fighter) => {
      fighter.rig.dispose();
    });
    this.fighters.clear();
    this.targets.clear();
    this.tracers.length = 0;
    this.splats.forEach((splat) => splat.sprite.material.dispose());
    this.splats.length = 0;
    this.group.clear();
    this.view.clear();
    this.view.removeFromParent();
    this.view = new THREE.Group();
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.geometries.length = 0;
    this.materials.length = 0;
    this.floorMap?.dispose();
    this.floorMap = null;
    this.local = null;
    this.built = false;
  }

  /** A purpose-built indoor range, not the Paint Blasters arena with a skin. */
  private buildTargetRange(setup: ShooterSetup) {
    const halfX = setup.half;
    const halfZ = setup.half_z ?? setup.half;
    const ground = floorTexture(true);
    this.floorMap = ground;
    const floor = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(halfX * 2, 0.45, halfZ * 2)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ map: ground, color: '#b7c6cf', roughness: 0.88 })),
    );
    floor.position.y = -0.23;
    floor.receiveShadow = true;
    this.group.add(floor);

    const wallMat = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#172532', roughness: 0.82 }));
    const dividerMat = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#263c4a', roughness: 0.72, metalness: 0.08 }));
    const trimMat = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#36d8b7', emissive: '#126455', emissiveIntensity: 0.42, roughness: 0.4 }));
    const ceilingMat = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#0d1822', roughness: 0.9 }));

    for (const x of [-halfX - 0.25, halfX + 0.25]) {
      const wall = new THREE.Mesh(this.track(new THREE.BoxGeometry(0.5, 5.2, halfZ * 2 + 1)), wallMat);
      wall.position.set(x, 2.6, 0);
      this.group.add(wall);
    }
    const frontWall = new THREE.Mesh(this.track(new THREE.BoxGeometry(halfX * 2 + 1, 5.2, 0.5)), wallMat);
    frontWall.position.set(0, 2.6, halfZ + 0.25);
    this.group.add(frontWall);

    // The server-provided solids are the actual lane partitions and target
    // backstop, so the visible range and authoritative collision map match.
    for (const block of setup.blocks) {
      const divider = block.hx < 0.5 && block.hz > 10;
      const mesh = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(block.hx * 2, block.top, block.hz * 2)),
        divider ? dividerMat : wallMat,
      );
      mesh.position.set(block.x, block.top / 2, block.z);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      const cap = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2 + 0.06, 0.08, block.hz * 2 + 0.06)), trimMat);
      cap.position.set(block.x, block.top + 0.04, block.z);
      this.group.add(cap);
    }

    const dividerXs = setup.blocks.filter((block) => block.hx < 0.5 && block.hz > 10).map((block) => block.x).sort((a, b) => a - b);
    const laneEdges = [-halfX, ...dividerXs, halfX];
    for (let lane = 0; lane < laneEdges.length - 1; lane++) {
      const center = (laneEdges[lane] + laneEdges[lane + 1]) * 0.5;
      const width = laneEdges[lane + 1] - laneEdges[lane];
      const light = new THREE.Mesh(this.track(new THREE.BoxGeometry(Math.max(2, width - 1.1), 0.12, 18)), trimMat);
      light.position.set(center, 4.45, -8);
      this.group.add(light);
      const booth = new THREE.Mesh(this.track(new THREE.BoxGeometry(Math.max(2, width - 1.0), 0.9, 1.3)), dividerMat);
      booth.position.set(center, 0.45, 25.2);
      booth.castShadow = true;
      this.group.add(booth);
      const firingMark = new THREE.Mesh(this.track(new THREE.BoxGeometry(Math.max(2, width - 0.7), 0.04, 0.25)), trimMat);
      firingMark.position.set(center, 0.03, 23.2);
      this.group.add(firingMark);
    }

    // Suspended acoustic baffles and a low ceiling make this read as an
    // interior shooting facility instead of an outdoor combat box.
    for (let z = -26; z <= 24; z += 10) {
      const baffle = new THREE.Mesh(this.track(new THREE.BoxGeometry(halfX * 2 - 1, 0.18, 1.1)), ceilingMat);
      baffle.position.set(0, 4.75, z);
      this.group.add(baffle);
    }
  }

  private build(setup: ShooterSetup) {
    this.setup = setup;
    if (setup.game === 'target') {
      this.buildTargetRange(setup);
      this.built = true;
      return;
    }
    const halfX = setup.half;
    const halfZ = setup.half_z ?? setup.half;
    // The target range builds above and returns, so it never reaches here.
    const isTargetRange = false;
    const isPopgunPit = setup.game === 'popgun';
    const isRicochetTable = setup.game === 'ricochet';
    const isRetakeSite = setup.game === 'retake';
    const ground = floorTexture(isTargetRange, isPopgunPit, isRicochetTable, isRetakeSite);
    this.floorMap = ground;
    const floor = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(halfX * 2, 0.5, halfZ * 2)),
      this.trackMaterial(
        isPopgunPit
          ? new THREE.MeshStandardMaterial({ color: '#557f3b', roughness: 1 })
          : new THREE.MeshStandardMaterial({ map: ground, color: isRetakeSite ? '#8a9aa2' : '#ffffff', roughness: 0.95 }),
      ),
    );
    floor.position.y = -0.25;
    floor.receiveShadow = true;
    this.group.add(floor);

    const wallMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: isRetakeSite ? '#15212a' : isTargetRange ? '#172331' : isPopgunPit ? '#4a3b2a' : isRicochetTable ? '#102637' : '#7f9bb5', roughness: 0.85 }));
    const stripeColor = isRetakeSite ? '#f59e0b' : isTargetRange ? '#32d5b5' : isPopgunPit ? '#5f8f3f' : isRicochetTable ? '#ffb347' : '#ffd23f';
    const stripe = this.trackMaterial(new THREE.MeshStandardMaterial({ color: stripeColor, roughness: 0.6, emissive: isTargetRange ? '#1a766c' : isRicochetTable ? '#80460d' : '#000000', emissiveIntensity: (isTargetRange || isRicochetTable) ? 0.42 : 0 }));
    for (const [dx, dz] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ]) {
      const wall = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(dx ? 0.6 : halfX * 2 + 1.2, 4, dz ? 0.6 : halfZ * 2 + 1.2)),
        wallMaterial,
      );
      wall.position.set(dx * (halfX + 0.3), 2, dz * (halfZ + 0.3));
      wall.receiveShadow = true;
      this.group.add(wall);
      const trim = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(dx ? 0.7 : halfX * 2 + 1.3, 0.25, dz ? 0.7 : halfZ * 2 + 1.3)),
        stripe,
      );
      trim.position.set(dx * (halfX + 0.3), 4.05, dz * (halfZ + 0.3));
      this.group.add(trim);
    }

    if (isTargetRange) {
      const lineMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#74f4d3', emissive: '#248a76', emissiveIntensity: 0.7, roughness: 0.45 }));
      // The lane marks make the range legible at a glance and lead the eye to
      // the target bank instead of looking like the general combat map.
      for (let x = -halfX + 1; x <= halfX - 1; x += 3.5) {
        const mark = new THREE.Mesh(this.track(new THREE.BoxGeometry(0.07, 0.035, 60)), lineMaterial);
        mark.position.set(x, 0.02, -2);
        this.group.add(mark);
      }
      const firingLine = new THREE.Mesh(this.track(new THREE.BoxGeometry(halfX * 2 - 1.5, 0.04, 0.26)), lineMaterial);
      firingLine.position.set(0, 0.03, 23.5);
      this.group.add(firingLine);
      const sign = new THREE.Mesh(this.track(new THREE.BoxGeometry(18, 1.9, 0.18)), this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#263d50', emissive: '#112c3a', emissiveIntensity: 0.35, roughness: 0.55 })));
      sign.position.set(0, 4.3, -35.55);
      this.group.add(sign);
    }
    if (isRicochetTable) {
      const bankMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#ffb347', emissive: '#7d410d', emissiveIntensity: 0.6, roughness: 0.38 }));
      for (const z of [-18, -6, 6, 18]) {
        const chevron = new THREE.Mesh(this.track(new THREE.BoxGeometry(3.8, 0.05, 0.28)), bankMaterial);
        chevron.position.set(0, 0.035, z);
        this.group.add(chevron);
      }
    }
    if (isRetakeSite) {
      const amber = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#fbbf24', emissive: '#d97706', emissiveIntensity: 1.2, roughness: 0.3 }));
      const cyan = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#22d3ee', emissive: '#0891b2', emissiveIntensity: 0.9, roughness: 0.35 }));
      const site = new THREE.Mesh(this.track(new THREE.RingGeometry(2.8, 3.15, 48)), amber);
      site.rotation.x = -Math.PI / 2;
      site.position.y = 0.035;
      this.group.add(site);
      const core = new THREE.Group();
      const caseMesh = new THREE.Mesh(this.track(new THREE.BoxGeometry(1.15, 0.55, 0.85)), this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#141b20', metalness: 0.55, roughness: 0.38 })));
      caseMesh.position.y = 0.3;
      const light = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.12, 0.12, 0.18, 16)), amber);
      light.position.y = 0.66;
      core.add(caseMesh, light);
      this.group.add(core);
      for (const x of [-18, 0, 18]) {
        const entry = new THREE.Mesh(this.track(new THREE.BoxGeometry(3.5, 0.04, 0.22)), cyan);
        entry.position.set(x, 0.03, 19);
        this.group.add(entry);
      }
    }

    const capMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: isRetakeSite ? '#f59e0b' : isTargetRange ? '#3dd6ba' : isPopgunPit ? '#ff7aaa' : isRicochetTable ? '#ffbd59' : '#4fa65f', roughness: 0.7, emissive: isRetakeSite ? '#92400e' : isTargetRange ? '#156f64' : isPopgunPit ? '#711d48' : isRicochetTable ? '#70400f' : '#000000', emissiveIntensity: (isTargetRange || isPopgunPit || isRicochetTable || isRetakeSite) ? 0.2 : 0 }));
    const colors = isRetakeSite ? ['#44515a', '#394650', '#53616a'] : BLOCK_COLORS;
    const blockMaterials = colors.map((color) =>
      this.trackMaterial(new THREE.MeshStandardMaterial({ color, roughness: 0.8 })),
    );
    const nature = isPopgunPit ? this.natureMaterials() : null;
    setup.blocks.forEach((block, index) => {
      if (nature && block.kind) {
        this.natureBlock(block, nature);
        return;
      }
      const blockMaterial = blockMaterials[index % blockMaterials.length];
      const mesh = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2, block.top, block.hz * 2)), blockMaterial);
      mesh.position.set(block.x, block.top / 2, block.z);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      const cap = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2 + 0.12, 0.12, block.hz * 2 + 0.12)), capMaterial);
      cap.position.set(block.x, block.top + 0.06, block.z);
      this.group.add(cap);
    });
    this.built = true;
  }

  private natureMaterials() {
    const make = (color: string, roughness = 0.95) => this.trackMaterial(new THREE.MeshStandardMaterial({ color, roughness, flatShading: true }));
    return {
      dirt: make('#6b4f32'),
      grass: make('#5f8f3f'),
      timber: make('#8a6a44', 0.85),
      timberDark: make('#6e5234', 0.85),
      rock: make('#8b8f94'),
      bark: make('#5b4028'),
      leaves: [make('#3f7a34'), make('#4a8a3a'), make('#356b2d')],
    };
  }

  /** Timberline's pieces: the collision stays a box, the look follows the kind. */
  private natureBlock(block: ShooterBlock, m: ReturnType<ShooterArena['natureMaterials']>) {
    const bottom = block.bottom ?? 0;
    const height = block.top - bottom;
    const midY = bottom + height / 2;
    const add = (mesh: THREE.Mesh, shadow = true) => {
      mesh.castShadow = shadow;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      return mesh;
    };
    switch (block.kind) {
      case 'earth': {
        add(new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2, block.top - 0.1, block.hz * 2)), m.dirt)).position.set(block.x, (block.top - 0.1) / 2, block.z);
        add(new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2 + 0.1, 0.12, block.hz * 2 + 0.1)), m.grass)).position.set(block.x, block.top - 0.05, block.z);
        return;
      }
      case 'step':
      case 'wood': {
        add(new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2, height, block.hz * 2)), block.kind === 'step' ? m.timber : m.timberDark)).position.set(block.x, midY, block.z);
        add(new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2 + 0.04, 0.05, block.hz * 2 + 0.04)), m.timber), false).position.set(block.x, block.top + 0.02, block.z);
        return;
      }
      case 'rock': {
        const rock = add(new THREE.Mesh(this.track(new THREE.DodecahedronGeometry(1, 0)), m.rock));
        rock.scale.set(block.hx * 1.08, block.top / 2, block.hz * 1.08);
        rock.position.set(block.x, block.top / 2, block.z);
        rock.rotation.y = (block.x * 7 + block.z * 3) % Math.PI;
        return;
      }
      case 'log': {
        const along = block.hx >= block.hz;
        const log = add(new THREE.Mesh(this.track(new THREE.CylinderGeometry(block.top / 2, block.top / 2, (along ? block.hx : block.hz) * 2, 10)), m.bark));
        log.rotation.set(along ? 0 : Math.PI / 2, 0, along ? Math.PI / 2 : 0);
        log.position.set(block.x, block.top / 2, block.z);
        return;
      }
      case 'trunk': {
        const trunk = add(new THREE.Mesh(this.track(new THREE.CylinderGeometry(block.hx * 0.8, block.hx * 1.05, block.top, 8)), m.bark));
        trunk.position.set(block.x, block.top / 2, block.z);
        return;
      }
      case 'leaves': {
        const blobs = [
          [0, 0, 0, 1],
          [0.45, -0.1, 0.3, 0.72],
          [-0.4, 0.05, -0.35, 0.7],
          [0.1, 0.25, -0.45, 0.62],
        ];
        blobs.forEach(([dx, dy, dz, r], i) => {
          const blob = add(new THREE.Mesh(this.track(new THREE.IcosahedronGeometry(1, 1)), m.leaves[i % m.leaves.length]));
          blob.scale.set(block.hx * r, (height / 2) * (0.9 + r * 0.2), block.hz * r);
          blob.position.set(block.x + dx * block.hx, midY + dy * height, block.z + dz * block.hz);
        });
        return;
      }
      default: {
        add(new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2, height, block.hz * 2)), m.rock)).position.set(block.x, midY, block.z);
      }
    }
  }

  /** Your own blaster, drawn in front of the camera. */
  viewModel(camera: THREE.Camera, def: CharacterDef | undefined) {
    if (this.view.children.length === 0) {
      const blaster = buildBlaster(def, this.keep, true);
      blaster.scale.setScalar(0.34);
      blaster.position.set(0.42, -0.36, -1.25);
      blaster.rotation.set(0.05, 0.07, 0);
      this.view.add(blaster);
      camera.add(this.view);
    }
    this.view.visible = this.firstPerson;
    return this.view;
  }

  sync(snap: ShooterSnap | null, setup: ShooterSetup | null, members: Member[], roster: Map<string, CharacterDef>, mySeat: number | null) {
    this.roster = roster;
    this.mySeat = mySeat;
    if (setup) {
      const key = JSON.stringify([setup.game, setup.half, setup.half_z, setup.blocks]);
      if (key !== this.setupKey) {
        this.clearBuild();
        this.setupKey = key;
        this.build(setup);
      }
    }
    if (!snap || !this.built) return;

    for (const [seat, fighter] of this.fighters) {
      const member = members.find((m) => m.seat === seat);
      if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== fighter.characterId) {
        fighter.rig.dispose();
        fighter.blaster.removeFromParent();
        this.fighters.delete(seat);
      }
    }

    for (const player of snap.players) {
      const member = members.find((m) => m.seat === player.seat);
      const def = member && roster.get(member.character);
      if (!def || this.fighters.has(player.seat)) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.92);
      const blaster = buildBlaster(def, this.keep);
      blaster.scale.setScalar(0.9);
      rig.group.add(blaster);
      // No name tags: in a shooter they give away where everyone is.
      this.group.add(rig.group);
      this.fighters.set(player.seat, {
        seat: player.seat,
        characterId: def.id,
        rig,
        blaster,
        position: new THREE.Vector3(player.x, player.y, player.z),
        yaw: player.yaw,
        alive: player.alive,
      });
    }

    for (const player of snap.players) {
      const fighter = this.fighters.get(player.seat);
      if (!fighter) continue;
      fighter.position.set(player.x, player.y, player.z);
      fighter.yaw = player.yaw;
      fighter.alive = player.alive;
      if (player.seat === mySeat) this.reconcile(player);
    }

    if (snap.objective) this.syncBomb(snap.objective);

    const liveTargets = new Set((snap.targets ?? []).map((target) => target.id));
    for (const [id, target] of this.targets) {
      if (!liveTargets.has(id)) { target.removeFromParent(); this.targets.delete(id); }
    }
    for (const target of snap.targets ?? []) {
      let view = this.targets.get(target.id);
      if (!view) {
        view = this.buildTarget();
        this.targets.set(target.id, view);
        this.group.add(view);
      }
      view.position.set(target.x, target.y, target.z);
      view.visible = true;
      const lamp = view.userData.lamp as THREE.Group | undefined;
      if (lamp) lamp.visible = target.up;
    }
  }

  /** The planted bomb, its defuse ring and how far the wire has been cut. */
  private syncBomb(objective: NonNullable<ShooterSnap['objective']>) {
    if (!this.bomb) {
      const group = new THREE.Group();
      const body = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(0.85, 0.55, 0.62)),
        this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#3b4354', roughness: 0.45 })),
      );
      body.position.y = 0.28;
      body.castShadow = true;
      const panel = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(0.5, 0.2, 0.02)),
        this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#0f1420', emissive: '#ff3b52', emissiveIntensity: 0.5 })),
      );
      panel.position.set(0, 0.32, 0.32);
      const lamp = new THREE.Mesh(
        this.track(new THREE.SphereGeometry(0.09, 12, 10)),
        this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#ff3b52', emissive: '#ff3b52', emissiveIntensity: 1.2 })),
      );
      lamp.position.set(0, 0.68, 0);
      const ring = new THREE.Mesh(
        this.track(new THREE.RingGeometry(objective.zone - 0.12, objective.zone, 48)),
        this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#ff5f6d', transparent: true, opacity: 0.55, depthWrite: false, side: THREE.DoubleSide })),
      );
      ring.rotation.x = -Math.PI / 2;
      ring.position.y = 0.03;
      const fill = new THREE.Mesh(
        this.track(new THREE.RingGeometry(objective.zone - 0.5, objective.zone - 0.16, 48, 1, -Math.PI / 2, 0.001)),
        this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#8ef5a4', transparent: true, opacity: 0.9, depthWrite: false, side: THREE.DoubleSide })),
      );
      fill.rotation.x = -Math.PI / 2;
      fill.position.y = 0.04;
      const beacon = new THREE.Mesh(
        this.track(new THREE.CylinderGeometry(0.06, 0.06, 6.0, 8, 1, true)),
        this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#ff5f6d', transparent: true, opacity: 0.22, depthWrite: false, side: THREE.DoubleSide })),
      );
      beacon.position.y = 3.0;
      group.add(body, panel, lamp, ring, fill, beacon);
      this.group.add(group);
      this.bomb = group;
      this.bombLamp = lamp;
      this.bombRing = ring;
      this.bombFill = fill;
    }
    this.bomb.position.set(objective.x, 0, objective.z);
    this.bomb.visible = objective.state !== 'exploded';
    this.fuse = objective.fuse_max > 0 ? objective.fuse / objective.fuse_max : 1;
    this.bombState = objective.state;
    if (this.bombFill) {
      const share = objective.needed > 0 ? Math.min(1, objective.progress / objective.needed) : 0;
      this.bombFill.geometry.dispose();
      this.bombFill.geometry = new THREE.RingGeometry(
        Math.max(0.1, (this.bombRing?.geometry.parameters.innerRadius ?? 2) - 0.4),
        Math.max(0.2, this.bombRing?.geometry.parameters.innerRadius ?? 2),
        48,
        1,
        -Math.PI / 2,
        Math.max(0.001, share * Math.PI * 2),
      );
      this.bombFill.visible = share > 0;
    }
    if (this.bombRing) {
      this.bombRing.material.color.set(objective.defusing ? '#8ef5a4' : '#ff5f6d');
    }
  }

  private buildTarget() {
    const group = new THREE.Group();
    const lamp = new THREE.Group();
    const recess = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(1.9, 1.9, 0.12)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#09131c', roughness: 0.5, metalness: 0.25 })),
    );
    recess.position.z = -0.04;
    group.add(recess);
    const makeDisc = (radius: number, depth: number, color: string, z: number) => {
      const mesh = new THREE.Mesh(
        this.track(new THREE.CylinderGeometry(radius, radius, depth, 28)),
        this.trackMaterial(new THREE.MeshStandardMaterial({ color, roughness: 0.32, emissive: color, emissiveIntensity: 0.75 })),
      );
      mesh.rotation.x = Math.PI / 2;
      mesh.position.z = z;
      lamp.add(mesh);
    };
    makeDisc(0.76, 0.11, '#d8fff8', 0.05);
    makeDisc(0.48, 0.13, '#33e0bd', 0.12);
    makeDisc(0.2, 0.15, '#fff36b', 0.2);
    group.add(lamp);
    group.userData.lamp = lamp;
    return group;
  }

  /** Keep the local guess honest without yanking it around. */
  private reconcile(me: ShooterPlayer) {
    if (!this.local || !me.alive) {
      this.local = { x: me.x, y: me.y, z: me.z, vx: me.vx, vy: 0, vz: me.vz, onGround: me.ground };
      return;
    }
    const error = Math.hypot(this.local.x - me.x, this.local.z - me.z);
    if (error > SNAP_ERROR) {
      this.local.x = me.x;
      this.local.z = me.z;
      this.local.y = me.y;
      this.local.vx = me.vx;
      this.local.vz = me.vz;
    } else if (error > NUDGE_ERROR) {
      this.local.x += (me.x - this.local.x) * 0.15;
      this.local.z += (me.z - this.local.z) * 0.15;
    }
  }

  /** Run your own movement for this frame. */
  control(input: LocalInput, dt: number) {
    if (!this.setup || !this.local) return;
    if (this.setup.game === 'target') {
      this.local.vx = 0;
      this.local.vz = 0;
      this.bobPower += (0 - this.bobPower) * Math.min(1, dt * 8);
      return;
    }
    this.crouched = input.crouch && this.local.onGround;
    stepLocal(this.local, input, dt, this.court());
    const speed = Math.hypot(this.local.vx, this.local.vz);
    // Your own steps, unless you are sneaking.
    if (this.local.onGround && !this.crouched) {
      this.stepRun += speed * dt;
      if (this.stepRun > 2.1) {
        this.stepRun = 0;
        this.onStep(0.45, false);
      }
    }
    this.viewBob += speed * dt;
    // The sway fades in and out with your pace instead of snapping on.
    const want = this.local.onGround ? Math.min(1, speed / MAX_SPEED) : 0;
    this.bobPower += (want - this.bobPower) * Math.min(1, dt * 5);
  }

  /** Where the camera goes in first person. */
  eye(yaw: number, pitch: number) {
    const setup = this.setup;
    const at = this.local ?? (this.mySeat !== null ? this.fighters.get(this.mySeat)?.position : null);
    if (!setup || !at) return null;
    const bob = Math.sin(this.viewBob * 4.5) * 0.011 * this.bobPower;
    const stance = this.crouched ? CROUCH_EYE : setup.eye;
    return {
      x: at.x,
      y: at.y + stance + bob,
      z: at.z,
      yaw,
      pitch,
    };
  }

  effect(event: GameEvent, mySeat: number | null, members: Member[]) {
    if (event.kind === 'exploded') {
      this.bombState = 'exploded';
      if (this.bomb) this.bomb.visible = false;
      const at = new THREE.Vector3(event.x, 0.6, event.z);
      for (let i = 0; i < 26; i++) {
        const spread = new THREE.Vector3(Math.random() - 0.5, Math.random() * 0.9, Math.random() - 0.5).multiplyScalar(6);
        this.paint(at.clone().add(spread), i % 2 ? '#ff9f1c' : '#ff3b52');
      }
      this.shake = 0.7;
      return;
    }
    if (event.kind === 'defused') {
      this.bombState = 'defused';
      const objective = this.bomb?.position;
      if (objective) {
        for (let i = 0; i < 12; i++) {
          const spread = new THREE.Vector3(Math.random() - 0.5, Math.random(), Math.random() - 0.5).multiplyScalar(2.4);
          this.paint(objective.clone().add(spread).setY(0.8 + Math.random()), '#8ef5a4');
        }
      }
      return;
    }
    if (event.kind === 'shot') {
      const color = this.colorOf(event.seat, members);
      const from = new THREE.Vector3(...event.from);
      const to = new THREE.Vector3(...event.to);
      if (event.seat === mySeat && this.firstPerson) {
        // Your own tracer starts at the blaster, not inside your head.
        from.y -= 0.12;
        this.muzzle = 0.06;
        this.shake = Math.min(0.12, this.shake + 0.06);
      }
      const length = from.distanceTo(to);
      if (length > 0.2) {
        const geometry = this.track(new THREE.CylinderGeometry(0.035, 0.02, length, 6));
        const material = this.trackMaterial(
          new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.85, depthWrite: false }),
        );
        const line = new THREE.Mesh(geometry, material);
        line.position.copy(from).add(to).multiplyScalar(0.5);
        line.lookAt(to);
        line.rotateX(Math.PI / 2);
        this.group.add(line);
        this.tracers.push({ line, life: 0.12 });
      }
    } else if (event.kind === 'splat') {
      this.paint(new THREE.Vector3(event.x, event.y, event.z), this.colorOf(event.by, members));
      if (event.by === mySeat) this.hitAt = performance.now();
      if (event.seat === mySeat) this.shake = Math.min(0.3, this.shake + 0.18);
    } else if (event.kind === 'down') {
      const fighter = this.fighters.get(event.seat);
      if (fighter) {
        for (let i = 0; i < 8; i++) {
          this.paint(
            fighter.position.clone().add(new THREE.Vector3(Math.random() - 0.5, Math.random() * 1.6, Math.random() - 0.5)),
            this.colorOf(event.by, members),
          );
        }
      }
    } else if (event.kind === 'target_hit') {
      this.paint(new THREE.Vector3(event.x, event.y, event.z), this.colorOf(event.seat, members));
      if (event.seat === mySeat) this.hitAt = performance.now();
    }
  }

  /** True while a hit marker should show on the crosshair. */
  get hitMarker() {
    return performance.now() - this.hitAt < 220;
  }

  private colorOf(seat: number, members: Member[]) {
    const member = members.find((m) => m.seat === seat);
    return (member && this.roster.get(member.character)?.body) ?? '#ffffff';
  }

  private paint(at: THREE.Vector3, color: string) {
    const material = new THREE.SpriteMaterial({ map: this.splatMap, color, transparent: true, opacity: 0.95, depthWrite: false });
    const sprite = new THREE.Sprite(material);
    sprite.position.copy(at);
    sprite.scale.setScalar(0.5 + Math.random() * 0.5);
    sprite.renderOrder = 22;
    this.group.add(sprite);
    this.splats.push({ sprite, life: 1.4 });
  }

  update(snap: ShooterSnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.4);
    this.muzzle = Math.max(0, this.muzzle - dt);
    for (const [seat, fighter] of this.fighters) {
      const player = snap?.players.find((p) => p.seat === seat);
      const hidden = this.firstPerson && seat === this.mySeat;
      fighter.rig.group.visible = !hidden;
      if (!player) continue;
      const at = seat === this.mySeat && this.local ? this.local : player;
      fighter.rig.group.position.set(at.x, player.alive ? at.y : at.y - 0.1, at.z);
      fighter.rig.group.rotation.y = fighter.yaw + Math.PI;
      fighter.blaster.position.set(0.32, 0.62, -0.34);
      fighter.blaster.rotation.set(-player.pitch, 0, 0);
      const moving = Math.hypot(player.vx, player.vz) > 0.6;
      fighter.rig.animate(player.alive ? (moving ? 'ride' : 'idle') : 'fall', time + seat * 0.3, moving ? 7 : 0);
      fighter.rig.group.scale.y = player.crouch ? 0.62 : 0.92;
      if (seat !== this.mySeat && player.alive) this.listen(seat, player, dt);
    }
    if (this.bombLamp) {
      const urgency = 1.5 + (1 - this.fuse) * 9;
      const on = this.bombState === 'ticking' ? Math.sin(time * urgency * Math.PI) > -0.2 : this.bombState === 'defused';
      this.bombLamp.material.color.set(this.bombState === 'defused' ? '#8ef5a4' : '#ff3b52');
      this.bombLamp.material.emissive.set(this.bombState === 'defused' ? '#8ef5a4' : '#ff3b52');
      this.bombLamp.material.emissiveIntensity = on ? 1.4 : 0.08;
    }
    for (const target of this.targets.values()) {
      const pulse = 1 + Math.sin(time * 7) * 0.025;
      const lamp = target.userData.lamp as THREE.Group | undefined;
      lamp?.scale.setScalar(pulse);
    }

    this.view.position.y = Math.sin(this.viewBob * 4.5) * 0.007 * this.bobPower - this.muzzle * 0.3;
    this.view.position.x = Math.sin(this.viewBob * 2.25) * 0.006 * this.bobPower;
    this.view.rotation.x = 0.04 + this.muzzle * 1.2;

    for (let i = this.tracers.length - 1; i >= 0; i--) {
      const tracer = this.tracers[i];
      tracer.life -= dt;
      const material = tracer.line.material as THREE.MeshBasicMaterial;
      material.opacity = Math.max(0, tracer.life / 0.12) * 0.85;
      if (tracer.life <= 0) {
        tracer.line.removeFromParent();
        this.tracers.splice(i, 1);
      }
    }
    for (let i = this.splats.length - 1; i >= 0; i--) {
      const splat = this.splats[i];
      splat.life -= dt;
      splat.sprite.material.opacity = Math.max(0, splat.life);
      if (splat.life <= 0) {
        splat.sprite.material.dispose();
        splat.sprite.removeFromParent();
        this.splats.splice(i, 1);
      }
    }
  }

  /** Footsteps from everyone else, quieter the further away they are. */
  private listen(seat: number, player: ShooterPlayer, dt: number) {
    const last = this.stepped.get(seat) ?? { x: player.x, z: player.z, run: 0 };
    const moved = Math.hypot(player.x - last.x, player.z - last.z);
    const run = last.run + (player.crouch || !player.ground ? 0 : moved);
    this.stepped.set(seat, { x: player.x, z: player.z, run: run > 2.1 ? 0 : run });
    void dt;
    if (run <= 2.1) return;
    const ear = this.local ?? { x: 0, z: 0 };
    const away = Math.hypot(player.x - ear.x, player.z - ear.z);
    const volume = Math.max(0, 1 - away / 26) ** 2;
    if (volume > 0.02) this.onStep(volume, false);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
    this.view.visible = visible && this.firstPerson;
  }

  /** Throw away everything built for the last game so the next starts clean. */
  reset() {
    this.fighters.forEach((fighter) => {
      fighter.rig.dispose();
    });
    this.fighters.clear();
    this.targets.clear();
    this.tracers.forEach((tracer) => tracer.line.removeFromParent());
    this.tracers = [];
    this.splats.forEach((splat) => splat.sprite.material.dispose());
    this.splats = [];
    this.bomb = null;
    this.bombLamp = null;
    this.bombRing = null;
    this.bombFill = null;
    for (const child of [...this.group.children]) this.group.remove(child);
    for (const child of [...this.view.children]) this.view.remove(child);
    this.geometries.forEach((geometry) => geometry.dispose());
    this.geometries = [];
    this.materials.forEach((material) => material.dispose());
    this.materials = [];
    this.setup = null;
    this.built = false;
    this.local = null;
  }

  dispose() {
    this.clearBuild();
    this.splatMap.dispose();
    this.group.removeFromParent();
  }
}

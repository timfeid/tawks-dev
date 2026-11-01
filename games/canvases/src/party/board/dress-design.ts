import * as THREE from 'three';
import { place } from './models';
import { WATER_Y } from '../stage';
import type { MapKey } from '../protocol';
import type { Design } from './design';
import { BASALT, CHARRED, FIR, SNOWCAPPED, groundPatch, iceSheet, lavaPool, smokePlume, snowfall, type Dresser } from './dressing';

/** Each board's look for the kits' models, when the map file does not give one. */
const LOOKS: Partial<Record<MapKey, { trees?: Record<string, string | null>; rocks?: Record<string, string> }>> = {
  volcano: { trees: CHARRED, rocks: BASALT },
  frozen: { trees: FIR, rocks: SNOWCAPPED },
};

/**
 * How big each model is drawn at a map file's scale of 1. The kits were made
 * at different sizes, so a map file only says how big a thing is for its kind.
 */
const SIZE: Record<string, number> = {
  'palm-bend': 0.62, 'palm-straight': 0.62, 'palm-detailed-bend': 0.62,
  'ship-wreck': 0.7, 'ship-pirate-medium': 0.62, 'boat-row-small': 0.8,
  barrel: 0.6, crate: 0.58, 'box-large': 0.62, chest: 0.8, 'resource-planks': 0.72,
  'rocks-sand-a': 0.45, 'rock-sand-b': 0.42, 'rocks-a': 0.5, 'rocks-b': 0.5, 'rocks-large': 0.6,
  cannon: 0.65, 'structure-platform-dock': 1.15, 'flag-pirate-high': 0.62, 'patch-grass-foliage': 0.4,
  'tower-watch': 0.9, 'tower-square-base': 1.5, cart: 0.9,
  tree_oak: 1.8, tree_default: 1.7, tree_detailed: 1.7, tree_tall: 1.7, tree_thin: 1.6,
  tree_palmTall: 1.7, tree_palmBend: 1.6,
  tree_pineRoundA: 1.7, tree_pineRoundC: 1.6, tree_pineSmallB: 1.4, tree_pineTallA: 1.8, tree_pineTallB: 1.8,
  rock_largeA: 1.4, rock_largeB: 1.4, rock_largeC: 1.4, rock_largeD: 1.4, rock_tallA: 1.5, rock_tallC: 1.5, rock_tallF: 1.5,
  cliff_rock: 1.8, cliff_large_rock: 1.8, cliff_half_rock: 1.8, cliff_cave_rock: 1.9, stone_largeB: 1.4, stone_smallG: 1.4,
  plant_bush: 2, plant_bushLarge: 2, plant_bushDetailed: 2.1, grass_leafs: 1.6, flowers: 1.6, flower_redA: 1.6, flower_yellowA: 1.6,
  mushroom_redGroup: 1.6, mushrooms: 1.6, stump_round: 1.5, log_stack: 1.3, bedroll: 1.4,
  'stall-red': 1.7, 'stall-green': 1.7, tent: 1.6, tent_smallOpen: 1.5, 'fountain-round': 1.5, lantern: 1.25,
  'lightpost-single': 1.25, signpost: 1.25, sign: 1.25, 'flag-pennant': 1.25, flag: 1.3, 'campfire-pit': 1.4, campfire_stones: 1.4,
  'column-large': 1.5, 'pillar-large': 1.5, statue_columnDamaged: 1.5, statue_obelisk: 1.5, 'stairs-stone': 1.4,
  'siege-catapult': 1.15, 'gravestone-cross': 1.25, 'gravestone-round': 1.25, 'crypt-small': 1.3, 'iron-fence': 1.25, bridge_stone: 1.4,
};
const sized = (model: string, scale: number) => (SIZE[model] ?? 1) * scale;

const TREE = /tree|palm|stump|log/;
const ROCK = /rock|stone|cliff/;

/**
 * A hand-built board's scenery, all of it placed: the map file's landmarks
 * where it puts them, and its clusters of trees, rocks and clutter grown in
 * the circles it gives, kept off the path and out of the sea. Rocks are
 * scattered along the coast, and each board's own touches go on last: lava
 * running down its rivers, ice over frozen pools, smoke, and snow.
 */
export function dressDesign(d: Dresser, design: Design, map: MapKey) {
  let seed = 7331 + design.coast.length;
  const random = () => {
    seed = (seed * 1664525 + 1013904223) % 4294967296;
    return seed / 4294967296;
  };
  const ground = (x: number, z: number) => d.terrain.height(x, z);
  const look = LOOKS[map] ?? {};
  const paintFor = (model: string, given?: Record<string, string | null> | string) => {
    if (typeof given === 'string') return { _all: given };
    if (given) return given;
    if (TREE.test(model)) return look.trees;
    if (ROCK.test(model)) return look.rocks;
    return undefined;
  };
  const clear = (x: number, z: number, room: number) =>
    d.spaces.every((space) => (space.x - x) ** 2 + (space.z - z) ** 2 > room * room) &&
    d.clearings.every((c) => (c.x - x) ** 2 + (c.z - z) ** 2 > (c.r + room * 0.5) ** 2);
  const dry = (x: number, z: number) => ground(x, z) > d.terrain.shoreline + 0.15;

  for (const district of design.districts ?? []) {
    const material = d.trackMaterial(new THREE.MeshStandardMaterial({ color: district.color, roughness: 1, transparent: true, opacity: 0.55, depthWrite: false }));
    groundPatch(d, district.x, district.z, district.r, material, 0.018);
    const canvas = document.createElement('canvas');
    canvas.width = 512;
    canvas.height = 80;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = 'rgba(5,15,28,0.8)';
    ctx.beginPath();
    ctx.moveTo(14, 6); ctx.lineTo(498, 6); ctx.lineTo(512, 70); ctx.lineTo(0, 70); ctx.closePath(); ctx.fill();
    ctx.fillStyle = district.color;
    ctx.fillRect(28, 69, 456, 3);
    ctx.fillStyle = '#ffffff';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = '900 27px system-ui, sans-serif';
    ctx.fillText(district.name.toUpperCase(), 256, 40, 450);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    // The board owns shared scenery; this sprite owns its one label texture.
    const labelMaterial = d.trackMaterial(new THREE.SpriteMaterial({ map: texture, depthWrite: false }));
    const label = new THREE.Sprite(labelMaterial);
    label.position.set(district.x, ground(district.x, district.z) + 6.5, district.z);
    label.scale.set(13, 2.03, 1);
    label.userData.districtTexture = texture;
    d.group.add(label);
  }

  for (const prop of design.props ?? []) {
    const placed=place(d.group, prop.model, {
      x: prop.x,
      z: prop.z,
      y: (prop.y ?? ground(prop.x, prop.z)) + (prop.lift ?? 0),
      yaw: prop.yaw ?? 0,
      scale: sized(prop.model, prop.scale ?? 1),
      paint: paintFor(prop.model, prop.paint) as Record<string, string | null> | undefined,
    });
    if(prop.label && prop.model.startsWith('town-building-')){
      // Signs belong to an actual loaded facade. A failed building fetch must
      // never leave a giant floating shop label where its mesh should be.
      void placed.then(model=>{
        if(!model)return;
        model.updateMatrixWorld(true);
        const local=new THREE.Box3(),inverse=model.matrixWorld.clone().invert();
        model.traverse(child=>{
          const mesh=child as THREE.Mesh;if(!mesh.isMesh)return;
          mesh.geometry.computeBoundingBox();
          if(mesh.geometry.boundingBox)local.union(mesh.geometry.boundingBox.clone().applyMatrix4(inverse.clone().multiply(mesh.matrixWorld)));
        });
        if(local.isEmpty())return;
        const canvas=document.createElement('canvas');canvas.width=512;canvas.height=96;const ctx=canvas.getContext('2d')!;
        ctx.fillStyle='#182b3a';ctx.fillRect(0,0,512,96);ctx.fillStyle='#a7f3d0';ctx.fillRect(8,8,496,3);ctx.fillRect(8,85,496,3);ctx.fillStyle='#f8fafc';ctx.textAlign='center';ctx.textBaseline='middle';ctx.font='900 40px system-ui';ctx.fillText(prop.label!,256,49,480);
        const texture=new THREE.CanvasTexture(canvas);texture.colorSpace=THREE.SRGBColorSpace;
        const material=d.trackMaterial(new THREE.MeshBasicMaterial({map:texture,side:THREE.FrontSide}));
        const scale=Math.max(.001,Math.abs(model.scale.x)),width=Math.min(3/scale,(local.max.x-local.min.x)*.82),height=width*96/512;
        const geometry=d.track(new THREE.PlaneGeometry(width,height)),sign=new THREE.Group();sign.name=`shop-sign-${prop.label}`;
        sign.position.set((local.min.x+local.max.x)/2,local.min.y+Math.min(1.8/scale,(local.max.y-local.min.y)*.45),local.max.z+.025/scale);
        // Two outward-facing planes, rather than DoubleSide text, keep the
        // lettering readable from either side of the physical signboard.
        const front=new THREE.Mesh(geometry,material),back=new THREE.Mesh(geometry,material);
        front.position.z=.002/scale;back.position.z=-.002/scale;back.rotation.y=Math.PI;
        sign.add(front,back);sign.userData.districtTexture=texture;sign.userData.shopSign=true;model.add(sign);
      });
    }
    // Nothing else grows through a landmark.
    d.clearings.push({ x: prop.x, z: prop.z, r: Math.max(1.5, (prop.scale ?? 1) * 2.2) });
  }

  for (const cluster of design.clusters ?? []) {
    const [low, high] = cluster.scale ?? [0.8, 1.2];
    const count = Math.round(cluster.count * Math.max(0.45, d.detail));
    let placed = 0;
    for (let tries = 0; tries < count * 8 && placed < count; tries++) {
      // Thickest toward the middle, or round the edge for a ring.
      const angle = random() * Math.PI * 2;
      const out = cluster.ring ? 0.82 + random() * 0.3 : Math.sqrt(random()) * (0.35 + 0.65 * random());
      const x = cluster.x + Math.cos(angle) * cluster.r * out;
      const z = cluster.z + Math.sin(angle) * cluster.r * out;
      const model = cluster.models[Math.floor(random() * cluster.models.length)];
      const scale = low + random() * (high - low);
      if (model === 'lava-pool') {
        if (!clear(x, z, scale * 3 + 1.2) || !dry(x, z)) continue;
        lavaPool(d, x, z, scale * 1.9);
      } else if (model === 'ice-sheet') {
        if (!clear(x, z, scale * 2.6 + 1) || !dry(x, z)) continue;
        iceSheet(d, x, z, scale * 2.6);
      } else {
        if (!clear(x, z, ROCK.test(model) ? 1.8 : 1.4) || !dry(x, z) || d.terrain.slope(x, z) > 1.6) continue;
        const paint = cluster.paint ? { _all: cluster.paint } : paintFor(model);
        void place(d.group, model, { x, z, y: ground(x, z) - (ROCK.test(model) ? 0.2 : 0), yaw: random() * Math.PI * 2, scale: sized(model, scale), paint: paint as Record<string, string | null> | undefined });
      }
      placed++;
    }
  }

  // Rocks breaking the waterline all the way round, so the coast is not a smooth edge.
  const coastRocks = ['rocks-sand-a', 'rock-sand-b', 'rocks-a', 'rocks-b', 'rock_largeC'];
  const outline = design.coast;
  for (let i = 0; i < outline.length; i++) {
    const [ax, az] = outline[i];
    const [bx, bz] = outline[(i + 1) % outline.length];
    const steps = Math.max(1, Math.round(Math.hypot(bx - ax, bz - az) / 9));
    for (let s = 0; s < steps; s++) {
      if (random() < 0.35) continue;
      const t = (s + random()) / steps;
      const x = ax + (bx - ax) * t + (random() - 0.5) * 4;
      const z = az + (bz - az) * t + (random() - 0.5) * 4;
      if (!clear(x, z, 2)) continue;
      const model = coastRocks[Math.floor(random() * coastRocks.length)];
      void place(d.group, model, {
        x,
        z,
        y: Math.max(ground(x, z), WATER_Y - 0.4) - 0.3,
        yaw: random() * Math.PI * 2,
        scale: sized(model, 0.8 + random() * 0.9),
        paint: paintFor(model) as Record<string, string | null> | undefined,
      });
    }
  }

  for (const feature of design.land) {
    if (feature.kind === 'river' && feature.lava) {
      // Lava lying the length of the flow.
      for (let i = 1; i < feature.points.length; i++) {
        const [ax, az] = feature.points[i - 1];
        const [bx, bz] = feature.points[i];
        const steps = Math.max(1, Math.round(Math.hypot(bx - ax, bz - az) / (feature.r * 0.9)));
        for (let s = 0; s <= steps; s++) {
          const x = ax + ((bx - ax) * s) / steps;
          const z = az + ((bz - az) * s) / steps;
          if (clear(x, z, feature.r * 0.6)) lavaPool(d, x, z, feature.r * 0.75);
        }
      }
    } else if (feature.kind === 'pool' && feature.frozen) {
      // Frozen over: a sheet of ice lying in the hollow.
      iceSheet(d, feature.x, feature.z, feature.r * 0.95);
    }
  }

  for (const [x, z] of design.smoke ?? []) smokePlume(d, { x, z }, random);
  if (map === 'frozen') snowfall(d, random);
}

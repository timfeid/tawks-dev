import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { CharacterDef } from '../protocol';

/** One bout on the ballot, as the stage shows it. */
export type BallotOption = { key: string; name: string; rules: string; tag: string };

const COLORS = ['#f97316', '#22d3ee', '#a78bfa', '#4ade80', '#f472b6'];
/** How high over the island the stage floats, and how far apart the podiums stand. */
const HEIGHT = 14;
const APART = 7.5;

type Podium = { key: string; group: THREE.Group; top: THREE.Mesh; sign: THREE.Sprite; voters: Map<number, CharacterRig> };

function signTexture(option: BallotOption, color: string) {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 256;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = 'rgba(15, 23, 42, 0.88)';
  ctx.beginPath();
  ctx.roundRect(8, 8, 496, 240, 36);
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 8;
  ctx.stroke();
  ctx.textAlign = 'center';
  ctx.fillStyle = '#ffffff';
  ctx.font = '900 56px system-ui, sans-serif';
  ctx.fillText(option.name, 256, 92, 460);
  ctx.fillStyle = 'rgba(255,255,255,0.75)';
  ctx.font = '500 26px system-ui, sans-serif';
  // The rules, wrapped onto two lines at most.
  const words = option.rules.split(' ');
  const lines: string[] = [''];
  for (const word of words) {
    const line = `${lines[lines.length - 1]} ${word}`.trim();
    if (ctx.measureText(line).width > 440 && lines.length < 2) lines.push(word);
    else lines[lines.length - 1] = line;
  }
  lines.forEach((line, index) => ctx.fillText(line, 256, 142 + index * 32, 460));
  ctx.fillStyle = color;
  ctx.font = '800 22px system-ui, sans-serif';
  ctx.fillText(option.tag.toUpperCase(), 256, 222);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The vote for the next bout, as a stage floating over the island: a podium
 * for each bout on the ballot with its name above it, and everyone who has
 * voted standing on the one they picked. A click on a podium votes for it.
 */
export class VoteStage {
  readonly group = new THREE.Group();
  private podiums: Podium[] = [];
  private ballot = '';
  private disposables: { dispose(): void }[] = [];

  constructor(scene: THREE.Scene) {
    this.group.visible = false;
    scene.add(this.group);
  }

  /** Where the stage stands: above a point of the island, facing the camera's way. */
  get center() {
    return this.group.position;
  }

  /**
   * Shows the ballot over `at`, with the podiums spread across the view from
   * `yaw`. Only rebuilt when the ballot itself changes.
   */
  show(at: THREE.Vector3, yaw: number, options: BallotOption[], votes: { seat: number; key: string }[], mine: string | null, character: (seat: number) => CharacterDef | undefined) {
    const ballot = options.map((option) => option.key).join('|');
    if (ballot !== this.ballot) this.build(options);
    this.ballot = ballot;
    this.group.visible = true;
    this.group.position.set(at.x, at.y + HEIGHT, at.z);
    this.group.rotation.y = yaw;
    for (const podium of this.podiums) {
      const picked = votes.filter((vote) => vote.key === podium.key);
      // Voters come and go as the votes change.
      for (const [seat, rig] of podium.voters) {
        if (picked.some((vote) => vote.seat === seat)) continue;
        rig.group.removeFromParent();
        rig.dispose();
        podium.voters.delete(seat);
      }
      picked.forEach((vote, index) => {
        let rig = podium.voters.get(vote.seat);
        if (!rig) {
          const def = character(vote.seat);
          if (!def) return;
          rig = buildCharacter(def);
          rig.group.scale.setScalar(0.7);
          podium.group.add(rig.group);
          podium.voters.set(vote.seat, rig);
        }
        const row = picked.length;
        rig.group.position.set((index - (row - 1) / 2) * 0.9, 1.05, 0.2);
      });
      const chosen = mine === podium.key;
      const material = podium.top.material as THREE.MeshStandardMaterial;
      material.emissiveIntensity = chosen ? 0.8 : 0.15;
      podium.group.userData.lift = chosen ? 0.5 : 0;
    }
  }

  hide() {
    this.group.visible = false;
  }

  /** The bout whose podium is under the pointer, if any. */
  pick(raycaster: THREE.Raycaster): string | null {
    if (!this.group.visible) return null;
    let best: { key: string; distance: number } | null = null;
    for (const podium of this.podiums) {
      const hit = raycaster.intersectObject(podium.group, true)[0];
      if (hit && (!best || hit.distance < best.distance)) best = { key: podium.key, distance: hit.distance };
    }
    return best?.key ?? null;
  }

  update(time: number, hovered: string | null) {
    if (!this.group.visible) return;
    this.podiums.forEach((podium, index) => {
      const lift = (podium.group.userData.lift as number) ?? 0;
      podium.group.position.y = Math.sin(time * 1.4 + index) * 0.25 + lift;
      podium.group.rotation.y = Math.sin(time * 0.5 + index) * 0.08;
      const grow = hovered === podium.key ? 1.08 : 1;
      podium.group.scale.lerp(new THREE.Vector3(grow, grow, grow), 0.2);
      for (const [seat, rig] of podium.voters) rig.animate('idle', time + seat * 0.4, 0);
    });
  }

  private build(options: BallotOption[]) {
    this.clear();
    const stone = new THREE.MeshStandardMaterial({ color: '#475569', roughness: 0.8, flatShading: true });
    const base = new THREE.CylinderGeometry(2.1, 1.4, 1.6, 8);
    const cap = new THREE.CylinderGeometry(2.2, 2.2, 0.3, 24);
    this.disposables.push(stone, base, cap);
    options.forEach((option, index) => {
      const color = COLORS[index % COLORS.length];
      const group = new THREE.Group();
      group.position.x = (index - (options.length - 1) / 2) * APART;
      const pillar = new THREE.Mesh(base, stone);
      pillar.castShadow = true;
      const topMaterial = new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.15, roughness: 0.5 });
      const top = new THREE.Mesh(cap, topMaterial);
      top.position.y = 0.9;
      const sign = new THREE.Sprite(new THREE.SpriteMaterial({ map: signTexture(option, color), transparent: true, depthWrite: false }));
      sign.scale.set(6, 3, 1);
      sign.position.set(0, 4.6, 0);
      group.add(pillar, top, sign);
      this.group.add(group);
      this.disposables.push(topMaterial, sign.material, sign.material.map!);
      this.podiums.push({ key: option.key, group, top, sign, voters: new Map() });
    });
  }

  private clear() {
    for (const podium of this.podiums) {
      for (const rig of podium.voters.values()) rig.dispose();
      podium.group.removeFromParent();
    }
    this.podiums = [];
    for (const thing of this.disposables) thing.dispose();
    this.disposables = [];
  }

  dispose() {
    this.clear();
    this.group.removeFromParent();
  }
}

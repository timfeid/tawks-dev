import * as THREE from 'three';
import type { BoutHost, Pose, Rider, Seat } from '@tawks/bout-sdk';
import type { BuzzerSnap, BuzzerState } from './types';

const SPACING = 2.7;
const SCALE = 1.6;
/** The stage's radius when the server has not said; it is built for four. */
const STAGE = 7.5;

const LAMP: Record<BuzzerState, string> = {
  thinking: '#475569',
  silent: '#475569',
  wrong: '#f43f5e',
  right: '#34d399',
};

type Contestant = {
  seat: number;
  rider: Rider;
  stand: THREE.Group;
  lamp: THREE.MeshStandardMaterial;
  front: THREE.MeshStandardMaterial;
};

/** A row of lecterns on a little stage, one per seat, lit by how each answered. */
export class BuzzerPodiums {
  private group = new THREE.Group();
  private stands = new Map<number, Contestant>();
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private lectern: THREE.BoxGeometry;
  private lampShape: THREE.BoxGeometry;
  private key = '';
  private stage: THREE.Group;

  constructor(private host: BoutHost) {
    this.group.name = 'buzzer-podiums';
    host.scene.add(this.group);

    const deck = this.material(new THREE.MeshStandardMaterial({ color: '#312e81', roughness: 0.7 }));
    const rim = this.material(new THREE.MeshStandardMaterial({ color: '#facc15', roughness: 0.5 }));
    const platform = new THREE.Mesh(this.geometry(new THREE.CylinderGeometry(7.5, 7.2, 0.6, 64)), [rim, deck, deck]);
    platform.position.y = -0.3;
    platform.receiveShadow = true;
    const pillar = new THREE.Mesh(
      this.geometry(new THREE.CylinderGeometry(3.4, 1.8, -host.waterY + 0.4, 24)),
      this.material(new THREE.MeshStandardMaterial({ color: '#f7d488', roughness: 0.9 })),
    );
    pillar.position.y = (host.waterY - 0.6) / 2;
    // Stretched out sideways for a bigger field, so a longer row still fits.
    this.stage = new THREE.Group();
    this.stage.add(platform, pillar);
    this.group.add(this.stage);

    this.lectern = this.geometry(new THREE.BoxGeometry(1.7, 0.95, 0.7));
    this.lampShape = this.geometry(new THREE.BoxGeometry(1.5, 0.18, 0.05));
  }

  private geometry<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private material<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  set visible(on: boolean) {
    this.group.visible = on;
  }

  /** How wide the row is, so the camera can keep everyone in shot. */
  get halfWidth() {
    return Math.max(2, this.stands.size) * SPACING * 0.5 + 1.5;
  }

  sync(seats: Seat[], radius = STAGE) {
    this.stage.scale.set(radius / STAGE, 1, radius / STAGE);
    const playing = seats.filter((seat) => seat.in_round || this.stands.has(seat.seat));
    const listed = playing.length ? playing : seats;
    const key = listed.map((seat) => `${seat.seat}:${seat.character}`).join();
    if (key === this.key) return;
    this.key = key;
    for (const stand of this.stands.values()) this.remove(stand);
    this.stands.clear();
    listed.forEach((seat, index) => {
      const character = this.host.character(seat.character);
      if (!character) return;
      const rider = this.host.rider(character);
      rider.group.scale.setScalar(SCALE);
      const x = (index - (listed.length - 1) / 2) * SPACING;
      rider.group.position.set(x, 0, -0.35);
      const stand = new THREE.Group();
      const front = new THREE.MeshStandardMaterial({ color: character.body, roughness: 0.6 });
      const lamp = new THREE.MeshStandardMaterial({ color: LAMP.thinking, emissive: LAMP.thinking, emissiveIntensity: 0.2 });
      const body = new THREE.Mesh(this.lectern, front);
      body.position.y = 0.475;
      body.castShadow = true;
      const light = new THREE.Mesh(this.lampShape, lamp);
      light.position.set(0, 0.7, 0.36);
      stand.add(body, light);
      stand.position.set(x, 0, 0.75);
      this.group.add(rider.group, stand);
      this.stands.set(seat.seat, { seat: seat.seat, rider, stand, lamp, front });
    });
  }

  update(snapshot: BuzzerSnap | null, winners: number[], time: number) {
    for (const stand of this.stands.values()) {
      const player = snapshot?.players.find((one) => one.seat === stand.seat);
      const state = player?.state ?? 'thinking';
      const color = LAMP[state];
      stand.lamp.color.set(color);
      stand.lamp.emissive.set(color);
      stand.lamp.emissiveIntensity = state === 'thinking' || state === 'silent' ? 0.2 : 0.9;
      let pose: Pose = 'idle';
      if (winners.includes(stand.seat) || state === 'right') pose = 'cheer';
      else if (state === 'wrong' && snapshot?.stage === 'reveal') pose = 'sad';
      stand.rider.animate(pose, time, 0);
    }
  }

  private remove(stand: Contestant) {
    this.group.remove(stand.rider.group, stand.stand);
    stand.rider.dispose();
    stand.lamp.dispose();
    stand.front.dispose();
  }

  dispose() {
    for (const stand of this.stands.values()) this.remove(stand);
    this.stands.clear();
    this.host.scene.remove(this.group);
    for (const geometry of this.geometries) geometry.dispose();
    for (const material of this.materials) material.dispose();
  }
}

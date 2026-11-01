import * as THREE from 'three';

const SPIN_SECONDS = 1.15;
const SHOW_SECONDS = 1.25;
const SIZE = 0.9;
const LIFT = 2.6;

/** Pip positions on a 3 by 3 grid, for each face value. */
const PIPS: Record<number, [number, number][]> = {
  1: [[1, 1]],
  2: [[0, 0], [2, 2]],
  3: [[0, 0], [1, 1], [2, 2]],
  4: [[0, 0], [0, 2], [2, 0], [2, 2]],
  5: [[0, 0], [0, 2], [1, 1], [2, 0], [2, 2]],
  6: [[0, 0], [0, 1], [0, 2], [2, 0], [2, 1], [2, 2]],
};

function face(value: number) {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 128;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#fbfbf7';
  ctx.fillRect(0, 0, 128, 128);
  ctx.fillStyle = value === 1 ? '#e11d48' : '#1f2937';
  for (const [row, column] of PIPS[value] ?? []) {
    ctx.beginPath();
    ctx.arc(28 + column * 36, 28 + row * 36, value === 1 ? 17 : 12, 0, Math.PI * 2);
    ctx.fill();
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The die a player hits on their turn: it spins over their head, lands on
 * the number rolled, holds it a moment, then shrinks away.
 */
export class Die {
  readonly group = new THREE.Group();
  private cube: THREE.Mesh<THREE.BoxGeometry, THREE.MeshStandardMaterial[]>;
  private faces = [1, 2, 3, 4, 5, 6].map(face);
  private clock = Infinity;
  private value = 1;
  private spin = new THREE.Vector3();

  constructor() {
    // BoxGeometry faces run +x, -x, +y, -y, +z, -z; the top (+y) shows the roll.
    const materials = [3, 4, 1, 6, 2, 5].map((value) => new THREE.MeshStandardMaterial({ map: this.faces[value - 1], roughness: 0.4 }));
    this.cube = new THREE.Mesh(new THREE.BoxGeometry(SIZE, SIZE, SIZE), materials);
    this.cube.castShadow = true;
    this.group.add(this.cube);
    this.group.visible = false;
  }

  /** `side` spreads a pair of dice apart; gold marks a golden die. */
  roll(value: number, over: THREE.Vector3, side = 0, gold = false) {
    this.value = Math.min(6, Math.max(1, Math.round(value)));
    this.clock = 0;
    for (const material of this.cube.material) material.color.set(gold ? '#ffd54a' : '#ffffff');
    this.group.position.copy(over).setY(over.y + LIFT);
    this.group.position.x += side * SIZE * 0.8;
    this.group.visible = true;
    this.cube.scale.setScalar(1);
    this.spin.set(9 + Math.random() * 6, 7 + Math.random() * 6, 5 + Math.random() * 4);
  }

  update(dt: number) {
    if (!this.group.visible) return;
    this.clock += dt;
    if (this.clock < SPIN_SECONDS) {
      this.cube.rotation.x += this.spin.x * dt;
      this.cube.rotation.y += this.spin.y * dt;
      this.cube.rotation.z += this.spin.z * dt;
      this.cube.position.y = Math.sin((this.clock / SPIN_SECONDS) * Math.PI) * 0.6;
      return;
    }
    if (this.cube.material[2].map !== this.faces[this.value - 1]) {
      this.cube.material[2].map = this.faces[this.value - 1];
      this.cube.material[2].needsUpdate = true;
    }
    // Settle flat with the roll on top, with a little bounce.
    this.cube.rotation.x *= 0.7;
    this.cube.rotation.y *= 0.85;
    this.cube.rotation.z *= 0.7;
    const shown = this.clock - SPIN_SECONDS;
    this.cube.position.y = Math.abs(Math.sin(shown * 9)) * Math.max(0, 0.25 - shown * 0.5);
    if (shown > SHOW_SECONDS) {
      const fade = Math.max(0, 1 - (shown - SHOW_SECONDS) * 5);
      this.cube.scale.setScalar(fade);
      if (fade === 0) this.group.visible = false;
    }
  }
}

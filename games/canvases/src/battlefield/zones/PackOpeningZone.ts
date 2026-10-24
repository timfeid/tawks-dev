import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';

function createTextSprite(text: string, count: number): THREE.Sprite {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 128;
  const ctx = canvas.getContext('2d')!;

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Name
  ctx.font = 'bold 36px monospace';
  ctx.fillStyle = '#e2e8f0';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(text, canvas.width / 2, 40);

  // Count
  ctx.font = '28px monospace';
  ctx.fillStyle = '#22d3ee';
  ctx.fillText(`×${count}`, canvas.width / 2, 90);

  const texture = new THREE.CanvasTexture(canvas);
  texture.needsUpdate = true;
  const mat = new THREE.SpriteMaterial({
    map: texture,
    transparent: true,
    depthTest: false,
  });
  const sprite = new THREE.Sprite(mat);
  sprite.scale.set(1.6, 0.4, 1);
  sprite.renderOrder = 200;
  return sprite;
}

export type PackPhase =
  | 'inventory'
  | 'grab'
  | 'tear'
  | 'opening'
  | 'burst'
  | 'reveal'
  | 'summary';

export class PackOpeningZone extends BaseZone {
  private phase: PackPhase = 'inventory';
  private packIds: string[] = [];
  private cardIds: string[] = [];
  private revealedIndices = new Set<number>();
  private openingPackId: string | null = null;
  private tearProgress = 0;

  // Layout parameters
  private inventoryScale = 0.75;
  private tearScale = 1.1;
  private openingScale = 1.6;
  private cardScale = 1.1;
  private summaryScale = 0.75;

  // Heights
  private tableHeight = 0.05;
  private ceilingHeight = 10.0;
  private floatHeight = 1.8;
  private summaryHeight = 0.05;

  private flyInCount = 0;
  private focusIndex = 0;
  private seatAngle = Math.PI / 2;

  private shakeOffset = new THREE.Vector3();
  private shakeTime = 0;
  private isShaking = false;
  private tearParticles: THREE.Points;
  private tearParticleMat: THREE.PointsMaterial;
  private tearParticlePositions: Float32Array;
  private tearParticleSeeds: Float32Array;
  private tearParticleCenter = new THREE.Vector3(0, -10, 0);
  private packLabels: THREE.Sprite[] = [];
  private tearParticleStrength = 0;
  private tearGlowLight: THREE.PointLight;

  constructor() {
    super('pack_opening');
    const particleCount = 120;
    const particleGeom = new THREE.BufferGeometry();
    this.tearParticlePositions = new Float32Array(particleCount * 3);
    this.tearParticleSeeds = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount; i += 1) {
      const idx = i * 3;
      this.tearParticlePositions[idx + 0] = 0;
      this.tearParticlePositions[idx + 1] = -10;
      this.tearParticlePositions[idx + 2] = 0;
      this.tearParticleSeeds[idx + 0] = Math.random() * Math.PI * 2; // angle
      this.tearParticleSeeds[idx + 1] = 0.2 + Math.random() * 0.8; // radius variance
      this.tearParticleSeeds[idx + 2] = 0.5 + Math.random() * 1.5; // speed variance
    }
    particleGeom.setAttribute(
      'position',
      new THREE.BufferAttribute(this.tearParticlePositions, 3),
    );

    this.tearParticleMat = new THREE.PointsMaterial({
      color: 0x22d3ee,
      size: 5,
      transparent: true,
      opacity: 0.0,
      blending: THREE.AdditiveBlending,
      depthTest: true,
      depthWrite: false,
      sizeAttenuation: false,
    });
    this.tearParticles = new THREE.Points(particleGeom, this.tearParticleMat);
    this.tearParticles.frustumCulled = false;
    this.tearParticles.renderOrder = 122;
    this.tearParticles.visible = false;

    this.tearGlowLight = new THREE.PointLight(0x66f1ff, 0, 4.2, 1.8);
    this.tearGlowLight.visible = false;
    this.tearGlowLight.position.set(0, -10, 0);
  }

  public setPhase(phase: PackPhase) {
    this.phase = phase;
  }

  public setPacks(packIds: string[]) {
    this.packIds = packIds;
  }

  public setCards(cardIds: string[], revealedIndices: Set<number>) {
    this.cardIds = cardIds;
    this.revealedIndices = revealedIndices;
  }

  public setOpeningPack(packId: string | null, isShaking: boolean) {
    this.openingPackId = packId;
    this.isShaking = isShaking;
  }

  public setTearState(packId: string | null, progress: number) {
    if (packId) this.openingPackId = packId;
    this.tearProgress = progress;
  }

  public setFlyInCount(count: number) {
    this.flyInCount = count;
  }

  public setFocusIndex(index: number) {
    this.focusIndex = index;
  }

  public setCamAngle(angle: number) {
    this.seatAngle = angle;
  }

  public tick(delta: number, time: number) {
    if (this.isShaking || this.phase === 'tear') {
      const intensity =
        this.phase === 'tear' ? 0.005 + this.tearProgress * 0.03 : 0.03;
      this.shakeTime += delta * 25; // Slower shake
      this.shakeOffset.set(
        Math.sin(this.shakeTime) * intensity,
        0,
        Math.cos(this.shakeTime * 1.2) * intensity,
      );
    } else {
      this.shakeOffset.set(0, 0, 0);
    }

    const strength = THREE.MathUtils.clamp(this.tearParticleStrength, 0, 1);
    if (strength <= 0.001) {
      this.tearParticles.visible = false;
      this.tearParticleMat.opacity = 0;
      this.tearGlowLight.visible = false;
      this.tearGlowLight.intensity = 0;
      return;
    }

    const intensity = THREE.MathUtils.smoothstep(strength, 0, 1);
    const posAttr = this.tearParticles.geometry.getAttribute(
      'position',
    ) as THREE.BufferAttribute;
    const count = this.tearParticlePositions.length / 3;

    // The seam runs along world Z at the tear center. The bag glows from within
    // so the interior light sits below the seam, inside the bag body.
    const seamCx = this.tearParticleCenter.x;
    const seamCy = this.tearParticleCenter.y;
    const seamCz = this.tearParticleCenter.z;
    // Half-width of the seam in world Z units (bag opening width).
    const seamHalfLen = 0.42;

    this.tearParticles.visible = true;
    this.tearParticleMat.opacity = Math.min(1, 0.45 + intensity * 0.7);
    this.tearParticleMat.size = 2.0 + intensity * 5.0;

    // Inner glow light sits inside the bag body, below the seam.
    this.tearGlowLight.visible = true;
    this.tearGlowLight.position.set(seamCx, seamCy - 0.75, seamCz);
    this.tearGlowLight.intensity = 1.2 + intensity * 5.5;
    this.tearGlowLight.distance = 1.6 + intensity * 2.8;

    for (let i = 0; i < count; i += 1) {
      const idx = i * 3;
      const seedAngle = this.tearParticleSeeds[idx + 0]; // 0 to 2π
      const seedRadius = this.tearParticleSeeds[idx + 1]; // 0.2 to 1.0
      const seedSpeed = this.tearParticleSeeds[idx + 2]; // 0.5 to 2.0

      // Smooth per-particle activation so particles fade in gradually.
      const threshold = 0.06 + seedRadius * 0.65;
      const activation = THREE.MathUtils.clamp(
        (strength - threshold + 0.22) / 0.22,
        0,
        1,
      );
      const a = activation * activation * (3 - 2 * activation); // smoothstep

      // Position along the seam (world Z axis).
      const seamT = seedAngle / (Math.PI * 2) - 0.5; // −0.5 to 0.5
      const baseZ = seamCz + seamT * seamHalfLen * 2;

      if (seedRadius > 0.55) {
        // SPARK — cycles upward from the seam, fanning out as it rises.
        const cycle =
          ((time * (0.45 + seedSpeed * 0.55) + seedAngle * 0.42) % 1.0) * a;
        const rise = cycle * (0.55 + intensity * 1.8);
        const fanX =
          (seedRadius - 0.55) *
          Math.sin(seedAngle * 3.7 + time * 0.4) *
          0.28 *
          cycle;
        const fanZ =
          (seedRadius - 0.55) *
          Math.cos(seedAngle * 2.3 + time * 0.3) *
          0.18 *
          cycle;

        this.tearParticlePositions[idx + 0] = seamCx + fanX;
        this.tearParticlePositions[idx + 1] = seamCy + rise;
        this.tearParticlePositions[idx + 2] = baseZ + fanZ;
      } else {
        // SEAM GLOW — hugs the seam line with a soft oscillating shimmer.
        const jX =
          Math.sin(time * 6.1 + seedAngle * 4.3) * 0.04 * intensity * a;
        const jY =
          Math.sin(time * 4.7 + seedAngle * 2.8) * 0.03 * intensity * a;
        const jZ =
          Math.cos(time * 5.3 + seedAngle * 3.1) * 0.025 * intensity * a;

        this.tearParticlePositions[idx + 0] = seamCx + jX;
        this.tearParticlePositions[idx + 1] = seamCy + jY;
        this.tearParticlePositions[idx + 2] = baseZ + jZ;
      }
    }

    posAttr.needsUpdate = true;
  }

  public layout(allActors: Map<string, CardActor>) {
    this.tearParticleStrength = 0;
    if (this.phase !== 'inventory') {
      // Hide labels when not in inventory phase
      this.packLabels.forEach((l) => l.visible = false);
    }
    if (this.phase === 'inventory') {
      this.layoutInventory(allActors);
    } else if (
      this.phase === 'grab' ||
      this.phase === 'tear' ||
      this.phase === 'opening'
    ) {
      this.layoutOpening(allActors);
    } else if (this.phase === 'burst') {
      this.layoutBurst(allActors);
    } else if (this.phase === 'reveal') {
      this.layoutReveal(allActors);
    } else if (this.phase === 'summary') {
      this.layoutSummary(allActors);
    }
  }

  private layoutInventory(allActors: Map<string, CardActor>) {
    const actors = this.packIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);

    // Remove old labels from scene
    this.packLabels.forEach((l) => {
      l.removeFromParent();
      (l.material as THREE.SpriteMaterial).map?.dispose();
      (l.material as THREE.SpriteMaterial).dispose();
    });
    this.packLabels = [];
    const scene = this.tearParticles.parent;

    if (actors.length === 0) return;

    const count = actors.length;
    const spacing = 1.3;
    const startX = -((count - 1) * spacing) / 2;

    actors.forEach((actor, index) => {
      actor.setZone('pack_inventory');
      actor.setFlatLayout(true);
      actor.setPileCount(6);

      const pos = new THREE.Vector3(
        startX + index * spacing,
        this.tableHeight,
        0,
      );
      const rot = new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2);
      actor.setTarget(pos, rot, this.inventoryScale, 0.2);

      // Add label above pack
      const card = actor.getCard();
      const pileCount = (card as any).pile_count ?? 1;
      const label = createTextSprite(card.name, pileCount);
      label.position.set(pos.x, this.tableHeight + 0.8, pos.z);
      this.packLabels.push(label);
      if (scene) scene.add(label);
    });

    this.cardIds.forEach((id) => {
      const actor = allActors.get(id);
      if (actor)
        actor.setTarget(new THREE.Vector3(0, -5, 0), new THREE.Euler(), 0, 0.5);
    });
  }

  private layoutOpening(allActors: Map<string, CardActor>) {
    const actor = this.openingPackId ? allActors.get(this.openingPackId) : null;
    if (actor) {
      actor.setZone('pack_opening');
      actor.setFlatLayout(true);
      actor.setPileCount(6);

      const isRising = this.phase === 'opening';
      const holdHeight = 0.5; // lifted off table while holding/tearing
      const targetY = isRising ? this.ceilingHeight : holdHeight;
      const targetScale = isRising ? this.openingScale : this.tearScale;

      const basePos = new THREE.Vector3(0, targetY, 0);
      const pos = basePos.clone().add(this.shakeOffset);
      const rot = new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2);

      actor.setTarget(pos, rot, targetScale, isRising ? 0.05 : 0.15);
      actor.setPackTearProgress(this.tearProgress);

      const p = THREE.MathUtils.clamp(this.tearProgress, 0, 1);
      const isTearing = this.phase === 'tear' || this.phase === 'grab';
      if (isTearing && p > 0.01) {
        // Place the center exactly at the seam: the bag's opening runs along
        // world Z at Y = bagY + packSeamZ (≈1.9 above the bag's table position).
        this.tearParticleCenter.copy(basePos).add(new THREE.Vector3(0, 1.9, 0));
        this.tearParticleStrength = p;
      }
    }

    this.packIds.forEach((id) => {
      if (id !== this.openingPackId) {
        const a = allActors.get(id);
        if (a)
          a.setTarget(new THREE.Vector3(0, -5, 0), new THREE.Euler(), 0, 0.3);
      }
    });
  }

  private layoutBurst(allActors: Map<string, CardActor>) {
    const actor = this.openingPackId ? allActors.get(this.openingPackId) : null;
    if (actor) {
      // Clear flat layout / pile meshes so the bag doesn't ghost on the table
      actor.setFlatLayout(false);
      actor.setPileCount(0);
      actor.setTarget(
        new THREE.Vector3(0, -5, 0),
        actor.group.rotation,
        0,
        0.2,
      );
    }

    const bagPos = new THREE.Vector3(0, this.ceilingHeight, 0);
    // Back-facing toward camera: faceYaw + π = π/2 - seatAngle + π
    const backFaceYaw = Math.PI / 2 - this.seatAngle + Math.PI;
    const backRot = new THREE.Euler(0, backFaceYaw, 0);
    this.cardIds.forEach((id) => {
      const a = allActors.get(id);
      if (a) {
        // Snap cards to bag position and back-facing on first entry so they
        // fly in hidden (art side away from camera).
        if (a.group.position.y < 5) {
          a.group.position.set(0, this.ceilingHeight, 0);
          a.group.rotation.y = backFaceYaw;
        }
        a.setTarget(bagPos.clone(), backRot.clone(), this.cardScale, 0.3);
      }
    });
  }

  private layoutReveal(allActors: Map<string, CardActor>) {
    const actors = this.cardIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);

    if (actors.length === 0) return;

    const count = actors.length;
    const allRevealed = this.revealedIndices.size === this.cardIds.length;

    // Once every card is flipped, snap rotation flat and lerp down to the table.
    // Component-wise Euler lerp produces ugly twisting, so we snap rotation
    // immediately and only animate the Y drop. This plays out during the 1-second
    // pre-summary delay before the camera pivots overhead.
    if (allRevealed) {
      // Rx(-π/2) maps local +Z → world +Y (face up). Rz(π/2) aligns card "top" (-X = screen-up
      // for the summary camera whose up vector is (-1,0,0)).
      const flatRot = new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2);
      const radius = count <= 5 ? 2.0 : 2.8;
      const arcSpan = Math.PI * 1.35;
      const angleStep = count > 1 ? arcSpan / (count - 1) : 0;
      const startAngle = Math.PI - arcSpan / 2;

      actors.forEach((actor, index) => {
        actor.setFlatLayout(true);  // enables direct Y lerp (not scale-anchored)
        actor.setPileCount(0);
        actor.group.rotation.copy(flatRot);  // snap — no Euler twisting
        const angle = startAngle + index * angleStep;
        const pos = new THREE.Vector3(
          Math.cos(angle) * radius,
          this.summaryHeight,
          Math.sin(angle) * radius,
        );
        actor.setTarget(pos, flatRot, this.summaryScale, 0.18);
        actor.setRevealShell(true);
      });
      return;
    }

    const radius = count <= 5 ? 2.2 : 2.6;
    const angleStep = (Math.PI * 2) / Math.max(1, count);
    const bagPos = new THREE.Vector3(0, this.ceilingHeight, 0);

    const focusAngle = this.seatAngle;
    const baseAngle = focusAngle - this.focusIndex * angleStep;
    const faceYaw = Math.PI / 2 - this.seatAngle;

    actors.forEach((actor, index) => {
      actor.setZone('pack_reveal');
      actor.setFlatLayout(false);
      actor.setPileCount(0);

      if (index >= this.flyInCount) {
        actor.setTarget(bagPos.clone(), new THREE.Euler(), this.cardScale, 0.3);
        return;
      }

      const isFocused = index === this.focusIndex;
      const revealed = this.revealedIndices.has(index);
      const angle = baseAngle + index * angleStep;
      const wave = Math.sin(Date.now() * 0.002 + index * 0.8) * 0.06;
      const r = isFocused ? radius * 1.12 : radius;
      const pos = new THREE.Vector3(
        Math.cos(angle) * r,
        this.floatHeight + wave + (isFocused ? 0.22 : 0),
        Math.sin(angle) * r,
      );
      const rot = revealed
        ? new THREE.Euler(0, faceYaw, 0)
        : new THREE.Euler(0, faceYaw + Math.PI, 0);

      const scale = isFocused ? this.cardScale * 1.45 : this.cardScale;
      actor.setTarget(pos, rot, scale, isFocused ? 0.04 : 0.05);
      actor.setRevealShell(revealed);
    });
  }

  private layoutSummary(allActors: Map<string, CardActor>) {
    const actors = this.cardIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);

    if (actors.length === 0) return;

    const count = actors.length;
    const radius = count <= 5 ? 2.0 : 2.8;
    // Horseshoe arc: ~240 degrees, open toward camera (+Z)
    const arcSpan = Math.PI * 1.35;
    const angleStep = count > 1 ? arcSpan / (count - 1) : 0;
    const centerAngle = Math.PI;
    const startAngle = centerAngle - arcSpan / 2;

    // Snap flat immediately — component-wise Euler lerp from upright → flat
    // produces ugly twisting, so we hard-set the rotation each frame.
    // Rx(-π/2) maps local +Z → world +Y (face up). Rz(π/2) aligns card "top" with
    // -X world = screen-up for the summary camera (camera.up = (-1,0,0)).
    const flatRot = new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2);

    actors.forEach((actor, index) => {
      actor.setZone('pack_summary');
      actor.setFlatLayout(true);
      actor.setPileCount(0);
      // Snap rotation so there's no Euler-lerp twist from the reveal orientation.
      actor.group.rotation.copy(flatRot);

      const angle = startAngle + index * angleStep;
      const pos = new THREE.Vector3(
        Math.cos(angle) * radius,
        this.summaryHeight,
        Math.sin(angle) * radius,
      );
      // Lay flat on table, face up toward top-down camera, top toward screen-up (-X)
      actor.setTarget(pos, flatRot, this.summaryScale, 0.15);
    });
  }

  // BaseZone stubs — pack state is managed externally via setPacks/setCards/setPhase
  public update(_cards: any[]): void {}
  public getDropIndex(_point: THREE.Vector3, _viewerId: string | null): number { return -1; }
  public getCardIds(_viewerId: string | null): string[] { return [...this.packIds, ...this.cardIds]; }

  public getInteractables(): THREE.Object3D[] {
    return [];
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.tearParticles, this.tearGlowLight, ...this.packLabels];
  }
}

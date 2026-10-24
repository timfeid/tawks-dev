import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import type { PlayerState } from '../types';
import { CDN_BASE } from '../cdn';

export class PlayerActor {
  public group: THREE.Group;
  public id: string;
  private collider: THREE.Mesh;
  private static modelCache: THREE.Group | null = null;
  private static loader = new GLTFLoader();
  private isHovered = false;
  private model: THREE.Group | null = null;
  // Damage-hit animation: red flash + shake for ~5 seconds after taking
  // uncontested damage so the player has time to see what happened.
  private hitAnimTime = 0;
  private static readonly HIT_ANIM_DURATION = 5.0;
  private basePos = new THREE.Vector3();

  constructor(player: PlayerState) {
    this.id = player.id;
    this.group = new THREE.Group();

    this.collider = new THREE.Mesh(
      new THREE.CylinderGeometry(1.2, 1.2, 3.8, 12),
      new THREE.MeshBasicMaterial({ visible: false }),
    );
    this.collider.position.y = 1.4;
    this.collider.position.z = -0.5;
    this.collider.userData.playerId = player.id;
    this.collider.userData.isPlayerCollider = true;
    this.collider.userData.playerActor = this;
    this.group.add(this.collider);
    
    if (PlayerActor.modelCache) {
        this.initModel(PlayerActor.modelCache.clone());
    } else {
        // Fallback while loading
        const cylinder = new THREE.Mesh(
            new THREE.CylinderGeometry(0.5, 0.5, 3.8, 16),
            new THREE.MeshStandardMaterial({ 
                color: 0x334155, 
                roughness: 0.2,
                emissive: new THREE.Color(0x0f172a),
                emissiveIntensity: 0.6
            })
        );
        cylinder.position.y = 1.9;
        this.group.add(cylinder);
        this.model = cylinder as any;

        PlayerActor.loader.load(`${CDN_BASE}/hm.glb`, (gltf) => {
            PlayerActor.modelCache = gltf.scene;
            
            // Apply materials
            PlayerActor.modelCache.traverse((child) => {
                if ((child as THREE.Mesh).isMesh) {
                    const mesh = child as THREE.Mesh;
                    const material = mesh.material;
                    const mat = Array.isArray(material)
                        ? (material[0] as THREE.MeshStandardMaterial)
                        : (material as THREE.MeshStandardMaterial);
                    if (mat) {
                        mat.emissive = new THREE.Color(0x0f172a);
                        mat.emissiveIntensity = 0.6;
                        mat.roughness = 0.6;
                        mat.metalness = 0.05;
                    }
                }
            });

            // Replace cylinder
            this.group.remove(cylinder);
            this.initModel(PlayerActor.modelCache.clone());
        });
    }

    // Face Light
    const light = new THREE.PointLight(0xff86e5, 0.5, 6);
    light.position.set(0, 2.2, -1.0);
    this.group.add(light);
  }

  private initModel(model: THREE.Group) {
      // Deep-clone materials so emissive changes (hover, target hint, hit
      // flash) stay scoped to this actor — otherwise every PlayerActor
      // shares the same material reference and damage animations bleed
      // across all players.
      model.traverse((child) => {
          if ((child as THREE.Mesh).isMesh) {
              const mesh = child as THREE.Mesh;
              if (Array.isArray(mesh.material)) {
                  mesh.material = mesh.material.map((m) => m.clone());
              } else if (mesh.material) {
                  mesh.material = (mesh.material as THREE.Material).clone();
              }
          }
      });
      this.model = model;
      this.group.add(this.model);
      this.applyVisualState();
  }

  public setHovered(hovered: boolean) {
    if (this.isHovered === hovered) return;
    this.isHovered = hovered;
    this.applyVisualState();
  }

  public setTargetHint(active: boolean) {
    this.applyVisualState(active);
  }

  private applyVisualState(targetHint = false) {
    if (!this.model) return;

    const hovered = this.isHovered;

    // Scale effect
    // Cylinder fallback is height 3.8 at scale 1.0
    // GLB model is roughly half size, so scaled by 2.0 to match
    const baseScale = (this.model as any).isMesh ? 1.0 : 2.0;
    const targetScale = hovered ? baseScale * 1.15 : baseScale;
    this.model.scale.setScalar(targetScale);

    // Glow/Emissive effect
    this.model.traverse((child) => {
      if ((child as THREE.Mesh).isMesh) {
        const mesh = child as THREE.Mesh;
        const mat = (Array.isArray(mesh.material) ? mesh.material[0] : mesh.material) as THREE.MeshStandardMaterial;
        if (mat && mat.emissive) {
          mat.emissiveIntensity = (hovered || targetHint) ? 1.4 : 0.6;
          // Color based on state
          if (targetHint) {
              mat.emissive.set(0xff4444); // Red for target
          } else if (hovered) {
              mat.emissive.set(0x00e5ff); // Cyan for hover
          } else {
              mat.emissive.set(0x0f172a); // Base
          }
        }
      }
    });
  }

  public setPosition(pos: THREE.Vector3, lookAt: THREE.Vector3) {
    this.group.position.copy(pos);
    this.basePos.copy(pos);
    this.group.lookAt(lookAt);
  }

  /** Start a 5s hit-animation (red flash + shake). Call when life drops. */
  public playHitAnim() {
    this.hitAnimTime = PlayerActor.HIT_ANIM_DURATION;
  }

  /** Per-frame update; advance the hit animation if active. */
  public tick(delta: number) {
    if (this.hitAnimTime <= 0) return;
    this.hitAnimTime = Math.max(0, this.hitAnimTime - delta);
    const t = this.hitAnimTime / PlayerActor.HIT_ANIM_DURATION; // 1 → 0
    // Shake: decays quickly in the first second then settles.
    const shakeAmp = Math.max(0, (t - 0.7) / 0.3) * 0.25;
    const shakeX = (Math.random() - 0.5) * shakeAmp;
    const shakeZ = (Math.random() - 0.5) * shakeAmp;
    this.group.position.x = this.basePos.x + shakeX;
    this.group.position.z = this.basePos.z + shakeZ;
    // Red emissive pulse across the whole model.
    if (this.model) {
      this.model.traverse((child) => {
        if ((child as THREE.Mesh).isMesh) {
          const mesh = child as THREE.Mesh;
          const mat = (Array.isArray(mesh.material)
            ? mesh.material[0]
            : mesh.material) as THREE.MeshStandardMaterial;
          if (mat && mat.emissive) {
            const pulse = Math.abs(Math.sin(this.hitAnimTime * 8));
            mat.emissive.set(0xff3030);
            mat.emissiveIntensity = 1.2 + pulse * 1.5;
          }
        }
      });
    }
    if (this.hitAnimTime === 0) {
      // Snap back and restore baseline materials via applyVisualState.
      this.group.position.copy(this.basePos);
      this.applyVisualState();
    }
  }

  public getInteractables() {
    return [this.collider];
  }
}

import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import type { CardView } from '../types';
import type { CardFactory } from './CardFactory';
import { CDN_BASE } from '../cdn';

export class CardActor {
  private static packModelCache: THREE.Group | null = null;
  private static packLoader = new GLTFLoader();
  private static packModelLoading = false;
  private static packAwaitingRefresh = new Set<CardActor>();

  public group: THREE.Group;
  public id: string;

  // State
  private targetPosition = new THREE.Vector3();
  private targetRotation = new THREE.Euler();
  private isHovered = false;
  private isInspected = false;
  private isSick = false;
  private zone: string | null = null;
  private isTargetHint = false;
  private isTargetingSource = false;
  private isLethal = false;
  private isDragOverDiscard = false;
  private isDiscardHighlight = false;
  private isDiscardSelected = false;
  private positionLerp = 0.15;
  private needsSnap = true;
  private baseScale = 1.0;
  private isDraggingActor = false;
  private isInfrastructure = false;
  private isPack = false;
  private fadeOut = false;
  private fadeAlpha = 1.0;
  private fadePrepared = false;
  private baseFrontOpacity = 1.0;
  private baseBackOpacity = 1.0;
  private baseNeonOpacity = 0.9;
  private baseNeonBackOpacity = 0.9;
  private pileCount = 0;
  private pileMeshes: THREE.Mesh[] = [];
  private pileMaterial: THREE.MeshBasicMaterial | null = null;
  private flatLayout = false;
  private frameStyle = 'card';
  private isAttachmentFx = false;
  private attachmentFx: THREE.Group | null = null;
  private ptBillboard: THREE.Sprite | null = null;
  private ptBadges: THREE.Sprite[] = [];
  private ptKey = '';
  private malwareBillboard: THREE.Sprite | null = null;
  private packModel: THREE.Group | null = null;
  private packFallbackMesh: THREE.Mesh | null = null;
  private packTearProgress = 0;
  private packBodyNode: THREE.Object3D | null = null;
  private packFlapTopNode: THREE.Object3D | null = null;
  private packFlapLeftNode: THREE.Object3D | null = null;
  private packFlapRightNode: THREE.Object3D | null = null;
  private packFlapTopBaseQuat: THREE.Quaternion | null = null;
  private packFlapLeftBaseQuat: THREE.Quaternion | null = null;
  private packFlapRightBaseQuat: THREE.Quaternion | null = null;
  private packHasFlapRig = false;
  private packPeelMorphTargets: Array<{ mesh: THREE.Mesh; index: number }> = [];
  private packSplitLeft: THREE.Group | null = null;
  private packSplitRight: THREE.Group | null = null;
  private packSeamTrack: THREE.Mesh | null = null;
  private packSeamFill: THREE.Mesh | null = null;
  private packSeamLength = 0.75;
  private packSeamY = 0;
  private packSeamZ = 0.1;

  // Damage feedback
  private damageTime = 0;
  private damageFlashActive = false;

  // Reveal shell (pack opening)
  private shellMesh: THREE.Mesh | null = null;

  private shellGlowMesh: THREE.Mesh | null = null;
  private shellRevealing = false;
  private shellRevealTime = 0;

  // Visual Components
  private frontMesh!: THREE.Mesh;
  private backMesh!: THREE.Mesh;
  private neonBorder: THREE.LineSegments | null = null;
  private neonBorderBack: THREE.LineSegments | null = null;
  // Animated white scan-line overlay shown on the targeting source card so
  // the player knows which card is asking them to pick targets. Rendered on
  // both the front and back face so the effect is visible regardless of
  // tap/flip orientation.
  private sourceScanMesh: THREE.Mesh | null = null;
  private sourceScanBackMesh: THREE.Mesh | null = null;
  private sourceScanMaterial: THREE.ShaderMaterial | null = null;
  private sicknessGroup: THREE.Group | null = null;
  private sparks: THREE.Sprite[] = [];

  constructor(
    private card: CardView,
    private factory: CardFactory,
  ) {
    this.id = card.id;
    this.group = new THREE.Group();
    this.group.renderOrder = 15;
    this.group.userData.actor = this; // Link back to actor for raycasting

    this.refreshMeshes();
  }

  private refreshMeshes() {
    // 1. Cleanup old components
    this.group.clear();
    this.sparks = [];
    this.sicknessGroup = null;
    this.fadePrepared = false;
    this.pileMeshes = [];
    this.attachmentFx = null;
    this.neonBorder = null;
    this.neonBorderBack = null;
    this.ptBillboard = null;
    this.ptBadges = [];
    this.ptKey = '';
    this.packModel = null;
    this.packFallbackMesh = null;
    this.shellMesh = null;
    this.shellGlowMesh = null;
    this.shellRevealing = false;
    this.shellRevealTime = 0;
    this.packBodyNode = null;
    this.packFlapTopNode = null;
    this.packFlapLeftNode = null;
    this.packFlapRightNode = null;
    this.packFlapTopBaseQuat = null;
    this.packFlapLeftBaseQuat = null;
    this.packFlapRightBaseQuat = null;
    this.packHasFlapRig = false;
    this.packPeelMorphTargets = [];
    this.packSplitLeft = null;
    this.packSplitRight = null;
    this.packTearProgress = 0;
    this.packSeamTrack = null;
    this.packSeamFill = null;

    // 2. Decide representation
    this.frameStyle = this.resolveFrameStyle(this.card);
    this.isAttachmentFx = this.frameStyle === 'attachment_fx';
    this.isPack = this.card.kind_tag === 'pack';
    this.isInfrastructure =
      !this.shouldForceCardRenderInZone() &&
      this.zone !== 'hand' &&
      this.frameStyle === 'infra_hex';
    const useFieldShape = this.isInfrastructure;

    if (this.isAttachmentFx) {
      this.buildAttachmentFx();
      this.syncPileMeshes();
      return;
    }

    if (this.isPack) {
      this.buildPackModel();
    } else if (useFieldShape) {
      // Infrastructure — hex rendering
      const material = this.factory.getInfrastructureMaterial(this.card);
      const size = 0.57;
      this.frontMesh = new THREE.Mesh(
        new THREE.PlaneGeometry(size, size),
        material,
      );

      this.frontMesh.renderOrder = 12;
      this.backMesh = new THREE.Mesh();
      this.group.add(this.frontMesh);
    } else {
      const isUsable =
        this.card.can_attack || this.card.can_tap || this.card.playable;
      this.isSick = this.card.summoning_sickness && !this.card.haste;

      const material = this.factory.getMaterial(
        this.card,
        this.isSick,
        isUsable,
      );

      this.frontMesh = new THREE.Mesh(this.factory.geometry, material);
      this.frontMesh.renderOrder = 15;

      this.backMesh = new THREE.Mesh(this.factory.backGeometry, material);
      this.backMesh.position.z = -0.002;
      this.backMesh.renderOrder = 15;

      this.group.add(this.frontMesh, this.backMesh);

      // Scan-line overlay — hidden until this card is the targeting source.
      this.sourceScanMaterial = new THREE.ShaderMaterial({
        transparent: true,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
        uniforms: {
          time: { value: 0 },
          // 'active' is a reserved GLSL word — use a non-reserved name.
          activeAmt: { value: 0 },
          scanColor: { value: new THREE.Color(1, 1, 1) },
        },
        vertexShader: /* glsl */ `
          varying vec2 vUv;
          void main() {
            vUv = uv;
            gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
          }
        `,
        fragmentShader: /* glsl */ `
          uniform float time;
          uniform float activeAmt;
          uniform vec3 scanColor;
          varying vec2 vUv;
          void main() {
            // Subtle wash — rendered on both faces so whichever face
            // the camera sees always shows the effect.
            float pulse = 0.5 + 0.5 * sin(time * 3.5);
            float wash = 0.16 + pulse * 0.05;
            // Horizontal scan band sweeping top->bottom.
            float scanY = fract(time * 0.55);
            float band = smoothstep(0.03, 0.0, abs(vUv.y - scanY));
            // Soft edge glow so the outline reads without overpowering.
            float edgeX = smoothstep(0.44, 0.5, abs(vUv.x - 0.5));
            float edgeY = smoothstep(0.48, 0.5, abs(vUv.y - 0.5));
            float edge = max(edgeX, edgeY);
            float intensity = (wash + band * 0.5 + edge * 0.45) * activeAmt;
            gl_FragColor = vec4(scanColor, intensity);
          }
        `,
      });
      this.sourceScanMesh = new THREE.Mesh(
        this.factory.geometry,
        this.sourceScanMaterial,
      );
      this.sourceScanMesh.renderOrder = 18;
      this.sourceScanMesh.position.z = 0.003;
      this.sourceScanMesh.visible = false;
      this.group.add(this.sourceScanMesh);

      // Mirrored back-face scan so the effect reads even when the card
      // is tapped/flipped.
      this.sourceScanBackMesh = new THREE.Mesh(
        this.factory.backGeometry,
        this.sourceScanMaterial,
      );
      this.sourceScanBackMesh.renderOrder = 18;
      this.sourceScanBackMesh.position.z = -0.003;
      this.sourceScanBackMesh.visible = false;
      this.group.add(this.sourceScanBackMesh);

      // Neon Borders
      const rarityColors: Record<string, number> = {
        public: 0x94a3b8, // Slate
        private: 0x10b981, // Emerald
        restricted: 0xa855f7, // Purple
        classified: 0xf43f5e, // Rose
        singularity: 0xfacc15, // Yellow/Gold
      };

      let baseColorHex = rarityColors[this.card.rarity] || 0xf59e0b;

      // CS:GO style skin variation using the visual seed
      if (this.card.visual_spec) {
        const seed = this.card.visual_spec.seed;
        const colorObj = new THREE.Color(baseColorHex);
        const h = { h: 0 };
        const s = { s: 0 };
        const l = { l: 0 };
        colorObj.getHSL(h as any);
        // Jitter hue slightly based on seed
        const jitter = ((seed % 100) / 100) * 0.1 - 0.05;
        colorObj.setHSL((h as any).h + jitter, 0.8, 0.6);
        baseColorHex = colorObj.getHex();
      }

      const neonColor = new THREE.Color(baseColorHex).multiplyScalar(2.0);
      const neonMaterial = new THREE.LineBasicMaterial({
        color: neonColor,
        transparent: true,
        opacity: 0.9,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      });

      this.neonBorder = new THREE.LineSegments(
        this.factory.edgesGeometry,
        neonMaterial,
      );
      this.neonBorder.position.z = 0.001;
      this.neonBorder.renderOrder = 20;

      this.neonBorderBack = new THREE.LineSegments(
        this.factory.edgesGeometry,
        neonMaterial,
      );
      this.neonBorderBack.position.z = -0.003;
      this.neonBorderBack.renderOrder = 20;

      this.group.add(this.neonBorder, this.neonBorderBack);

      if (this.isSick) {
        this.addSicknessVFX();
      }
    }

    this.syncPileMeshes();
    this.applyPackTearVisual();
    this.syncPTBillboard();
    this.syncMalwareBillboard();
    this.updateRenderOrderState();
    this.applyBorderState();
    if (this.fadeOut) {
      this.prepareFadeMaterials();
      this.applyFade();
    }
  }

  public update(card: CardView) {
    const oldZone = this.zone;
    const oldKind = this.card.kind_tag;
    const oldIsSick = this.isSick;
    const oldFrameStyle = this.frameStyle;
    const oldTapped = this.card.tapped;

    this.card = card;
    this.fadeOut = Boolean((card as any).fade_out);
    this.frameStyle = this.resolveFrameStyle(card);
    this.isAttachmentFx = this.frameStyle === 'attachment_fx';
    this.isPack = this.card.kind_tag === 'pack';
    this.isInfrastructure =
      !this.shouldForceCardRenderInZone() &&
      this.zone !== 'hand' &&
      this.frameStyle === 'infra_hex';
    const newIsSick = card.summoning_sickness && !card.haste;

    // If major state changed, rebuild meshes
    if (
      oldZone !== this.zone ||
      oldKind !== card.kind_tag ||
      oldIsSick !== newIsSick ||
      oldFrameStyle !== this.frameStyle ||
      oldTapped !== card.tapped
    ) {
      this.refreshMeshes();
    } else {
      // Subtle updates (materials only)
      if (this.isPack) {
        // Pack model visuals are static for now.
      } else if (this.isInfrastructure && this.zone !== 'hand') {
        this.frontMesh.material = this.factory.getInfrastructureMaterial(card);
      } else if (this.isAttachmentFx) {
        // Attachment FX materials are rebuilt on refresh only
      } else {
        const isUsable = card.can_attack || card.can_tap || card.playable;
        const material = this.factory.getMaterial(card, this.isSick, isUsable);
        this.frontMesh.material = material;
        this.backMesh.material = material;

        // Update uniforms if it's a shader material
        if (material instanceof THREE.ShaderMaterial) {
          material.uniforms.isSick.value = this.isSick ? 1.0 : 0.0;
          material.uniforms.isUsable.value = isUsable ? 1.0 : 0.0;
        }
      }
      this.applyBorderState();
      this.syncPTBillboard();
      this.syncMalwareBillboard();
    }

    if (this.fadeOut) {
      this.prepareFadeMaterials();
      this.applyFade();
    } else if (this.fadeAlpha < 1.0) {
      this.fadeAlpha = 1.0;
      this.applyFade();
    }
  }

  private prepareFadeMaterials() {
    if (this.fadePrepared) return;
    this.fadePrepared = true;

    if (this.frontMesh?.material) {
      this.frontMesh.material = (
        this.frontMesh.material as THREE.Material
      ).clone();
      const mat = this.frontMesh.material as any;
      this.baseFrontOpacity =
        typeof mat.opacity === 'number' ? mat.opacity : 1.0;
    }
    if (this.backMesh?.material) {
      this.backMesh.material = (
        this.backMesh.material as THREE.Material
      ).clone();
      const mat = this.backMesh.material as any;
      this.baseBackOpacity =
        typeof mat.opacity === 'number' ? mat.opacity : 1.0;
    }
    if (this.neonBorder?.material) {
      this.neonBorder.material = (
        this.neonBorder.material as THREE.Material
      ).clone();
      const mat = this.neonBorder.material as any;
      this.baseNeonOpacity =
        typeof mat.opacity === 'number' ? mat.opacity : 0.9;
    }
    if (this.neonBorderBack?.material) {
      this.neonBorderBack.material = (
        this.neonBorderBack.material as THREE.Material
      ).clone();
      const mat = this.neonBorderBack.material as any;
      this.baseNeonBackOpacity =
        typeof mat.opacity === 'number' ? mat.opacity : 0.9;
    }
  }

  private applyFade() {
    const applyOpacity = (mat: any, opacity: number) => {
      if (!mat || typeof mat.opacity !== 'number') return;
      mat.transparent = true;
      mat.opacity = opacity;
    };

    applyOpacity(
      this.frontMesh?.material as any,
      this.baseFrontOpacity * this.fadeAlpha,
    );
    applyOpacity(
      this.backMesh?.material as any,
      this.baseBackOpacity * this.fadeAlpha,
    );
    applyOpacity(
      this.neonBorder?.material as any,
      this.baseNeonOpacity * this.fadeAlpha,
    );
    applyOpacity(
      this.neonBorderBack?.material as any,
      this.baseNeonBackOpacity * this.fadeAlpha,
    );
    this.pileMeshes.forEach((mesh) => {
      applyOpacity(mesh.material as any, this.fadeAlpha);
    });

    if (this.sicknessGroup) {
      this.sicknessGroup.visible = this.fadeAlpha > 0.1;
    }
    if (this.ptBillboard) {
      applyOpacity(this.ptBillboard.material as any, this.fadeAlpha);
    }
    for (const b of this.ptBadges) {
      applyOpacity(b.material as any, this.fadeAlpha);
    }
  }

  private buildPackModel() {
    this.frontMesh = new THREE.Mesh();
    this.backMesh = new THREE.Mesh();

    const applyModelVisuals = (model: THREE.Group) => {
      model.traverse((child) => {
        if (!(child as THREE.Mesh).isMesh) return;
        const mesh = child as THREE.Mesh;
        mesh.material = new THREE.MeshStandardMaterial({
          color: 0xf59e0b,
          roughness: 0.58,
          metalness: 0.18,
          emissive: new THREE.Color(0x78350f),
          emissiveIntensity: 0.2,
        });
      });
    };

    if (CardActor.packModelCache) {
      this.packModel = CardActor.packModelCache.clone(true);
      applyModelVisuals(this.packModel);
      this.fitPackModelToCard(this.packModel);
      this.bindPackRigNodes(this.packModel);
      this.bindPackMorphTargets(this.packModel);
      this.group.add(this.packModel);
      this.createPackSeamGuide();
      return;
    }

    this.packFallbackMesh = new THREE.Mesh(
      new THREE.BoxGeometry(0.62, 0.09, 0.92),
      new THREE.MeshStandardMaterial({
        color: 0xf59e0b,
        roughness: 0.6,
        metalness: 0.1,
        emissive: new THREE.Color(0x78350f),
        emissiveIntensity: 0.25,
      }),
    );
    this.packFallbackMesh.position.set(0, 0.05, 0);
    this.group.add(this.packFallbackMesh);
    this.frontMesh = this.packFallbackMesh;
    this.backMesh = this.packFallbackMesh;
    this.createPackSeamGuide();

    CardActor.packAwaitingRefresh.add(this);
    if (CardActor.packModelLoading) return;
    CardActor.packModelLoading = true;
    CardActor.packLoader.load(
      `${CDN_BASE}/bag.glb`,
      (gltf) => {
        CardActor.packModelCache = gltf.scene;
        CardActor.packModelLoading = false;
        Array.from(CardActor.packAwaitingRefresh).forEach((actor) =>
          actor.refreshMeshes(),
        );
        CardActor.packAwaitingRefresh.clear();
      },
      undefined,
      () => {
        CardActor.packModelLoading = false;
      },
    );
  }

  private normalizeNodeName(name: string) {
    return (name || '')
      .toLowerCase()
      .replace(/[\s-]+/g, '_')
      .replace(/[^a-z0-9_]/g, '');
  }

  private findNodeByNames(
    root: THREE.Object3D,
    expectedNames: string[],
  ): THREE.Object3D | null {
    const expected = new Set(
      expectedNames.map((name) => this.normalizeNodeName(name)),
    );
    let found: THREE.Object3D | null = null;
    root.traverse((child) => {
      if (found) return;
      const normalized = this.normalizeNodeName(child.name || '');
      if (expected.has(normalized)) found = child;
    });
    return found;
  }

  private findNodeByNameIncludes(
    root: THREE.Object3D,
    requiredTokens: string[],
  ): THREE.Object3D | null {
    let found: THREE.Object3D | null = null;
    root.traverse((child) => {
      if (found) return;
      const normalized = this.normalizeNodeName(child.name || '');
      if (!normalized) return;
      const hasAll = requiredTokens.every((token) =>
        normalized.includes(this.normalizeNodeName(token)),
      );
      if (hasAll) found = child;
    });
    return found;
  }

  private bindPackRigNodes(model: THREE.Group) {
    this.packBodyNode = this.findNodeByNames(model, ['pack_body', 'pack-body']);
    if (!this.packBodyNode) {
      this.packBodyNode = this.findNodeByNameIncludes(model, ['body']);
    }
    this.packFlapTopNode = this.findNodeByNames(model, [
      'pack_flap_top',
      'pack-flap-top',
      'pack_top_flap',
      'pack-top-flap',
      'pack_flap',
      'pack-flap',
    ]);
    if (!this.packFlapTopNode) {
      this.packFlapTopNode = this.findNodeByNameIncludes(model, [
        'flap',
        'top',
      ]);
    }
    if (!this.packFlapTopNode) {
      this.packFlapTopNode = this.findNodeByNameIncludes(model, ['flap']);
    }
    this.packFlapLeftNode = this.findNodeByNames(model, [
      'pack_flap_left',
      'pack-flap-left',
      'pack_left_flap',
      'pack-left-flap',
    ]);
    if (!this.packFlapLeftNode) {
      this.packFlapLeftNode = this.findNodeByNameIncludes(model, [
        'flap',
        'left',
      ]);
    }
    this.packFlapRightNode = this.findNodeByNames(model, [
      'pack_flap_right',
      'pack-flap-right',
      'pack_right_flap',
      'pack-right-flap',
    ]);
    if (!this.packFlapRightNode) {
      this.packFlapRightNode = this.findNodeByNameIncludes(model, [
        'flap',
        'right',
      ]);
    }
    this.packFlapTopBaseQuat = this.packFlapTopNode
      ? this.packFlapTopNode.quaternion.clone()
      : null;
    this.packFlapLeftBaseQuat = this.packFlapLeftNode
      ? this.packFlapLeftNode.quaternion.clone()
      : null;
    this.packFlapRightBaseQuat = this.packFlapRightNode
      ? this.packFlapRightNode.quaternion.clone()
      : null;
    this.packHasFlapRig =
      !!this.packFlapTopNode ||
      (!!this.packFlapLeftNode && !!this.packFlapRightNode);
  }

  private bindPackMorphTargets(model: THREE.Group) {
    this.packPeelMorphTargets = [];
    const preferredTokens = [
      'peelcurl_1',
      'peelcurl',
      'peel',
      'curl',
      'tear',
      'open',
      'flap',
    ];

    model.traverse((child) => {
      if (!(child as THREE.Mesh).isMesh) return;
      const mesh = child as THREE.Mesh;
      const dict = (mesh as any).morphTargetDictionary as
        | Record<string, number>
        | undefined;
      const influences = (mesh as any).morphTargetInfluences as
        | number[]
        | undefined;
      if (!dict || !influences || influences.length === 0) return;

      const entries = Object.entries(dict);
      if (entries.length === 0) return;

      let targetIndex = -1;
      for (const token of preferredTokens) {
        const found = entries.find(([name]) =>
          this.normalizeNodeName(name).includes(token),
        );
        if (found) {
          targetIndex = found[1];
          break;
        }
      }
      if (targetIndex < 0) targetIndex = entries[0][1];
      if (targetIndex < 0 || targetIndex >= influences.length) return;

      this.packPeelMorphTargets.push({ mesh, index: targetIndex });
    });
  }

  private setPackPeelInfluence(value: number) {
    const influence = THREE.MathUtils.clamp(value, 0, 1);
    this.packPeelMorphTargets.forEach(({ mesh, index }) => {
      const influences = (mesh as any).morphTargetInfluences as
        | number[]
        | undefined;
      if (!influences || index >= influences.length) return;
      influences[index] = influence;
    });
  }

  private fitPackModelToCard(model: THREE.Group) {
    // Keep pack upright in battlefield space.
    model.rotation.set(Math.PI / 2, 0, 0);

    const initialBox = new THREE.Box3().setFromObject(model);
    const initialSize = initialBox.getSize(new THREE.Vector3());
    const maxDim = Math.max(initialSize.x, initialSize.y, initialSize.z, 0.001);

    // Keep pack roughly card-sized on table.
    const targetMaxDim = this.factory.cardH * 0.78 * 2.5;
    const scale = targetMaxDim / maxDim;
    model.scale.setScalar(scale);

    // Center on origin and lift so it sits on table plane.
    const scaledBox = new THREE.Box3().setFromObject(model);
    const center = scaledBox.getCenter(new THREE.Vector3());
    model.position.sub(center);

    const groundedBox = new THREE.Box3().setFromObject(model);
    // Card actors are laid flat by parent rotation; local Z maps to table height.
    model.position.z -= groundedBox.min.z;

    const finalBox = new THREE.Box3().setFromObject(model);
    const finalSize = finalBox.getSize(new THREE.Vector3());
    this.packSeamLength = Math.max(0.5, finalSize.x * 0.95);
    // For this asset/actor orientation, local Z is the visible vertical axis.
    this.packSeamY = 0;
    this.packSeamZ = finalBox.max.z - 0.3;
  }

  public setPackTearProgress(progress: number) {
    const next = THREE.MathUtils.clamp(progress, 0, 1);
    if (Math.abs(this.packTearProgress - next) < 0.001) return;
    this.packTearProgress = next;
    this.applyPackTearVisual();
  }

  private createSplitPackHalf(side: -1 | 1) {
    const base = CardActor.packModelCache
      ? CardActor.packModelCache.clone(true)
      : null;
    if (!base) return null;
    this.fitPackModelToCard(base);
    // Split into two flap halves without vertical displacement cues.
    base.scale.x *= 0.5;
    base.position.x += side * 0.08;
    base.traverse((child) => {
      if (!(child as THREE.Mesh).isMesh) return;
      const mesh = child as THREE.Mesh;
      mesh.material = new THREE.MeshStandardMaterial({
        color: 0xf59e0b,
        roughness: 0.58,
        metalness: 0.18,
        emissive: new THREE.Color(0x78350f),
        emissiveIntensity: 0.2,
      });
    });
    return base;
  }

  private applyPackTearVisual() {
    if (!this.isPack) return;

    const progress = this.packTearProgress;

    if (this.packSeamFill) {
      const showSeam = progress > 0.001 && progress < 0.98;
      this.packSeamFill.visible = showSeam;
      this.packSeamFill.scale.x = Math.max(0.001, progress);
      this.packSeamFill.position.x =
        -this.packSeamLength * 0.5 + this.packSeamLength * progress * 0.5;
    }

    if (progress <= 0.001) {
      if (this.packModel) this.packModel.visible = true;
      if (this.packFallbackMesh) this.packFallbackMesh.visible = true;
      this.setPackPeelInfluence(0);
      if (this.packFlapTopNode && this.packFlapTopBaseQuat) {
        this.packFlapTopNode.quaternion.copy(this.packFlapTopBaseQuat);
      }
      return;
    }

    if (this.packModel) this.packModel.visible = true;
    if (this.packFallbackMesh) this.packFallbackMesh.visible = false;

    // Priority 1: Morph Targets (e.g. PeelCurl 1)
    if (this.packPeelMorphTargets.length > 0) {
      this.setPackPeelInfluence(Math.pow(progress, 0.85));
    }

    // Priority 2: Rig Flaps
    if (this.packFlapTopNode && this.packFlapTopBaseQuat) {
      const rotationScale = this.packPeelMorphTargets.length > 0 ? 0.2 : 1.0;
      const flapOpen = Math.pow(progress, 0.5) * Math.PI * 0.8 * rotationScale;

      const deltaLift = new THREE.Quaternion().setFromAxisAngle(
        new THREE.Vector3(1, 0, 0),
        -flapOpen,
      );
      this.packFlapTopNode.quaternion.copy(this.packFlapTopBaseQuat);
      this.packFlapTopNode.quaternion.premultiply(deltaLift);
    }
  }

  private createPackSeamGuide() {
    const seamThickness = 0.01;
    const fillMat = new THREE.MeshBasicMaterial({
      color: 0x22d3ee,
      transparent: true,
      opacity: 1.0,
      depthTest: false,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    this.packSeamTrack = null;

    this.packSeamFill = new THREE.Mesh(
      new THREE.BoxGeometry(
        this.packSeamLength,
        seamThickness * 0.55,
        seamThickness * 0.75,
      ),
      fillMat,
    );
    this.packSeamFill.position.set(
      -this.packSeamLength * 0.5,
      this.packSeamY,
      this.packSeamZ + 0.001,
    );
    this.packSeamFill.scale.x = 0.001;
    this.packSeamFill.visible = false;
    this.packSeamFill.renderOrder = 41;
    this.group.add(this.packSeamFill);
  }

  private addSicknessVFX() {
    if (this.sicknessGroup) return;

    this.sicknessGroup = new THREE.Group();
    this.group.add(this.sicknessGroup);

    // Sparks
    const sparkCount = 12;
    for (let j = 0; j < sparkCount; j++) {
      const spark = new THREE.Sprite(this.factory.sparkMaterial.clone());
      spark.renderOrder = 102;
      this.resetSpark(spark);
      this.sicknessGroup.add(spark);
      this.sparks.push(spark);
    }
  }

  private removeSicknessVFX() {
    if (this.sicknessGroup) {
      this.group.remove(this.sicknessGroup);
      this.sicknessGroup = null;
      this.sparks = [];
    }
  }

  private syncPileMeshes() {
    if (this.pileMeshes.length > 0) {
      this.pileMeshes.forEach((mesh) => this.group.remove(mesh));
      this.pileMeshes = [];
    }

    // Pack actors use a dedicated 3D model; card-tail pile meshes look wrong behind it.
    if (this.isPack) {
      return;
    }

    if (this.pileCount <= 0) {
      return;
    }

    if (!this.pileMaterial) {
      this.pileMaterial = this.factory.getBackMaterial().clone();
      this.pileMaterial.transparent = true;
      this.pileMaterial.opacity = 0.9;
      this.pileMaterial.depthWrite = false;
      this.pileMaterial.side = THREE.DoubleSide;
    }

    // Visual cap for piles to prevent massive "tails"
    const visualPileCap = 6;
    const effectivePileCount = Math.min(this.pileCount, visualPileCap);

    for (let i = 0; i < effectivePileCount; i += 1) {
      const mesh = new THREE.Mesh(this.factory.backGeometry, this.pileMaterial);

      if (this.flatLayout) {
        // Tightened fan out logic to prevent overlapping with neighboring cards
        const step = i + 1;
        const fanX = step * 0.045; // Reduced from 0.12
        const fanY = step * 0.005; // Reduced from 0.06 to keep it closer to the table
        const fanRot = step * 0.02; // Subtler rotation

        mesh.position.set(fanX, fanY, -0.005 - i * 0.005);
        mesh.rotation.z = fanRot;
      } else {
        // Stacked logic - Tight 3D Deck look
        // Cards are vertical, so stack goes in local Z (depth)
        mesh.position.set(
          (Math.random() - 0.5) * 0.005, // Tiny jitter for realism
          (Math.random() - 0.5) * 0.005,
          -0.012 * (i + 1),
        );
        mesh.rotation.z = (Math.random() - 0.5) * 0.02;
      }

      mesh.renderOrder = 14; // Just below front/back meshes (15)
      this.group.add(mesh);
      this.pileMeshes.push(mesh);
    }
  }

  private syncPTBillboard() {
    const c = this.card;
    const show =
      this.zone === 'battlefield' &&
      !this.isInfrastructure &&
      !this.isAttachmentFx &&
      c.power != null &&
      c.toughness != null;

    const bp = (c as any).base_power ?? null;
    const bt = (c as any).base_toughness ?? null;
    const damage = (c as any).damage ?? 0;
    const nextKey = show ? `pt:${c.power}/${c.toughness}:${bp}/${bt}:${damage}` : '';

    if (nextKey === this.ptKey) return;
    this.ptKey = nextKey;

    // Tear down any previous billboard and any legacy hex badges.
    if (this.ptBillboard) {
      this.group.remove(this.ptBillboard);
      this.ptBillboard = null;
    }
    for (const b of this.ptBadges) this.group.remove(b);
    this.ptBadges = [];

    if (!show) return;

    const textureUtils = this.factory.getTextureUtils();
    const sprite = textureUtils.createPTBillboard(
      c.power!,
      c.toughness!,
      bp,
      bt,
      this.factory.cardW,
      damage,
    );
    if (!sprite) return;

    sprite.position.set(
      0,
      this.factory.cardH * 0.5 + 0.12,
      0.02,
    );
    sprite.center.set(0.5, 0.5);
    this.ptBillboard = sprite;
    this.group.add(sprite);
  }

  private syncMalwareBillboard() {
    const c = this.card;
    const show =
      this.zone === 'battlefield' &&
      !this.isInfrastructure &&
      !this.isAttachmentFx &&
      !!(c as any).malware_infected;

    if (show && this.malwareBillboard) return;
    if (!show && !this.malwareBillboard) return;

    if (this.malwareBillboard) {
      this.group.remove(this.malwareBillboard);
      const mat = this.malwareBillboard.material as THREE.SpriteMaterial;
      mat.map?.dispose();
      mat.dispose();
      this.malwareBillboard = null;
    }

    if (!show) return;

    const textureUtils = this.factory.getTextureUtils();
    const sprite = textureUtils.createMalwareBillboard?.();
    if (!sprite) return;

    // Position at top-center of card, slightly above
    sprite.position.set(0, this.factory.cardH * 0.52, 0.06);
    sprite.center.set(0.5, 0.5);

    this.malwareBillboard = sprite;
    this.group.add(sprite);
  }

  private resetSpark(spark: THREE.Sprite) {
    const boundsW = this.factory.cardW * 0.6;
    const boundsH = this.factory.cardH * 0.6;

    spark.position.x = (Math.random() * 2 - 1) * boundsW;
    spark.position.y = -boundsH;
    spark.position.z = (Math.random() - 0.5) * 0.15;

    const scale = 0.03 + Math.random() * 0.03;
    spark.scale.set(scale, scale, 1);

    spark.userData = {
      velocity: new THREE.Vector3((Math.random() - 0.5) * 0.12, 1, 0)
        .normalize()
        .multiplyScalar(0.25 + Math.random() * 0.35),
      life: 0.4 + Math.random() * 0.6,
      maxLife: 0.4 + Math.random() * 0.6,
      spin: (Math.random() - 0.5) * 0.4,
      phase: Math.random() * Math.PI * 2,
    };
    (spark.material as THREE.SpriteMaterial).opacity = 0;
  }

  private savedBackMaterial: THREE.Material | null = null;

  private updateRenderOrderState() {
    const inspectedOffset = this.isInspected ? 100 : 0;
    const hoveredHandOffset = this.isHovered && this.zone === 'hand' ? 40 : 0;
    const offset = inspectedOffset + hoveredHandOffset;

    this.group.renderOrder = 15 + offset;
    if (this.frontMesh) this.frontMesh.renderOrder = 15 + offset;
    if (this.backMesh) this.backMesh.renderOrder = 15 + offset;
    if (this.neonBorder) this.neonBorder.renderOrder = 20 + offset;
    if (this.neonBorderBack) this.neonBorderBack.renderOrder = 20 + offset;
    if (this.ptBillboard) this.ptBillboard.renderOrder = 100 + offset;
    for (const b of this.ptBadges) b.renderOrder = 102 + offset;
    if (this.shellMesh) this.shellMesh.renderOrder = 25 + offset;
    if (this.shellGlowMesh) this.shellGlowMesh.renderOrder = 24 + offset;
    // Source scan overlay must sit above the card's front mesh — otherwise a
    // hover offset pushes the front past the hard-coded overlay renderOrder
    // and the scan becomes hidden behind the card art.
    if (this.sourceScanMesh) this.sourceScanMesh.renderOrder = 18 + offset;
    if (this.sourceScanBackMesh) this.sourceScanBackMesh.renderOrder = 18 + offset;
    this.pileMeshes.forEach((m) => (m.renderOrder = 14 + offset));
  }

  public setInspected(inspected: boolean) {
    if (this.isInspected === inspected) return;

    this.isInspected = inspected;
    this.updateRenderOrderState();

    // Swap back mesh to card back texture when inspecting
    if (this.backMesh && !this.isPack) {
      if (inspected) {
        this.savedBackMaterial = this.backMesh.material as THREE.Material;
        this.backMesh.material = this.factory.getBackMaterial();
      } else if (this.savedBackMaterial) {
        this.backMesh.material = this.savedBackMaterial;
        this.savedBackMaterial = null;
      }
    }
  }

  /** Mark this card as predicted to die in the current combat resolution. */
  public setLethal(active: boolean) {
    this.isLethal = active;
    this.applyBorderState();
  }

  public setHovered(hovered: boolean) {
    this.isHovered = hovered;
    this.updateRenderOrderState();
    this.applyBorderState();
  }

  public isHoveredState() {
    return this.isHovered;
  }

  public setTargetHint(active: boolean) {
    this.isTargetHint = active;
    this.applyBorderState();
  }

  /**
   * Mark this card as the source of an in-progress targeting action so it
   * stands out distinctly from the valid-target glow. Shows a white scan-
   * line overlay on the card face. Client-local — other players' viewers
   * don't see this highlight.
   */
  public setTargetingSource(active: boolean) {
    this.isTargetingSource = active;
    if (this.sourceScanMesh && this.sourceScanMaterial) {
      const shouldShow = active || this.isDiscardHighlight;
      this.sourceScanMesh.visible = shouldShow;
      if (this.sourceScanBackMesh) this.sourceScanBackMesh.visible = shouldShow;
      this.sourceScanMaterial.uniforms.activeAmt.value = shouldShow ? 1.0 : 0.0;
      const color = this.sourceScanMaterial.uniforms.scanColor.value as THREE.Color;
      if (this.isDiscardHighlight) color.setHex(0xa855f7);
      else if (active) color.setHex(0xffffff);
    }
    this.applyBorderState();
  }

  public isTargetingSourceState() {
    return this.isTargetingSource;
  }

  public setDragOverDiscard(active: boolean) {
    this.isDragOverDiscard = active;
    this.applyBorderState();
  }

  /**
   * Mark this hand card as selected for the current forced discard — shows
   * a purple scan overlay (reusing the targeting-source shader, tinted).
   * Cleared when the card leaves discard selection.
   */
  public setDiscardHighlight(active: boolean) {
    this.isDiscardHighlight = active;
    if (this.sourceScanMesh && this.sourceScanMaterial) {
      const shouldShow = active || this.isTargetingSource;
      this.sourceScanMesh.visible = shouldShow;
      if (this.sourceScanBackMesh) this.sourceScanBackMesh.visible = shouldShow;
      this.sourceScanMaterial.uniforms.activeAmt.value = shouldShow ? 1.0 : 0.0;
      const color = this.sourceScanMaterial.uniforms.scanColor.value as THREE.Color;
      if (active) color.setHex(0xa855f7);
      else if (this.isTargetingSource) color.setHex(0xffffff);
    }
    this.applyBorderState();
  }

  public setDiscardSelected(active: boolean) {
    this.isDiscardSelected = active;
    this.applyBorderState();
  }

  /**
   * Sucks the card into a portal position on the table: lerps the card to
   * the portal center while shrinking and spinning, then fades it out.
   * Used for discard commit so selected cards visibly drop into the hole.
   */
  public animateIntoPortal(portalPos: THREE.Vector3, delaySpinMs = 0) {
    this.positionLerp = 0.32;
    this.targetPosition.copy(portalPos);
    this.targetRotation.set(-Math.PI / 2, 0, Math.random() * Math.PI * 2);
    this.baseScale = 0.15;
    this.isDiscardHighlight = false;
    this.isDiscardSelected = false;
    this.isTargetHint = false;
    this.applyBorderState();
    if (this.sourceScanMesh) this.sourceScanMesh.visible = false;
    if (this.sourceScanBackMesh) this.sourceScanBackMesh.visible = false;
    if (delaySpinMs > 0) {
      setTimeout(() => {
        this.fadeOut = true;
      }, delaySpinMs);
    } else {
      this.fadeOut = true;
    }
  }

  private applyBorderState() {
    if (!this.neonBorder || !this.neonBorderBack) return;

    if (this.damageFlashActive) {
      // Red flash on damage
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(0xff0000);
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 1.0;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(0xff0000);
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 1.0;
      return;
    }

    if (this.isSick) {
      // Dim gray border for sick cards — the only color accent on an
      // otherwise grayscale card, kept subtle so it doesn't shout.
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(
        0x64748b,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 0.35;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(
        0x64748b,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 0.35;
      return;
    }

    if (this.isDragOverDiscard) {
      // Purple border when dragging over discard zone
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(
        0x9333ea,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 1.0;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(
        0x9333ea,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 1.0;
    } else if (this.isDiscardHighlight) {
      // Purple border while in discard-selection mode. Selected cards get
      // a brighter, more saturated purple so selection state reads.
      const hex = this.isDiscardSelected ? 0xc084fc : 0x9333ea;
      const opacity = this.isDiscardSelected ? 1.0 : 0.75;
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(hex);
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = opacity;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(hex);
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = opacity;
    } else if (this.isTargetingSource) {
      // Source of an in-progress targeting action — amber, bright, and
      // checked before isHovered so hovering the source still reads as the
      // "chosen" card rather than a generic hover highlight.
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(
        0xfbbf24,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 1.0;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(
        0xfbbf24,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 1.0;
    } else if (this.isLethal) {
      // Pulsing red border — this unit is predicted to die in combat.
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(
        0xff2222,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 1.0;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(
        0xff2222,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 1.0;
    } else if (this.isTargetHint) {
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(
        0xffffff,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 0.95;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(
        0xffffff,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 0.95;
    } else if (this.isHovered) {
      (this.neonBorder.material as THREE.LineBasicMaterial).color.setHex(
        0xffffff,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 1.0;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.setHex(
        0xffffff,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 1.0;
    } else {
      const baseColor =
        this.card.colors && this.card.colors.length > 0
          ? this.card.colors[0]
          : '#f59e0b';
      const neonColor = new THREE.Color(baseColor).multiplyScalar(2.0);
      (this.neonBorder.material as THREE.LineBasicMaterial).color.copy(
        neonColor,
      );
      (this.neonBorder.material as THREE.LineBasicMaterial).opacity = 0.9;
      (this.neonBorderBack.material as THREE.LineBasicMaterial).color.copy(
        neonColor,
      );
      (this.neonBorderBack.material as THREE.LineBasicMaterial).opacity = 0.9;
    }
  }

  public getCard() {
    return this.card;
  }

  public isInHand() {
    return this.zone === 'hand';
  }

  public getZone() {
    return this.zone;
  }

  public setZone(zone: string | null) {
    if (this.zone === zone) return;
    this.zone = zone;
    this.refreshMeshes();
  }

  private shouldForceCardRenderInZone() {
    return this.zone === 'draft_pyramid' || this.zone === 'draft_side';
  }

  public setDragging(dragging: boolean) {
    this.isDraggingActor = dragging;
  }

  public setPileCount(count: number) {
    const next = Math.max(0, count);
    if (this.pileCount === next) return;
    this.pileCount = next;
    this.syncPileMeshes();
  }

  public setFlatLayout(flat: boolean) {
    if (this.flatLayout === flat) return;
    this.flatLayout = flat;
    this.syncPileMeshes();
  }

  public getInteractables() {
    if (this.isAttachmentFx) return [];
    if (this.isPack) {
      const packHits: THREE.Object3D[] = [];
      if (this.packSplitLeft?.visible) {
        this.packSplitLeft.traverse((child) => {
          if ((child as THREE.Mesh).isMesh) packHits.push(child);
        });
      }
      if (this.packSplitRight?.visible) {
        this.packSplitRight.traverse((child) => {
          if ((child as THREE.Mesh).isMesh) packHits.push(child);
        });
      }
      if (this.packModel) {
        this.packModel.traverse((child) => {
          if ((child as THREE.Mesh).isMesh) {
            packHits.push(child);
          }
        });
      }
      if (packHits.length > 0) return packHits;
    }
    // Only raycast against the solid card meshes, ignoring VFX/Borders
    const interactables = [this.frontMesh, this.backMesh];
    if (this.shellMesh?.visible) interactables.push(this.shellMesh);
    return interactables;
  }

  public setTarget(
    pos: THREE.Vector3,
    rot: THREE.Euler,
    scale: number = 1.0,
    positionLerp: number = 0.15,
  ) {
    this.targetPosition.copy(pos);
    this.targetRotation.copy(rot);
    this.baseScale = scale;
    this.positionLerp = positionLerp;

    if (this.needsSnap) {
      this.needsSnap = false;
      this.group.position.copy(pos);
      this.group.rotation.copy(rot);
      this.group.scale.setScalar(scale);
    }
  }

  /**
   * Disable the initial snap-to-target behavior so the actor lerps from
   * its current position (useful for fly-in animations where the caller
   * has seeded the start position).
   */
  public disableSnap() {
    this.needsSnap = false;
  }

  public takeDamage(amount?: number) {
    this.damageTime = 0.8;
    this.damageFlashActive = true;
    this.applyBorderState();
    if (amount && amount > 0) {
      this.spawnDamageNumber(amount);
    }
  }

  private spawnDamageNumber(amount: number) {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 192;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.font = '900 140px "Orbitron", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.shadowColor = '#7f1d1d';
    ctx.shadowBlur = 18;
    ctx.lineWidth = 8;
    ctx.strokeStyle = '#000000';
    ctx.fillStyle = '#ff3030';
    const text = `-${amount}`;
    ctx.strokeText(text, canvas.width / 2, canvas.height / 2);
    ctx.fillText(text, canvas.width / 2, canvas.height / 2);

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const mat = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(mat);
    sprite.scale.set(0.55, 0.42, 1);
    sprite.position.set(0, this.factory.cardH * 0.18, 0.12);
    sprite.renderOrder = 9999;
    this.group.add(sprite);

    const startY = sprite.position.y;
    const start = performance.now();
    const duration = 1100;
    const animate = () => {
      const elapsed = performance.now() - start;
      const t = Math.min(1, elapsed / duration);
      sprite.position.y = startY + t * 0.55;
      mat.opacity = 1.0 - Math.pow(t, 2.4);
      const scale = 1.0 + t * 0.4;
      sprite.scale.set(0.55 * scale, 0.42 * scale, 1);
      if (t >= 1) {
        this.group.remove(sprite);
        texture.dispose();
        mat.dispose();
        return;
      }
      requestAnimationFrame(animate);
    };
    requestAnimationFrame(animate);
  }

  public tick(delta: number, time: number) {
    // Update shader time if applicable
    if (this.frontMesh?.material instanceof THREE.ShaderMaterial) {
      this.frontMesh.material.uniforms.time.value = time;
    }
    if (this.backMesh?.material instanceof THREE.ShaderMaterial) {
      this.backMesh.material.uniforms.time.value = time;
    }
    if (
      this.sourceScanMaterial &&
      (this.isTargetingSource || this.isDiscardHighlight)
    ) {
      this.sourceScanMaterial.uniforms.time.value = time;
    }

    if (this.fadeOut) {
      this.fadeAlpha = Math.max(0, this.fadeAlpha - delta * 1.4);
      this.applyFade();
    }

    // Damage feedback
    if (this.damageTime > 0) {
      this.damageTime -= delta;
      if (this.damageTime <= 0) {
        this.damageFlashActive = false;
        this.applyBorderState();
      }
    }

    // Optimization: If stationary and not hovered/dragging, skip expensive lerps
    const isMoving =
      this.group.position.distanceToSquared(this.targetPosition) > 0.0001;
    const isRotating =
      Math.abs(this.group.rotation.x - this.targetRotation.x) > 0.001 ||
      Math.abs(this.group.rotation.y - this.targetRotation.y) > 0.001 ||
      Math.abs(this.group.rotation.z - this.targetRotation.z) > 0.001;
    if (
      !isMoving &&
      !isRotating &&
      !this.isHovered &&
      !this.isDraggingActor &&
      !this.isTargetHint &&
      !this.isTargetingSource &&
      this.fadeAlpha >= 1.0 &&
      !this.shellMesh?.visible &&
      this.damageTime <= 0 &&
      !this.isSick &&
      !this.isAttachmentFx
    ) {
      return;
    }

    // 1. Lerp Scale (Always lerp scale for hover/drag feedback)
    let targetScaleMult = 1.0;
    if (this.isHovered || this.isDraggingActor) {
      targetScaleMult = 1.1;
    }
    if (this.isTargetHint) {
      targetScaleMult = Math.max(targetScaleMult, 1.08);
    }
    if (this.isAttachmentFx) {
      targetScaleMult = 1.0;
    }

    const currentScale = this.group.scale.x;
    const targetScale = this.baseScale * targetScaleMult;
    const newScale = THREE.MathUtils.lerp(currentScale, targetScale, 0.15);
    this.group.scale.setScalar(newScale);

    if (this.isDraggingActor) return;

    // 2. Lerp Position (X/Z only), constrain Y to scale
    const handHoverDepth = 0;
    const targetX = this.targetPosition.x;
    const targetZ = this.targetPosition.z + handHoverDepth;

    let posX = THREE.MathUtils.lerp(
      this.group.position.x,
      targetX,
      this.positionLerp,
    );
    let posZ = THREE.MathUtils.lerp(
      this.group.position.z,
      targetZ,
      this.positionLerp,
    );

    // Apply damage shake
    if (this.damageTime > 0) {
      const shakeAmt = this.damageTime * 0.4;
      posX += (Math.random() - 0.5) * shakeAmt;
      posZ += (Math.random() - 0.5) * shakeAmt;
    }

    this.group.position.x = posX;
    this.group.position.z = posZ;

    const handHoverLift = 0;

    if (this.isInfrastructure || this.flatLayout || this.isAttachmentFx) {
      // Infrastructure stays flat on the table
      let posY = THREE.MathUtils.lerp(
        this.group.position.y,
        this.targetPosition.y + handHoverLift,
        this.positionLerp,
      );
      if (this.damageTime > 0) {
        posY += (Math.random() - 0.5) * this.damageTime * 0.4;
      }
      this.group.position.y = posY;
    } else {
      // Anchor bottom while scaling: adjust Y based on current scale
      const height = this.factory.cardH;
      const baseBottom =
        this.targetPosition.y + handHoverLift - height * this.baseScale * 0.5;
      const newY = baseBottom + height * newScale * 0.5;
      let posY = THREE.MathUtils.lerp(
        this.group.position.y,
        newY,
        Math.min(1, this.positionLerp * 2.5),
      );
      if (this.damageTime > 0) {
        posY += (Math.random() - 0.5) * this.damageTime * 0.4;
      }
      this.group.position.y = posY;
    }

    // 3. Lerp Rotation
    this.group.rotation.x = THREE.MathUtils.lerp(
      this.group.rotation.x,
      this.targetRotation.x,
      0.15,
    );
    this.group.rotation.y = THREE.MathUtils.lerp(
      this.group.rotation.y,
      this.targetRotation.y,
      0.15,
    );
    this.group.rotation.z = THREE.MathUtils.lerp(
      this.group.rotation.z,
      this.targetRotation.z,
      0.15,
    );

    if (this.isAttachmentFx && this.attachmentFx) {
      this.attachmentFx.rotation.z += delta * 0.45;
    }

    // Animate Sparks
    if (this.isSick && this.sparks.length > 0) {
      this.sparks.forEach((spark) => {
        const data = spark.userData;
        spark.position.addScaledVector(data.velocity, delta);
        data.life -= delta;

        const age = 1 - Math.max(0, data.life / data.maxLife);
        const fadeIn = Math.min(1, age / 0.18);
        const fadeOut = Math.max(0, (1 - age) / 0.7);
        const flicker = 0.7 + 0.3 * Math.sin(time * 18 + data.phase);

        (spark.material as THREE.SpriteMaterial).opacity =
          Math.min(1, fadeIn) * Math.min(1, fadeOut) * flicker * 0.3;
        (spark.material as THREE.SpriteMaterial).rotation += data.spin * delta;

        if (data.life <= 0) {
          this.resetSpark(spark);
        }
      });
    }

    // Shell glow pulse (while unrevealed, static)
    if (this.shellMesh?.visible && !this.shellRevealing && this.shellGlowMesh?.visible) {
      const strength = (this.shellGlowMesh.material as THREE.MeshBasicMaterial).userData.glowStrength as number ?? 0.2;
      const pulse = strength * (0.6 + 0.4 * Math.sin(time * 2.8));
      (this.shellGlowMesh.material as THREE.MeshBasicMaterial).opacity = pulse;
    }

    // Shell reveal animation
    if (this.shellMesh?.visible && this.shellRevealing) {
      this.shellRevealTime += delta;
      const t = this.shellRevealTime;
      const mat = this.shellMesh.material as THREE.MeshBasicMaterial;
      if (t < 0.22) {
        // Shake phase — rapid position oscillation with decreasing amplitude
        const amp = 0.03 * (1 - t / 0.22);
        this.shellMesh.position.set(
          Math.sin(t * 140) * amp,
          Math.cos(t * 110) * amp * 0.5,
          -0.004,
        );
        mat.opacity = 1.0;
        if (this.shellGlowMesh) {
          (this.shellGlowMesh.material as THREE.MeshBasicMaterial).opacity =
            0.5 + 0.4 * Math.sin(t * 80);
        }
      } else {
        // Explode phase — scale up and fade out
        const fadeT = (t - 0.22) / 0.35;
        const alpha = Math.max(0, 1 - fadeT);
        mat.opacity = alpha;
        this.shellMesh.scale.setScalar(1 + fadeT * 0.6);
        this.shellMesh.position.set(0, 0, -0.004);
        if (this.shellGlowMesh) {
          (this.shellGlowMesh.material as THREE.MeshBasicMaterial).opacity = alpha * 0.8;
          this.shellGlowMesh.scale.setScalar(1 + fadeT * 1.1);
        }
        if (alpha <= 0) {
          this.shellMesh.visible = false;
          this.shellRevealing = false;
          if (this.shellGlowMesh) {
            this.shellGlowMesh.visible = false;
            this.shellGlowMesh.scale.setScalar(1);
          }
        }
      }
    }
  }

  private createRoundedBorderTexture(physW: number, physH: number, radiusFraction = 0.09, borderFraction = 0.04, fillAlpha = 0): THREE.CanvasTexture {
    const res = 512;
    const aspect = physH / physW;
    const canvas = document.createElement('canvas');
    canvas.width = res;
    canvas.height = Math.round(res * aspect);
    const ctx = canvas.getContext('2d')!;
    const cw = canvas.width;
    const ch = canvas.height;
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, cw, ch);
    const bw = Math.round(borderFraction * cw);
    const inset = bw / 2;
    const r = Math.round(radiusFraction * cw);
    const ri = Math.max(r - inset, 0);

    const roundedPath = () => {
      ctx.beginPath();
      ctx.moveTo(inset + ri, inset);
      ctx.lineTo(cw - inset - ri, inset);
      ctx.arcTo(cw - inset, inset, cw - inset, inset + ri, ri);
      ctx.lineTo(cw - inset, ch - inset - ri);
      ctx.arcTo(cw - inset, ch - inset, cw - inset - ri, ch - inset, ri);
      ctx.lineTo(inset + ri, ch - inset);
      ctx.arcTo(inset, ch - inset, inset, ch - inset - ri, ri);
      ctx.lineTo(inset, inset + ri);
      ctx.arcTo(inset, inset, inset + ri, inset, ri);
      ctx.closePath();
    };

    if (fillAlpha > 0) {
      const g = Math.round(fillAlpha * 255);
      ctx.fillStyle = `rgb(${g},${g},${g})`;
      roundedPath();
      ctx.fill();
    }

    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = bw;
    roundedPath();
    ctx.stroke();
    return new THREE.CanvasTexture(canvas);
  }

  private ensureShell() {
    if (this.shellMesh) return;

    const rarityColors: Record<string, number> = {
      public: 0x94a3b8,
      private: 0x10b981,
      restricted: 0xa855f7,
      classified: 0xf43f5e,
      singularity: 0xfacc15,
    };
    const rarityHex = rarityColors[this.card.rarity] ?? 0x10b981;
    const rarityColor = new THREE.Color(rarityHex);

    const shellW = this.factory.cardW + 0.05;
    const shellH = this.factory.cardH + 0.05;
    const shellAlphaMap = this.createRoundedBorderTexture(shellW, shellH, 0.09, 0.04, 1.0);

    // Dark shell with a subtle rarity tint — card art hidden, glow mesh provides the colored border
    const shellColor = rarityColor.clone().lerp(new THREE.Color(0x000000), 0.88);
    const shellMat = new THREE.MeshBasicMaterial({
      color: shellColor,
      transparent: true,
      opacity: 1.0,
      alphaMap: shellAlphaMap,
      side: THREE.DoubleSide,
      depthWrite: false,
      // depthTest:true lets the shell be hidden by the card's own front face
      // (when revealed) and by closer revealed cards, without blocking them.
      depthTest: true,
    });
    const geom = new THREE.PlaneGeometry(shellW, shellH);
    this.shellMesh = new THREE.Mesh(geom, shellMat);
    // z=-0.004: puts shell in front of backMesh (z=-0.002) when unrevealed,
    // but behind frontMesh (z=0) when revealed — hides itself automatically.
    this.shellMesh.position.z = -0.004;
    this.shellMesh.renderOrder = 25;
    this.shellMesh.visible = false;
    this.group.add(this.shellMesh);

    // Pulsing rarity glow border (slightly larger plane, additive blending)
    const glowStrength =
      this.card.rarity === 'singularity' ? 0.55
      : this.card.rarity === 'classified' ? 0.42
      : this.card.rarity === 'restricted' ? 0.32
      : this.card.rarity === 'private' ? 0.22
      : 0.14;
    const glowW = this.factory.cardW + 0.18;
    const glowH = this.factory.cardH + 0.18;
    const glowAlphaMap = this.createRoundedBorderTexture(glowW, glowH, 0.09, 0.07);
    const glowMat = new THREE.MeshBasicMaterial({
      color: new THREE.Color(rarityHex).multiplyScalar(2.5),
      transparent: true,
      opacity: 0,
      alphaMap: glowAlphaMap,
      blending: THREE.AdditiveBlending,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: false,
    });
    glowMat.userData.glowStrength = glowStrength;
    const glowGeom = new THREE.PlaneGeometry(glowW, glowH);
    this.shellGlowMesh = new THREE.Mesh(glowGeom, glowMat);
    this.shellGlowMesh.position.z = -0.005;
    this.shellGlowMesh.renderOrder = 50;
    this.shellGlowMesh.visible = false;
    this.group.add(this.shellGlowMesh);
  }

  private setRevealedRenderOrder(revealed: boolean) {
    // Revealed cards render at 28 — after shells (25) so no shell overdraws them.
    // Shells have depthWrite:false so the depth test still passes here.
    this.frontMesh.renderOrder = revealed ? 28 : 15;
    this.backMesh.renderOrder = revealed ? 28 : 15;
  }

  public setRevealShell(revealed: boolean) {
    this.ensureShell();
    if (!this.shellMesh) return;
    if (revealed) {
      this.setRevealedRenderOrder(true);
      // If shell is still showing, trigger the break-off animation
      if (this.shellMesh.visible && !this.shellRevealing) {
        this.shellRevealing = true;
        this.shellRevealTime = 0;
      }
    } else {
      this.setRevealedRenderOrder(false);
      // Unrevealed — keep shell visible and fully reset (unless mid-animation)
      if (!this.shellRevealing) {
        const mat = this.shellMesh.material as THREE.MeshBasicMaterial;
        mat.opacity = 1.0;
        mat.transparent = true;
        this.shellMesh.scale.setScalar(1);
        this.shellMesh.position.set(0, 0, -0.004);
        this.shellMesh.visible = true;
        if (this.shellGlowMesh) {
          this.shellGlowMesh.visible = true;
          this.shellGlowMesh.scale.setScalar(1);
        }
      }
    }
  }

  private resolveFrameStyle(card: CardView) {
    if (
      card.frame_style === 'pack_half_left' ||
      card.frame_style === 'pack_half_right'
    )
      return card.frame_style;
    if (card.kind_tag === 'pack') return 'pack_model';
    if (card.frame_style) return card.frame_style;
    if (card.kind_tag === 'infrastructure' || card.kind_tag === 'land')
      return 'infra_hex';
    if (card.kind_tag === 'relic' || card.kind_tag === 'artifact') return 'card';
    if (card.kind_tag === 'aura' || card.kind_tag === 'enchantment') return 'enchantment_card';
    if (card.kind_tag === 'instant' || card.kind_tag === 'ritual' || card.kind_tag === 'sorcery')
      return 'spell_card';
    if (card.kind_tag === 'agent') return 'agent_card';
    return 'card';
  }

  private buildAttachmentFx() {
    const baseColor = this.card.colors?.[0] || '#7dd3fc';
    const glowColor = new THREE.Color(baseColor).multiplyScalar(1.4);

    const ringMat = new THREE.MeshBasicMaterial({
      color: glowColor,
      transparent: true,
      opacity: 0.55,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    const ringGeom = new THREE.TorusGeometry(
      this.factory.cardW * 0.32,
      0.012,
      6,
      60,
    );
    const ring = new THREE.Mesh(ringGeom, ringMat);
    ring.renderOrder = 22;

    const ringInnerMat = ringMat.clone();
    ringInnerMat.opacity = 0.35;
    const ringInner = new THREE.Mesh(
      new THREE.TorusGeometry(this.factory.cardW * 0.22, 0.01, 5, 48),
      ringInnerMat,
    );
    ringInner.renderOrder = 21;
    ringInner.rotation.z = Math.PI / 3;

    const spriteMat = this.factory.sparkMaterial.clone();
    spriteMat.color = new THREE.Color(baseColor);
    spriteMat.opacity = 0.75;
    const sprite = new THREE.Sprite(spriteMat);
    sprite.position.set(
      this.factory.cardW * 0.18,
      this.factory.cardH * 0.15,
      0.04,
    );
    sprite.scale.set(0.12, 0.12, 1);
    sprite.renderOrder = 23;

    const sprite2 = new THREE.Sprite(spriteMat.clone());
    sprite2.position.set(
      -this.factory.cardW * 0.2,
      -this.factory.cardH * 0.1,
      0.03,
    );
    sprite2.scale.set(0.08, 0.08, 1);
    sprite2.renderOrder = 23;

    this.attachmentFx = new THREE.Group();
    this.attachmentFx.add(ring, ringInner, sprite, sprite2);
    this.group.add(this.attachmentFx);

    this.frontMesh = ring;
    this.backMesh = new THREE.Mesh();
  }

  public getTargetPosition() {
    return this.targetPosition.clone();
  }

  public getTargetRotation() {
    return this.targetRotation.clone();
  }

  public getAttachmentAnchor() {
    const pos = this.targetPosition.clone();
    const rot = this.targetRotation.clone();
    const forward = new THREE.Vector3(0, 0, 1).applyEuler(rot);
    const up = new THREE.Vector3(0, 1, 0).applyEuler(rot);
    const lift = this.flatLayout || this.isInfrastructure ? 0.04 : 0.12;
    pos.add(up.multiplyScalar(lift));
    pos.add(forward.multiplyScalar(0.04));
    return { pos, rot };
  }
}

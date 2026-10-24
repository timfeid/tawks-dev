import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';
import { PlayerActor } from '../entities/PlayerActor';
import type { PlayerState, CombatPairView } from '../types';
import { computeRowLayout } from '../engine/layoutUtils';
import { createArcLabelMesh } from '../environment';
import { createTextureUtils } from '../textures';

export class BattlefieldZone extends BaseZone {
  private players: PlayerState[] = [];
  private viewerId: string | null = null;
  private attackerIds: string[] = [];
  private combatPairs: CombatPairView[] = [];
  private previewOrder: string[] | null = null;
  private gameUiTarget = 0;
  private gameUiOpacity = 0;
  private lastAppliedUiOpacity = -1;
  private flat = false;
  private currentTime = 0;

  private cardW = 1.0;
  private cardH = 1.46;
  // Effective world-space radius of the outer table ring. Written by
  // `setTableScale` on every state change so all layout math (player seats,
  // card arcs, inner rings, mana hexes) agrees with the visible mesh.
  private tableRadius = 10.0;
  private battlefieldCardScale = 1.25;

  // Visuals
  private envGroup = new THREE.Group();
  private wireframeGroup = new THREE.Group();
  private labelGroup = new THREE.Group();
  private playerGroup = new THREE.Group();
  private pilesGroup = new THREE.Group();
  private statGroup = new THREE.Group();
  private manaGroup = new THREE.Group();
  private combatLineGroup = new THREE.Group();
  private extraInteractables: THREE.Object3D[] = [];

  private playerActors = new Map<string, PlayerActor>();
  private textureUtils: ReturnType<typeof createTextureUtils>;
  private manaHexCache = new Map<string, THREE.MeshBasicMaterial>();
  private manaHexOutlineCache = new Map<string, THREE.MeshBasicMaterial>();

  constructor() {
    super('battlefield');
    this.textureUtils = createTextureUtils({
      cardTextureScale: 2,
      cardW: this.cardW,
      cardH: this.cardH,
    });
    this.envGroup.add(this.wireframeGroup);
    this.envGroup.add(this.labelGroup);
    this.envGroup.add(this.playerGroup);
    this.envGroup.add(this.pilesGroup);
    this.envGroup.add(this.statGroup);
    this.envGroup.add(this.manaGroup);
    this.envGroup.add(this.combatLineGroup);
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.envGroup];
  }

  /**
   * Update the battlefield's effective radius to match the visible table
   * mesh. Called by the caller with `tableManager.getEffectiveRadius()` so
   * all zones and the camera derive from one source of truth instead of
   * each re-multiplying their own `base * scale`.
   */
  public setTableRadius(radius: number): void {
    if (Math.abs(radius - this.tableRadius) < 0.001) return;
    this.tableRadius = radius;
    if (this.players.length > 0) this.rebuildEnvironment();
  }

  /** Backwards-compat shim: caller was passing a scale factor. */
  public setTableScale(scale: number): void {
    const newRadius = 10.0 * scale;
    if (Math.abs(newRadius - this.tableRadius) < 0.001) return;
    this.tableRadius = newRadius;
    // Rebuild so the wireframe / mana hexes / debug ring reposition
    // to the new radius. Without this, zone layout drifts to match an
    // earlier tableRadius value (e.g. the default 10.0 before scale
    // was first applied).
    if (this.players.length > 0) this.rebuildEnvironment();
  }

  public updateState(
    players: PlayerState[],
    viewerId: string | null,
    attackerIds: string[] = [],
    combatPairs: CombatPairView[] = [],
  ) {
    this.players = players;
    this.viewerId = viewerId;
    this.attackerIds = attackerIds;
    this.combatPairs = combatPairs;
    this.rebuildEnvironment();
  }

  public setGameElementsVisible(visible: boolean) {
    this.gameUiTarget = visible ? 1 : 0;
    this.envGroup.userData.showGameElements = visible;
  }

  public setPreviewOrder(order: string[] | null) {
    this.previewOrder = order;
  }

  public setFlatLayout(flat: boolean) {
    this.flat = flat;
  }

  public getViewerCardIds(): string[] {
    if (!this.viewerId) return [];
    return this.getCardIds(this.viewerId);
  }

  public getDropIndexForViewer(point: THREE.Vector3): number {
    if (!this.viewerId) return 0;
    return this.getDropIndex(point, this.viewerId);
  }

  public getPlayerInteractables(): THREE.Object3D[] {
    return Array.from(this.playerActors.values()).flatMap((actor) =>
      actor.getInteractables(),
    );
  }

  public getPlayerActors(): PlayerActor[] {
    return Array.from(this.playerActors.values());
  }

  public getExtraInteractables(): THREE.Object3D[] {
    return this.extraInteractables;
  }

  private getHexSpiralCoords(count: number): Array<{ q: number; r: number }> {
    if (count <= 0) return [];

    const results: Array<{ q: number; r: number }> = [{ q: 0, r: 0 }];
    if (count === 1) return results;

    const directions = [
      { q: 1, r: 0 },
      { q: 1, r: -1 },
      { q: 0, r: -1 },
      { q: -1, r: 0 },
      { q: -1, r: 1 },
      { q: 0, r: 1 },
    ];

    for (let ring = 1; results.length < count; ring++) {
      let q = -ring;
      let r = ring;

      for (const dir of directions) {
        for (let step = 0; step < ring && results.length < count; step++) {
          results.push({ q, r });
          q += dir.q;
          r += dir.r;
        }
      }
    }

    return results;
  }

  private getCombatTargetPosition(
    pair: CombatPairView,
    allActors: Map<string, CardActor>,
  ): THREE.Vector3 | null {
    if (pair.target_kind === 'unit' && pair.target_card_id) {
      const targetActor = allActors.get(pair.target_card_id);
      if (targetActor) {
        const pos = targetActor.getTargetPosition();
        return new THREE.Vector3(pos.x, pos.y, pos.z);
      }
    }

    if (pair.target_player_id) {
      const targetIndex = this.players.findIndex(
        (p) => p.id === pair.target_player_id,
      );
      if (targetIndex >= 0) {
        const targetSeatAngle =
          (targetIndex / this.players.length) * Math.PI * 2;
        const edgeRadius = this.tableRadius - 0.5;
        return new THREE.Vector3(
          Math.cos(targetSeatAngle) * edgeRadius,
          this.flat ? 0.12 : this.cardH * 0.5 + 0.12,
          Math.sin(targetSeatAngle) * edgeRadius,
        );
      }
    }

    return null;
  }

  private rebuildEnvironment() {
    // Clear
    this.wireframeGroup.clear();
    this.labelGroup.clear();
    this.pilesGroup.clear();
    this.statGroup.clear();
    this.manaGroup.clear();
    this.combatLineGroup.clear();
    this.extraInteractables = [];

    // Sync Player Actors
    const validIds = new Set(this.players.map((p) => p.id));
    for (const [id, actor] of this.playerActors) {
      if (!validIds.has(id)) {
        this.playerGroup.remove(actor.group);
        this.playerActors.delete(id);
      }
    }
    this.players.forEach((p, index) => {
      if (!this.playerActors.has(p.id)) {
        const actor = new PlayerActor(p);
        this.playerGroup.add(actor.group);
        this.playerActors.set(p.id, actor);
      }

      // Position Player — scales with the dynamic table radius so players
      // stay aligned with the table edge regardless of player count.
      const seatAngle = (index / this.players.length) * Math.PI * 2;
      const dist = this.tableRadius + 5;
      const actor = this.playerActors.get(p.id)!;
      const pos = new THREE.Vector3(
        Math.cos(seatAngle) * dist,
        0.0,
        Math.sin(seatAngle) * dist,
      );
      actor.setPosition(pos, new THREE.Vector3(0, 0, 0));
    });

    const count = this.players.length;
    if (count === 0) return;

    const seatArc = (Math.PI * 2) / count;
    // Radial layering inside the table (all fractions of tableRadius):
    //   1.00 R  table surface / outer glow
    //   0.78–0.88 R  wireframe band + name-tag arc (near the edge)
    //   0.68 R  row 0 (front line) cards — rows step further inward
    //   0.55 R  infrastructure / mana hex cluster (pulled toward middle)
    const sectionInner = this.tableRadius * 0.78;
    const sectionOuter = this.tableRadius * 0.88;

    // DEBUG: bright magenta ring at what THIS zone believes the table
    // outer edge to be (this.tableRadius, already the effective radius
    // = TableManager.tableRadius * appliedScale). If this ring lines up
    // with the green edge glow, zones + mesh agree. If it's inside or
    // outside, they disagree.
    {
      const debugSegments = 128;
      const debugPts: THREE.Vector3[] = [];
      for (let j = 0; j <= debugSegments; j++) {
        const t = (j / debugSegments) * Math.PI * 2;
        debugPts.push(
          new THREE.Vector3(
            Math.cos(t) * this.tableRadius,
            0.06,
            Math.sin(t) * this.tableRadius,
          ),
        );
      }
      const debugGeo = new THREE.BufferGeometry().setFromPoints(debugPts);
      const debugMat = new THREE.LineBasicMaterial({
        color: 0xff00ff,
        transparent: true,
        opacity: 0.9,
        blending: THREE.AdditiveBlending,
      });
      this.wireframeGroup.add(new THREE.Line(debugGeo, debugMat));
    }

    // Wireframe Material
    const wireframeMat = new THREE.LineBasicMaterial({
      color: 0xf59e0b,
      transparent: true,
      opacity: 0.25,
      blending: THREE.AdditiveBlending,
    });

    // 1. Divider Lines
    for (let i = 0; i < count; i++) {
      const angle = i * seatArc - seatArc / 2;
      const points = [
        new THREE.Vector3(
          Math.cos(angle) * sectionInner,
          0.015,
          Math.sin(angle) * sectionInner,
        ),
        new THREE.Vector3(
          Math.cos(angle) * sectionOuter,
          0.015,
          Math.sin(angle) * sectionOuter,
        ),
      ];
      const geo = new THREE.BufferGeometry().setFromPoints(points);
      this.wireframeGroup.add(new THREE.Line(geo, wireframeMat));
    }

    // 2. Rings & Piles
    const r = sectionInner;
    const gapSize = 0.46;
    const fadeZone = 0.15;

    const deckMaterial = new THREE.MeshStandardMaterial({
      color: 0x111827,
      roughness: 0.35,
      metalness: 0.15,
    });
    const discardMaterial = new THREE.MeshStandardMaterial({
      color: 0x1f2937,
      roughness: 0.4,
      metalness: 0.1,
    });

    for (let i = 0; i < count; i++) {
      const seatAngle = (i / count) * Math.PI * 2;
      const nextSeatAngle = ((i + 1) / count) * Math.PI * 2;
      const startAngle = seatAngle + gapSize / 2;
      const endAngle = nextSeatAngle - gapSize / 2;

      if (startAngle < endAngle) {
        // ... (Wireframe logic mostly same, keeping brief)
        const points = [];
        const colors = [];
        const segments = 64;
        for (let j = 0; j <= segments; j++) {
          const theta = startAngle + (j / segments) * (endAngle - startAngle);
          points.push(
            new THREE.Vector3(Math.cos(theta) * r, 0.015, Math.sin(theta) * r),
          );
          const angleFromStart = (j / segments) * (endAngle - startAngle);
          const angleFromEnd = (1 - j / segments) * (endAngle - startAngle);
          const alpha = Math.min(
            1.0,
            angleFromStart / fadeZone,
            angleFromEnd / fadeZone,
          );
          colors.push(0.96 * alpha, 0.62 * alpha, 0.04 * alpha);
        }
        const geo = new THREE.BufferGeometry().setFromPoints(points);
        geo.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
        const ringMat = new THREE.LineBasicMaterial({
          vertexColors: true,
          transparent: true,
          opacity: 0.25,
          blending: THREE.AdditiveBlending,
        });
        this.wireframeGroup.add(new THREE.Line(geo, ringMat));
      }

      // 3. Labels
      const player = this.players[i];
      const labelMesh = createArcLabelMesh(
        player.name,
        sectionInner,
        seatAngle,
      );
      if (labelMesh) {
        this.labelGroup.add(labelMesh);
      }

      // 4. Mana Pool Hex Visualizer — rendered once per seat via a shared
      // helper so every player gets the same cluster.
      this.renderManaHexesForSeat(player, seatAngle);
    }

    this.applyGroupOpacity(this.wireframeGroup, this.gameUiOpacity);
    this.applyGroupOpacity(this.labelGroup, this.gameUiOpacity);
    this.applyGroupOpacity(this.pilesGroup, this.gameUiOpacity);
    this.applyGroupOpacity(this.statGroup, this.gameUiOpacity);
    this.applyGroupOpacity(this.manaGroup, this.gameUiOpacity);
    this.applyGroupOpacity(this.combatLineGroup, this.gameUiOpacity);
  }

  private getManaHexMaterial(color: string) {
    const cached = this.manaHexCache.get(color);
    if (cached) return cached;

    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');
    if (ctx) {
      ctx.clearRect(0, 0, 256, 256);
      const centerX = 128;
      const centerY = 128;
      const drawR = 90;

      ctx.beginPath();
      for (let j = 0; j < 6; j++) {
        const angle = (j * 2 * Math.PI) / 6 - Math.PI / 2;
        const px = centerX + Math.cos(angle) * drawR;
        const py = centerY + Math.sin(angle) * drawR;
        if (j === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.closePath();

      ctx.fillStyle = color;
      ctx.shadowColor = color;
      ctx.shadowBlur = 22;
      ctx.fill();

      ctx.strokeStyle = 'rgba(255, 255, 255, 0.65)';
      ctx.lineWidth = 3;
      ctx.stroke();
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      blending: THREE.AdditiveBlending,
      opacity: 0.75,
      depthWrite: false,
    });
    this.manaHexCache.set(color, material);
    return material;
  }

  private getManaHexOutlineMaterial(color: string) {
    const cacheKey = `outline_${color}`;
    const cached = this.manaHexOutlineCache.get(cacheKey);
    if (cached) return cached;

    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');
    if (ctx) {
      ctx.clearRect(0, 0, 256, 256);
      const centerX = 128;
      const centerY = 128;
      const drawR = 90;

      ctx.beginPath();
      for (let j = 0; j < 6; j++) {
        const angle = (j * 2 * Math.PI) / 6 - Math.PI / 2;
        const px = centerX + Math.cos(angle) * drawR;
        const py = centerY + Math.sin(angle) * drawR;
        if (j === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.closePath();

      // Dashed outline only, no fill
      ctx.setLineDash([12, 8]);
      ctx.strokeStyle = color;
      ctx.lineWidth = 3;
      ctx.shadowColor = color;
      ctx.shadowBlur = 10;
      ctx.stroke();
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      blending: THREE.AdditiveBlending,
      opacity: 0.5,
      depthWrite: false,
    });
    this.manaHexOutlineCache.set(cacheKey, material);
    return material;
  }

  private renderManaHexesForSeat(player: any, seatAngle: number): void {
    const gridNodes = (player?.grid || []) as any[];
    const infraCards = (player?.battlefield_infrastructure || []) as any[];
    if (gridNodes.length === 0 && infraCards.length === 0) return;

    const forward = new THREE.Vector3(
      Math.cos(seatAngle),
      0,
      Math.sin(seatAngle),
    );
    const tangent = new THREE.Vector3(
      -Math.sin(seatAngle),
      0,
      Math.cos(seatAngle),
    );

    const manaColorMap: Record<string, string> = {
      neon: '#00e5ff',
      glitch: '#a855f7',
      chrome: '#cbd5e1',
      slag: '#f97316',
      bio: '#22c55e',
      colorless: '#64748b',
    };

    const availableByType: Record<string, number> = {};
    for (const key of Object.keys(manaColorMap)) {
      availableByType[key] = (player.mana_pool as any)?.[key] ?? 0;
    }

    const hexCells: { color: string; filled: boolean }[] = [];
    for (const node of gridNodes) {
      const poolKey = node.mana_type ?? 'colorless';
      const color = node.color || manaColorMap[poolKey] || '#64748b';
      const hasMana = (availableByType[poolKey] ?? 0) > 0;
      if (hasMana) availableByType[poolKey]--;
      hexCells.push({ color, filled: hasMana });
    }
    // Infrastructure cards are a parallel mana source (separate from grid
    // nodes): each infra card taps for 1 mana of its color. Render them
    // alongside grid nodes so the visible total matches the real mana pool.
    for (const infra of infraCards) {
      let poolKey = 'colorless';
      const hex = (infra.colors?.[0] ?? '').toLowerCase();
      const match = Object.entries(manaColorMap).find(
        ([, c]) => c.toLowerCase() === hex,
      );
      if (match) {
        poolKey = match[0];
      } else {
        const name = (infra.name ?? '').toLowerCase();
        if (name.includes('neon')) poolKey = 'neon';
        else if (name.includes('glitch')) poolKey = 'glitch';
        else if (name.includes('chrome')) poolKey = 'chrome';
        else if (name.includes('slag')) poolKey = 'slag';
        else if (name.includes('bio')) poolKey = 'bio';
      }
      const color = manaColorMap[poolKey] || '#64748b';
      const hasMana = !infra.tapped && (availableByType[poolKey] ?? 0) > 0;
      if (hasMana) availableByType[poolKey]--;
      hexCells.push({ color, filled: hasMana });
    }

    const gridHexR = 0.26;
    const gridHStep = Math.sqrt(3) * gridHexR * 0.8;
    const gridVStep = 1.5 * gridHexR * 0.8;
    // Infrastructure/mana cluster pulled INWARD toward the middle of
    // the table so cards/nametags own the outer band.
    const outwardDistance = this.tableRadius * 0.9;
    const gridAnchor = forward.clone().multiplyScalar(outwardDistance);
    const inward = forward.clone().multiplyScalar(-1);
    const planeSize = gridHexR * (256 / 90) * 0.8;
    const hexCoords = this.getHexSpiralCoords(hexCells.length);

    hexCells.forEach((cell, idx) => {
      const coord = hexCoords[idx] ?? { q: 0, r: 0 };
      const localX = gridHStep * (coord.q + coord.r / 2);
      const localZ = gridVStep * coord.r;
      const pos = gridAnchor
        .clone()
        .add(tangent.clone().multiplyScalar(localX))
        .add(inward.clone().multiplyScalar(localZ));
      pos.y = 0.02;

      const material = cell.filled
        ? this.getManaHexMaterial(cell.color)
        : this.getManaHexOutlineMaterial(cell.color);
      const mesh = new THREE.Mesh(
        new THREE.PlaneGeometry(planeSize, planeSize),
        material,
      );
      mesh.position.copy(pos);
      mesh.rotation.x = -Math.PI / 2;
      mesh.rotation.z = -seatAngle + Math.PI / 2;
      mesh.renderOrder = 10;
      this.manaGroup.add(mesh);
    });
  }

  private createStack(
    count: number,
    pos: THREE.Vector3,
    mat: THREE.Material,
    parent: THREE.Group,
    topCard?: 'card_back',
  ) {
    if (count <= 0 && !topCard) return null;
    // Visual cap
    const visualCount = Math.min(count, 40);
    const stackGroup = new THREE.Group();
    stackGroup.position.copy(pos);

    const dirToCenter = pos.clone().multiplyScalar(-1);
    const rotationY = Math.atan2(dirToCenter.x, dirToCenter.z);
    stackGroup.rotation.set(0, rotationY, 0);

    const stackGeo = new THREE.BoxGeometry(this.cardW, 0.012, this.cardH);
    for (let i = 0; i < visualCount; i += 1) {
      const mesh = new THREE.Mesh(stackGeo, mat);
      mesh.position.y = 0.006 + i * 0.004;
      mesh.position.x = (Math.random() - 0.5) * 0.02;
      mesh.position.z = (Math.random() - 0.5) * 0.02;
      mesh.rotation.y = (Math.random() - 0.5) * 0.08;
      stackGroup.add(mesh);
    }

    if (topCard === 'card_back') {
      const topTexture = this.textureUtils.createCardBackTexture();
      if (topTexture) {
        const topMesh = new THREE.Mesh(
          new THREE.PlaneGeometry(this.cardW, this.cardH),
          new THREE.MeshBasicMaterial({
            map: topTexture,
            transparent: true,
            depthTest: true,
            depthWrite: true,
          }),
        );
        topMesh.rotation.x = -Math.PI / 2;
        // Place exactly on top of the last box: base half-height (0.006) + stack height + small offset
        topMesh.position.y = 0.006 + visualCount * 0.004 + 0.006;
        topMesh.renderOrder = 30;
        stackGroup.add(topMesh);
      }
    }

    parent.add(stackGroup);
    return stackGroup;
  }

  public update(cards: any[]) {
    // No-op
  }

  public triggerHitAnim(playerId: string) {
    this.playerActors.get(playerId)?.playHitAnim();
  }

  public tick(delta: number) {
    this.currentTime += delta;
    for (const actor of this.playerActors.values()) {
      actor.tick(delta);
    }
    const nextOpacity = THREE.MathUtils.damp(
      this.gameUiOpacity,
      this.gameUiTarget,
      5,
      delta,
    );
    this.gameUiOpacity = nextOpacity;
    if (Math.abs(this.gameUiOpacity - this.lastAppliedUiOpacity) > 0.01) {
      this.lastAppliedUiOpacity = this.gameUiOpacity;
      this.applyGroupOpacity(this.wireframeGroup, this.gameUiOpacity);
      this.applyGroupOpacity(this.labelGroup, this.gameUiOpacity);
      this.applyGroupOpacity(this.pilesGroup, this.gameUiOpacity);
      this.applyGroupOpacity(this.statGroup, this.gameUiOpacity);
      this.applyGroupOpacity(this.manaGroup, this.gameUiOpacity);
      this.applyGroupOpacity(this.combatLineGroup, this.gameUiOpacity);
    }
  }

  public layout(allActors: Map<string, CardActor>) {
    if (this.players.length === 0) return;

    // Clear lethal flags up front — combat loops below re-apply them
    // per pair. Also set them for any unit whose accumulated damage
    // already meets or exceeds its toughness (pending state-based
    // removal) so the player can see "this thing is dead" even when
    // it's not currently in a combat pair.
    for (const p of this.players) {
      const units = [
        ...(p.battlefield_units || []),
        ...(p.battlefield_infrastructure || []),
      ];
      for (const c of units) {
        const actor = allActors.get(c.id);
        if (!actor) continue;
        const damage = (c as any).damage ?? 0;
        const toughness = Number(c.toughness ?? 0);
        const indestructible = (c.keywords ?? []).some((k: any) => {
          const kw = typeof k === 'string' ? k : (k?.key ?? k?.text ?? '');
          return String(kw).toLowerCase() === 'indestructible';
        });
        const pendingLethal =
          !indestructible && toughness > 0 && damage >= toughness;
        actor.setLethal(pendingLethal);
      }
    }

    this.players.forEach((player, index) => {
      let units = player.battlefield_units || [];
      // Only cards attached to another *card* (not a player) are true attachments
      const cardIds = new Set(units.map((c) => c.id));
      const isCardAttachment = (card: (typeof units)[0]) =>
        Boolean(card.attached_to) && cardIds.has(card.attached_to!);
      const attachments = units.filter(isCardAttachment);
      let baseUnits = units.filter((card) => !isCardAttachment(card));
      // Units form the FRONT LINE (row 0, outermost); non-units
      // (artifacts, enchantments, agents) go in back rows, closer to
      // center.
      baseUnits = [
        ...baseUnits.filter((c) => c.kind_tag === 'unit'),
        ...baseUnits.filter((c) => c.kind_tag !== 'unit'),
      ];
      if (
        player.id === this.viewerId &&
        this.previewOrder &&
        this.previewOrder.length === baseUnits.length
      ) {
        const unitMap = new Map(baseUnits.map((u) => [u.id, u]));
        const ordered = this.previewOrder
          .map((id) => unitMap.get(id))
          .filter(Boolean) as typeof baseUnits;
        if (ordered.length === baseUnits.length) {
          baseUnits = ordered;
        }
      }
      const infra = player.battlefield_infrastructure || [];

      const seatAngle = (index / this.players.length) * Math.PI * 2;
      const seatArc = (Math.PI * 2) / this.players.length;

      // 1. UNIT LAYOUT (Arcs) — row 0 (front line) tucked up under the
      // name-tag arc (0.78 R) so cards sit near the player label.
      // Units and non-units get SEPARATE row layouts so non-units
      // (artifacts/auras/agents) always start on their own row behind
      // the unit rows, even when a unit row has slack space left over.
      const layoutRadius = this.tableRadius * 0.65;

      const unitCount = baseUnits.filter((c) => c.kind_tag === 'unit').length;
      const nonUnitCount = baseUnits.length - unitCount;

      const unitLayout = computeRowLayout(
        unitCount,
        layoutRadius,
        this.cardW,
        this.cardH,
        seatArc,
      );
      // Non-units sit along the infrastructure line, pulled slightly
      // inward from the edge, with tighter spacing since they're
      // rendered at half scale anyway.
      const nonUnitLayoutRadius = this.tableRadius * 0.78;
      const nonUnitLayout = computeRowLayout(
        nonUnitCount,
        nonUnitLayoutRadius,
        this.cardW * 0.5,
        this.cardH * 0.5,
        seatArc,
      );

      // Precompute per-card angular widths for each bucket/row — a
      // tapped card (rotated 90° on Z) occupies `cardH` along the arc
      // instead of `cardW`, so its slot has to be wider. We sum the
      // widths in each row and place each card at its cumulative offset
      // so adjacent cards don't overlap regardless of their tap state.
      const widthFor = (
        card: { tapped?: boolean },
        inUnit: boolean,
      ): number => {
        // Use the actual rendered card size (not logical cardW=1.0), since
        // cards draw at battlefieldCardScale*0.7 ≈ 0.875× of the factory
        // geometry (0.75 × 1.1). Without this, spacing over-allocates by
        // ~35% and adjacent cards look far apart.
        const renderScale = this.battlefieldCardScale * 0.7;
        const renderW = 0.75 * renderScale; // ≈ 0.656
        const renderH = 1.1 * renderScale; //  ≈ 0.9625
        const margin = inUnit ? 0.1 : 0.15;
        return card.tapped ? renderH + margin : renderW + margin;
      };

      // Build a row width list per (bucket, row) = [width0, width1, ...]
      const rowWidths: number[][] = [[], [], [], [], [], [], [], [], [], []];
      const rowBucket: boolean[] = [];
      baseUnits.forEach((card, i) => {
        const inUnit = i < unitCount;
        const bucket = inUnit ? unitLayout : nonUnitLayout;
        const idxInBucket = inUnit ? i : i - unitCount;
        let row = 0;
        for (let r = 0; r < 5; r++) {
          if (idxInBucket >= bucket.rowStarts[r]) row = r;
        }
        const key = inUnit ? row : row + 5;
        rowWidths[key].push(widthFor(card, inUnit));
        rowBucket[key] = inUnit;
      });

      baseUnits.forEach((card, i) => {
        const actor = allActors.get(card.id);
        if (!actor) return;
        actor.setZone('battlefield');
        actor.setFlatLayout(this.flat);

        const isAttacking = this.attackerIds.includes(card.id);

        // Pick the layout bucket for this card (units vs non-units) and
        // compute its row within that bucket.
        const inUnitBucket = i < unitCount;
        const bucket = inUnitBucket ? unitLayout : nonUnitLayout;
        const idxInBucket = inUnitBucket ? i : i - unitCount;
        let row = 0;
        for (let r = 0; r < 5; r++) {
          if (idxInBucket >= bucket.rowStarts[r]) row = r;
        }
        const cols = bucket.rowCounts[row] ?? 1;
        const col = idxInBucket - (bucket.rowStarts[row] ?? 0);

        // Attacking cards sit on a tighter inner ring (pulled toward the
        // center) but keep the *same angular spacing* as their resting
        // row, so neighbors don't fan out just because the radius shrunk.
        const restingRowRadius = bucket.rowRadiusFor(row);
        const rowRadius = isAttacking ? 2.5 : restingRowRadius;
        const spacingRadius = restingRowRadius;

        // Cumulative-offset layout: each card's angular center is the
        // sum of half-widths of cards before it + its own half-width.
        const key = inUnitBucket ? row : row + 5;
        const widths = rowWidths[key] || [];
        let cumulative = 0;
        for (let c = 0; c < col; c++) cumulative += widths[c] ?? 0;
        cumulative += (widths[col] ?? 0) / 2;
        const totalWidth = widths.reduce((s, w) => s + w, 0);
        // Cap total arc at 80% of the seat slice — shrink widths uniformly
        // if we'd overflow (e.g. too many tapped cards in a tiny seat).
        const maxArc = seatArc * 0.8;
        const widthScale = Math.min(1, (maxArc * spacingRadius) / totalWidth);
        const centeredOffsetLinear = (cumulative - totalWidth / 2) * widthScale;
        const centeredOffsetAngle = centeredOffsetLinear / spacingRadius;

        const rowOffset = !isAttacking
          ? row === 0
            ? this.cardW * 2
            : row === 1
              ? this.cardW * 0.45
              : 0
          : 0;
        const rowAngleOffset = -rowOffset / rowRadius;

        const cardAngle = seatAngle + rowAngleOffset + centeredOffsetAngle;
        void cols;

        // Non-units lay FLAT on the table face-up; units stand upright.
        const layFlat = !inUnitBucket || this.flat;

        // Add tiny Y offset to prevent Z-fighting
        const yOffset = i * 0.002;

        const pos = new THREE.Vector3(
          Math.cos(cardAngle) * rowRadius,
          (layFlat ? 0.12 : this.cardH * 0.5 + 0.12) + yOffset,
          Math.sin(cardAngle) * rowRadius,
        );

        // Orientation
        const baseRotY = -cardAngle - Math.PI / 2;
        // If attacking, keep upright even if tapped
        const tapRotZ = card.tapped && !isAttacking ? Math.PI / 2 : 0;

        // When laid flat, tapping rotates the card 90° around the
        // surface normal (world Y) — which is the local Z after the
        // -π/2 X-rotation, so we add tapRotZ to the Z component.
        const flatTapRot = card.tapped && !isAttacking ? Math.PI / 2 : 0;
        const rot = layFlat
          ? new THREE.Euler(
              -Math.PI / 2 + (i % 2 === 0 ? 0.01 : -0.01),
              0,
              -cardAngle +
                Math.PI / 2 +
                flatTapRot +
                (i % 2 === 0 ? 0.005 : -0.005),
            )
          : new THREE.Euler(0, baseRotY, tapRotZ);

        if (!inUnitBucket) actor.setFlatLayout(true);
        // Uniform size across units and non-units; orientation stays
        // per-kind (units upright, non-units flat).
        const cardScale = this.battlefieldCardScale * 0.7;
        actor.setTarget(pos, rot, cardScale);
      });

      // 2. INFRASTRUCTURE LAYOUT — infra cards are represented by the
      // mana-hex visualizer drawn later, so the flat CardActor copies get
      // parked off-screen (scale 0) to avoid duplicate hexes. They stay
      // in the scene graph for click targeting / future reuse.
      infra.forEach((card) => {
        const actor = allActors.get(card.id);
        if (!actor) return;
        actor.setZone('battlefield');
        actor.setTarget(
          new THREE.Vector3(0, -1000, 0),
          new THREE.Euler(-Math.PI / 2, 0, 0),
          0.0001,
        );
      });

      // 3. ATTACHMENTS (Follow target)
      attachments.forEach((card) => {
        const actor = allActors.get(card.id);
        if (!actor) return;
        if (!card.attached_to) return;
        const targetActor = allActors.get(card.attached_to);
        if (!targetActor) return;
        actor.setZone('battlefield');
        actor.setFlatLayout(this.flat);
        const { pos, rot } = targetActor.getAttachmentAnchor();
        actor.setTarget(
          pos,
          rot,
          this.battlefieldCardScale * 0.85 * 1.333,
          0.2,
        );
      });
    });

    // Leave blockers in their normal battlefield positions — the
    // combat-line renderer below draws a line from each attacker to each
    // of its blockers, which naturally shows the cross-pattern when
    // pairings aren't aligned. We still need to loop to set lethal
    // predictions on attackers and blockers.
    const blockedPairs = this.combatPairs.filter(
      (p) => p.is_blocked && p.blockers?.length,
    );
    for (let pairIndex = 0; pairIndex < blockedPairs.length; pairIndex += 1) {
      const pair = blockedPairs[pairIndex];
      const attackerActor = allActors.get(pair.attacker.id);
      if (!attackerActor) continue;

      if (pair.attacker_dies === true) {
        attackerActor.setLethal(true);
      }

      pair.blockers.forEach((blocker, blockerIndex) => {
        const blockerActor = allActors.get(blocker.id);
        if (!blockerActor) return;
        if (pair.blocker_dies?.[blockerIndex] === true) {
          blockerActor.setLethal(true);
        }
      });
    }

    // Combat lines: Cyber Hexagonal Data Link
    this.combatLineGroup.clear();
    if (this.combatPairs.length > 0) {
      for (const pair of this.combatPairs) {
        const attackerActor = allActors.get(pair.attacker.id);
        if (!attackerActor) continue;

        const attackerPos = attackerActor.getTargetPosition();

        // Collect every endpoint that needs a combat line from this attacker.
        // If blocked, one line per blocker. Otherwise one line to the
        // defending player/unit.
        const endpoints: { pos: THREE.Vector3; blocked: boolean }[] = [];

        if (pair.is_blocked && pair.blockers?.length) {
          for (const blocker of pair.blockers) {
            const blockerActor = allActors.get(blocker.id);
            if (!blockerActor) continue;
            const bp = blockerActor.getTargetPosition();
            endpoints.push({
              pos: new THREE.Vector3(bp.x, 0.15, bp.z),
              blocked: true,
            });
          }
        }

        if (endpoints.length === 0) {
          const combatTarget = this.getCombatTargetPosition(pair, allActors);
          if (combatTarget) {
            endpoints.push({
              pos: new THREE.Vector3(combatTarget.x, 0.15, combatTarget.z),
              blocked: false,
            });
          }
        }

        for (const endpoint of endpoints) {
          const targetPos = endpoint.pos;

          const startPt = new THREE.Vector3(attackerPos.x, 0.4, attackerPos.z);
          const endPt = targetPos.clone();
          endPt.y = 0.4; // Keep the laser strictly horizontal

          const dist = startPt.distanceTo(endPt);
          const midPt = startPt.clone().lerp(endPt, 0.5);

          const isBlocked = endpoint.blocked;
          // Cyan for direct hits, Amber/Red for blocked shots
          const color = isBlocked ? 0xff4400 : 0xf59e0b;

          const linkGroup = new THREE.Group();
          linkGroup.position.copy(midPt);
          linkGroup.lookAt(endPt);

          // --- 1. SIMPLE CONNECTION LINE ---
          const lineGeo = new THREE.BufferGeometry().setFromPoints([
            new THREE.Vector3(0, 0, -dist / 2),
            new THREE.Vector3(0, 0, dist / 2),
          ]);
          const lineMat = new THREE.LineBasicMaterial({
            color: color,
            transparent: true,
            opacity: 0.4,
            blending: THREE.AdditiveBlending,
          });
          linkGroup.add(new THREE.Line(lineGeo, lineMat));

          // --- 2. SUBTLE PULSING HEXAGONS ---
          const hexCount = Math.min(3, Math.max(1, Math.floor(dist / 2.5)));
          for (let i = 0; i < hexCount; i++) {
            const offset =
              ((this.currentTime * 0.8 + i / hexCount) % 1.0) - 0.5;
            const hexZ = offset * dist;

            const hexGeo = new THREE.CylinderGeometry(0.12, 0.12, 0.02, 6);
            hexGeo.rotateX(Math.PI / 2);
            const hexMat = new THREE.MeshBasicMaterial({
              color: color,
              transparent: true,
              opacity: 0.6 * (1.0 - Math.abs(offset * 2)),
              blending: THREE.AdditiveBlending,
            });
            const hex = new THREE.Mesh(hexGeo, hexMat);
            hex.position.z = hexZ;
            hex.rotation.z = (i * Math.PI) / 3;
            linkGroup.add(hex);
          }

          // --- 3. TARGET AMBIENT PARTICLES ---
          const reticleGroup = new THREE.Group();
          reticleGroup.position.set(endPt.x, 0.05, endPt.z);

          // Circular cloud of ascending "data motes"
          const partCount = 8;
          for (let i = 0; i < partCount; i++) {
            const spriteMat = new THREE.SpriteMaterial({
              color: color,
              transparent: true,
              opacity: 0.6,
              blending: THREE.AdditiveBlending,
            });
            const sprite = new THREE.Sprite(spriteMat);

            // Orbit and ascend based on time
            const t = this.currentTime * 1.5 + (i / partCount) * Math.PI * 2;
            const radius = 0.3;
            const x = Math.cos(t) * radius;
            const z = Math.sin(t) * radius;
            const y = ((this.currentTime * 0.5 + i / partCount) % 1.0) * 0.8;

            sprite.position.set(x, y, z);
            sprite.scale.setScalar(0.08 * (1.0 - y / 0.8)); // Fade out as they rise
            reticleGroup.add(sprite);
          }

          this.combatLineGroup.add(linkGroup);
          this.combatLineGroup.add(reticleGroup);
        } // end for endpoint
      } // end for pair
    }
  }

  public getDropIndex(
    point: THREE.Vector3,
    viewerId: string | null,
    draggingId?: string,
  ): number {
    if (!this.players.length || !viewerId) return 0;

    const viewerIndex = this.players.findIndex((p) => p.id === viewerId);
    if (viewerIndex === -1) return 0;

    const player = this.players[viewerIndex];
    const allUnits = player.battlefield_units || [];
    const unitIds = new Set(allUnits.map((c) => c.id));
    const base = allUnits.filter(
      (card) => !card.attached_to || !unitIds.has(card.attached_to),
    );
    if (base.length === 0) return 0;

    // Match the sort order used in layout() — units FRONT LINE first,
    // non-units behind.
    const nonUnits = base.filter((c) => c.kind_tag !== 'unit');
    const unitCards = base.filter((c) => c.kind_tag === 'unit');
    const sortedUnits = [...unitCards, ...nonUnits];

    const draggingCard = draggingId
      ? sortedUnits.find((c) => c.id === draggingId)
      : null;
    const draggingIsNonUnit = draggingCard
      ? draggingCard.kind_tag !== 'unit'
      : false;

    const seatAngle = (viewerIndex / this.players.length) * Math.PI * 2;
    const seatArc = (Math.PI * 2) / this.players.length;
    // Keep in sync with layout() above — matching split bucketing so
    // drop previews align with actual card positions.
    const layoutRadius = this.tableRadius * 0.75;
    const unitCount = unitCards.length;
    const unitLayout = computeRowLayout(
      unitCount,
      layoutRadius,
      this.cardW,
      this.cardH,
      seatArc,
    );
    // Non-units sit in the outer band along the infrastructure line.
    const nonUnitLayoutRadius = this.tableRadius;
    const nonUnitLayout = computeRowLayout(
      nonUnits.length,
      nonUnitLayoutRadius,
      this.cardW,
      this.cardH,
      seatArc,
    );

    let closestIndex = 0;
    let minDistSq = Infinity;

    // Generate slots and find closest
    for (let i = 0; i < sortedUnits.length; i++) {
      const inUnitBucket = i < unitCount;
      const bucket = inUnitBucket ? unitLayout : nonUnitLayout;
      const idxInBucket = inUnitBucket ? i : i - unitCount;
      let row = 0;
      for (let r = 0; r < 5; r++) {
        if (idxInBucket >= bucket.rowStarts[r]) row = r;
      }
      const cols = bucket.rowCounts[row] ?? 1;
      const col = idxInBucket - (bucket.rowStarts[row] ?? 0);

      const denom = Math.max(1, cols - 1);
      const targetArc = seatArc * 0.8;
      const rowRadius = bucket.rowRadiusFor(row);
      const angleStep = Math.min(
        (this.cardW + 0.12) / rowRadius,
        targetArc / denom,
      );

      const rowOffset =
        row === 0 ? this.cardW * 2 : row === 1 ? this.cardW * 0.45 : 0;
      const rowAngleOffset = -rowOffset / rowRadius;

      const cardAngle =
        seatAngle + rowAngleOffset + (col - (cols - 1) / 2) * angleStep;

      const slotX = Math.cos(cardAngle) * rowRadius;
      const slotZ = Math.sin(cardAngle) * rowRadius;

      const dx = point.x - slotX;
      const dz = point.z - slotZ;
      const distSq = dx * dx + dz * dz;

      if (distSq < minDistSq) {
        minDistSq = distSq;
        closestIndex = i;
      }
    }

    // Enforce kind boundaries: non-units stay in non-unit range, units stay in unit range
    if (draggingCard) {
      if (draggingIsNonUnit) {
        return Math.min(closestIndex, Math.max(0, nonUnits.length - 1));
      } else {
        return Math.max(closestIndex, nonUnits.length);
      }
    }

    return closestIndex;
  }

  public getCardIds(viewerId: string | null): string[] {
    const vid = viewerId || this.viewerId;
    if (!vid) return [];
    const player = this.players.find((p) => p.id === vid);
    if (!player) return [];
    const allUnits = player.battlefield_units || [];
    const unitIds = new Set(allUnits.map((c) => c.id));
    const base = allUnits.filter(
      (c) => !c.attached_to || !unitIds.has(c.attached_to),
    );
    return [
      ...base.filter((c) => c.kind_tag === 'unit'),
      ...base.filter((c) => c.kind_tag !== 'unit'),
    ].map((c) => c.id);
  }

  private applyGroupOpacity(group: THREE.Group, opacity: number) {
    const clamped = THREE.MathUtils.clamp(opacity, 0, 1);
    group.visible = clamped > 0.02;
    group.traverse((obj) => {
      const material = (obj as any).material;
      if (!material) return;
      const apply = (mat: any) => {
        if (!mat || typeof mat.opacity !== 'number') return;
        if (!mat.userData) mat.userData = {};
        if (typeof mat.userData.baseOpacity !== 'number') {
          mat.userData.baseOpacity = mat.opacity;
        }
        mat.transparent = true;
        mat.opacity = mat.userData.baseOpacity * clamped;
        mat.needsUpdate = true;
      };
      if (Array.isArray(material)) {
        material.forEach(apply);
      } else {
        apply(material);
      }
    });
  }
}

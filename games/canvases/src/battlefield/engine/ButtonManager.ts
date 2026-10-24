import * as THREE from 'three';
import { CardActor } from '../entities/CardActor';
import type { CardFactory } from '../entities/CardFactory';
import type { CardView } from '../types';
import { createFallbackOperator } from '../CardViewFactory';
import type { HudManager, HudElement, HudBar } from './HudManager';

export interface ButtonContext {
  isPackOpening: boolean;
  isLobby: boolean;
  isDraftMode: boolean;
  isDeckBuilding: boolean;
  operatorSelectMode: boolean;
  deckBuilderMode: boolean;
  cardBrowserMode: boolean;
  deckSelectMode: boolean;
  // pack
  packPhase: string;
  userPackCount: number;
  // deck select
  selectedDeckId: string | null;
  selectedDeckCard: CardView | null;
  selectSelectedIndex: number;
  ringCards: CardView[];
  deckSelectLibrary: CardView[];
  // builder
  builderSelectedId: string | null;
  builderSelectedCard: CardView | null;
  selectedQty: number;
  deckBuilderTotalCards: number;
  deckBuilder: any | null;
  operatorCard: CardView | null;
  // filters
  manaOptions: any[];
  kindOptions: any[];
  kindDragOffset: number;
  activeFilters: Set<string>;
  sortOrder: string | null;
  // viewer / game
  gameState: any;
  viewer: any;
  // discard prompt
  pendingDiscard: {
    amount: number;
    source_description?: string | null;
    source_card?: CardView | null;
  } | null;
  discardSelected: Set<string>;
  discardCommitted: boolean;
}

const BASE_Y = 0.03;

export class ButtonManager {
  private buttons: THREE.Object3D[] = [];
  private labels: THREE.Mesh[] = [];
  private deckSelectLabel: THREE.Mesh | null = null;
  private operatorCardActor: CardActor | null = null;
  private currentKey = '__uninitialized__';
  private lobbyType: 'constructed' | 'draft' = 'constructed';

  private hudManager: HudManager | null = null;

  // Discard prompt is managed independently from the mode-switching buttons
  // so it can appear during normal gameplay.
  private discardPromptObjects: THREE.Object3D[] = [];
  private discardPromptKey: string | null = null;
  private discardBannerMesh: THREE.Mesh | null = null;

  constructor(
    private scene: THREE.Scene,
    private factory: CardFactory,
  ) {}

  public setHudManager(hud: HudManager): void {
    this.hudManager = hud;
  }

  public update(ctx: ButtonContext): void {
    // Cleanup operator actor when leaving builder mode
    if (!ctx.deckBuilderMode && this.operatorCardActor) {
      this.scene.remove(this.operatorCardActor.group);
      this.operatorCardActor = null;
    }

    const key = this.computeKey(ctx);
    if (key !== this.currentKey) {
      this.clearAll();
      this.currentKey = key;
      if (ctx.isPackOpening) this.buildPackButtons(ctx);
      else if (ctx.isDraftMode) {
        /* Draft HUD is handled by DraftOverlay.svelte */
      } else if (ctx.operatorSelectMode) this.buildOperatorSelectButtons(ctx);
      else if (ctx.cardBrowserMode) this.buildCardBrowserButtons(ctx);
      else if (ctx.deckBuilderMode) this.buildDeckBuilderButtons(ctx);
      else if (ctx.deckSelectMode) this.buildDeckSelectButtons(ctx);
      else if (ctx.isLobby) this.buildLobbyButtons(ctx);
    }
    this.updateDiscardPrompt(ctx);
    this.updateVisibility(ctx);
  }

  public tick(delta: number, time: number): void {
    if (this.operatorCardActor) {
      this.operatorCardActor.tick(delta, time);
    }
    if (this.discardBannerMesh) {
      this.discardBannerMesh.rotation.y += delta * 0.35;
    }
  }

  private updateDiscardPrompt(ctx: ButtonContext): void {
    const showPrompt = Boolean(ctx.pendingDiscard) && !ctx.discardCommitted;
    const nextKey = showPrompt
      ? `discard|${ctx.pendingDiscard!.amount}|${ctx.discardSelected.size}|${ctx.pendingDiscard!.source_card?.id ?? ''}|${ctx.pendingDiscard!.source_description ?? ''}`
      : null;
    if (nextKey === this.discardPromptKey) return;

    for (const obj of this.discardPromptObjects) {
      this.scene.remove(obj);
      obj.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          o.geometry?.dispose();
          const m = o.material as any;
          if (Array.isArray(m)) m.forEach((mm: any) => mm?.dispose?.());
          else m?.dispose?.();
        }
      });
    }
    this.discardPromptObjects = [];
    this.discardBannerMesh = null;
    this.discardPromptKey = nextKey;

    if (!showPrompt || !ctx.pendingDiscard) return;

    const pd = ctx.pendingDiscard;
    const remaining = Math.max(0, pd.amount - ctx.discardSelected.size);
    const action =
      remaining === 0
        ? `READY TO DISCARD ${pd.amount}`
        : `DISCARD ${remaining} MORE CARD${remaining === 1 ? '' : 'S'}`;
    const reason = this.discardReasonLabel(pd);
    const text = `${action} · ${reason}`;

    const portal = this.createDiscardPortal();
    if (portal) {
      this.scene.add(portal);
      this.discardPromptObjects.push(portal);
    }

    const banner = this.createRotatingDiscardBanner(text);
    if (banner) {
      banner.position.set(0, 0.55, 0);
      this.scene.add(banner);
      this.discardPromptObjects.push(banner);
      this.discardBannerMesh = banner;
    }
  }

  private discardReasonLabel(pd: {
    source_card?: { name?: string | null } | null;
    source_description?: string | null;
  }): string {
    if (pd.source_card?.name) return pd.source_card.name.toUpperCase();
    const desc = (pd.source_description || '').trim();
    if (!desc) return 'HAND SIZE';
    if (/discard down to|end of turn|end.? your turn|hand (size|limit)/i.test(desc)) {
      return 'HAND SIZE';
    }
    const stripped = desc.replace(/\.$/, '').toUpperCase();
    return stripped.length > 14 ? `${stripped.slice(0, 13)}…` : stripped;
  }

  private createDiscardPortal(): THREE.Group | null {
    const group = new THREE.Group();
    const innerR = 1.6;
    const outerR = 2.1;

    const discGeo = new THREE.CircleGeometry(innerR, 64);
    const discMat = new THREE.MeshBasicMaterial({
      color: 0xa855f7,
      transparent: true,
      opacity: 0.22,
      side: THREE.DoubleSide,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const disc = new THREE.Mesh(discGeo, discMat);
    disc.rotation.x = -Math.PI / 2;
    disc.position.y = 0.03;
    group.add(disc);

    const ringGeo = new THREE.RingGeometry(innerR - 0.04, innerR, 96);
    const ringMat = new THREE.LineBasicMaterial({
      color: 0xc084fc,
      transparent: true,
      opacity: 0.85,
      blending: THREE.AdditiveBlending,
    });
    const ringEdges = new THREE.EdgesGeometry(ringGeo);
    const ring = new THREE.LineSegments(ringEdges, ringMat);
    ring.rotation.x = -Math.PI / 2;
    ring.position.y = 0.035;
    group.add(ring);

    const outerGeo = new THREE.RingGeometry(outerR - 0.02, outerR, 96);
    const outerMat = new THREE.LineBasicMaterial({
      color: 0x9333ea,
      transparent: true,
      opacity: 0.45,
      blending: THREE.AdditiveBlending,
    });
    const outerEdges = new THREE.EdgesGeometry(outerGeo);
    const outerRing = new THREE.LineSegments(outerEdges, outerMat);
    outerRing.rotation.x = -Math.PI / 2;
    outerRing.position.y = 0.034;
    group.add(outerRing);

    return group;
  }

  private createRotatingDiscardBanner(text: string): THREE.Mesh | null {
    const radius = 2.2;
    const height = 0.9;
    const segments = 128;
    const repeats = 3;

    const canvas = document.createElement('canvas');
    const canvasW = 4096;
    const canvasH = 256;
    canvas.width = canvasW;
    canvas.height = canvasH;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    ctx.clearRect(0, 0, canvasW, canvasH);
    const slotW = canvasW / repeats;
    const paddedSlot = slotW - 140;
    let fontSize = 112;
    ctx.font = `bold ${fontSize}px "Orbitron", monospace`;
    while (fontSize > 28 && ctx.measureText(text).width > paddedSlot) {
      fontSize -= 4;
      ctx.font = `bold ${fontSize}px "Orbitron", monospace`;
    }
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (let i = 0; i < repeats; i++) {
      const cx = slotW * i + slotW / 2;
      const cy = canvasH / 2;
      ctx.save();
      ctx.shadowColor = 'rgba(168, 85, 247, 0.9)';
      ctx.shadowBlur = 24;
      ctx.fillStyle = '#f3e8ff';
      ctx.fillText(text, cx, cy);
      ctx.restore();
      ctx.fillStyle = '#a855f7';
      ctx.beginPath();
      ctx.arc(slotW * i + 18, canvasH / 2, 8, 0, Math.PI * 2);
      ctx.fill();
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.wrapS = THREE.RepeatWrapping;
    texture.wrapT = THREE.ClampToEdgeWrapping;
    texture.anisotropy = 4;

    const geometry = new THREE.CylinderGeometry(
      radius,
      radius,
      height,
      segments,
      1,
      true,
    );
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      side: THREE.DoubleSide,
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.renderOrder = 25;
    mesh.userData.isDiscardBanner = true;
    return mesh;
  }

  public getButtons(): THREE.Object3D[] {
    return this.buttons;
  }

  public getDeckSelectLabel(): THREE.Mesh | null {
    return this.deckSelectLabel;
  }

  public dispose(): void {
    this.clearAll();
  }

  private buildFilterTogglePayload(
    value: string,
    activeFilters: Set<string>,
  ): { kind: string | null } {
    const next = new Set(activeFilters);
    if (next.has(value)) next.delete(value);
    else next.add(value);
    return { kind: next.size > 0 ? Array.from(next).join(',') : null };
  }

  // ─── Key Computation ─────────────────────────────────────────────────────

  private computeKey(ctx: ButtonContext): string {
    const {
      isPackOpening,
      isLobby,
      isDeckBuilding,
      operatorSelectMode,
      deckBuilderMode,
      deckSelectMode,
    } = ctx;

    if (!isDeckBuilding && !isLobby && !isPackOpening && !ctx.isDraftMode)
      return '';

    const modeKey = isPackOpening
      ? 'packs'
      : ctx.isDraftMode
        ? 'draft'
        : operatorSelectMode
          ? 'operator'
          : ctx.cardBrowserMode
            ? 'browser'
            : deckBuilderMode
              ? 'builder'
              : deckSelectMode
                ? 'select'
                : 'lobby';

    const manaVals = ctx.manaOptions.map((o: any) => o.value).join('|');
    const kindVals = ctx.kindOptions.map((o: any) => o.value).join('|');

    if (isPackOpening) {
      return `packs|${ctx.packPhase}|${ctx.userPackCount}`;
    }
    if (modeKey === 'draft') {
      return 'draft';
    }
    if (modeKey === 'select') {
      return `${modeKey}|${ctx.selectedDeckId ?? 'none'}`;
    }
    if (modeKey === 'lobby') {
      const viewerKey = !ctx.viewer
        ? 'unjoined'
        : ctx.viewer?.is_host
          ? 'host'
          : 'guest';
      const playerKeys = (ctx.gameState?.players ?? [])
        .map((p: any) => `${p.id}:${p.deck_name ?? 'none'}`)
        .join(',');
      return `${modeKey}|v3|${ctx.gameState?.players?.length ?? 0}|${ctx.gameState?.format ?? 'classic'}|${ctx.gameState?.mode ?? 'competitive'}|${ctx.gameState?.match_state?.format ?? 'bo1'}|${this.lobbyType}|${viewerKey}|${playerKeys}`;
    }
    if (modeKey === 'operator') {
      return `${modeKey}|${ctx.builderSelectedId ?? 'none'}|${ctx.deckBuilder?.operator_card_id ?? 'none'}|${ctx.kindDragOffset}|${ctx.deckBuilder?.filter_kind ?? 'all'}|mana:${manaVals}|kind:${kindVals}`;
    }
    if (modeKey === 'browser') {
      return `${modeKey}|browser-ui-v1|${ctx.kindDragOffset}|${ctx.deckBuilder?.filter_kind ?? 'all'}|sort:${ctx.sortOrder ?? 'none'}|${ctx.builderSelectedId ?? 'none'}|mana:${manaVals}|kind:${kindVals}`;
    }
    // builder
    const autobuildColors = (ctx.deckBuilder?.autobuild_colors ?? []).join(',');
    return `${modeKey}|builder-ui-v21|stage:${ctx.gameState?.stage ?? 'none'}|total:${ctx.deckBuilderTotalCards}|format:${ctx.deckBuilder?.format ?? 'none'}|operator:${ctx.deckBuilder?.operator_card_id ?? 'none'}|autobuild:${autobuildColors || 'none'}|${ctx.kindDragOffset}|${ctx.deckBuilder?.filter_kind ?? 'all'}|sort:${ctx.sortOrder ?? 'none'}|${ctx.builderSelectedId ?? 'none'}|qty:${ctx.selectedQty}|mana:${manaVals}|kind:${kindVals}`;
  }

  // ─── Cleanup ─────────────────────────────────────────────────────────────

  private clearAll(): void {
    // Clear HUD when rebuilding
    this.hudManager?.clear();

    const operatorGroup = this.operatorCardActor?.group;
    this.buttons.forEach((btn) => {
      if (btn === operatorGroup) return; // kept alive across rebuilds
      this.scene.remove(btn);
      btn.traverse((obj) => {
        if (
          obj instanceof THREE.Mesh ||
          obj instanceof THREE.LineSegments ||
          obj instanceof THREE.Line
        ) {
          obj.geometry?.dispose();
          if (Array.isArray(obj.material))
            obj.material.forEach((m) => m.dispose());
          else obj.material?.dispose();
        }
      });
    });
    this.labels.forEach((lbl) => {
      this.scene.remove(lbl);
      lbl.traverse((obj) => {
        if (obj instanceof THREE.Mesh) {
          obj.geometry?.dispose();
          if (Array.isArray(obj.material))
            obj.material.forEach((m) => m.dispose());
          else obj.material?.dispose();
        }
      });
    });
    if (this.deckSelectLabel) {
      this.scene.remove(this.deckSelectLabel);
      this.deckSelectLabel = null;
    }
    // Remove operator from scene temporarily — buildOperatorCardActor re-adds it
    if (this.operatorCardActor) {
      this.scene.remove(this.operatorCardActor.group);
    }
    this.buttons = [];
    this.labels = [];
  }

  // ─── Visibility & Animation (per-frame) ──────────────────────────────────

  private updateVisibility(ctx: ButtonContext): void {
    const {
      isPackOpening,
      isDeckBuilding,
      isLobby,
      operatorSelectMode,
      deckBuilderMode,
      deckSelectMode,
    } = ctx;

    this.buttons.forEach((btn) => {
      const mode = btn.userData?.mode as string | undefined;
      const shouldShow =
        (isPackOpening && mode === 'packs') ||
        (operatorSelectMode && mode === 'operator') ||
        (ctx.cardBrowserMode && mode === 'browser') ||
        (!operatorSelectMode && deckBuilderMode && mode === 'builder') ||
        (deckSelectMode && mode === 'select') ||
        (isLobby && !deckBuilderMode && !deckSelectMode && mode === 'lobby') ||
        (ctx.isDraftMode && mode === 'draft');
      btn.visible =
        (isDeckBuilding || isLobby || isPackOpening || ctx.isDraftMode) &&
        shouldShow;

      if (
        (!isDeckBuilding && !isPackOpening && !ctx.isDraftMode) ||
        !btn.visible
      ) {
        btn.scale.setScalar(1.0);
      } else {
        const base = btn.userData.baseScale ?? 1.0;
        const target = btn.userData.isHovered ? base * 1.1 : base;
        btn.scale.x = THREE.MathUtils.lerp(btn.scale.x, target, 0.15);
        btn.scale.y = THREE.MathUtils.lerp(btn.scale.y, target, 0.15);
        btn.scale.z = THREE.MathUtils.lerp(btn.scale.z, target, 0.15);
      }
    });

    if (this.deckSelectLabel) {
      this.deckSelectLabel.visible =
        isLobby && !deckBuilderMode && !deckSelectMode;
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────

  private get utils() {
    return this.factory.getTextureUtils();
  }

  private addButton(btn: THREE.Object3D | null, mode: string): void {
    if (!btn) return;
    btn.userData.mode = mode;
    this.scene.add(btn);
    this.buttons.push(btn);
  }

  private addLabel(label: THREE.Mesh | null): void {
    if (!label) return;
    this.scene.add(label);
    this.labels.push(label);
  }

  private placeFlat(obj: THREE.Object3D, x: number, z: number): void {
    obj.position.set(x, BASE_Y, z);
    obj.rotation.x = -Math.PI / 2;
    obj.rotation.z = Math.PI / 2;
  }

  // ─── Pack Buttons ─────────────────────────────────────────────────────────

  private buildPackButtons(ctx: ButtonContext): void {
    const { packPhase, userPackCount } = ctx;
    const px = 3.2;

    if (
      packPhase === 'inventory' ||
      packPhase === 'grab' ||
      packPhase === 'tear'
    ) {
      const closeBtn = this.utils.create3DButton(
        'Close',
        'internal.ui.close_packs',
        '#f43f5e',
        { width: 1.4, height: 0.5 },
      );
      if (closeBtn) {
        this.placeFlat(closeBtn, px, 0);
        this.addButton(closeBtn, 'packs');
      }
    } else if (packPhase === 'reveal') {
      const revealAllBtn = this.utils.create3DButton(
        'Reveal All',
        'internal.ui.reveal_all_packs',
        '#38bdf8',
        { width: 2.0, height: 0.5 },
      );
      if (revealAllBtn) {
        this.placeFlat(revealAllBtn, px, 0);
        this.addButton(revealAllBtn, 'packs');
      }
    } else if (packPhase === 'summary') {
      const anotherBtn = this.utils.create3DButton(
        `Open Another (${userPackCount})`,
        'internal.ui.open_another_pack',
        '#facc15',
        { width: 3.5, height: 0.6 },
      );
      if (anotherBtn) {
        this.placeFlat(anotherBtn, 0, 0);
        this.addButton(anotherBtn, 'packs');
      }
      const closeBtn = this.utils.create3DButton(
        'Close',
        'internal.ui.close_packs',
        '#94a3b8',
        { width: 1.4, height: 0.5 },
      );
      if (closeBtn) {
        this.placeFlat(closeBtn, 1.2, 0);
        this.addButton(closeBtn, 'packs');
      }
    }
  }

  // ─── Operator Select Buttons ─────────────────────────────────────────────

  private buildOperatorSelectButtons(ctx: ButtonContext): void {
    const { deckBuilder, builderSelectedId } = ctx;

    const backBtn = this.utils.create3DButton(
      'Back',
      'internal.ui.close_operator_select',
      '#94a3b8',
      { width: 1.6, height: 0.5, fontSize: 54 },
    );
    if (backBtn) {
      this.placeFlat(backBtn, 3.2, 2.0);
      this.addButton(backBtn, 'operator');
    }

    if (deckBuilder?.deck_id && builderSelectedId) {
      const setBtn = this.utils.create3DButton(
        'Set Operator',
        'games.cardgame.builder_set_operator',
        '#22d3ee',
        { width: 2.4, height: 0.55, fontSize: 46 },
      );
      if (setBtn) {
        this.placeFlat(setBtn, 3.2, -2.0);
        setBtn.userData.payload = {
          deck_id: deckBuilder.deck_id,
          card_id: builderSelectedId,
        };
        this.addButton(setBtn, 'operator');
      }
    }

    this.buildManaHoneycomb(ctx);
  }

  // ─── Deck Builder Buttons ────────────────────────────────────────────────

  private buildDeckBuilderButtons(ctx: ButtonContext): void {
    const { deckBuilder, builderSelectedId, builderSelectedCard, selectedQty } =
      ctx;

    const isDraftBuilding = ctx.gameState?.stage === 'draft_building';

    if (!isDraftBuilding) {
      const backBtn = this.utils.create3DButton(
        'Back',
        'games.cardgame.builder_close',
        '#94a3b8',
        { width: 1.6, height: 0.5, fontSize: 54 },
      );
      if (backBtn) {
        this.placeFlat(backBtn, 1, 2.3);
        this.addButton(backBtn, 'builder');
      }
    }

    if (deckBuilder?.deck_id) {
      const autobuildLabel =
        deckBuilder?.format === 'operator' ? 'Best Operator' : 'Best Standard';
      const autobuildBtn = this.utils.create3DButton(
        autobuildLabel,
        'games.cardgame.builder_autobuild',
        '#f59e0b',
        { width: 2.3, height: 0.5, fontSize: 42 },
      );
      if (autobuildBtn) {
        this.placeFlat(autobuildBtn, -3.4, 2.2);
        this.addButton(autobuildBtn, 'builder');
      }

      const readyAction = isDraftBuilding
        ? 'games.cardgame.draft_confirm_pool'
        : 'games.cardgame.select_deck';
      const readyBtn = this.utils.create3DButton(
        'Ready',
        readyAction,
        '#22d3ee',
        {
          width: 1.6,
          height: 0.5,
          fontSize: 54,
        },
      );
      if (readyBtn) {
        this.placeFlat(readyBtn, 1, -2.3);
        readyBtn.userData.payload = isDraftBuilding
          ? {}
          : { deck_id: deckBuilder.deck_id };
        this.addButton(readyBtn, 'builder');
      }

      const totalLabel = this.utils.create3DLabel(
        `${ctx.deckBuilderTotalCards} Cards`,
        '#ffffff',
      );
      if (totalLabel) {
        this.placeFlat(totalLabel, 1.9, 0);
        this.addLabel(totalLabel);
      }

      if (deckBuilder?.name) {
        const autobuildColors = (deckBuilder?.autobuild_colors ??
          []) as string[];
        const colorNames = autobuildColors
          .map((color) => this.colorNameForHex(color))
          .filter(Boolean);
        const subtitle =
          colorNames.length === 3
            ? `Best 3-Color: ${colorNames.join(' / ')}`
            : null;
        const infoBillboard = this.utils.createDeckInfoBillboard(
          deckBuilder.name,
          deckBuilder.description || null,
          autobuildColors,
          subtitle,
        );
        if (infoBillboard) {
          infoBillboard.position.set(3.0, 5.5, 4.0);
          this.addLabel(infoBillboard as any);
        }
      }

      this.buildOperatorCardActor(ctx);
    }

    this.buildKindFilters(ctx);
    this.buildManaHoneycomb(ctx);
    this.buildSortButtons(ctx);
    if (ctx.gameState?.stage !== 'draft_building') {
      this.buildOwnedFilter(ctx);
    }
    if (builderSelectedCard && deckBuilder?.deck_id && builderSelectedId) {
      this.buildSelectedCardUI(ctx);
    }
  }

  // ─── Card Browser Buttons ─────────────────────────────────────────────────

  private buildCardBrowserButtons(ctx: ButtonContext): void {
    const browserTitle = ctx.deckBuilder?.browser_title || 'Card Collection';
    const browserSubtitle =
      ctx.deckBuilder?.browser_subtitle || 'Mana Clash • Browse All Cards';

    this.buildCardBrowserHud(ctx);

    // Mana honeycomb stays on the table — it's a custom hex layout that doesn't
    // translate cleanly to camera HUD space. Kind/sort/owned/back now live on the HUD.
    this.buildManaHoneycomb(ctx, 'games.cardgame.browser_set_filter');

    const infoBillboard = this.utils.createDeckInfoBillboard(
      browserTitle,
      browserSubtitle,
      [],
      null,
    );
    if (infoBillboard) {
      infoBillboard.position.set(3.0, 5.5, 4.0);
      this.addLabel(infoBillboard as any);
    }

    // Card info labels for focused card
    const card = ctx.builderSelectedCard;
    if (card) {
      const rarityColors: Record<string, string> = {
        public: '#94a3b8',
        private: '#22c55e',
        restricted: '#3b82f6',
        classified: '#a855f7',
        singularity: '#f59e0b',
      };
      const rarityColor = rarityColors[card.rarity] ?? '#94a3b8';

      // Card name + value below the ring
      const valuePart =
        card.value != null ? ` · $${card.value.toFixed(2)}` : '';
      const nameBillboard = this.utils.createBillboardLabel(
        `${card.name}${valuePart}`,
        null,
        rarityColor,
      );
      if (nameBillboard) {
        const cardH = this.factory.cardH;
        nameBillboard.position.set(0, 0.03 + 2 - 0.5, 0);
        nameBillboard.position.addScaledVector(
          new THREE.Vector3(1, 0, 0),
          cardH,
        );
        nameBillboard.scale.multiplyScalar(0.5);
        this.addLabel(nameBillboard as any);
      }

      // Group label above the card ring — shows current sort category
      const sort = ctx.sortOrder;
      let groupText: string | null = null;
      let groupColor = '#94a3b8';
      if (sort?.startsWith('rarity')) {
        groupText = card.rarity.charAt(0).toUpperCase() + card.rarity.slice(1);
        groupColor = rarityColor;
      } else if (sort?.startsWith('color')) {
        const primaryColor = card.colors?.[0];
        const hexToMana: Record<string, string> = {
          '#22d3ee': 'Neon',
          '#a855f7': 'Glitch',
          '#f8fafc': 'Chrome',
          '#ef4444': 'Slag',
          '#22c55e': 'Bio',
        };
        groupText = (primaryColor && hexToMana[primaryColor]) || 'Colorless';
        groupColor = primaryColor || '#94a3b8';
      } else if (sort?.startsWith('name')) {
        groupText = card.name.charAt(0).toUpperCase();
        groupColor = '#22d3ee';
      }

      if (groupText) {
        const groupBillboard = this.utils.createBillboardLabel(
          groupText,
          null,
          groupColor,
        );
        if (groupBillboard) {
          groupBillboard.position.set(0, 2.6, 0);
          groupBillboard.scale.multiplyScalar(0.7);
          this.addLabel(groupBillboard as any);
        }
      }
    }
  }

  private buildCardBrowserHud(ctx: ButtonContext): void {
    const darkBg = '#09090b';
    const cyan = '#22d3ee';
    const slate = '#e2e8f0'; // slate-200 — legible inactive outline on the dark HUD bar
    const green = '#22c55e';
    const amber = '#facc15';
    const orangeDesc = '#f97316';

    const topBarH = 0.65;
    const bottomBarH = 0.65;

    const bars: HudBar[] = [
      {
        id: 'browser-top-bar',
        anchor: 'top-center',
        width: 'full',
        height: topBarH,
        color: darkBg,
        opacity: 0.95,
      },
      {
        id: 'browser-top-accent',
        anchor: 'top-center',
        width: 'full',
        height: 0.02,
        color: cyan,
        offsetY: -topBarH,
        opacity: 1.0,
      },
      {
        id: 'browser-bottom-bar',
        anchor: 'bottom-center',
        width: 'full',
        height: bottomBarH,
        color: darkBg,
        opacity: 0.95,
      },
      {
        id: 'browser-bottom-accent',
        anchor: 'bottom-center',
        width: 'full',
        height: 0.02,
        color: cyan,
        offsetY: bottomBarH,
        opacity: 1.0,
      },
    ];

    const elements: HudElement[] = [];

    // Back — top-left
    elements.push({
      id: 'browser-back',
      type: 'text-button',
      label: 'Back',
      actionId: 'games.cardgame.card_browser_back',
      color: slate,
      anchor: 'top-left',
      offsetX: 1.3,
      offsetY: -topBarH / 2,
      options: { width: 1.8, height: 0.6, fontSize: 128 },
    });

    // Sort row — top-center. Each click cycles off → asc → desc → off.
    const sortOptions = [
      { value: 'name', label: 'Name' },
      { value: 'color', label: 'Color' },
      { value: 'rarity', label: 'Rarity' },
      { value: 'value', label: 'Value' },
      { value: 'obtained', label: 'Obtained' },
    ];
    const currentSort = ctx.sortOrder;
    const sortSpacing = 1.5;
    const sortCenter = (sortOptions.length - 1) / 2;
    sortOptions.forEach((opt, idx) => {
      const isAsc = currentSort === opt.value;
      const isDesc = currentSort === `${opt.value}_desc`;
      const isActive = isAsc || isDesc;
      const color = isAsc ? amber : isDesc ? orangeDesc : slate;
      const label = isDesc
        ? `${opt.label} ↓`
        : isAsc
          ? `${opt.label} ↑`
          : opt.label;
      const nextSort = isAsc ? `${opt.value}_desc` : isDesc ? null : opt.value;
      elements.push({
        id: `browser-sort-${opt.value}`,
        type: 'text-button',
        label,
        actionId: 'games.cardgame.browser_set_sort',
        color,
        active: isActive,
        anchor: 'top-center',
        offsetX: (idx - sortCenter) * sortSpacing,
        offsetY: -topBarH / 2,
        options: { width: 1.6, height: 0.6, fontSize: 128 },
        payload: { sort: nextSort },
      });
    });

    // Owned checkbox — top-right
    if (ctx.deckBuilder?.browser_source === 'catalog') {
      const ownedActive = ctx.activeFilters.has('owned');
      elements.push({
        id: 'browser-owned',
        type: 'text-button-icon',
        label: 'Owned',
        iconText: ownedActive ? '☑' : '☐',
        actionId: 'games.cardgame.browser_set_filter',
        color: ownedActive ? amber : slate,
        active: ownedActive,
        anchor: 'top-right',
        offsetX: -2.2,
        offsetY: -topBarH / 2,
        options: { width: 2.0, height: 0.6, fontSize: 128 },
        payload: this.buildFilterTogglePayload('owned', ctx.activeFilters),
      });
    }

    // Kind filters — bottom-center. One tab per kind.
    const kindOptions = ctx.kindOptions ?? [];
    if (kindOptions.length > 0) {
      const kindSpacing = 1.9;
      const kindCenter = (kindOptions.length - 1) / 2;
      const kindCounts: Record<string, number> = {};
      const countSource = (ctx.deckBuilder?.library || []).map((c: any) => ({
        card: c,
        quantity: 1,
      }));
      countSource.forEach((entry: any) => {
        const kind = entry?.card?.kind_tag;
        if (kind)
          kindCounts[kind] = (kindCounts[kind] || 0) + (entry.quantity || 0);
      });
      kindOptions.forEach((opt: any, idx: number) => {
        const isActive = ctx.activeFilters.has(opt.value);
        const count = kindCounts[opt.value] || 0;
        elements.push({
          id: `browser-kind-${opt.value}`,
          type: 'text-button',
          label: opt.label,
          actionId: 'games.cardgame.browser_set_filter',
          color: isActive ? cyan : slate,
          active: isActive,
          anchor: 'bottom-center',
          offsetX: (idx - kindCenter) * kindSpacing,
          offsetY: bottomBarH / 2,
          options: {
            width: 2.0,
            height: 0.7,
            fontSize: 160,
            subtitle: count.toString(),
          },
          payload: this.buildFilterTogglePayload(opt.value, ctx.activeFilters),
        });
      });
    }

    this.hudManager?.setConfig({ bars, elements });
  }

  private colorNameForHex(color: string): string | null {
    switch (color.toLowerCase()) {
      case '#00e5ff':
        return 'Neon';
      case '#a855f7':
        return 'Glitch';
      case '#cbd5e1':
        return 'Chrome';
      case '#f97316':
        return 'Slag';
      case '#22c55e':
        return 'Bio';
      default:
        return null;
    }
  }

  private buildOperatorCardActor(ctx: ButtonContext): void {
    const { deckBuilder, operatorCard } = ctx;
    if (!deckBuilder?.deck_id) return;

    const operatorCardView = operatorCard ?? createFallbackOperator();
    let actor = this.operatorCardActor;
    if (!actor) {
      actor = new CardActor(operatorCardView, this.factory);
      actor.setZone('hand');
      actor.setFlatLayout(true);
      const cmdPos = new THREE.Vector3(-3.4, BASE_Y + 0.05, -1.8);
      const cmdRot = new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2);
      actor.setTarget(cmdPos, cmdRot, 0.9, 0.2);
      this.operatorCardActor = actor;
    } else {
      actor.update(operatorCardView);
    }

    const cmdGroup = actor.group;
    cmdGroup.userData.isButton = true;
    cmdGroup.userData.actionId = 'internal.ui.open_operator_select';
    cmdGroup.userData.mode = 'builder';
    cmdGroup.userData.baseScale = 0.9;
    cmdGroup.userData.hoverScale = 1.05;
    cmdGroup.userData.hoverGlow = '#22d3ee';

    this.scene.add(cmdGroup);
    this.buttons.push(cmdGroup);
  }

  private buildKindFilters(ctx: ButtonContext, actionOverride?: string): void {
    const { kindOptions, deckBuilder, operatorSelectMode } = ctx;
    if (!kindOptions || kindOptions.length === 0) return;

    const filterAction = actionOverride ?? 'games.cardgame.builder_set_filter';
    const filterMode = operatorSelectMode
      ? 'operator'
      : ctx.cardBrowserMode
        ? 'browser'
        : 'builder';
    const activeFilters = ctx.activeFilters;
    const kindCounts: Record<string, number> = {};
    const countSource = ctx.cardBrowserMode
      ? (deckBuilder?.library || []).map((c: any) => ({ card: c, quantity: 1 }))
      : deckBuilder?.cards || [];
    countSource.forEach((entry: any) => {
      const kind = entry?.card?.kind_tag;
      if (kind)
        kindCounts[kind] = (kindCounts[kind] || 0) + (entry.quantity || 0);
    });

    const kindSpacing = 0.4;
    const kindX = 2.8;
    const len = kindOptions.length;

    kindOptions.forEach((opt: any, idx: number) => {
      const isActive = activeFilters.has(opt.value);
      const color = isActive ? '#22d3ee' : '#64748b';
      const count = kindCounts[opt.value] || 0;

      let row = 0,
        rowIndex = 0,
        rowCount = 0,
        gap = 1.1;
      if (idx < 2) {
        row = 0;
        rowIndex = idx;
        rowCount = Math.min(2, len);
        gap = 1.8;
      } else if (idx < 5) {
        row = 1;
        rowIndex = idx - 2;
        rowCount = Math.min(3, Math.max(0, len - 2));
        gap = 1.4;
      } else {
        row = 2;
        rowIndex = idx - 5;
        rowCount = Math.min(3, Math.max(0, len - 5));
        gap = 1.8;
      }

      const centerIndex = (rowCount - 1) / 2;
      const offset = rowIndex - centerIndex;
      const scale = 1.2;

      const btn = this.utils.create3DTextButton(
        opt.label,
        filterAction,
        color,
        isActive,
        { fontSize: 32, subtitle: count.toString() },
      );
      if (!btn) return;

      btn.position.set(kindX + row * kindSpacing, BASE_Y, offset * gap);
      btn.rotation.x = -Math.PI / 2;
      btn.rotation.z = Math.PI / 2;
      btn.scale.setScalar(scale);
      btn.userData.baseScale = scale;
      btn.userData.filterGroup = 'kind';
      btn.userData.payload = this.buildFilterTogglePayload(
        opt.value,
        activeFilters,
      );
      btn.userData.mode = filterMode;
      this.scene.add(btn);
      this.buttons.push(btn);
    });
  }

  private buildManaHoneycomb(
    ctx: ButtonContext,
    actionOverride?: string,
  ): void {
    const { manaOptions, operatorSelectMode } = ctx;
    if (!manaOptions || manaOptions.length === 0) return;
    const filterAction = actionOverride ?? 'games.cardgame.builder_set_filter';

    const manaColors: Record<string, string> = {
      neon: '#00e5ff',
      glitch: '#a855f7',
      chrome: '#cbd5e1',
      slag: '#f97316',
      bio: '#22c55e',
      colorless: '#64748b',
    };

    const activeFilters = ctx.activeFilters;
    const filterMode = operatorSelectMode
      ? 'operator'
      : ctx.cardBrowserMode
        ? 'browser'
        : 'builder';
    const honeycombScale = 2.0;
    const hexR = 0.2 * honeycombScale;
    const hStep = Math.sqrt(3) * hexR;
    const vStep = 1.5 * hexR;
    // Browser mode uses a shifted camera lookAt. Place the honeycomb just
    // above the card ring (slightly -X, which is screen-up with up = -X) so
    // it sits between the top HUD and the cards without crowding either.
    // Deck builder keeps the original placement.
    const center = ctx.cardBrowserMode
      ? new THREE.Vector3(-2.25, BASE_Y, 0.0)
      : new THREE.Vector3(-3.4, BASE_Y, 0.0);
    const rot = (60 * Math.PI) / 180;
    const rawOffsets = [
      new THREE.Vector3(0, 0, 0),
      new THREE.Vector3(hStep, 0, 0),
      new THREE.Vector3(hStep * 0.5, 0, vStep),
      new THREE.Vector3(-hStep * 0.5, 0, vStep),
      new THREE.Vector3(-hStep, 0, 0),
      new THREE.Vector3(-hStep * 0.5, 0, -vStep),
      new THREE.Vector3(hStep * 0.5, 0, -vStep),
    ].map(
      (v) =>
        new THREE.Vector3(
          v.x * Math.cos(rot) - v.z * Math.sin(rot),
          0,
          v.x * Math.sin(rot) + v.z * Math.cos(rot),
        ),
    );
    const manaSlotByValue: Record<string, number> = {
      colorless: 0,
      glitch: 1,
      neon: 2,
      slag: 3,
      bio: 4,
      chrome: 5,
      // Keep any unknown value in the last available slot.
    };

    manaOptions.forEach((item: any, idx: number) => {
      const isActive = activeFilters.has(item.value);
      const color = manaColors[item.value] ?? '#22d3ee';
      const hexGeom = new THREE.CircleGeometry(hexR, 6);
      const btnGroup = new THREE.Group();

      const fill = new THREE.Mesh(
        hexGeom,
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: isActive ? 0.95 : 0.0,
          side: THREE.DoubleSide,
          depthWrite: false,
          depthTest: false,
          blending: THREE.AdditiveBlending,
        }),
      );
      (fill as any).raycast = () => null;
      fill.rotation.x = -Math.PI / 2;
      fill.rotation.z = Math.PI / 2;
      fill.renderOrder = 60;

      const edgeGeom = new THREE.EdgesGeometry(hexGeom);
      edgeGeom.translate(0, 0.001, 0);
      let edge: THREE.Object3D;
      if (isActive) {
        edge = new THREE.LineSegments(
          edgeGeom,
          new THREE.LineBasicMaterial({
            color,
            transparent: true,
            opacity: 1.0,
            depthTest: false,
          }),
        );
      } else {
        const dashed = new THREE.LineSegments(
          edgeGeom,
          new THREE.LineDashedMaterial({
            color,
            transparent: true,
            opacity: 0.55,
            dashSize: 0.08 * honeycombScale,
            gapSize: 0.05 * honeycombScale,
            depthTest: false,
          }),
        );
        dashed.computeLineDistances();
        edge = dashed;
      }
      (edge as any).raycast = () => null;
      (edge as any).renderOrder = 61;
      fill.add(edge);

      const hoverHalo = new THREE.Mesh(
        new THREE.RingGeometry(hexR * 1.25, hexR * 1.45, 6),
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: 0.0,
          depthWrite: false,
          depthTest: false,
          blending: THREE.AdditiveBlending,
          side: THREE.DoubleSide,
        }),
      );
      (hoverHalo as any).raycast = () => null;
      hoverHalo.rotation.x = -Math.PI / 2;
      hoverHalo.rotation.z = Math.PI / 2;
      hoverHalo.renderOrder = 62;

      const hit = new THREE.Mesh(
        hexGeom,
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: 0.0,
          side: THREE.DoubleSide,
          depthWrite: false,
          depthTest: false,
        }),
      );
      hit.rotation.x = -Math.PI / 2;
      hit.rotation.z = Math.PI / 2;

      btnGroup.add(hit);
      btnGroup.add(fill);
      btnGroup.add(hoverHalo);
      btnGroup.userData.isButton = true;
      btnGroup.userData.actionId = filterAction;
      btnGroup.userData.baseScale = 1.0;
      btnGroup.userData.hoverTarget = fill;
      btnGroup.userData.hoverEdge = edge;
      btnGroup.userData.hoverHalo = hoverHalo;
      btnGroup.userData.payload = this.buildFilterTogglePayload(
        item.value,
        activeFilters,
      );
      btnGroup.userData.mode = filterMode;

      const slotIndex =
        manaSlotByValue[item.value] ?? Math.min(idx, rawOffsets.length - 1);
      if (slotIndex < rawOffsets.length) {
        const pos = center.clone().add(rawOffsets[slotIndex]);
        btnGroup.position.set(pos.x, BASE_Y, pos.z);
      }

      this.scene.add(btnGroup);
      this.buttons.push(btnGroup);
    });
  }

  private buildSortButtons(ctx: ButtonContext, actionOverride?: string): void {
    const sortOptions = [
      { value: 'name', label: 'Name' },
      { value: 'color', label: 'Color' },
      { value: 'rarity', label: 'Rarity' },
      { value: 'value', label: 'Value' },
      { value: 'obtained', label: 'Obtained' },
    ];
    const sortAction = actionOverride ?? 'games.cardgame.builder_set_sort';
    const currentSort = ctx.sortOrder;
    // Position below the mana honeycomb grid (less negative X = lower on screen)
    const sortX = -1.8;
    const spacing = 1.2;
    const totalWidth = (sortOptions.length - 1) * spacing;

    sortOptions.forEach((opt, idx) => {
      const isAsc = currentSort === opt.value;
      const isDesc = currentSort === `${opt.value}_desc`;
      const isActive = isAsc || isDesc;
      const color = isAsc ? '#facc15' : isDesc ? '#f97316' : '#64748b';
      const label = isDesc
        ? `${opt.label} ↓`
        : isAsc
          ? `${opt.label} ↑`
          : opt.label;
      const btn = this.utils.create3DTextButton(
        label,
        sortAction,
        color,
        isActive,
        { fontSize: 28 },
      );
      if (!btn) return;

      const z = -totalWidth / 2 + idx * spacing;
      btn.position.set(sortX, BASE_Y, z);
      btn.rotation.x = -Math.PI / 2;
      btn.rotation.z = Math.PI / 2;
      const scale = 1.1;
      btn.scale.setScalar(scale);
      btn.userData.baseScale = scale;
      btn.userData.isButton = true;
      // Cycle: off → asc → desc → off
      const nextSort = isAsc ? `${opt.value}_desc` : isDesc ? null : opt.value;
      btn.userData.payload = { sort: nextSort };
      btn.userData.mode = ctx.cardBrowserMode ? 'browser' : 'builder';
      this.scene.add(btn);
      this.buttons.push(btn);
    });
  }

  private buildOwnedFilter(ctx: ButtonContext, actionOverride?: string): void {
    const filterAction = actionOverride ?? 'games.cardgame.builder_set_filter';
    const isActive = ctx.activeFilters.has('owned');
    const color = isActive ? '#22c55e' : '#64748b';
    const btn = this.utils.create3DTextButton(
      'Owned',
      filterAction,
      color,
      isActive,
      { fontSize: 28 },
    );
    if (!btn) return;

    const sortCount = 5;
    const spacing = 1.2;
    const totalWidth = (sortCount - 1) * spacing;
    const ownedZ = totalWidth / 2 + spacing;
    btn.position.set(-1.8, BASE_Y, ownedZ);
    btn.rotation.x = -Math.PI / 2;
    btn.rotation.z = Math.PI / 2;
    const scale = 1.1;
    btn.scale.setScalar(scale);
    btn.userData.baseScale = scale;
    btn.userData.isButton = true;
    btn.userData.payload = this.buildFilterTogglePayload(
      'owned',
      ctx.activeFilters,
    );
    btn.userData.mode = ctx.cardBrowserMode ? 'browser' : 'builder';
    this.scene.add(btn);
    this.buttons.push(btn);
  }

  private buildSelectedCardUI(ctx: ButtonContext): void {
    const { deckBuilder, builderSelectedId, builderSelectedCard, selectedQty } =
      ctx;
    if (!builderSelectedCard || !deckBuilder?.deck_id || !builderSelectedId)
      return;

    const tint = builderSelectedCard?.colors?.[0] ?? '#22d3ee';
    const builderLineY = 0.03;
    const builderSelectedLift = 2;
    const cardW = this.factory.cardW;
    const cardH = this.factory.cardH;

    const billboard = this.utils.createBillboardLabel(
      `In deck · ${selectedQty}`,
      null,
      tint,
    );
    if (billboard) {
      billboard.position.set(0, builderLineY + builderSelectedLift - 0.5, 0);
      billboard.position.addScaledVector(new THREE.Vector3(-1, 0, 0), cardH);
      this.addLabel(billboard as any);
    }

    const buttonOffset = cardW * 1.25;
    const bottomY = builderLineY + builderSelectedLift - cardH * 0.5;
    const glyphSize = cardW * 0.7;
    const glyphThickness = 0.08;
    const glyphDepth = 0.16;
    const glyphCenterY = bottomY + cardH * 0.8 * 0.5;

    const makeGlyph = (kind: 'plus' | 'minus', color: string) => {
      const group = new THREE.Group();
      const mat = new THREE.MeshStandardMaterial({
        color: new THREE.Color(color),
        emissive: new THREE.Color(color).multiplyScalar(0.5),
        metalness: 0.25,
        roughness: 0.3,
      });
      group.add(
        new THREE.Mesh(
          new THREE.BoxGeometry(glyphSize, glyphThickness, glyphDepth),
          mat,
        ),
      );
      if (kind === 'plus') {
        group.add(
          new THREE.Mesh(
            new THREE.BoxGeometry(glyphThickness, glyphSize, glyphDepth),
            mat,
          ),
        );
      }
      const hit = new THREE.Mesh(
        new THREE.BoxGeometry(glyphSize * 1.2, glyphSize * 1.2, glyphDepth * 2),
        new THREE.MeshBasicMaterial({
          transparent: true,
          opacity: 0.0,
          depthWrite: false,
        }),
      );
      group.add(hit);
      return group;
    };

    const addBtn = makeGlyph('plus', '#22d3ee');
    addBtn.position.set(0, glyphCenterY, -buttonOffset);
    addBtn.rotation.x = -Math.PI / 2;
    addBtn.rotation.z = Math.PI / 2;
    addBtn.userData.baseScale = 1.0;
    addBtn.userData.hoverScale = 1.25;
    addBtn.userData.hoverGlow = '#7dd3fc';
    addBtn.userData.isButton = true;
    addBtn.userData.actionId = 'games.cardgame.builder_add_card';
    addBtn.userData.payload = {
      deck_id: deckBuilder.deck_id,
      card_id: builderSelectedId,
    };
    addBtn.userData.mode = 'builder';
    this.scene.add(addBtn);
    this.buttons.push(addBtn as any);

    const removeBtn = makeGlyph('minus', '#ff2d55');
    removeBtn.position.set(0, glyphCenterY, buttonOffset);
    removeBtn.rotation.x = -Math.PI / 2;
    removeBtn.rotation.z = Math.PI / 2;
    removeBtn.userData.baseScale = 1.0;
    removeBtn.userData.hoverScale = 1.25;
    removeBtn.userData.hoverGlow = '#ff2d55';
    removeBtn.userData.hoverEmissive = 2.2;
    removeBtn.userData.isButton = true;
    removeBtn.userData.actionId = 'games.cardgame.builder_remove_card';
    removeBtn.userData.payload = {
      deck_id: deckBuilder.deck_id,
      card_id: builderSelectedId,
    };
    removeBtn.userData.mode = 'builder';
    this.scene.add(removeBtn);
    this.buttons.push(removeBtn as any);
  }

  // ─── Deck Select Buttons ─────────────────────────────────────────────────

  private buildDeckSelectButtons(ctx: ButtonContext): void {
    const {
      selectedDeckId,
      viewer,
      gameState,
      deckSelectLibrary,
      selectSelectedIndex,
      ringCards,
    } = ctx;

    this.buildDeckSelectHud(ctx);

    const deck =
      selectSelectedIndex >= 0
        ? ringCards[selectSelectedIndex]
        : deckSelectLibrary[0];
    const labelColor = deck?.colors?.[0] ?? '#22d3ee';

    // Deck name billboard stays in 3D, positioned relative to the selected card.
    if (deck) {
      const nameBillboard = this.utils.createBillboardLabel(
        deck.name ?? 'Select Deck',
        null,
        labelColor,
      );
      if (nameBillboard) {
        nameBillboard.position.set(1.2, 0.2, 0);
        this.addLabel(nameBillboard as any);
      }
    }

    // Silence unused
    void selectedDeckId;
    void gameState;
    void viewer;
  }

  private buildDeckSelectHud(ctx: ButtonContext): void {
    const darkBg = '#09090b';
    const cyan = '#22d3ee';
    const purple = '#a855f7';
    const slate = '#e2e8f0';
    const green = '#22c55e';
    const orange = '#f97316';

    const topBarH = 0.65;
    const bottomBarH = 0.65;

    const bars: HudBar[] = [
      {
        id: 'select-top-bar',
        anchor: 'top-center',
        width: 'full',
        height: topBarH,
        color: darkBg,
        opacity: 0.95,
      },
      {
        id: 'select-top-accent',
        anchor: 'top-center',
        width: 'full',
        height: 0.02,
        color: cyan,
        offsetY: -topBarH,
        opacity: 1.0,
      },
    ];

    const elements: HudElement[] = [];

    // Format toggle — top-center (host only).
    if (ctx.viewer?.is_host) {
      const currentFormat = ctx.gameState?.format;
      elements.push({
        id: 'select-format-classic',
        type: 'text-button',
        label: 'Classic',
        actionId: 'games.cardgame.set_format',
        color: currentFormat === 'classic' ? cyan : slate,
        active: currentFormat === 'classic',
        anchor: 'top-center',
        offsetX: -1.1,
        offsetY: -topBarH / 2,
        options: { width: 1.8, height: 0.6, fontSize: 128 },
        payload: { format: 'classic' },
      });
      elements.push({
        id: 'select-format-operator',
        type: 'text-button',
        label: 'Operator',
        actionId: 'games.cardgame.set_format',
        color: currentFormat === 'operator' ? purple : slate,
        active: currentFormat === 'operator',
        anchor: 'top-center',
        offsetX: 1.1,
        offsetY: -topBarH / 2,
        options: { width: 1.8, height: 0.6, fontSize: 128 },
        payload: { format: 'operator' },
      });
    }

    // Action buttons — bottom bar.
    const selectedDeckId = ctx.selectedDeckId;
    const isSpecial =
      selectedDeckId === '__create__' || selectedDeckId === '__collection__';

    if (selectedDeckId) {
      bars.push({
        id: 'select-bottom-bar',
        anchor: 'bottom-center',
        width: 'full',
        height: bottomBarH,
        color: darkBg,
        opacity: 0.95,
      });
      bars.push({
        id: 'select-bottom-accent',
        anchor: 'bottom-center',
        width: 'full',
        height: 0.02,
        color: cyan,
        offsetY: bottomBarH,
        opacity: 1.0,
      });

      if (isSpecial) {
        const action =
          selectedDeckId === '__create__'
            ? 'games.cardgame.builder_create_deck'
            : 'games.cardgame.open_card_browser';
        const label =
          selectedDeckId === '__create__' ? 'Create' : 'Browse';
        const color = selectedDeckId === '__create__' ? green : purple;
        elements.push({
          id: 'select-go',
          type: 'text-button',
          label,
          actionId: action,
          color,
          active: true,
          anchor: 'bottom-center',
          offsetX: 0,
          offsetY: bottomBarH / 2,
          options: { width: 2.2, height: 0.6, fontSize: 128 },
        });
      } else {
        elements.push({
          id: 'select-choose',
          type: 'text-button',
          label: 'Choose',
          actionId: 'games.cardgame.select_deck',
          color: cyan,
          active: true,
          anchor: 'bottom-center',
          offsetX: -1.4,
          offsetY: bottomBarH / 2,
          options: { width: 2.2, height: 0.6, fontSize: 128 },
          payload: { deck_id: selectedDeckId },
        });
        elements.push({
          id: 'select-customize',
          type: 'text-button',
          label: 'Customize',
          actionId: 'games.cardgame.builder_load_deck',
          color: orange,
          active: true,
          anchor: 'bottom-center',
          offsetX: 1.4,
          offsetY: bottomBarH / 2,
          options: { width: 2.2, height: 0.6, fontSize: 128 },
          payload: { deck_id: selectedDeckId },
        });
      }
    }

    this.hudManager?.setConfig({ bars, elements });
  }

  // ─── Lobby Buttons ────────────────────────────────────────────────────────

  private buildLobbyButtons(ctx: ButtonContext): void {
    const { viewer, gameState } = ctx;

    const amber = '#f59e0b';
    const deepPurple = '#7c3aed';
    const green = '#22c55e';
    const slate = '#64748b';
    const orange = '#ea580c';
    const darkBg = '#09090b';

    // ── Not joined: single Join Lobby button via HUD ──
    if (!viewer) {
      this.hudManager?.setConfig({
        bars: [
          {
            id: 'bottom-bar',
            anchor: 'bottom-center',
            width: 'full',
            height: 0.8,
            color: darkBg,
            opacity: 0.95,
          },
          {
            id: 'bottom-accent',
            anchor: 'bottom-center',
            width: 'full',
            height: 0.02,
            color: amber,
            offsetY: 0.8,
            opacity: 1.0,
          },
        ],
        elements: [
          {
            id: 'join',
            type: 'button',
            label: 'JOIN LOBBY',
            actionId: 'games.cardgame.join',
            color: amber,
            anchor: 'bottom-center',
            offsetY: 0.28,
            options: { width: 3.6, height: 0.7, fontSize: 56 },
          },
        ],
      });
      return;
    }

    const isHost = viewer?.is_host;
    const currentFormat = gameState?.format ?? 'classic';
    const currentMode = gameState?.mode ?? 'competitive';
    const currentMatchFormat = gameState?.match_state?.format ?? 'bo1';
    const players: any[] = gameState?.players ?? [];

    // ── Build HUD config for screen-anchored elements ──
    // Note: elements are scaled by HUD_SCALE (0.4), so offsets are in world units at HUD distance
    const hudElements: HudElement[] = [];

    // Top bar - Type toggles (left)
    hudElements.push({
      id: 'constructed',
      type: 'text-button',
      label: 'CONSTRUCTED',
      actionId: 'internal.ui.lobby_type_constructed',
      color: this.lobbyType === 'constructed' ? green : slate,
      active: this.lobbyType === 'constructed',
      anchor: 'top-center',
      offsetX: -2.0,
      offsetY: -0.18,
      options: { fontSize: 26 },
    });
    hudElements.push({
      id: 'draft',
      type: 'text-button-icon',
      label: 'DRAFT',
      iconText: '\u2661 30',
      actionId: 'internal.ui.lobby_type_draft',
      color: this.lobbyType === 'draft' ? green : slate,
      active: this.lobbyType === 'draft',
      anchor: 'top-center',
      offsetX: -0.9,
      offsetY: -0.18,
      options: { fontSize: 26 },
    });

    // Top bar - Format toggles (center)
    hudElements.push({
      id: 'classic',
      type: 'text-button',
      label: 'CLASSIC',
      actionId: isHost ? 'games.cardgame.set_format' : '',
      color: currentFormat === 'classic' ? amber : slate,
      active: currentFormat === 'classic',
      anchor: 'top-center',
      offsetX: 0.1,
      offsetY: -0.18,
      payload: { format: 'classic' },
      interactive: isHost,
      options: { fontSize: 26 },
    });
    hudElements.push({
      id: 'operator',
      type: 'text-button',
      label: 'OPERATOR',
      actionId: isHost ? 'games.cardgame.set_format' : '',
      color: currentFormat === 'operator' ? deepPurple : slate,
      active: currentFormat === 'operator',
      anchor: 'top-center',
      offsetX: 1.0,
      offsetY: -0.18,
      payload: { format: 'operator' },
      interactive: isHost,
      options: { fontSize: 26 },
    });

    // Format deck-size hint — shows required deck size under the active format toggle
    hudElements.push({
      id: 'format-hint',
      type: 'label',
      label: currentFormat === 'operator'
        ? '67-card singleton + 1 Apex Operator'
        : '34-card deck, max 3 copies',
      actionId: '',
      color: slate,
      anchor: 'top-center',
      offsetX: 0.55,
      offsetY: -0.32,
      options: { fontSize: 18 },
    });

    // Top bar - Mode toggles (right)
    hudElements.push({
      id: 'competitive',
      type: 'text-button',
      label: 'COMPETITIVE',
      actionId: isHost ? 'games.cardgame.set_mode' : '',
      color: currentMode === 'competitive' ? orange : slate,
      active: currentMode === 'competitive',
      anchor: 'top-center',
      offsetX: 2.0,
      offsetY: -0.18,
      payload: { mode: 'competitive' },
      interactive: isHost,
      options: { fontSize: 26 },
    });
    hudElements.push({
      id: 'casual',
      type: 'text-button',
      label: 'CASUAL',
      actionId: isHost ? 'games.cardgame.set_mode' : '',
      color: currentMode === 'casual' ? orange : slate,
      active: currentMode === 'casual',
      anchor: 'top-center',
      offsetX: 2.9,
      offsetY: -0.18,
      payload: { mode: 'casual' },
      interactive: isHost,
      options: { fontSize: 26 },
    });

    // Top bar - Best-of format (BO1 / BO3)
    hudElements.push({
      id: 'bo1',
      type: 'text-button',
      label: 'BO1',
      actionId: isHost ? 'games.cardgame.set_match_format' : '',
      color: currentMatchFormat === 'bo1' ? amber : slate,
      active: currentMatchFormat === 'bo1',
      anchor: 'top-center',
      offsetX: 3.9,
      offsetY: -0.18,
      payload: { format: 'bo1' },
      interactive: isHost,
      options: { fontSize: 26 },
    });
    hudElements.push({
      id: 'bo3',
      type: 'text-button',
      label: 'BO3',
      actionId: isHost ? 'games.cardgame.set_match_format' : '',
      color: currentMatchFormat === 'bo3' ? amber : slate,
      active: currentMatchFormat === 'bo3',
      anchor: 'top-center',
      offsetX: 4.6,
      offsetY: -0.18,
      payload: { format: 'bo3' },
      interactive: isHost,
      options: { fontSize: 26 },
    });

    // Bottom bar - Utility buttons (left)
    hudElements.push({
      id: 'browse',
      type: 'text-button',
      label: 'BROWSE',
      actionId: 'games.cardgame.open_card_browser',
      color: deepPurple,
      active: false,
      anchor: 'bottom-center',
      offsetX: -1.8,
      offsetY: 0.28,
      options: { fontSize: 28 },
    });
    hudElements.push({
      id: 'packs',
      type: 'text-button',
      label: 'PACKS',
      actionId: 'internal.ui.toggle_packs',
      color: amber,
      active: false,
      anchor: 'bottom-center',
      offsetX: -0.9,
      offsetY: 0.28,
      options: { fontSize: 28 },
    });

    // Bottom bar - Action buttons (right side)
    hudElements.push({
      id: 'deck',
      type: 'button',
      label: 'SELECT DECK',
      actionId: 'internal.ui.open_deck_selector',
      color: amber,
      anchor: 'bottom-center',
      offsetX: 0.5,
      offsetY: 0.28,
      options: { width: 3.2, height: 0.7, fontSize: 46 },
    });

    if (isHost) {
      hudElements.push({
        id: 'add-ai',
        type: 'button',
        label: 'ADD AI',
        actionId: 'games.cardgame.add_ai',
        color: deepPurple,
        anchor: 'bottom-center',
        offsetX: 2.2,
        offsetY: 0.28,
        options: { width: 2.7, height: 0.7, fontSize: 42 },
      });

      if (players.length >= 2) {
        const startAction =
          this.lobbyType === 'draft'
            ? 'games.cardgame.draft_start'
            : 'games.cardgame.start';
        const startLabel =
          this.lobbyType === 'draft' ? 'START DRAFT' : 'START GAME';
        hudElements.push({
          id: 'start',
          type: 'button',
          label: startLabel,
          actionId: startAction,
          color: green,
          anchor: 'bottom-center',
          offsetX: 4.0,
          offsetY: 0.28,
          options: { width: 3.2, height: 0.7, fontSize: 46 },
        });
      } else {
        hudElements.push({
          id: 'debug',
          type: 'button',
          label: 'DEBUG',
          actionId: 'games.cardgame.debug_setup',
          color: deepPurple,
          anchor: 'bottom-center',
          offsetX: 4.0,
          offsetY: 0.28,
          options: { width: 3.2, height: 0.7, fontSize: 46 },
        });
      }
    }

    const hudBars: HudBar[] = [
      {
        id: 'top-bar',
        anchor: 'top-center',
        width: 'full',
        height: 0.6,
        color: darkBg,
        opacity: 0.95,
      },
      {
        id: 'top-accent',
        anchor: 'top-center',
        width: 'full',
        height: 0.02,
        color: amber,
        offsetY: -0.6,
        opacity: 1.0,
      },
      {
        id: 'bottom-bar',
        anchor: 'bottom-center',
        width: 'full',
        height: 0.8,
        color: darkBg,
        opacity: 0.95,
      },
      {
        id: 'bottom-accent',
        anchor: 'bottom-center',
        width: 'full',
        height: 0.02,
        color: green,
        offsetY: 0.8,
        opacity: 1.0,
      },
    ];

    this.hudManager?.setConfig({ bars: hudBars, elements: hudElements });

    // ── Player Seat Cards (world-space billboard sprites) ──
    const seatPositions = this.getPlayerSeatPositions(players.length);
    for (let i = 0; i < players.length; i++) {
      const player = players[i];
      const pos = seatPositions[i];
      const isReady = !!player.deck_name;
      const seatCard = this.utils.createPlayerSeatCard(
        (player.name || 'Player').toUpperCase(),
        player.deck_name
          ? `${player.deck_name.toUpperCase()}${player.is_ai ? ' • AI' : ''}`
          : player.is_ai
            ? 'AI SELECTING DECK...'
            : 'SELECTING DECK...',
        isReady,
        { color: player.id === viewer?.id ? amber : slate },
      );
      if (seatCard) {
        // Sprites float above the table — position in world space
        seatCard.position.set(pos.x, 1.2, pos.z);
        seatCard.scale.setScalar(1.2);
        seatCard.userData.mode = 'lobby';
        this.scene.add(seatCard);
        this.labels.push(seatCard as any);
      }
    }
  }

  private getPlayerSeatPositions(count: number): { x: number; z: number }[] {
    if (count <= 1) return [{ x: 0, z: -1.0 }];
    if (count === 2)
      return [
        { x: -2.0, z: -1.0 },
        { x: 2.0, z: -1.0 },
      ];
    if (count === 3)
      return [
        { x: -3.0, z: -1.0 },
        { x: 0, z: -1.0 },
        { x: 3.0, z: -1.0 },
      ];
    return [
      { x: -4.5, z: -1.0 },
      { x: -1.5, z: -1.0 },
      { x: 1.5, z: -1.0 },
      { x: 4.5, z: -1.0 },
    ];
  }

  // ─── Internal UI Action Handling ──────────────────────────────────────────

  public handleInternalAction(actionId: string): boolean {
    if (actionId === 'internal.ui.lobby_type_constructed') {
      this.lobbyType = 'constructed';
      this.currentKey = ''; // Force rebuild
      return true;
    }
    if (actionId === 'internal.ui.lobby_type_draft') {
      this.lobbyType = 'draft';
      this.currentKey = ''; // Force rebuild
      return true;
    }
    return false;
  }
}

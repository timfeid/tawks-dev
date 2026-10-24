import * as THREE from 'three';
import type { ZoneManager } from './ZoneManager';
import type { TableManager } from './TableManager';
import type { BattlefieldZone } from '../zones/BattlefieldZone';
import type { HandZone } from '../zones/HandZone';
import type { OperatorZone } from '../zones/OperatorZone';
import type { SelectionZone } from '../zones/SelectionZone';
import type { LibraryRingZone } from '../zones/LibraryRingZone';
import type { PackOpeningZone } from '../zones/PackOpeningZone';
import type { DangerDropZone } from '../zones/DangerDropZone';
import type { DiscardDropZone } from '../zones/DiscardDropZone';
import type { DraftPyramidZone } from '../zones/DraftPyramidZone';
import type { DraftSideZone } from '../zones/DraftSideZone';
import type { CardView, PlayerState } from '../types';

export interface SyncContext {
  zoneManager: ZoneManager;
  tableManager: TableManager;
  cameraPosition: THREE.Vector3;

  // Drop zones (registered separately)
  dangerDropZone: DangerDropZone | null;
  discardDropZone: DiscardDropZone | null;
  // Sacrifice is handled via the action menu — drop zone removed.
  sacrificeDropZone: null;

  // Game state
  players: PlayerState[];
  viewer: any;
  hand: CardView[];
  gameState: any;

  // Mode flags
  showPacks: boolean;
  showGameElements: boolean;
  deckBuilderMode: boolean;
  cardBrowserMode: boolean;
  deckSelectMode: boolean;
  awardsMode: boolean;
  operatorSelectMode: boolean;
  searchActive: boolean;
  searchCommitted: boolean;

  // Scry state
  scry: CardView[];
  scryPool: CardView[];
  scryTop: CardView[];
  scryBottom: CardView[];

  // Ring state
  ringCards: CardView[];
  selectedDeckId: string | null;
  builderSelectedId: string | null;

  // Deck builder
  deckBuilderTableCards: CardView[];
  filteredTableCards: CardView[];

  // Pack state
  packInventoryCards: CardView[];
  openingPackCard: any | null;
  packResultCards: CardView[];
  packPhase: string;
  revealedPackIndices: Set<number>;
  packOpeningShaking: boolean;
  packOpeningOpeningId: string | null;
  packTearTypeId: string | null;
  packTearProgress: number;
  packFlyInCount: number;
  packFocusIndex: number;

  // Draft state
  isDraftMode: boolean;
  draftView: any;

  // Drag/drop state
  isDragging: boolean;
  isDraggingFromDeck: boolean;
  draggingFromZone: string | null;
  pendingDiscard: any;
  discardSelected: Set<string>;
  discardCommitted: boolean;
  pendingSacrifice: any;
  isHoveringDanger: boolean;
  isHoveringDiscard: boolean;
  isHoveringSacrifice: boolean;
}

export interface SyncResult {
  searchCameraTarget: THREE.Vector3 | null;
  searchCameraLook: THREE.Vector3 | null;
  scryCameraTarget: THREE.Vector3 | null;
  scryCameraLook: THREE.Vector3 | null;
  packCameraTarget: THREE.Vector3 | null;
  packCameraLook: THREE.Vector3 | null;
}

export class ZoneSyncController {
  private lastBFKey = '';
  private lastAllCardsKey = '';

  // Spotlight hold: when a card leaves the stack (resolves), keep it visible
  // for a short time so the animation can complete. Keyed by original card id.
  private spotlightHoldUntil = new Map<string, number>();
  private spotlightCardCache = new Map<string, any>();
  // Last computed spotlight card list (live + held). Shared between
  // syncSpotlight (which tells the SpotlightZone which ids to lay out) and
  // syncAllActors (which spawns/destroys the underlying 3D CardActors).
  private lastSpotlightCards: any[] = [];
  // Earliest expiry timestamp among *held* (not live) spotlight cards, or
  // null if none are held. Used by the caller to schedule a re-sync.
  private nextSpotlightHeldExpiry: number | null = null;
  private static readonly SPOTLIGHT_HOLD_MS = 5000;

  public getNextSpotlightHoldExpiry(): number | null {
    return this.nextSpotlightHeldExpiry;
  }

  private makeDraftCardKey(
    scope: 'pyramid' | 'drafted',
    index: number,
    packsCompleted: number,
  ): string {
    return `draft-${scope}-${packsCompleted}-${index}`;
  }

  public sync(ctx: SyncContext): SyncResult {
    const result: SyncResult = {
      searchCameraTarget: null,
      searchCameraLook: null,
      scryCameraTarget: null,
      scryCameraLook: null,
      packCameraTarget: null,
      packCameraLook: null,
    };

    const { zoneManager } = ctx;
    if (!zoneManager) return result;

    this.syncBattlefield(ctx);
    this.syncTable(ctx);
    this.syncHand(ctx);
    this.syncSpotlight(ctx);
    this.syncOperatorZone(ctx);
    this.syncSearch(ctx, result);
    this.syncScry(ctx, result);
    this.syncLibraryRing(ctx);
    this.syncDeckRings(ctx);
    this.syncPackOpening(ctx, result);
    this.syncDraftPyramid(ctx);
    this.syncDropZones(ctx);
    this.syncAllActors(ctx);

    return result;
  }

  // ─── Battlefield ─────────────────────────────────────────────────────────

  private syncBattlefield(ctx: SyncContext): void {
    const bfZone = ctx.zoneManager.get('battlefield') as
      | BattlefieldZone
      | undefined;
    if (!bfZone || !ctx.players) return;

    const attackerIds = (ctx.gameState?.combat_attackers || []).map(
      (c: any) => c.id,
    );
    const combatPairs = ctx.gameState?.combat_pairs || [];

    bfZone.setGameElementsVisible(ctx.showGameElements && !ctx.showPacks);
    bfZone.setFlatLayout(ctx.deckBuilderMode || ctx.awardsMode || ctx.searchActive);

    const manaKey = (mp: any) =>
      mp
        ? `${mp.neon || 0},${mp.glitch || 0},${mp.chrome || 0},${mp.slag || 0},${mp.bio || 0},${mp.colorless || 0}`
        : '0';
    const combatKey = combatPairs
      .map(
        (p: any) =>
          `${p.attacker?.id}>${p.target_player_id || ''}:${p.is_blocked ? 'b' : 'u'}:${p.attacker_dies ? 'd' : 'l'}:${(p.blockers || []).map((b: any, i: number) => `${b.id}${p.blocker_dies?.[i] ? 'd' : 'l'}`).join('+')}`,
      )
      .join(',');
    const bfKey = `${ctx.players.length}|${ctx.viewer?.id}|${attackerIds.join(',')}|${combatKey}|${ctx.players
      .map(
        (p) =>
          `${p.id}:${p.life}:h${p.heat ?? 0}:${manaKey(p.mana_pool)}:${(p.battlefield_units || []).map((c) => `${c.id}:${c.tapped ? 't' : 'u'}`).join(',')}:${(p.battlefield_infrastructure || []).map((c) => `${c.id}:${c.tapped ? 't' : 'u'}`).join(',')}:g${(p.grid || []).length}`,
      )
      .join('|')}`;

    if (bfKey !== this.lastBFKey && !ctx.showPacks) {
      this.lastBFKey = bfKey;
      bfZone.updateState(ctx.players, ctx.viewer?.id, attackerIds, combatPairs);
    } else if (ctx.showPacks) {
      bfZone.updateState([], null, [], []);
    }
  }

  // ─── Table ────────────────────────────────────────────────────────────────

  private syncTable(ctx: SyncContext): void {
    if (!ctx.tableManager) return;
    ctx.tableManager.setReflectionOpacity(
      ctx.deckBuilderMode || ctx.awardsMode || ctx.searchActive ? 0 : 1,
    );
    ctx.tableManager.setGlow(Boolean(ctx.viewer?.is_active));
  }

  // ─── Hand ────────────────────────────────────────────────────────────────

  private syncHand(ctx: SyncContext): void {
    const handZone = ctx.zoneManager.get('hand') as HandZone | undefined;
    if (!handZone) return;

    // During a forced discard, the only valid action is discard — strip the
    // "playable" glow so the player isn't confused into thinking they can cast.
    const discardMode = ctx.pendingDiscard && !ctx.discardCommitted;
    const handData = ctx.showPacks
      ? []
      : discardMode
        ? ctx.hand.map((c) => ({
            ...c,
            playable: false,
            can_attack: false,
            can_tap: false,
          }))
        : ctx.hand;
    handZone.update(handData);

    for (const actor of ctx.zoneManager.getAllActors()) {
      const inHand = actor.getZone() === 'hand';
      const selected =
        discardMode &&
        !ctx.showPacks &&
        inHand &&
        ctx.discardSelected.has(actor.id);
      // Only the cards actually chosen for discard get the purple scan +
      // bright border so the player can read which are queued; everything
      // else stays as its normal card border.
      if ((actor as any).setDiscardHighlight) {
        (actor as any).setDiscardHighlight(selected);
      }
      if ((actor as any).setDiscardSelected) {
        (actor as any).setDiscardSelected(selected);
      }
      if (selected) actor.setTargetHint(false);
    }
  }

  // ─── Spotlight (stack / showcase) ────────────────────────────────────────

  private syncSpotlight(ctx: SyncContext): void {
    const spotlightZone = ctx.zoneManager.get('spotlight') as
      | { update: (cards: CardView[]) => void }
      | undefined;
    if (!spotlightZone) return;

    const now = performance.now();
    const items: any[] = (ctx.gameState?.stack || []) as any[];

    // Cards already present on any battlefield / operator zone. Once a
    // resolved card has reached its final home, we stop holding it in the
    // spotlight — the battlefield actor takes over (and spawns from the
    // spotlight's last position for a smooth hand-off).
    const settledCardIds = new Set<string>();
    for (const p of ctx.players || []) {
      for (const c of p.battlefield_units || []) settledCardIds.add(c.id);
      for (const c of p.battlefield_infrastructure || []) settledCardIds.add(c.id);
      for (const c of p.operator_zone || []) settledCardIds.add(c.id);
    }

    // Track which cards are currently on the stack — their hold timer keeps
    // refreshing while they're there. Anything that *was* in the hold map
    // but isn't in the stack anymore gets to stay visible until its hold
    // timestamp expires OR it resolves onto the battlefield.
    //
    // Skip spells/abilities that have a target: those are visualised as a
    // projectile beam from caster → target instead of a big floating card.
    const liveCards = new Map<string, { card: any; index: number }>();
    items.forEach((item, index) => {
      const card = (item && item.card) || item;
      if (!card || !card.id) return;
      const targets: any[] = (item && item.targets) ?? [];
      const kind = String(card.kind_tag || '').toLowerCase();
      const isSummon = kind === 'unit' || kind === 'agent';
      if (targets.length > 0 && !isSummon) return;
      liveCards.set(card.id, { card, index });
      this.spotlightHoldUntil.set(
        card.id,
        now + ZoneSyncController.SPOTLIGHT_HOLD_MS,
      );
    });

    // Held-over resolved cards: in the map but no longer on the stack.
    // They appear after the live stack items, in insertion order.
    const heldEntries: Array<{ id: string; until: number }> = [];
    let earliestHeld: number | null = null;
    for (const [id, until] of this.spotlightHoldUntil) {
      if (liveCards.has(id)) continue;
      // Card has landed on the battlefield — hand off and drop the hold.
      if (settledCardIds.has(id)) {
        this.spotlightHoldUntil.delete(id);
        this.spotlightCardCache.delete(id);
        continue;
      }
      if (until <= now) {
        this.spotlightHoldUntil.delete(id);
        this.spotlightCardCache.delete(id);
        continue;
      }
      heldEntries.push({ id, until });
      if (earliestHeld === null || until < earliestHeld) earliestHeld = until;
    }
    this.nextSpotlightHeldExpiry = earliestHeld;

    // Build the spotlight card list: live stack items first, then held.
    // Use a STABLE id (no index) so the actor persists when a card
    // transitions from "live" to "held" — otherwise the 3D actor would be
    // destroyed and respawned, losing animation continuity.
    const cards: any[] = [];
    for (const { card } of liveCards.values()) {
      cards.push({ ...card, id: `spotlight-${card.id}`, _zone: 'spotlight' });
      this.spotlightCardCache.set(card.id, card);
    }
    for (const { id } of heldEntries) {
      const card = this.spotlightCardCache.get(id);
      if (!card) continue;
      cards.push({ ...card, id: `spotlight-${card.id}`, _zone: 'spotlight' });
    }

    this.lastSpotlightCards = cards;
    spotlightZone.update(cards as CardView[]);
  }

  // ─── Operator Zone ───────────────────────────────────────────────────────

  private syncOperatorZone(ctx: SyncContext): void {
    const opZone = ctx.zoneManager.get('operator') as OperatorZone | undefined;
    if (!opZone || !ctx.players) return;
    opZone.updateState(ctx.showPacks ? [] : ctx.players);
  }

  // ─── Search Grid ──────────────────────────────────────────────────────────

  private syncSearch(ctx: SyncContext, result: SyncResult): void {
    const sGridZone = ctx.zoneManager.get('search_grid') as
      | LibraryRingZone
      | undefined;
    if (!sGridZone) return;

    if (
      ctx.searchActive &&
      !ctx.searchCommitted &&
      ctx.scryPool.length > 0 &&
      !ctx.showPacks
    ) {
      sGridZone.setLayoutConfig({
        layoutMode: 'grid',
        gridColumns: 10,
        lineSpacing: 0.9,
        lineY: 0.03,
        scale: 0.55,
        flat: true,
      });
      sGridZone.update(ctx.scryPool);
      result.searchCameraTarget = new THREE.Vector3(0, 8, 0.01);
      result.searchCameraLook = new THREE.Vector3(0, 0, 0);
    } else {
      sGridZone.update([]);
    }
  }

  // ─── Scry / Selection ────────────────────────────────────────────────────

  private syncScry(ctx: SyncContext, result: SyncResult): void {
    const scryZone = ctx.zoneManager.get('scry') as SelectionZone | undefined;
    if (!scryZone) return;

    if (ctx.scry.length > 0 && !ctx.searchActive && !ctx.showPacks) {
      scryZone.setScryState(ctx.scryPool, ctx.scryTop, ctx.scryBottom);

      const viewerIndex = ctx.players.findIndex((p) => p.id === ctx.viewer?.id);
      if (viewerIndex >= 0) {
        const seatAngle = (viewerIndex / ctx.players.length) * Math.PI * 2;
        const focusPoint = new THREE.Vector3(0, 2, 8).applyEuler(
          new THREE.Euler(0, seatAngle, 0),
        );
        const camPos = new THREE.Vector3(0, 7, 13).applyEuler(
          new THREE.Euler(0, seatAngle, 0),
        );
        result.scryCameraTarget = camPos;
        result.scryCameraLook = focusPoint;
        scryZone.setTransform(focusPoint, new THREE.Euler(-0.5, seatAngle, 0));
      }
    } else {
      scryZone.update([]);
    }
  }

  // ─── Library Ring ────────────────────────────────────────────────────────

  private syncLibraryRing(ctx: SyncContext): void {
    const ringZone = ctx.zoneManager.get('library_ring') as
      | LibraryRingZone
      | undefined;
    if (!ringZone) return;

    const showingRing =
      (ctx.deckBuilderMode || ctx.deckSelectMode || ctx.awardsMode) &&
      !ctx.showPacks;
    const selectedId = ctx.deckSelectMode
      ? ctx.selectedDeckId
      : ctx.awardsMode
        ? ctx.builderSelectedId
      : ctx.deckBuilderMode
        ? ctx.builderSelectedId
        : null;

    ringZone.update(showingRing ? ctx.ringCards : [], selectedId);

    if (!showingRing) return;

    if (ctx.deckSelectMode) {
      ringZone.setLayoutConfig({
        layoutMode: 'line',
        lineSpacing: -0.75,
        lineAxis: 'z',
        lineOffset: 0,
        lineCross: 0,
        lineRotation: 0,
        lineY: 0.03,
        scale: 0.5,
        selectedScale: 2.2,
        selectedLift: 2.5,
        lineDepthLift: 0.22,
        lineScaleByDistance: [1.0, 0.5, 0.32, 0.22, 0.16],
        pileCount: 1,
        pileSelectedCount: 1,
        flat: true,
        centerSelected: true,
        lineCurveDepth: 0,
      });
    } else if (ctx.operatorSelectMode) {
      ringZone.setLayoutConfig({
        layoutMode: 'line',
        lineSpacing: 0.55,
        lineAxis: 'z',
        lineOffset: 0,
        lineCross: 0,
        lineRotation: 0,
        lineY: 0.03,
        scale: 0.9,
        selectedScale: 1.35,
        selectedLift: 0.7,
        lineDepthLift: 0.18,
        lineScaleByDistance: [1.0, 0.88, 0.72, 0.56, 0.42],
        pileCount: -1,
        pileSelectedCount: -1,
        flat: true,
      });
    } else if (ctx.deckBuilderMode || ctx.awardsMode) {
      if (ctx.cardBrowserMode) {
        // Browser: thumbnail fan with a large, legible selected card at center.
        ringZone.setLayoutConfig({
          layoutMode: 'line',
          lineSpacing: -0.75,
          lineAxis: 'z',
          lineOffset: 0,
          // Shift the entire row toward +X so it sits lower on screen
          // (up = -X in browser camera).
          lineCross: 0.6,
          lineRotation: 0,
          lineY: 0.03,
          scale: 0.5,
          selectedScale: 2.2,
          selectedLift: 2.5,
          lineDepthLift: 0.22,
          // Immediate neighbors stay sizeable; falloff past that.
          lineScaleByDistance: [1.0, 0.5, 0.32, 0.22, 0.16],
          pileCount: -1,
          pileSelectedCount: -1,
          flat: true,
          centerSelected: true,
          lineCurveDepth: 0,
        });
      } else {
        ringZone.setLayoutConfig({
          layoutMode: 'line',
          lineSpacing: 0.35,
          lineAxis: 'z',
          lineOffset: 0,
          lineCross: 0,
          lineRotation: 0,
          lineY: 0.03,
          scale: ctx.awardsMode ? 0.88 : 0.8,
          selectedScale: 1.35,
          selectedLift: ctx.awardsMode ? 1.1 : 2,
          lineDepthLift: 0.22,
          lineScaleByDistance: [1.0, 0.88, 0.75, 0.6, 0.45],
          pileCount: -1,
          pileSelectedCount: -1,
          flat: true,
        });
      }
    }
  }

  // ─── Deck Rings ───────────────────────────────────────────────────────────

  private syncDeckRings(ctx: SyncContext): void {
    const showRings =
      ctx.deckBuilderMode && !ctx.operatorSelectMode && !ctx.showPacks;
    const leftCards = ctx.filteredTableCards.filter((_, i) => i % 2 === 0);
    const rightCards = ctx.filteredTableCards.filter((_, i) => i % 2 !== 0);

    const deckRingConfig = {
      layoutMode: 'ring' as const,
      radius: 4.9,
      totalArc: Math.PI * 0.5,
      scale: 0.55,
      selectedScale: 0.55,
      selectedLift: 0.0,
      rotationOffset: 0,
      pileCount: -1,
      pileSelectedCount: -1,
      flat: true,
      lineY: 0.12,
      rows: 2,
    };

    const dlZone = ctx.zoneManager.get('deck_ring_left') as
      | LibraryRingZone
      | undefined;
    if (dlZone) {
      dlZone.update(showRings ? leftCards : []);
      dlZone.setShowTrack(showRings);
      dlZone.setLayoutConfig({ ...deckRingConfig, startAngle: Math.PI * 0.25 });
    }

    const drZone = ctx.zoneManager.get('deck_ring_right') as
      | LibraryRingZone
      | undefined;
    if (drZone) {
      drZone.update(showRings ? rightCards : []);
      drZone.setShowTrack(showRings);
      drZone.setLayoutConfig({ ...deckRingConfig, startAngle: Math.PI * 1.25 });
    }
  }

  // ─── Pack Opening ─────────────────────────────────────────────────────────

  private syncPackOpening(ctx: SyncContext, result: SyncResult): void {
    const packZone = ctx.zoneManager.get('pack_opening') as
      | PackOpeningZone
      | undefined;
    if (!packZone) return;

    if (ctx.showPacks) {
      packZone.setPhase(ctx.packPhase as any);
      packZone.setPacks(ctx.packInventoryCards.map((c) => c.id));
      packZone.setCards(
        ctx.packResultCards.map((c) => c.id),
        ctx.revealedPackIndices,
      );
      packZone.setOpeningPack(
        ctx.openingPackCard?.id || null,
        ctx.packOpeningShaking,
      );
      packZone.setTearState(
        ctx.packTearTypeId ? `pack-type-${ctx.packTearTypeId}` : null,
        ctx.packTearProgress,
      );
      packZone.setFlyInCount(ctx.packFlyInCount);
      packZone.setFocusIndex(ctx.packFocusIndex);
      packZone.setCamAngle(
        Math.atan2(ctx.cameraPosition.z, ctx.cameraPosition.x),
      );

      const phase = ctx.packPhase;
      if (phase === 'inventory' || phase === 'grab' || phase === 'tear') {
        result.packCameraTarget = new THREE.Vector3(0, 6, 8);
        result.packCameraLook = new THREE.Vector3(0, 0, 0);
      } else if (phase === 'opening' || phase === 'burst') {
        result.packCameraTarget = new THREE.Vector3(0, 14, 18);
        result.packCameraLook = new THREE.Vector3(0, 10.0, 0);
      } else if (phase === 'summary') {
        result.packCameraTarget = new THREE.Vector3(0, 8, 0.01);
        result.packCameraLook = new THREE.Vector3(0, 0, 0);
      }
    } else {
      packZone.setPacks([]);
      packZone.setCards([], new Set());
      packZone.setOpeningPack(null, false);
      packZone.setTearState(null, 0);
    }
  }

  // ─── Draft Pyramid ───────────────────────────────────────────────────────

  private syncDraftPyramid(ctx: SyncContext): void {
    const draftZone = ctx.zoneManager.get('draft_pyramid') as
      | DraftPyramidZone
      | undefined;
    const sideZone = ctx.zoneManager.get('draft_side') as
      | DraftSideZone
      | undefined;

    if (!ctx.isDraftMode || !ctx.draftView) {
      // Hide draft zone objects when not in draft
      draftZone?.getSceneObjects().forEach((obj) => (obj.visible = false));
      sideZone?.getSceneObjects().forEach((obj) => (obj.visible = false));
      return;
    }

    draftZone?.getSceneObjects().forEach((obj) => (obj.visible = true));
    sideZone?.getSceneObjects().forEach((obj) => (obj.visible = true));

    const draft = ctx.draftView;
    const pyramidCards: any[] = draft.pyramid || [];
    const availableIndices: number[] = draft.available_indices || [];
    const packsCompleted = draft.packs_completed || 0;

    // Build card ID array for the current draft board, null for picked slots
    const cardIds: (string | null)[] = pyramidCards.map(
      (c: any, index: number) =>
        c ? this.makeDraftCardKey('pyramid', index, packsCompleted) : null,
    );

    if (draftZone) {
      draftZone.setPyramid(cardIds, availableIndices);
    }

    if (sideZone) {
      const draftedCards: any[] = draft.my_drafted_cards || [];
      sideZone.setDrafted(
        draftedCards.map((_: any, index: number) =>
          this.makeDraftCardKey('drafted', index, 0),
        ),
      );
    }
  }

  // ─── Drop Zones ───────────────────────────────────────────────────────────

  private syncDropZones(ctx: SyncContext): void {
    const { dangerDropZone, discardDropZone } = ctx;

    if (dangerDropZone) {
      dangerDropZone.setActive(
        ctx.deckBuilderMode &&
          !ctx.operatorSelectMode &&
          ctx.isDraggingFromDeck,
      );
      dangerDropZone.setHovered(ctx.isHoveringDanger);
      dangerDropZone.setPosition(1.4, 0);
    }

    if (discardDropZone) {
      // Discard is handled via the 3D confirm prompt + in-hand selection
      // highlight. The drag-to-zone drop target has been retired.
      discardDropZone.setActive(false);
      discardDropZone.setHovered(false);
    }

    // Sacrifice drop zone removed — use the action menu instead.
  }

  // ─── All Actors (card sync) ───────────────────────────────────────────────

  public syncAllActors(
    ctx: SyncContext,
    combatAnimator?: { saveActorPositions: () => void },
  ): void {
    const { zoneManager, gameState } = ctx;
    if (!zoneManager) return;

    const isLobby = gameState?.stage === 'lobby';
    if (
      isLobby &&
      !ctx.deckBuilderMode &&
      !ctx.deckSelectMode &&
      !ctx.awardsMode &&
      !ctx.showPacks &&
      !ctx.isDraftMode
    )
      return;

    const attackerIds = (gameState?.combat_attackers || []).map(
      (c: any) => c.id,
    );

    const syncCards = ctx.showPacks
      ? [
          ...ctx.packInventoryCards.map((c) => ({
            ...c,
            _zone: 'pack_inventory',
          })),
          ...(ctx.openingPackCard
            ? [{ ...ctx.openingPackCard, _zone: 'pack_opening' }]
            : []),
          ...ctx.packResultCards.map((c) => ({ ...c, _zone: 'pack_opening' })),
        ]
      : [
          // Draft cards
          ...(ctx.isDraftMode && ctx.draftView
            ? [
                ...(ctx.draftView.pyramid || [])
                  .map((c: any, index: number) =>
                    c == null
                      ? null
                      : {
                          ...c,
                          id: this.makeDraftCardKey(
                            'pyramid',
                            index,
                            ctx.draftView.packs_completed || 0,
                          ),
                          _zone: 'draft_pyramid',
                        },
                  )
                  .filter((c: any) => c != null),
                ...(ctx.draftView.my_drafted_cards || []).map(
                  (c: any, index: number) => ({
                    ...c,
                    id: this.makeDraftCardKey('drafted', index, 0),
                    _zone: 'draft_side',
                  }),
                ),
              ]
            : []),
          ...(ctx.hand || []).map((c) => ({ ...c, _zone: 'hand' })),
          ...(ctx.deckBuilderMode || ctx.deckSelectMode || ctx.awardsMode
            ? ctx.ringCards
            : []
          ).map((c) => ({ ...c, _zone: 'ring' })),
          ...(ctx.deckBuilderMode
            ? ctx.filteredTableCards.map((c) => ({ ...c, _zone: 'table' }))
            : []),
          ...(ctx.scryPool || []).map((c) => ({ ...c, _zone: 'scry' })),
          ...(ctx.players || []).flatMap((p) => [
            ...(p.battlefield_units || []).map((c) => ({
              ...c,
              _zone: 'bf_units',
              _owner: p.id,
            })),
            ...(p.battlefield_infrastructure || []).map((c) => ({
              ...c,
              _zone: 'bf_infra',
              _owner: p.id,
            })),
            ...(p.operator_zone || []).map((c) => ({
              ...c,
              _zone: 'operator',
              _owner: p.id,
            })),
          ]),
          // Spotlight cards — use the list computed by syncSpotlight so
          // both the SpotlightZone and the actor pool agree on which cards
          // are visible (including held-after-resolve cards).
          ...this.lastSpotlightCards,
        ];

    const bfCount = (ctx.players || []).reduce(
      (acc, p) =>
        acc +
        (p.battlefield_units?.length || 0) +
        (p.battlefield_infrastructure?.length || 0) +
        (p.operator_zone?.length || 0),
      0,
    );

    const currentKey = [
      gameState?.stage,
      (ctx.hand || []).length,
      bfCount,
      attackerIds.join(','),
      ctx.showPacks,
      ctx.packPhase,
      ctx.revealedPackIndices.size,
      syncCards
        .map(
          (c) =>
            c.id +
            (c.tapped ? 't' : 'u') +
            (c.playable ? 'P' : 'p') +
            (c.can_attack ? 'A' : '') +
            (c.can_block ? 'B' : '') +
            (c.can_tap ? 'T' : '') +
            (c.power ?? '') +
            (c.toughness ?? '') +
            (c.loyalty ?? '') +
            c.armor +
            c.shield +
            ((c as any).pile_count || 1) +
            ((c as any)._zone || '') +
            ((c as any)._owner || ''),
        )
        .join(','),
    ].join('|');

    if (currentKey !== this.lastAllCardsKey) {
      this.lastAllCardsKey = currentKey;

      // Save positions before sync for death animations
      combatAnimator?.saveActorPositions();

      // Spawn new battlefield/spotlight cards from their owner's seat so
      // they fly in along a meaningful arc instead of popping from origin.
      const bfZoneAny: any = zoneManager.get('battlefield');
      const playerActorById = new Map<string, any>();
      if (bfZoneAny && typeof bfZoneAny.getPlayerActors === 'function') {
        for (const pa of bfZoneAny.getPlayerActors() as any[]) {
          if (pa?.id) playerActorById.set(pa.id, pa);
        }
      }
      zoneManager.syncActors(syncCards as any, (card: any) => {
        const zone = card._zone;
        // Battlefield card just resolved from the stack — spawn it at the
        // spotlight actor's current position so the card visually continues
        // from where it was being showcased rather than popping from the
        // caster's seat.
        if (zone === 'bf_units' || zone === 'bf_infra' || zone === 'operator') {
          const spotlightActor = zoneManager.getActor(`spotlight-${card.id}`);
          if (spotlightActor && (spotlightActor as any).group?.position) {
            return (spotlightActor as any).group.position.clone();
          }
        }
        if (zone === 'spotlight' || zone === 'bf_units' || zone === 'bf_infra') {
          const owner = card._owner;
          const pa = owner ? playerActorById.get(owner) : null;
          if (pa?.group?.position) {
            const p = pa.group.position.clone();
            // Spawn high above the seat and slightly pulled toward table
            // center so the card arcs in rather than popping flat from
            // the player's hand area.
            p.y += 4.0;
            const toCenter = p.clone().multiplyScalar(-0.35);
            p.x += toCenter.x;
            p.z += toCenter.z;
            return p;
          }
        }
        return null;
      });

      const actors = zoneManager.getAllActors();
      syncCards.forEach((card) => {
        const actor = actors.find((a) => a.id === card.id);
        if (actor) {
          // Only show pile stacking on deck table cards in builder mode
          const isTableCard = (card as any)._zone === 'table';
          const count = isTableCard ? ((card as any).pile_count ?? 1) : 1;
          actor.setPileCount(count - 1);
          actor.setFlatLayout(
            ctx.deckBuilderMode || ctx.deckSelectMode || ctx.awardsMode,
          );
        }
      });

      zoneManager.updateLayouts();
    }
  }

  public resetKeys(): void {
    this.lastBFKey = '';
    this.lastAllCardsKey = '';
  }
}

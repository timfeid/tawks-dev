<script lang="ts">
  import { onMount, untrack, getContext } from "svelte";
  import * as THREE from "three";
  import type { WorkflowResource } from "@tawks/workflow-types";
  import { submitServerAction } from "@tawks/workflow-ui/components/workflow";
  import type { WorkflowStateProvider } from "@tawks/workflow-ui/workflow";

  // State Logic
  import {
    deriveHand,
    derivePlayers,
    deriveScry,
    deriveScryKey,
    deriveViewer,
  } from "./battlefield/state";

  import type { CardView, PlayerState } from "./battlefield/types";
  import type { CardAction } from "./battlefield/actions";

  // Engine
  import { SceneManager } from "./battlefield/engine/SceneManager";
  import { TableManager } from "./battlefield/engine/TableManager";
  import { PongManager } from "./battlefield/engine/PongManager";
  import { InputManager } from "./battlefield/engine/InputManager";
  import { ZoneManager } from "./battlefield/engine/ZoneManager";
  import { InteractionController, type ActionHint } from "./battlefield/engine/InteractionController";
  import { CameraController } from "./battlefield/engine/CameraController";
  import { ButtonManager } from "./battlefield/engine/ButtonManager";
  import { HudManager } from "./battlefield/engine/HudManager";
  import { InspectPanel } from "./battlefield/engine/InspectPanel";
  import { CombatAnimator } from "./battlefield/engine/CombatAnimator";
  import {
    PackStateMachine,
    type PackPhase,
    type PackStateUpdate,
  } from "./battlefield/engine/PackStateMachine";
  import { ZoneSyncController } from "./battlefield/engine/ZoneSyncController";
  import { KeyboardController } from "./battlefield/engine/KeyboardController";

  // Zones
  import { HandZone } from "./battlefield/zones/HandZone";
  import { BattlefieldZone } from "./battlefield/zones/BattlefieldZone";
  import { OperatorZone } from "./battlefield/zones/OperatorZone";
  import { SelectionZone } from "./battlefield/zones/SelectionZone";
  import { LibraryRingZone } from "./battlefield/zones/LibraryRingZone";
  import { PackOpeningZone } from "./battlefield/zones/PackOpeningZone";
  import { SpotlightZone } from "./battlefield/zones/SpotlightZone";
  import { DraftPyramidZone } from "./battlefield/zones/DraftPyramidZone";
  import { DraftSideZone } from "./battlefield/zones/DraftSideZone";
  import { DangerDropZone } from "./battlefield/zones/DangerDropZone";
  import { DiscardDropZone } from "./battlefield/zones/DiscardDropZone";
  import { createArcLabelMesh } from "./battlefield/environment";

  // Factories
  import {
    EMPTY_MANA,
    createPackInventoryCard,
    createOpeningPackCard,
    createPackResultCard,
    createDeckSelectCard,
    createDeployCard,
  } from "./battlefield/CardViewFactory";
  import { preloadImages, collectImageUrls } from "./battlefield/preloadImages";
  import {
    computeDeckBuilderLibrary,
    computeOperatorLibrary,
    computeDeckBuilderTableCards,
    computeDeckBuilderTotalCards,
    buildRingCards,
  } from "./battlefield/DeckBuilderState";

  // Overlays
  import GameHudOverlay from "./battlefield/overlays/GameHudOverlay.svelte";
  import TargetingOverlay from "./battlefield/overlays/TargetingOverlay.svelte";
  import ActionPanel from "./battlefield/overlays/ActionPanel.svelte";
  import ActionMenu from "./battlefield/overlays/ActionMenu.svelte";
  import HelpOverlay from "./battlefield/overlays/HelpOverlay.svelte";
  import DraftOverlay from "./battlefield/overlays/DraftOverlay.svelte";
  import AwardsOverlay from "./battlefield/overlays/AwardsOverlay.svelte";
  import ModifierTooltip from "./battlefield/overlays/ModifierTooltip.svelte";
  import InspectOverlay from "./battlefield/overlays/InspectOverlay.svelte";
  import SideboardOverlay from "./battlefield/overlays/SideboardOverlay.svelte";
  import MatchScoreBanner from "./battlefield/overlays/MatchScoreBanner.svelte";
  import DeckBuilderSideboardPanel from "./battlefield/overlays/DeckBuilderSideboardPanel.svelte";
  import InterGameBanner from "./battlefield/overlays/InterGameBanner.svelte";

  let { workflow }: { workflow: WorkflowResource } = $props();
  const provider = getContext<WorkflowStateProvider>("workflow-provider");

  function hashCardId(id: string): number {
    let h = 0;
    for (let i = 0; i < id.length; i += 1) {
      h = (Math.imul(31, h) + id.charCodeAt(i)) | 0;
    }
    return (h >>> 0) / 0xffffffff;
  }

  function fallbackVisualSpec(card: CardView) {
    const hash = hashCardId(card.id);
    return {
      seed: Math.round(hash * 1000),
      wear: 0.4 + hash * 0.6,
    };
  }

  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;

  // ─── UI State ──────────────────────────────────────────────────────────────
  let hoveredCard: CardView | null = $state(null);
  let hoveredActionHints: ActionHint[] | null = $state(null);
  let inspectedCard: CardView | null = $state(null);
  let inspectTilt = { x: -0.18, y: 0 }; // slight backward lean by default
  let targetingSource = $state<CardView | null>(null);
  let targetingMode = $state<"cast" | "attack" | "block" | "activate" | null>(
    null,
  );
  // Projected screen position of the targeting source card, updated each
  // animation frame so the targeting dialog can float just above the card
  // in world space.
  let targetingScreenPos = $state<{ x: number; y: number } | null>(null);
  // Brief error message shown when clicking an invalid target (e.g. "has Stealth").
  let targetingError = $state<string | null>(null);
  let targetingErrorTimer: ReturnType<typeof setTimeout>;
  let targetingAbilityId = $state<string | null>(null);
  let pointerPos = $state({ x: 0, y: 0 });
  let mousePixelPos = $state({ x: 0, y: 0 });
  let cameraHud = $state("");
  let fps = $state(0);
  let manualCamera = $state(false);
  let isFullscreen = $state(false);
  let showHelp = $state(false);
  let showPacks = $state(false);
  let preloadProgress = $state(0);
  let preloadTotal = $state(0);
  let preloadDone = $state(false);
  let highestSeenPingTimestamp = $state(0);
  let handHoverActive = $state(false);
  let cameraController: CameraController;
  let localDeckSelect = $state(false);
  let deckSelectDismissed = $state(false);
  // When true, library clicks in the deck builder add to the sideboard
  // zone instead of the main deck. Local UI state — not workflow state.
  let builderSideboardMode = $state(false);
  let highlightedCardId = $state<string | null>(null);
  let prevHighlightedId: string | null = null;

  // ─── Pack Opening State ────────────────────────────────────────────────────
  let packPhase = $state<PackPhase>("inventory");
  let openedPackResult = $state<any>(null);
  let revealedPackIndices = $state<Set<number>>(new Set());
  let packOpeningShaking = $state(false);
  let packOpeningOpeningId = $state<string | null>(null);
  let packOpeningTypeId = $state<string | null>(null);
  let packTearTypeId = $state<string | null>(null);
  let packTearProgress = $state(0);
  let flashRarity = $state<string | null>(null);
  let packBurstStartedAt = $state(0);
  let packFlyInCount = $state(0);
  let packFocusIndex = $state(0);
  let packRevealZoomed = $state(false);

  // ─── Game State ────────────────────────────────────────────────────────────
  let gameState = $derived(workflow?.responses?.game as any);

  // Cards revealed by effects (Chrome Cipher etc.) — shown briefly in the
  // center with a "X reveals ..." dialog so everyone sees what came up.
  let revealedCards = $derived(
    (gameState?.revealed_cards || []) as Array<{
      card: any;
      owner_id: string;
      owner_name: string;
      outcome_label: string;
    }>,
  );

  let _prevStackLength = $state(0);
  let _autoResolveTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const stack = gameState?.stack || [];
    const len = stack.length;
    if (len > untrack(() => _prevStackLength)) {
      // Something was added to the stack - play an animation on the table!
      tableManager?.addBurst(0, 0, 0xf59e0b, 0.5);
      tableManager?.addBurst(0, 0, 0x10b981, 0.8);
      // Optional: shake screen or add a second burst
      setTimeout(() => tableManager?.addBurst((Math.random()-0.5)*2, (Math.random()-0.5)*2, 0x0ea5e9, 0.2), 200);
      setTimeout(() => tableManager?.addBurst((Math.random()-0.5)*2, (Math.random()-0.5)*2, 0xf43f5e, 0.2), 400);
    }
    _prevStackLength = len;

    // Auto-pass priority on our own spells so we don't have to click "PASS"
    // (the opponent gets the response window; we just yield).
    if (len > 0 && workflow?.instance_id && viewer?.id) {
      const top = stack[0];
      if (top.owner_id === viewer.id) {
        if (!_autoResolveTimer) {
          _autoResolveTimer = setTimeout(() => {
            submitAction("games.cardgame.pass_priority", {});
            _autoResolveTimer = null;
          }, 1200);
        }
      }
    } else {
      if (_autoResolveTimer) {
        clearTimeout(_autoResolveTimer);
        _autoResolveTimer = null;
      }
    }
  });

  let deckBuilder = $derived(gameState?.deck_builder ?? null);
  let browserTitle = $derived(deckBuilder?.browser_title ?? "Card Collection");
  let cardBrowserMode = $derived(
    workflow?.current_node_id === "card_browser" ||
      deckBuilder?.browser_active === true,
  );
  let deckBuilderMode = $derived(
    !cardBrowserMode &&
      (workflow?.current_node_id === "deck_builder" ||
        deckBuilder?.active === true),
  );
  let isOperatorDeck = $derived(deckBuilder?.format === "operator_100");
  let localOperatorSelect = $state(false);
  let operatorSelectMode = $derived(deckBuilderMode && localOperatorSelect);
  let deckSelectMode = $derived(
    !showPacks &&
      !deckBuilderMode &&
      !cardBrowserMode &&
      gameState?.stage === "lobby" &&
      !!gameState?.viewer &&
      (localDeckSelect ||
        (!gameState?.viewer?.deck_id && !deckSelectDismissed)),
  );
  // Reset deckSelectDismissed once server confirms deck_id, so future clears re-open deck select
  $effect(() => {
    if (gameState?.viewer?.deck_id) deckSelectDismissed = false;
  });
  let isLobbyMode = $derived(
    gameState?.stage === "lobby" ||
      (gameState?.stage === "draft_building" &&
        !deckBuilderMode &&
        !cardBrowserMode),
  );
  let isDraftMode = $derived(gameState?.stage === "draft");
  let isDraftBuilding = $derived(gameState?.stage === "draft_building");
  let isAwardsMode = $derived(gameState?.stage === "awards");
  let draftView = $derived(gameState?.draft ?? null);
  let awardsView = $derived(gameState?.draft_awards ?? null);
  let draftPyramidZone: DraftPyramidZone | null = null;

  let showGameElements = $derived(
    Boolean(
      gameState?.stage &&
        gameState.stage !== "lobby" &&
        gameState.stage !== "draft" &&
        gameState.stage !== "draft_building" &&
        gameState.stage !== "awards" &&
        !deckBuilderMode &&
        !cardBrowserMode &&
        !deckSelectMode,
    ),
  );

  // ─── Trigger Log ──────────────────────────────────────────────────────────
  let triggerLog = $derived<any[]>(gameState?.trigger_log || []);
  let visibleTriggers = $state<any[]>([]);
  let triggerFadeTimers: ReturnType<typeof setTimeout>[] = [];

  $effect(() => {
    const entries = triggerLog;
    if (entries.length > 0) {
      const prev = untrack(() => visibleTriggers);
      visibleTriggers = [...prev, ...entries];
      for (const entry of entries) {
        const timer = setTimeout(() => {
          visibleTriggers = visibleTriggers.filter((t) => t !== entry);
        }, 4000);
        triggerFadeTimers.push(timer);
      }
    }
  });

  // ─── Game Log ─────────────────────────────────────────────────────────────
  let gameLogEntries = $derived<any[]>(gameState?.game_log ?? []);
  let availableLogRounds = $derived.by(() => {
    const rounds = new Set<number>();
    for (const entry of gameLogEntries) {
      rounds.add(Number(entry.round));
    }
    return Array.from(rounds).sort((a, b) => a - b);
  });
  let currentLogRound = $derived.by(() => {
    const stateRound = Number(gameState?.round ?? 0);
    if (stateRound > 0) return stateRound;
    return availableLogRounds.at(-1) ?? null;
  });
  let selectedLogRound = $state<number | null>(null);
  let lastObservedCurrentLogRound = $state<number | null>(null);

  $effect(() => {
    const rounds = availableLogRounds;
    const currentRound = currentLogRound;

    if (!rounds.length || currentRound == null) {
      selectedLogRound = null;
      lastObservedCurrentLogRound = null;
      return;
    }

    const shouldFollowCurrentRound =
      selectedLogRound == null ||
      selectedLogRound === lastObservedCurrentLogRound ||
      !rounds.includes(selectedLogRound);

    if (shouldFollowCurrentRound) {
      selectedLogRound = currentRound;
    }

    lastObservedCurrentLogRound = currentRound;
  });

  let visibleGameLogEntries = $derived.by(() => {
    if (selectedLogRound == null) return [];
    return gameLogEntries
      .filter((entry) => Number(entry.round) === selectedLogRound)
      .slice(-120)
      .reverse();
  });

  function jumpLogRound(direction: -1 | 1) {
    if (selectedLogRound == null) return;
    const currentIndex = availableLogRounds.indexOf(selectedLogRound);
    if (currentIndex === -1) return;
    const nextIndex = currentIndex + direction;
    if (nextIndex < 0 || nextIndex >= availableLogRounds.length) return;
    selectedLogRound = availableLogRounds[nextIndex];
  }

  function resetLogRound() {
    selectedLogRound = currentLogRound;
  }

  function gameLogKindLabel(kind: string | null | undefined): string {
    switch (kind) {
      case "cast_spell":
        return "CAST";
      case "resolved":
        return "RESOLVE";
      case "activate_ability":
        return "ABILITY";
      case "play_land":
        return "LAND";
      case "deploy_node":
        return "NODE";
      case "turn_start":
        return "TURN";
      case "attack_declared":
        return "ATTACK";
      case "block_declared":
        return "BLOCK";
      case "combat_damage":
        return "DAMAGE";
      case "card_discarded":
        return "DISCARD";
      case "unit_died":
        return "DEATH";
      default:
        return "LOG";
    }
  }

  function gameLogKindTone(kind: string | null | undefined): string {
    switch (kind) {
      case "cast_spell":
        return "text-amber-200 border-amber-400/25 bg-amber-500/10";
      case "resolved":
        return "text-emerald-200 border-emerald-400/25 bg-emerald-500/10";
      case "activate_ability":
        return "text-violet-200 border-violet-400/25 bg-violet-500/10";
      case "play_land":
      case "deploy_node":
        return "text-sky-200 border-sky-400/25 bg-sky-500/10";
      case "turn_start":
        return "text-cyan-200 border-cyan-400/25 bg-cyan-500/10";
      case "card_discarded":
        return "text-fuchsia-200 border-fuchsia-400/25 bg-fuchsia-500/10";
      case "attack_declared":
      case "block_declared":
      case "combat_damage":
      case "unit_died":
        return "text-rose-200 border-rose-400/25 bg-rose-500/10";
      default:
        return "text-slate-200 border-white/10 bg-white/5";
    }
  }

  // ─── Pack Derived State ────────────────────────────────────────────────────
  let packTypes = $derived<any[]>(gameState?.pack_types || []);
  let userPacks = $derived<any[]>(
    (gameState?.user_packs || []).filter((p: any) => !p.opened),
  );
  let packOpenResult = $derived<any>(gameState?.pack_open_result || null);

  let packsByType = $derived.by(() => {
    const map = new Map<string, { packType: any | null; packs: any[] }>();
    for (const pack of userPacks) {
      if (!map.has(pack.pack_type_id)) {
        const pt =
          packTypes.find((t) => t.pack_type_id === pack.pack_type_id) || null;
        map.set(pack.pack_type_id, { packType: pt, packs: [] });
      }
      map.get(pack.pack_type_id)!.packs.push(pack);
    }
    return map;
  });

  let packInventoryCards = $derived.by(() =>
    Array.from(packsByType.entries()).map(([typeId, { packType, packs }]) =>
      createPackInventoryCard(
        typeId,
        packType?.name || typeId,
        packType?.description || "",
        packs.length,
      ),
    ),
  );

  let openingPackCard = $derived.by(() =>
    createOpeningPackCard(
      packOpeningOpeningId,
      packOpeningTypeId,
      packTypes,
      userPacks,
    ),
  );

  let packResultCards = $derived.by(() => {
    if (!openedPackResult) return [];
    return openedPackResult.cards.map((c: any, i: number) =>
      createPackResultCard(c, i),
    );
  });

  // ─── Deck Builder / Select State ───────────────────────────────────────────
  $effect(() => {
    if (!deckBuilderMode || !deckBuilder?.deck_id) localOperatorSelect = false;
  });

  // Auto-reload stale browser: if browser is active but library is empty, re-open.
  // Guard with a flag so we don't loop when the collection is genuinely empty.
  let browserReloadSent = $state(false);
  $effect(() => {
    if (
      cardBrowserMode &&
      deckBuilder?.browser_active &&
      (deckBuilder?.library?.length ?? 0) === 0 &&
      (deckBuilder?.collection_total ?? 0) === 0 &&
      !browserReloadSent
    ) {
      browserReloadSent = true;
      submitAction("games.cardgame.open_card_browser", {});
    }
    if (!cardBrowserMode || !deckBuilder?.browser_active) {
      browserReloadSent = false;
    }
  });

  let allCards = $derived.by(() => {
    const cards: CardView[] = [];
    const players = gameState?.players || [];
    const viewer = gameState?.viewer;

    if (showPacks) {
      if (
        packPhase === "inventory" ||
        packPhase === "grab" ||
        packPhase === "tear"
      ) {
        packInventoryCards.forEach((c) => cards.push(c));
      } else if (packPhase === "opening") {
        if (openingPackCard) cards.push(openingPackCard);
      } else if (packPhase === "burst") {
        if (packResultCards.length > 0) cards.push(packResultCards[0]);
      } else if (packPhase === "reveal" || packPhase === "summary") {
        packResultCards.forEach((c: CardView) => cards.push(c));
      }
      return cards;
    }

    players.forEach((p: any) => {
      (p.battlefield_units || []).forEach((c: any) =>
        cards.push({ ...c, _zone: "battlefield", _owner: p.id } as any),
      );
      (p.battlefield_infrastructure || []).forEach((c: any) =>
        cards.push({ ...c, _zone: "battlefield", _owner: p.id } as any),
      );
      (p.operator_zone || []).forEach((c: any) =>
        cards.push({ ...c, _zone: "operator", _owner: p.id } as any),
      );
    });

    if (viewer) {
      (viewer.hand || []).forEach((c: any) =>
        cards.push({ ...c, _zone: "hand", _owner: viewer.id } as any),
      );
      (viewer.scry || []).forEach((c: any) =>
        cards.push({ ...c, _zone: "scry", _owner: viewer.id } as any),
      );
    }

    const libraryCards = (gameState?.cards || []) as CardView[];
    libraryCards.forEach((c) => {
      if (!cards.find((existing) => existing.id === c.id))
        cards.push({ ...c, _zone: "pool" } as any);
    });

    // Stack items — render the top item as a spotlight card in 3D.
    // The server sends StackItemView with { card, owner_name, targets },
    // so we unwrap `.card` and give it a spotlight-scoped id.
    const stackItems = (gameState?.stack || []) as any[];
    stackItems.forEach((item, index) => {
      const card = (item && item.card) || item;
      if (!card || !card.id) return;
      const spotlightCard = {
        ...card,
        id: `spotlight-${index}-${card.id}`,
        _zone: "spotlight",
        _stack_index: index,
      } as any;
      cards.push(spotlightCard);
    });

    return cards;
  });

  // ─── Asset Preloader ─────────────────────────────────────────────────────
  let preloadedUrls = new Set<string>();

  // Preload static CDN assets immediately
  $effect(() => {
    const staticUrls = collectImageUrls([]);
    preloadTotal = staticUrls.length;
    let pending = 0;
    let timer: ReturnType<typeof setTimeout> | null = null;
    preloadImages(staticUrls, () => {
      pending++;
      if (!timer) {
        timer = setTimeout(() => {
          preloadProgress += pending;
          pending = 0;
          timer = null;
          if (preloadProgress >= preloadTotal) preloadDone = true;
        }, 200);
      }
    });
  });

  // As cards appear (game state, deck builder, etc.), preload their images incrementally.
  // Batch progress updates to avoid 40+ rapid-fire reactive state mutations.
  let _preloadPending = 0;
  let _preloadFlushTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const urls = collectImageUrls(allCards);
    const newUrls = urls.filter((u) => !preloadedUrls.has(u));
    if (newUrls.length === 0) return;
    for (const u of newUrls) preloadedUrls.add(u);
    preloadTotal = preloadedUrls.size;
    preloadImages(newUrls, () => {
      _preloadPending++;
      if (!_preloadFlushTimer) {
        _preloadFlushTimer = setTimeout(() => {
          preloadProgress += _preloadPending;
          _preloadPending = 0;
          _preloadFlushTimer = null;
          if (preloadProgress >= preloadTotal) preloadDone = true;
        }, 200);
      }
    });
  });

  let displayInspectedCard = $derived.by(() => {
    if (!inspectedCard) return null;
    const currentInspectedCard = inspectedCard;
    return (
      allCards.find((card) => card.id === currentInspectedCard.id) ??
      currentInspectedCard
    );
  });

  let targetingAbility = $derived.by(() => {
    if (targetingMode !== "activate" || !targetingSource || !targetingAbilityId)
      return null;
    return (
      targetingSource.abilities?.find(
        (ability) => ability.id === targetingAbilityId,
      ) ?? null
    );
  });

  let deckBuilderLibrary = $derived.by(() => {
    if (!deckBuilderMode && !cardBrowserMode) return [];
    const deckLib = deckBuilder?.library || gameState?.cards || [];
    // Browser mode: never dedup — backend controls whether cards are catalog or instances
    if (cardBrowserMode) return deckLib as CardView[];
    return untrack(() => computeDeckBuilderLibrary(deckLib as CardView[]));
  });

  let operatorLibrary = $derived.by(() => {
    if (!operatorSelectMode) return [];
    const source = [...allCards, ...deckBuilderLibrary];
    const seen = new Set<string>();
    const unique: CardView[] = [];
    source.forEach((c) => {
      if (!seen.has(c.id)) {
        seen.add(c.id);
        unique.push(c);
      }
    });
    let candidates = computeOperatorLibrary(unique);
    const manaFilters = new Set(
      (deckBuilder?.filter_kind || "")
        .split(",")
        .map((s: string) => s.trim())
        .filter(Boolean),
    );
    if (manaFilters.size > 0) {
      candidates = candidates.filter((c) => {
        const cost = (c as any).cost;
        if (!cost) return false;
        for (const f of manaFilters) {
          if (cost[f] > 0) return true;
        }
        return false;
      });
    }
    return candidates;
  });

  let operatorCard = $derived(
    deckBuilderMode ? (deckBuilder?.operator ?? null) : null,
  );

  const deckSelectLibrary = $derived.by(() => {
    if (!deckSelectMode) return [];
    const decks = (
      (gameState?.deck_choices || []) as Array<{
        id: string;
        name: string;
        description: string;
        colors: string[];
        image_url?: string | null;
      }>
    ).map(createDeckSelectCard);
    decks.push(
      createDeckSelectCard({
        id: "__create__",
        name: "Create New Deck",
        description: "Build a custom deck from scratch",
        colors: ["#22c55e"],
      }),
    );
    decks.push(
      createDeckSelectCard({
        id: "__collection__",
        name: "Collection",
        description: "Browse your card collection",
        colors: ["#a855f7"],
      }),
    );
    return decks;
  });

  let activeRingLibrary = $derived.by(() => {
    if (operatorSelectMode) return operatorLibrary;
    if (deckBuilderMode || cardBrowserMode) return deckBuilderLibrary;
    if (isAwardsMode) return awardsView?.cards || [];
    if (deckSelectMode) return deckSelectLibrary;
    return [];
  });
  let pagedRingMode = $derived(
    cardBrowserMode || (deckBuilderMode && !operatorSelectMode),
  );
  let ringWindowSize = $derived.by(() => {
    if (deckSelectMode) return 12;
    if (operatorSelectMode) return 16;
    if (deckBuilderMode || cardBrowserMode || isAwardsMode) return 17;
    return 20;
  });
  let pagedRingCenterOffset = $derived.by(() => {
    const visibleCount = Math.min(ringWindowSize, activeRingLibrary.length);
    return visibleCount > 0 ? Math.floor((visibleCount - 1) / 2) : 0;
  });
  let pagedRingVisibleStart = $derived.by(() => {
    if (!pagedRingMode) return ringStartIndex;
    const visibleCount = Math.min(ringWindowSize, activeRingLibrary.length);
    const maxStart = Math.max(0, activeRingLibrary.length - visibleCount);
    return Math.max(
      0,
      Math.min(maxStart, ringStartIndex - pagedRingCenterOffset),
    );
  });

  const deckBuilderTableCap = 80;
  let deckBuilderTableCards = $derived.by(() => {
    if (!deckBuilderMode || operatorSelectMode) return [];
    const db = deckBuilder;
    return untrack(() => computeDeckBuilderTableCards(db));
  });

  let deckBuilderUnits = $derived.by(() => {
    if (operatorSelectMode) return [];
    return untrack(() =>
      deckBuilderTableCards.filter(
        (card) => card.kind_tag !== "infrastructure",
      ),
    );
  });
  let deckBuilderInfrastructure = $derived.by(() => {
    if (operatorSelectMode) return [];
    return untrack(() =>
      deckBuilderTableCards.filter(
        (card) => card.kind_tag === "infrastructure",
      ),
    );
  });

  let deckBuilderTotalCards = $derived.by(() =>
    computeDeckBuilderTotalCards(deckBuilder),
  );

  let viewer = $derived.by(() => {
    if (!deckBuilderMode && !cardBrowserMode) return deriveViewer(gameState);
    const lib = deckBuilderLibrary;
    const units = cardBrowserMode ? [] : deckBuilderUnits;
    const infra = cardBrowserMode ? [] : deckBuilderInfrastructure;
    return untrack(
      () =>
        ({
          id: cardBrowserMode ? "card-browser" : "deck-builder",
          name: cardBrowserMode
            ? browserTitle
            : (deckBuilder?.name ?? "Deck Builder"),
          avatar_url: "",
          life: 0,
          mana_pool: EMPTY_MANA,
          infrastructures_played_this_turn: 0,
          is_active: false,
          deck_id: deckBuilder?.deck_id ?? null,
          deck_name: deckBuilder?.name ?? null,
          hand: lib,
          scry: [],
          battlefield_units: units,
          battlefield_infrastructure: infra,
        }) as any,
    );
  });

  let players = $derived.by<PlayerState[]>(() => {
    if (!deckBuilderMode && !cardBrowserMode) {
      const p = derivePlayers(gameState, viewer?.id);
      if (tableManager && p.length > 0) {
        tableManager.setPlayerCount(p.length);
        // Single source of truth: the TableManager owns the effective
        // outer-ring radius and every zone derives layout from it rather
        // than each re-multiplying `base * scale` independently.
        const radius = tableManager.getEffectiveRadius();
        battlefieldZone?.setTableRadius(radius);
        operatorZone?.setTableRadius(radius);
      }
      return p;
    }
    return untrack(() => []);
  });

  // Hit animation: when a player's life drops, flash red + shake the
  // PlayerActor so the player can see they just took damage.
  const lastLifeSeen = new Map<string, number>();
  $effect(() => {
    if (!battlefieldZone) return;
    for (const p of players as any[]) {
      const prev = lastLifeSeen.get(p.id);
      if (prev !== undefined && p.life < prev) {
        battlefieldZone.triggerHitAnim(p.id);
      }
      lastLifeSeen.set(p.id, p.life);
    }
  });

  // Tracks stack items we've already fired a projectile effect for so the
  // beam doesn't re-fire on every re-render of the stack view.
  const firedStackEffects = new Set<string>();

  // Fire a visual "zap" beam from the caster (source card on battlefield or
  // their seat if it's an in-hand spell) to each of the stack item's targets.
  $effect(() => {
    if (!tableManager || !zoneManager) return;
    const items: any[] = (gameState?.stack || []) as any[];
    const liveKeys = new Set<string>();
    for (const item of items) {
      const card = item?.card;
      if (!card?.id) continue;
      const targets: any[] = item?.targets ?? [];
      if (targets.length === 0) continue;
      // Skip Unit/Agent showcases — they're summons, not zaps.
      const tag = String(card.kind_tag || '').toLowerCase();
      if (tag === 'unit' || tag === 'agent') continue;

      const key = `${card.id}:${targets.map((t) => `${t.kind}:${t.id}`).join('|')}`;
      liveKeys.add(key);
      if (firedStackEffects.has(key)) continue;
      firedStackEffects.add(key);

      // Source position: the caster's card if it's already on the field,
      // else the caster's seat.
      let srcPos: THREE.Vector3 | null = null;
      const srcActor = zoneManager.getActor(card.id);
      if (srcActor) {
        srcPos = new THREE.Vector3();
        srcActor.group.getWorldPosition(srcPos);
      }
      if (!srcPos) {
        const bfZone: any = zoneManager.get('battlefield');
        const playerActors: any[] =
          bfZone && typeof bfZone.getPlayerActors === 'function'
            ? bfZone.getPlayerActors()
            : [];
        const ownerActor = playerActors.find((pa) => pa?.id === item.owner_id);
        if (ownerActor?.group) {
          srcPos = new THREE.Vector3();
          ownerActor.group.getWorldPosition(srcPos);
          srcPos.y += 1.2;
        }
      }
      if (!srcPos) continue;

      // Color by card color if known; fall back to cyan.
      const colorHex = (() => {
        const raw = (card.colors && card.colors[0]) || '#22d3ee';
        return parseInt(String(raw).replace('#', ''), 16) || 0x22d3ee;
      })();

      for (const t of targets) {
        let dstPos: THREE.Vector3 | null = null;
        if (t.kind === 'card') {
          const actor = zoneManager.getActor(t.id);
          if (actor) {
            dstPos = new THREE.Vector3();
            actor.group.getWorldPosition(dstPos);
          }
        } else if (t.kind === 'player') {
          const bfZone: any = zoneManager.get('battlefield');
          const playerActors: any[] =
            bfZone && typeof bfZone.getPlayerActors === 'function'
              ? bfZone.getPlayerActors()
              : [];
          const pa = playerActors.find((p) => p?.id === t.id);
          if (pa?.group) {
            dstPos = new THREE.Vector3();
            pa.group.getWorldPosition(dstPos);
            dstPos.y += 1.2;
          }
        }
        if (!dstPos) continue;
        tableManager.fireProjectile(srcPos, dstPos, colorHex);
      }
    }
    // Drop keys that are no longer on the stack so re-cast of the same
    // spell can re-fire.
    for (const k of firedStackEffects) {
      if (!liveKeys.has(k)) firedStackEffects.delete(k);
    }
  });

  let hand = $state<CardView[]>([]);
  let scry = $state<CardView[]>([]);
  let scryPool = $state<CardView[]>([]);
  let scryTop = $state<CardView[]>([]);
  let scryBottom = $state<CardView[]>([]);
  let lastScryKey = $state("");
  let scryCommitted = $state(false);
  let searchActive = $derived(viewer?.library_search_active === true);
  let searchCommitted = $state(false);
  let searchCameraTarget = $state<THREE.Vector3 | null>(null);
  let searchCameraLook = $state<THREE.Vector3 | null>(null);
  let discardSelected = $state<Set<string>>(new Set());
  let discardCommitted = $state(false);
  let pendingDiscard = $derived(viewer?.pending_discard ?? null);
  let lastDiscardKey = $state("");
  let pendingSacrifice = $derived(viewer?.pending_sacrifice ?? null);
  let pendingTriggerTarget = $derived(viewer?.pending_trigger_target ?? null);
  let pendingTriggerChoice = $derived(viewer?.pending_trigger_choice ?? null);
  let ringCards = $state<CardView[]>([]);
  let ringStartIndex = $state(0);
  let kindScrollOffset = $state(0);
  let kindDragOffset = $state(0);
  let ringKey = $state("");
  let selectSelectedIndex = $derived(
    deckSelectMode ? Math.floor((ringCards.length - 1) / 2) : -1,
  );
  let selectedDeckId = $derived(
    deckSelectMode && selectSelectedIndex >= 0
      ? (ringCards[selectSelectedIndex]?.id ?? deckSelectLibrary[0]?.id ?? null)
      : null,
  );
  let builderSelectedIndex = $derived.by(() => {
    if (!(deckBuilderMode || cardBrowserMode || isAwardsMode)) return -1;
    if (!pagedRingMode) return Math.floor((ringCards.length - 1) / 2);
    if (ringCards.length === 0) return -1;
    return Math.max(
      0,
      Math.min(ringCards.length - 1, ringStartIndex - pagedRingVisibleStart),
    );
  });
  let builderSelectedId = $derived(
    (deckBuilderMode || cardBrowserMode || isAwardsMode) &&
      builderSelectedIndex >= 0
      ? (ringCards[builderSelectedIndex]?.id ?? null)
      : null,
  );
  let builderSelectedCard = $derived(
    (deckBuilderMode || cardBrowserMode || isAwardsMode) &&
      builderSelectedIndex >= 0
      ? (ringCards[builderSelectedIndex] ??
          deckBuilderLibrary[0] ??
          awardsView?.cards?.[0] ??
          null)
      : null,
  );
  let selectedDeckCard = $derived(
    deckSelectMode && selectSelectedIndex >= 0
      ? (ringCards[selectSelectedIndex] ?? deckSelectLibrary[0] ?? null)
      : null,
  );
  let hiddenRingCards = $state<{ id: string; until: number }[]>([]);
  let fadingCards = $state<{ card: CardView; until: number }[]>([]);
  let deckBuilderAutoLoad = $state(false);
  let lastLibraryLoadScope = $state("");
  let lastViewerId = "";
  let awardsSelected = $state<Set<string>>(new Set());
  let pendingBrowserPageAnchor = $state<"start" | "end" | null>(null);

  // ─── Engine Refs ───────────────────────────────────────────────────────────
  let sceneManager: SceneManager;
  let tableManager: TableManager;
  let pongManager: PongManager;
  let zoneManager = $state<ZoneManager | null>(null);
  let battlefieldZone: BattlefieldZone;
  let operatorZone: OperatorZone;
  let interactionController: InteractionController;
  let dangerDropZone: DangerDropZone;
  let discardDropZone: DiscardDropZone;
  let buttonManager: ButtonManager;
  let combatAnimator: CombatAnimator;
  let targetingBeamGroup: THREE.Group | null = null;
  let inspectPanel: InspectPanel | null = null;
  let showGameLog = $state(false);
  let packMachine: PackStateMachine;
  let zoneSyncController: ZoneSyncController;
  let keyboardController: KeyboardController;

  // Ticker bumped while a spotlight hold is active, so the sync effect
  // can re-evaluate hold expiry even without a game-state change.
  let spotlightHoldTick = $state(0);
  let _spotlightHoldTimer: ReturnType<typeof setTimeout> | null = null;

  let isDragging = $state(false);
  let isDraggingFromDeck = $state(false);
  let draggingFromZone = $state<string | null>(null);
  let isHoveringDanger = $state(false);
  let isHoveringDiscard = $state(false);
  let isHoveringSacrifice = $state(false);
  let scryCameraTarget = $state<THREE.Vector3 | null>(null);
  let scryCameraLook = $state<THREE.Vector3 | null>(null);
  let packCameraTarget = $state<THREE.Vector3 | null>(null);
  let packCameraLook = $state<THREE.Vector3 | null>(null);
  let actionMenu = $state<{
    card: CardView;
    actions: CardAction[];
    x: number;
    y: number;
  } | null>(null);

  const clamp = (value: number, min: number, max: number) =>
    Math.max(min, Math.min(max, value));

  // ─── Effects ───────────────────────────────────────────────────────────────

  $effect(() => {
    if (!showPacks) return;
    untrack(() => {
      if (inspectedCard) {
        const actor = zoneManager?.getActor(inspectedCard.id);
        if (actor) actor.setInspected(false);
        inspectedCard = null;
      }
      if (hoveredCard?.kind_tag === "deck") hoveredCard = null;
    });
  });

  $effect(() => {
    if (players.length > 0 && viewer?.id && viewer.id !== untrack(() => lastViewerId)) {
      const viewerIndex = players.findIndex((p) => p.id === viewer.id);
      if (viewerIndex >= 0) {
        lastViewerId = viewer.id;
        cameraController?.setRotationStep(viewerIndex);
        if (cameraController && !cameraController.getOrbitState().angle) {
          cameraController.setAngle(
            (viewerIndex * Math.PI * 2) / players.length,
          );
        }
      }
    }
  });

  $effect(() => {
    const realHand =
      deckBuilderMode || cardBrowserMode || searchActive
        ? []
        : deriveHand(viewer);
    const deployCards: CardView[] = [];
    if (viewer?.can_deploy_node && viewer?.deployable_node_types?.length > 0) {
      for (const node of viewer.deployable_node_types) {
        deployCards.push(createDeployCard(node));
      }
    }
    hand = [...deployCards, ...realHand];
  });

  $effect(() => {
    scry = deckBuilderMode || cardBrowserMode ? [] : deriveScry(viewer);
  });

  $effect(() => {
    if (!searchActive) searchCommitted = false;
  });

  $effect(() => {
    if (
      searchActive &&
      !searchCommitted &&
      (!viewer?.scry || viewer.scry.length === 0)
    ) {
      searchCommitted = true;
      submitAction("games.cardgame.search_cancel", {});
    }
  });

  $effect(() => {
    if (deckBuilderMode) {
      lastScryKey = "";
      scryPool = [];
      scryTop = [];
      scryBottom = [];
      scryCommitted = false;
      return;
    }
    const nextKey = deriveScryKey(viewer);
    if (nextKey !== untrack(() => lastScryKey)) {
      lastScryKey = nextKey;
      scryPool = viewer?.scry ? [...viewer.scry] : [];
      if (searchActive) {
        console.log('[Search] scryPool updated:', scryPool.length, 'cards, searchActive:', searchActive, 'searchCommitted:', searchCommitted);
      }
      scryTop = [];
      scryBottom = [];
      scryCommitted = false;
    }
  });

  $effect(() => {
    const nextKey = pendingDiscard
      ? `${pendingDiscard.amount}:${(pendingDiscard.hand || []).map((c) => c.id).join(",")}`
      : "";
    if (nextKey !== untrack(() => lastDiscardKey)) {
      lastDiscardKey = nextKey;
      discardSelected = new Set<string>();
      discardCommitted = false;
    }
  });

  let _seenPingKeys = new Set<string>();
  $effect(() => {
    const remotePings = (gameState?.pings || []) as any[];
    remotePings.forEach((p) => {
      const key = `${p.player_id}-${p.timestamp}`;
      if (!_seenPingKeys.has(key)) {
        _seenPingKeys.add(key);
        tableManager?.addBurst(p.x, p.z);
      }
    });
    // Prune old keys to prevent memory leak
    if (_seenPingKeys.size > 100) {
      const arr = [..._seenPingKeys];
      _seenPingKeys = new Set(arr.slice(-50));
    }
  });

  $effect(() => {
    if (
      !deckBuilderMode &&
      !deckSelectMode &&
      !cardBrowserMode &&
      !isAwardsMode
    ) {
      untrack(() => {
        if (zoneManager && ringCards.length > 0) {
          zoneManager.syncActors([]);
          zoneSyncController?.resetKeys();
        }
        ringCards = [];
        ringStartIndex = 0;
        ringKey = "";
        hiddenRingCards = [];
        fadingCards = [];
      });
      return;
    }
    const firstId = activeRingLibrary[0]?.id ?? "";
    const lastId = activeRingLibrary[activeRingLibrary.length - 1]?.id ?? "";
    const nextKey = `${activeRingLibrary.length}|${firstId}|${lastId}`;
    untrack(() => {
      if (nextKey !== ringKey) {
        ringKey = nextKey;
        if (pagedRingMode) {
          ringStartIndex =
            pendingBrowserPageAnchor === "end"
              ? Math.max(
                  0,
                  activeRingLibrary.length - 1 - pagedRingCenterOffset,
                )
              : Math.min(
                  pagedRingCenterOffset,
                  Math.max(0, activeRingLibrary.length - 1),
                );
        } else {
          ringStartIndex = 0;
        }
        pendingBrowserPageAnchor = null;
        collectionLoadingMore = false;
        hiddenRingCards = [];
        fadingCards = [];
      }
    });
  });

  $effect(() => {
    if (
      !deckBuilderMode &&
      !deckSelectMode &&
      !cardBrowserMode &&
      !isAwardsMode
    )
      return;
    const now = performance.now();
    const currentHidden = untrack(() => hiddenRingCards);
    const nextHidden = currentHidden.filter((entry) => entry.until > now);
    if (nextHidden.length !== currentHidden.length)
      hiddenRingCards = nextHidden;
    const hidden = new Set(nextHidden.map((entry) => entry.id));
    ringCards = buildRingCards(
      activeRingLibrary,
      pagedRingVisibleStart,
      hidden,
      ringWindowSize,
      !pagedRingMode,
    );
  });

  $effect(() => {
    if (!isAwardsMode || !awardsView || awardsView.submitted) {
      const current = untrack(() => awardsSelected);
      if (current.size > 0) awardsSelected = new Set<string>();
      return;
    }
    const allowed = new Set(
      (awardsView.cards || []).map((card: CardView) => card.id),
    );
    const current = untrack(() => awardsSelected);
    const filtered = [...current].filter((cardId) =>
      allowed.has(cardId),
    );
    if (filtered.length !== current.size) {
      awardsSelected = new Set(filtered);
    }
  });

  // Pack result watcher — delegates to PackStateMachine
  $effect(() => {
    if (showPacks && packOpenResult && packMachine) {
      packMachine.onPackResultReceived(
        packOpenResult,
        packPhase,
        packBurstStartedAt,
        openedPackResult,
      );
    }
  });

  // Combat/destruction effects — delegate to CombatAnimator
  $effect(() => {
    if (!combatAnimator) return;
    const currentPairs = gameState?.combat_pairs || [];
    combatAnimator.onCombatUpdate(currentPairs, players as any[]);
  });

  // Zone sync mega-effect — delegates to ZoneSyncController
  $effect(() => {
    if (!zoneManager || !zoneSyncController) return;
    // Read spotlightHoldTick so this effect re-runs when a spotlight hold
    // is about to expire (scheduled via setTimeout below).
    void spotlightHoldTick;

    const activeFilters = new Set(
      (deckBuilder?.filter_kind || "")
        .split(",")
        .map((entry: string) => entry.trim())
        .filter(Boolean),
    );
    const filtered =
      operatorSelectMode || showPacks ? [] : deckBuilderTableCards;

    const result = zoneSyncController.sync({
      zoneManager: zoneManager!,
      tableManager,
      cameraPosition: sceneManager?.camera.position ?? new THREE.Vector3(),
      dangerDropZone,
      discardDropZone,
      sacrificeDropZone: null,
      players: players as PlayerState[],
      viewer,
      hand,
      gameState,
      showPacks,
      showGameElements,
      deckBuilderMode: deckBuilderMode || cardBrowserMode,
      cardBrowserMode,
      deckSelectMode,
      awardsMode: isAwardsMode,
      operatorSelectMode,
      searchActive,
      searchCommitted,
      scry,
      scryPool,
      scryTop,
      scryBottom,
      ringCards,
      selectedDeckId,
      builderSelectedId,
      deckBuilderTableCards,
      filteredTableCards: filtered,
      packInventoryCards: packInventoryCards as CardView[],
      openingPackCard,
      packResultCards: packResultCards as CardView[],
      packPhase,
      revealedPackIndices,
      packOpeningShaking,
      packOpeningOpeningId,
      packTearTypeId,
      packTearProgress,
      packFlyInCount,
      packFocusIndex,
      isDraftMode,
      draftView,
      isDragging,
      isDraggingFromDeck,
      draggingFromZone,
      pendingDiscard,
      discardSelected,
      discardCommitted,
      pendingSacrifice,
      isHoveringDanger,
      isHoveringDiscard,
      isHoveringSacrifice,
    });

    searchCameraTarget = result.searchCameraTarget;
    searchCameraLook = result.searchCameraLook;
    scryCameraTarget = result.scryCameraTarget;
    scryCameraLook = result.scryCameraLook;
    packCameraTarget = result.packCameraTarget;
    packCameraLook = result.packCameraLook;

    // Schedule a re-sync when the earliest held spotlight card expires, so
    // the actor is removed even if no game-state change occurs in between.
    const nextExpiry = zoneSyncController.getNextSpotlightHoldExpiry();
    if (_spotlightHoldTimer) {
      clearTimeout(_spotlightHoldTimer);
      _spotlightHoldTimer = null;
    }
    if (nextExpiry !== null) {
      const wait = Math.max(50, nextExpiry - performance.now());
      _spotlightHoldTimer = setTimeout(() => {
        spotlightHoldTick += 1;
      }, wait);
    }
  });

  $effect(() => {
    if (!combatAnimator) return;
    const currentUnits = (players || []).flatMap((p) => [
      ...(p.battlefield_units || []),
      ...(p.battlefield_infrastructure || []),
    ]);
    combatAnimator.onBattlefieldUpdate(currentUnits);
  });

  $effect(() => {
    if (!deckBuilderMode && !cardBrowserMode) {
      deckBuilderAutoLoad = false;
      lastLibraryLoadScope = "";
      return;
    }
    if (gameState?.stage === "draft_building") {
      deckBuilderAutoLoad = false;
      lastLibraryLoadScope = "";
      return;
    }
    const loadScope = cardBrowserMode
      ? `browser:${deckBuilder?.browser_source ?? ""}:${deckBuilder?.filter_kind ?? ""}:${deckBuilder?.sort_order ?? ""}`
      : `builder:${deckBuilder?.deck_id ?? ""}:${deckBuilder?.filter_kind ?? ""}:${deckBuilder?.sort_order ?? ""}`;
    if (loadScope !== untrack(() => lastLibraryLoadScope)) {
      lastLibraryLoadScope = loadScope;
      deckBuilderAutoLoad = false;
    }
    if (workflow.waiting || workflow.responses?.$loading === true) {
      return;
    }
    const browserLibraryCount = deckBuilder?.library?.length ?? 0;

    // Browser sources hydrate the library server-side, but there is a brief
    // state where the source is set before the page payload arrives. Refresh
    // that source once so the collection view does not get stuck empty.
    if (
      cardBrowserMode &&
      deckBuilder?.browser_source &&
      browserLibraryCount === 0
    ) {
      if (deckBuilderAutoLoad) return;
      deckBuilderAutoLoad = true;
      submitAction("games.cardgame.browser_set_filter", {
        kind: deckBuilder?.filter_kind ?? null,
      });
      return;
    }

    // Graveyard / catalog / other server-backed browser sources already hydrate
    // `deckBuilder.library`. Do not overwrite them with a generic list_cards call.
    if (cardBrowserMode && deckBuilder?.browser_source) {
      deckBuilderAutoLoad = true;
      return;
    }
    if (browserLibraryCount > 0) {
      deckBuilderAutoLoad = true;
      return;
    }
    if (deckBuilderAutoLoad) return;
    deckBuilderAutoLoad = true;
    submitAction("games.cardgame.list_cards", { pagination: { first: 200 } });
  });

  let lastHeartbeatInstance = "";
  $effect(() => {
    const iid = workflow?.instance_id;
    if (!iid || iid === lastHeartbeatInstance) return;
    lastHeartbeatInstance = iid;
    submitAction("games.cardgame.view", {});
    const interval = setInterval(
      () => submitAction("games.cardgame.view", {}),
      20000,
    );
    return () => clearInterval(interval);
  });

  // ─── Actions ───────────────────────────────────────────────────────────────

  const submitAction = (action: string, payload: any) => {
    if (
      action === "games.cardgame.play_land" &&
      payload.card_id?.startsWith("deploy-")
    ) {
      submitAction("games.cardgame.deploy_node", {
        mana_type: payload.card_id.replace("deploy-", ""),
      });
      return;
    }
    if (action === "internal.ui.open_deck_selector") {
      localDeckSelect = true;
      deckSelectDismissed = false;
      return;
    }
    if (action === "internal.ui.open_operator_select") {
      if (deckBuilder?.deck_id) localOperatorSelect = true;
      return;
    }
    if (action === "internal.ui.close_operator_select") {
      localOperatorSelect = false;
      return;
    }
    if (action === "internal.ui.toggle_packs") {
      showPacks = !showPacks;
      return;
    }
    if (action === "internal.ui.close_packs") {
      showPacks = false;
      packMachine?.close();
      return;
    }
    if (action === "internal.ui.reveal_all_packs") {
      packMachine?.revealAll(openedPackResult);
      return;
    }
    if (action === "internal.ui.open_another_pack") {
      packMachine?.openAnother();
      return;
    }
    if (action === "internal.ui.discard_commit") {
      submitDiscardCommit();
      return;
    }
    if (buttonManager?.handleInternalAction(action)) return;

    if (!workflow.instance_id) return;

    if (
      action === "games.cardgame.select_deck" ||
      action === "games.cardgame.deck_builder_back" ||
      action === "games.cardgame.builder_close" ||
      action === "games.cardgame.open_deck_builder" ||
      action === "games.cardgame.card_browser_back" ||
      action === "games.cardgame.open_card_browser"
    ) {
      localDeckSelect = false;
      localOperatorSelect = false;
      if (action === "games.cardgame.select_deck") deckSelectDismissed = true;
      if (
        action === "games.cardgame.builder_close" ||
        action === "games.cardgame.deck_builder_back"
      ) {
        builderSideboardMode = false;
      }
    }
    if (action === "games.cardgame.builder_set_operator")
      localOperatorSelect = false;

    const lobbyId = workflow?.responses?.lobby_id;
    submitServerAction(
      provider,
      workflow.instance_id,
      { ...payload, lobby_id: lobbyId },
      action,
    );
  };

  const clearTargeting = () => {
    targetingSource = null;
    targetingMode = null;
    targetingAbilityId = null;
    interactionController?.clearPendingTargets();
  };

  const toggleDiscardSelection = (cardId: string) => {
    if (discardCommitted) return;
    const pd = pendingDiscard;
    if (!pd) return;
    const next = new Set(discardSelected);
    if (next.has(cardId)) {
      next.delete(cardId);
    } else {
      if (next.size >= pd.amount) return;
      next.add(cardId);
    }
    discardSelected = next;
  };

  const submitDiscardCommit = () => {
    if (discardCommitted) return;
    const pd = pendingDiscard;
    if (!pd) return;
    if (discardSelected.size !== pd.amount) return;
    discardCommitted = true;

    // Animate the selected cards into the center portal before the server
    // state update removes them from hand.
    const ids = [...discardSelected];
    const portalPos = new THREE.Vector3(0, 0.1, 0);
    if (zoneManager) {
      ids.forEach((id, i) => {
        const actor = zoneManager!.getActor(id);
        (actor as any)?.animateIntoPortal?.(portalPos, i * 80);
      });
    }
    // Give the animation a beat before submitting so the state update
    // doesn't immediately yank the cards away mid-flight.
    setTimeout(() => {
      submitAction("games.cardgame.discard_commit", { card_ids: ids });
    }, 420);
  };

  const moveScryCard = (
    cardId: string,
    target: "top" | "bottom" | "pool",
    insertIndex?: number,
  ) => {
    if (scryCommitted) return;
    const nextPool = [...scryPool];
    const nextTop = [...scryTop];
    const nextBottom = [...scryBottom];

    const poolIndex = nextPool.findIndex((c) => c.id === cardId);
    const topIndex = nextTop.findIndex((c) => c.id === cardId);
    const bottomIndex = nextBottom.findIndex((c) => c.id === cardId);
    const fromZone =
      poolIndex >= 0
        ? "pool"
        : topIndex >= 0
          ? "top"
          : bottomIndex >= 0
            ? "bottom"
            : null;
    if (!fromZone) return;

    const fromIndex =
      fromZone === "pool"
        ? poolIndex
        : fromZone === "top"
          ? topIndex
          : bottomIndex;
    const fromList =
      fromZone === "pool"
        ? nextPool
        : fromZone === "top"
          ? nextTop
          : nextBottom;
    const card = fromList[fromIndex];

    if (fromZone === target) {
      if (insertIndex === undefined || insertIndex === fromIndex) return;
      fromList.splice(fromIndex, 1);
      let nextIndex = insertIndex;
      if (nextIndex > fromIndex) nextIndex -= 1;
      fromList.splice(nextIndex, 0, card);
    } else {
      fromList.splice(fromIndex, 1);
      const toList =
        target === "pool" ? nextPool : target === "top" ? nextTop : nextBottom;
      toList.splice(insertIndex ?? toList.length, 0, card);
    }

    scryPool = nextPool;
    scryTop = nextTop;
    scryBottom = nextBottom;

    if (
      !scryCommitted &&
      scry.length > 0 &&
      nextTop.length + nextBottom.length === scry.length
    ) {
      scryCommitted = true;
      if (!workflow?.instance_id) return;
      submitServerAction(
        provider,
        workflow.instance_id,
        { top: nextTop.map((c) => c.id), bottom: nextBottom.map((c) => c.id) },
        "games.cardgame.scry_commit",
      );
    }
  };

  const getScryTarget = (cardId: string) => {
    const topIndex = scryTop.findIndex((c) => c.id === cardId);
    if (topIndex >= 0) return { zone: "top" as const, index: topIndex };
    const bottomIndex = scryBottom.findIndex((c) => c.id === cardId);
    if (bottomIndex >= 0)
      return { zone: "bottom" as const, index: bottomIndex };
    const poolIndex = scryPool.findIndex((c) => c.id === cardId);
    if (poolIndex >= 0) return { zone: "pool" as const, index: poolIndex };
    return null;
  };

  const fadeOutCard = (card: CardView) => {
    const until = performance.now() + 900;
    fadingCards = [
      ...fadingCards,
      { card: { ...card, fade_out: true }, until },
    ];
    if (!hiddenRingCards.find((e) => e.id === card.id)) {
      hiddenRingCards = [...hiddenRingCards, { id: card.id, until }];
    }
  };

  let collectionLoadingMore = false;
  const rotateRing = (delta: number) => {
    if (
      !deckBuilderMode &&
      !deckSelectMode &&
      !operatorSelectMode &&
      !cardBrowserMode &&
      !isAwardsMode
    )
      return;
    const len = activeRingLibrary.length;
    if (len === 0) return;

    if (pagedRingMode && !collectionLoadingMore) {
      const hasNext = deckBuilder?.collection_has_next === true;
      const hasPrev = deckBuilder?.collection_has_prev === true;
      if (delta > 0 && ringStartIndex >= len - 1 && hasNext) {
        collectionLoadingMore = true;
        pendingBrowserPageAnchor = "start";
        submitAction("games.cardgame.browser_page", { delta: 1 });
        setTimeout(() => {
          collectionLoadingMore = false;
        }, 500);
        return;
      }
      if (delta < 0 && ringStartIndex <= 0 && hasPrev) {
        collectionLoadingMore = true;
        pendingBrowserPageAnchor = "end";
        submitAction("games.cardgame.browser_page", { delta: -1 });
        setTimeout(() => {
          collectionLoadingMore = false;
        }, 500);
        return;
      }
    }

    if (pagedRingMode) {
      ringStartIndex = Math.max(0, Math.min(len - 1, ringStartIndex + delta));
    } else {
      ringStartIndex = (ringStartIndex + delta + len) % len;
    }
  };

  const rotateKindFilters = (delta: number) => {
    if ((!deckBuilderMode && !cardBrowserMode) || operatorSelectMode) return;
    kindDragOffset += delta;
  };

  const handleDeckBuilderDrop = (cardId: string, deckId: string) => {
    if (operatorSelectMode) return;
    const card = ringCards.find((e) => e.id === cardId);
    if (!card) return;
    fadeOutCard(card);
    const action = builderSideboardMode
      ? "games.cardgame.builder_add_sideboard_card"
      : "games.cardgame.builder_add_card";
    submitAction(action, {
      deck_id: deckId,
      card_id: card.id,
    });
  };

  const openActionMenu = (
    card: CardView,
    actions: CardAction[],
    screenPos: { x: number; y: number },
  ) => {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const localX = screenPos.x - rect.left;
    const localY = screenPos.y - rect.top;
    const menuWidth = 224;
    const menuHeight = 120 + actions.length * 40;
    let x = localX;
    let y = localY + 10;
    if (y + menuHeight > rect.height - 8) y = localY - menuHeight - 10;
    x = clamp(x, menuWidth / 2 + 8, rect.width - menuWidth / 2 - 8);
    y = clamp(y, 8, rect.height - menuHeight - 8);
    actionMenu = { card, actions, x, y };
  };

  const toggleAwardsSelection = (cardId: string) => {
    if (!isAwardsMode || !awardsView || awardsView.submitted) return;
    if (!(awardsView.cards || []).find((card: CardView) => card.id === cardId))
      return;

    const next = new Set(awardsSelected);
    if (next.has(cardId)) {
      next.delete(cardId);
    } else {
      if (
        awardsView.required_count > 0 &&
        next.size >= awardsView.required_count
      )
        return;
      next.add(cardId);
    }
    awardsSelected = next;
  };

  const submitAwards = () => {
    if (!isAwardsMode || !awardsView || awardsView.submitted) return;
    if (awardsView.selection_mode !== "claim") return;
    if (awardsSelected.size !== awardsView.required_count) return;
    submitAction("games.cardgame.draft_keep_cards", {
      card_ids: [...awardsSelected],
    });
  };

  const beamLabelCache = new Map<string, THREE.Sprite>();
  const buildBeamLabel = (
    text: string,
    color: number,
  ): THREE.Sprite | null => {
    const key = `${text}|${color}`;
    const cached = beamLabelCache.get(key);
    if (cached) return cached.clone() as THREE.Sprite;
    const canvas = document.createElement("canvas");
    canvas.width = 1024;
    canvas.height = 192;
    const ctx = canvas.getContext("2d");
    if (!ctx) return null;
    ctx.clearRect(0, 0, 1024, 192);
    const label = text.toUpperCase();
    ctx.font = 'bold 68px "Orbitron", monospace';
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    const metrics = ctx.measureText(label);
    const padX = 24;
    const boxW = Math.min(1024 - 16, metrics.width + padX * 2);
    const boxH = 104;
    const x = 512 - boxW / 2;
    const y = 96 - boxH / 2;
    const hex = `#${color.toString(16).padStart(6, "0")}`;
    ctx.fillStyle = "rgba(2, 6, 14, 0.78)";
    ctx.strokeStyle = hex;
    ctx.lineWidth = 3;
    const r = 14;
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + boxW - r, y);
    ctx.quadraticCurveTo(x + boxW, y, x + boxW, y + r);
    ctx.lineTo(x + boxW, y + boxH - r);
    ctx.quadraticCurveTo(x + boxW, y + boxH, x + boxW - r, y + boxH);
    ctx.lineTo(x + r, y + boxH);
    ctx.quadraticCurveTo(x, y + boxH, x, y + boxH - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
    ctx.shadowColor = hex;
    ctx.shadowBlur = 14;
    ctx.fillStyle = "#ffffff";
    ctx.fillText(label, 512, 100);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const mat = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthWrite: false,
      depthTest: false,
    });
    const sprite = new THREE.Sprite(mat);
    const aspect = 1024 / 192;
    sprite.scale.set(1.2 * aspect * 0.5, 0.5, 1);
    beamLabelCache.set(key, sprite);
    return sprite.clone() as THREE.Sprite;
  };

  const buildTargetingBeam = (
    start: THREE.Vector3,
    end: THREE.Vector3,
    locked: boolean,
    time: number,
    label?: string | null,
  ): THREE.Group => {
    const group = new THREE.Group();
    const dist = start.distanceTo(end);
    // Vertical arc so the direction reads at any camera angle.
    const mid = start.clone().lerp(end, 0.5);
    mid.y += Math.min(1.6, dist * 0.3);
    const curve = new THREE.QuadraticBezierCurve3(
      start.clone(),
      mid,
      end.clone(),
    );
    const points = curve.getPoints(40);
    const geo = new THREE.BufferGeometry().setFromPoints(points);
    const color = locked ? 0x22d3ee : 0xfbbf24;
    const lineMat = new THREE.LineBasicMaterial({
      color,
      transparent: true,
      opacity: locked ? 0.85 : 0.55,
      blending: THREE.AdditiveBlending,
    });
    group.add(new THREE.Line(geo, lineMat));

    // Motes travelling along the curve.
    const moteCount = 3;
    for (let i = 0; i < moteCount; i++) {
      const t = ((time * 0.8 + i / moteCount) % 1.0 + 1.0) % 1.0;
      const p = curve.getPoint(t);
      const spriteMat = new THREE.SpriteMaterial({
        color,
        transparent: true,
        opacity: 0.85 * (1 - Math.abs(t - 0.5) * 1.4),
        blending: THREE.AdditiveBlending,
      });
      const sprite = new THREE.Sprite(spriteMat);
      sprite.position.copy(p);
      sprite.scale.setScalar(locked ? 0.22 : 0.16);
      group.add(sprite);
    }

    // Label in the middle.
    if (label) {
      const labelSprite = buildBeamLabel(label, color);
      if (labelSprite) {
        const labelPos = curve.getPoint(0.5);
        labelSprite.position.copy(labelPos);
        group.add(labelSprite);
      }
    }

    // Rotating reticle at the target.
    const ringGeo = new THREE.RingGeometry(0.18, 0.24, 32);
    const ringMat = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity: 0.9,
      side: THREE.DoubleSide,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.position.copy(end);
    ring.rotation.x = -Math.PI / 2;
    ring.rotation.z = time * 1.2;
    group.add(ring);

    return group;
  };

  // ─── onMount ──────────────────────────────────────────────────────────────

  onMount(() => {
    // 1. Engine
    sceneManager = new SceneManager({ canvas, useBloom: true });
    tableManager = new TableManager(sceneManager.scene);
    pongManager = new PongManager(sceneManager.scene);

    // 2. Zones
    zoneManager = new ZoneManager(sceneManager.scene);

    zoneManager.register(new HandZone(sceneManager.camera));
    battlefieldZone = new BattlefieldZone();
    operatorZone = new OperatorZone();
    zoneManager.register(battlefieldZone);
    zoneManager.register(operatorZone);

    const scryZone = new SelectionZone("scry");
    scryZone.setTransform(
      new THREE.Vector3(0, 2, 8),
      new THREE.Euler(-0.5, 0, 0),
    );
    zoneManager.register(scryZone);

    zoneManager.register(new LibraryRingZone());

    const searchGridZone = new LibraryRingZone("search_grid");
    zoneManager.register(searchGridZone);

    const deckRingLeftZone = new LibraryRingZone("deck_ring_left");
    zoneManager.register(deckRingLeftZone);
    deckRingLeftZone
      .getSceneObjects()
      .forEach((obj) => sceneManager.scene.add(obj));

    const deckRingRightZone = new LibraryRingZone("deck_ring_right");
    zoneManager.register(deckRingRightZone);
    deckRingRightZone
      .getSceneObjects()
      .forEach((obj) => sceneManager.scene.add(obj));

    dangerDropZone = new DangerDropZone();
    zoneManager.register(dangerDropZone);
    dangerDropZone
      .getSceneObjects()
      .forEach((obj) => sceneManager.scene.add(obj));

    discardDropZone = new DiscardDropZone();
    zoneManager.register(discardDropZone);
    discardDropZone.setPosition(-1.4, 0);
    discardDropZone
      .getSceneObjects()
      .forEach((obj) => sceneManager.scene.add(obj));

    // Sacrifice is handled via the action menu now — no drop zone.

    zoneManager.register(new PackOpeningZone());
    zoneManager.register(new SpotlightZone(sceneManager.camera));

    draftPyramidZone = new DraftPyramidZone();
    zoneManager.register(draftPyramidZone);
    zoneManager.register(new DraftSideZone());

    targetingBeamGroup = new THREE.Group();
    sceneManager.scene.add(targetingBeamGroup);

    // InspectPanel attaches itself to the camera; make sure the camera is
    // in the scene graph so its child group renders.
    if (!sceneManager.camera.parent) sceneManager.scene.add(sceneManager.camera);
    inspectPanel = new InspectPanel(sceneManager.camera, zoneManager.getFactory());

    // 3. Camera
    cameraController = new CameraController(sceneManager.camera);

    // 4. Controllers
    buttonManager = new ButtonManager(
      sceneManager.scene,
      zoneManager.getFactory(),
    );
    const hudManager = new HudManager(
      sceneManager.camera,
      zoneManager.getFactory(),
    );
    buttonManager.setHudManager(hudManager);
    combatAnimator = new CombatAnimator(
      zoneManager,
      tableManager,
      sceneManager.scene,
    );
    zoneSyncController = new ZoneSyncController();

    // 5. Pack machine
    packMachine = new PackStateMachine({
      submitAction,
      addPackBurst: (x, y, z) => tableManager.addPackBurst(x, y, z),
      onSetState: (updates: PackStateUpdate) => {
        if (updates.packPhase !== undefined) packPhase = updates.packPhase;
        if (updates.packOpeningOpeningId !== undefined)
          packOpeningOpeningId = updates.packOpeningOpeningId;
        if (updates.packOpeningTypeId !== undefined)
          packOpeningTypeId = updates.packOpeningTypeId;
        if (updates.packTearTypeId !== undefined)
          packTearTypeId = updates.packTearTypeId;
        if (updates.packTearProgress !== undefined)
          packTearProgress = updates.packTearProgress;
        if (updates.packOpeningShaking !== undefined)
          packOpeningShaking = updates.packOpeningShaking;
        if (updates.openedPackResult !== undefined)
          openedPackResult = updates.openedPackResult;
        if (updates.revealedPackIndices !== undefined)
          revealedPackIndices = updates.revealedPackIndices;
        if (updates.packRevealZoomed !== undefined)
          packRevealZoomed = updates.packRevealZoomed;
        if (updates.packBurstStartedAt !== undefined)
          packBurstStartedAt = updates.packBurstStartedAt;
        if (updates.packFlyInCount !== undefined)
          packFlyInCount = updates.packFlyInCount;
        if (updates.packFocusIndex !== undefined)
          packFocusIndex = updates.packFocusIndex;
        if (updates.flashRarity !== undefined)
          flashRarity = updates.flashRarity;
      },
    });

    // 6. Input
    const inputManager = new InputManager(canvas, sceneManager.camera);
    const getModeFilteredButtons = () => {
      const base = buttonManager.getButtons().filter((btn) => {
        const mode = btn.userData?.mode as string | undefined;
        if (!btn.visible) return false;
        if (!mode) return true;
        if (showPacks) return mode === "packs";
        if (isDraftMode) return mode === "draft";
        if (operatorSelectMode) return mode === "operator";
        if (deckBuilderMode) return mode === "builder";
        if (cardBrowserMode) return mode === "browser" || mode === "builder";
        if (deckSelectMode) return mode === "select";
        if (isLobbyMode) return mode === "lobby";
        return false;
      });
      const extras: THREE.Object3D[] = [];
      if (deckBuilderMode && isDraggingFromDeck)
        extras.push(...dangerDropZone.getExtraInteractables());
      if (!deckBuilderMode && !deckSelectMode) {
        extras.push(...discardDropZone.getExtraInteractables());
      }
      return [...base, ...extras];
    };

    inputManager.getInteractables = () => {
      const base = zoneManager?.getInteractables() ?? [];
      return [
        ...base,
        ...getModeFilteredButtons(),
        ...hudManager.getInteractables(),
      ];
    };

    interactionController = new InteractionController(
      inputManager,
      zoneManager,
      submitAction,
      (card, hints) => {
        hoveredCard = card;
        const next = hints ?? null;
        if (JSON.stringify(next) !== JSON.stringify(hoveredActionHints)) {
          hoveredActionHints = next;
        }
      },
      (x, z) => {
        tableManager.addBurst(x, z);
      },
      () => ({
        source: targetingSource,
        mode: targetingMode,
        abilityId: targetingAbilityId,
      }),
      (source, mode, abilityId) => {
        targetingSource = source;
        targetingMode = mode;
        targetingAbilityId = abilityId ?? null;
      },
      () =>
        (scry.length > 0 && !scryCommitted) ||
        (searchActive && !searchCommitted),
      getScryTarget,
      (cardId, zone, index) => moveScryCard(cardId, zone, index),
      () => {},
      () => viewer?.id ?? null,
      (cardId, zone, beforeId) => {
        if (beforeId) {
          const list =
            zone === "top"
              ? scryTop
              : zone === "bottom"
                ? scryBottom
                : scryPool;
          const index = list.findIndex((c) => c.id === beforeId);
          moveScryCard(cardId, zone, index >= 0 ? index : undefined);
        } else {
          moveScryCard(cardId, zone);
        }
      },
      (card, actions, screenPos) => openActionMenu(card, actions, screenPos),
      () => ({
        enabled: deckBuilderMode && !cardBrowserMode,
        deckId: gameState?.deck_builder?.deck_id ?? null,
      }),
      (cardId, deckId) => handleDeckBuilderDrop(cardId, deckId),
      () => ({ enabled: false }),
      () => {},
      () => getModeFilteredButtons(),
      (step) => {
        if (!manualCamera) rotateRing(step);
      },
      (step) => {
        if (!manualCamera) rotateKindFilters(step);
      },
      (dragging, cardId) => {
        isDragging = dragging;
        if (!dragging) {
          isDraggingFromDeck = false;
          draggingFromZone = null;
          return;
        }
        if (deckBuilderMode) {
          isDraggingFromDeck = !!cardId && cardId.includes("::");
        } else {
          isDraggingFromDeck = false;
          if (cardId) {
            const inHand = viewer?.hand?.some((c: any) => c.id === cardId);
            const inBattlefield = [
              ...(viewer?.battlefield_units || []),
              ...(viewer?.battlefield_infrastructure || []),
            ].some((c: any) => c.id === cardId);
            if (inHand) draggingFromZone = "hand";
            else if (inBattlefield) draggingFromZone = "battlefield";
            else {
              const card = gameState?.cards?.find((c: any) => c.id === cardId);
              draggingFromZone = card?.zone || null;
            }
          }
        }
      },
      (cardId) => {
        if (!deckBuilderMode || !gameState?.deck_builder?.deck_id) return;
        submitAction("games.cardgame.builder_remove_card", {
          deck_id: gameState.deck_builder.deck_id,
          card_id: cardId.split("::")[0],
        });
      },
      (hovered) => {
        isHoveringDanger = hovered;
      },
      (hovered) => {
        isHoveringDiscard = hovered;
      },
      (hovered) => {
        isHoveringSacrifice = hovered;
      },
      (card) => {
        if (inspectedCard) {
          const prevActor = zoneManager?.getActor(inspectedCard.id);
          if (prevActor) prevActor.setInspected(false);
        }
        inspectedCard = card;
        inspectTilt = { x: -0.18, y: 0 };
      },
      (index) =>
        packMachine.revealCard(
          index,
          packPhase,
          revealedPackIndices,
          openedPackResult,
        ),
      (typeId) => {
        if (!showPacks || packPhase !== "inventory") return;
        packMachine.tearStart(typeId);
      },
      (progress) => {
        if (!showPacks) return;
        packMachine.tearProgress(progress, packPhase);
      },
      (typeId) => {
        if (!showPacks) return;
        packMachine.commitOpen(typeId, packsByType, packTearProgress);
      },
      () => {
        if (!showPacks) return;
        packMachine.tearCancel();
      },
      () => ({
        active: !!pendingDiscard && !discardCommitted,
        handIds: new Set(
          (pendingDiscard?.hand ?? []).map((c: any) => c.id as string),
        ),
      }),
      (cardId: string) => {
        toggleDiscardSelection(cardId);
      },
      () => !!pendingSacrifice,
      (reason: string) => {
        targetingError = reason;
        clearTimeout(targetingErrorTimer);
        targetingErrorTimer = setTimeout(() => {
          targetingError = null;
        }, 3000);
      },
      () => {
        // Flatten every target across every stack item into a list of
        // {kind, id} so the 3D highlight pass can light them up.
        const out: Array<{ kind: 'card' | 'player'; id: string }> = [];
        const items: any[] = (gameState?.stack || []) as any[];
        for (const it of items) {
          const ts: any[] = it?.targets ?? [];
          for (const t of ts) {
            if (!t?.id) continue;
            if (t.kind === 'card' || t.kind === 'player') {
              out.push({ kind: t.kind, id: t.id });
            }
          }
        }
        return out;
      },
    );

    interactionController.onInspectDrag = (dx, dy) => {
      inspectTilt.y += dx * 0.007;
      inspectTilt.x += dy * 0.007;
    };

    // Click override for search/discard/sacrifice
    // Middle-click drag rotates the camera around the table (like holding A/D).
    // The CameraController holds this angle for 5s before snapping back to seat.
    inputManager.onMiddleDrag = (dx, _dy) => {
      cameraController?.orbitByDelta(dx);
    };

    // Mouse wheel zooms (same as W/S).
    inputManager.onWheel = (deltaY) => {
      cameraController?.zoomByDelta(deltaY);
    };

    inputManager.onClick = (hit, button = 0) => {
      if (inspectedCard && button === 0) {
        const actor = zoneManager?.getActor(inspectedCard.id);
        if (actor) actor.setInspected(false);
        inspectedCard = null;
        return;
      }
      // Draft pick
      if (isDraftMode && draftView?.is_my_turn && hit && button === 0) {
        let obj: THREE.Object3D | null = hit.object;
        while (obj && !obj.userData?.actor) obj = obj.parent;
        const actor = obj?.userData?.actor as any;
        if (actor && draftPyramidZone) {
          const card = actor.getCard();
          const pyramidIndex = draftPyramidZone.pyramidIndexForCardId(card.id);
          if (
            pyramidIndex !== undefined &&
            draftView.available_indices?.includes(pyramidIndex)
          ) {
            submitAction("games.cardgame.draft_pick", { index: pyramidIndex });
            return;
          }
        }
      }
      if (searchActive && !searchCommitted && hit && button === 0) {
        let obj: THREE.Object3D | null = hit.object;
        while (obj && !obj.userData?.actor) obj = obj.parent;
        const actor = obj?.userData?.actor as any;
        if (actor) {
          const card = actor.getCard();
          if (scryPool.find((c: CardView) => c.id === card.id)) {
            searchCommitted = true;
            submitAction("games.cardgame.search_commit", { card_id: card.id });
            return;
          }
        }
      }
      if (pendingDiscard && !discardCommitted && hit && button === 0) {
        let obj: THREE.Object3D | null = hit.object;
        while (obj && !obj.userData?.actor) obj = obj.parent;
        const actor = obj?.userData?.actor as any;
        const cardId = actor?.getCard?.()?.id as string | undefined;
        if (cardId && pendingDiscard.hand?.some((card) => card.id === cardId)) {
          toggleDiscardSelection(cardId);
          return;
        }
      }
      if (pendingSacrifice && hit && button === 0) {
        let obj: THREE.Object3D | null = hit.object;
        while (obj && !obj.userData?.actor) obj = obj.parent;
        const actor = obj?.userData?.actor as any;
        if (actor?.getZone?.() === "battlefield") {
          submitAction("games.cardgame.sacrifice", {
            card_id: actor.getCard().id,
          });
          return;
        }
      }
      if (pendingTriggerTarget && hit && button === 0) {
        const kind = pendingTriggerTarget.target_kind as string;
        const wantsPlayer = kind === "player" || kind === "player_or_unit";
        const wantsUnit =
          kind === "unit" || kind === "card" || kind === "player_or_unit";
        // Check for player hit
        if (wantsPlayer && hit.object?.userData?.playerId) {
          submitAction("games.cardgame.trigger_target_select", {
            target_type: "player",
            target_id: hit.object.userData.playerId,
          });
          return;
        }
        // Check for unit hit
        if (wantsUnit) {
          let obj: THREE.Object3D | null = hit.object;
          while (obj && !obj.userData?.actor) obj = obj.parent;
          const actor = obj?.userData?.actor as any;
          if (actor?.getZone?.() === "battlefield") {
            submitAction("games.cardgame.trigger_target_select", {
              target_type: "card",
              target_id: actor.getCard().id,
            });
            return;
          }
        }
      }
      if (isAwardsMode && hit && button === 0) {
        let obj: THREE.Object3D | null = hit.object;
        while (obj && !obj.userData?.actor) obj = obj.parent;
        const actor = obj?.userData?.actor as any;
        if (actor?.getZone?.() === "ring") {
          toggleAwardsSelection(actor.getCard().id);
          return;
        }
      }
      // Left-click inspect in collection browser
      if (cardBrowserMode && hit && button === 0) {
        let obj: THREE.Object3D | null = hit.object;
        while (obj && !obj.userData?.actor) obj = obj.parent;
        const actor = obj?.userData?.actor as any;
        if (actor) {
          const card = actor.getCard();
          inspectedCard = card;
          inspectTilt = { x: -0.18, y: 0 };
          return;
        }
      }
      interactionController.handleClick(hit, button);
    };

    // 7. Keyboard
    keyboardController = new KeyboardController(
      {
        showPacks: () => showPacks,
        showHelp: () => showHelp,
        packPhase: () => packPhase,
        packFlyInCount: () => packFlyInCount,
        packFocusIndex: () => packFocusIndex,
        setPackFocusIndex: (i) => {
          packFocusIndex = i;
        },
        isInspecting: () => !!inspectedCard,
        deckBuilderMode: () =>
          deckBuilderMode || cardBrowserMode || isAwardsMode,
        cardBrowserMode: () => cardBrowserMode,
        deckSelectMode: () => deckSelectMode || isAwardsMode,
        manualCamera: () => manualCamera,
        hasActionMenu: () => !!actionMenu,
        playerCount: () => players.length,
        isTargeting: () => !!targetingSource,
        hasCancellableTriggerTarget: () =>
          !!pendingTriggerTarget?.optional,
      },
      {
        toggleManualCamera: () => {
          manualCamera = !manualCamera;
          if (manualCamera) cameraController?.enterManualCamera();
        },
        togglePacks: () => {
          showPacks = packMachine.toggle(showPacks, submitAction);
        },
        toggleHelp: () => {
          showHelp = !showHelp;
        },
        toggleFullscreen: () => {
          if (!document.fullscreenElement) {
            container?.requestFullscreen();
            isFullscreen = true;
          } else {
            document.exitFullscreen();
            isFullscreen = false;
          }
        },
        rotateRing,
        rotateKindFilters,
        revealCard: (index) =>
          packMachine.revealCard(
            index,
            packPhase,
            revealedPackIndices,
            openedPackResult,
          ),
        rotateCamera: () => cameraController?.rotateClockwise(players.length),
        closeActionMenu: () => {
          actionMenu = null;
        },
        exitPacks: () => {
          showPacks = false;
        },
        exitHelp: () => {
          showHelp = false;
        },
        exitCardBrowser: () => {
          if (inspectedCard) {
            const actor = zoneManager?.getActor(inspectedCard.id);
            if (actor) actor.setInspected(false);
            inspectedCard = null;
          }
          submitAction("games.cardgame.card_browser_back", {});
        },
        cancelTargeting: () => {
          clearTargeting();
        },
        cancelOptionalTriggerTarget: () => {
          submitAction("games.cardgame.trigger_target_cancel", {});
        },
        advanceTurn: () => {
          if (!workflow.instance_id) return;
          const stackLen = (gameState?.stack || []).length;
          const stage = gameState?.stage;
          const phase = gameState?.combat_phase;
          const isActive = !!viewer?.is_active;
          const isCurrentBlocker =
            phase === "declare_blockers" && gameState?.combat_blocker_id === viewer?.id;

          if (stackLen > 0) {
            submitAction("games.cardgame.pass_priority", {});
          } else if (stage === "main" || stage === "post_combat_main" || stage === "end") {
            submitAction("games.cardgame.next", {});
          } else if (stage === "combat" && phase === "declare_attackers" && isActive) {
            submitAction("games.cardgame.confirm_attackers", {});
          } else if (stage === "combat" && phase === "declare_blockers" && isCurrentBlocker) {
            submitAction("games.cardgame.confirm_blockers", {});
          }
        },
        packInspectNavigate: (delta) => {
          // Reveal current card, move focus, reveal new card, update inspect
          packMachine.revealCard(
            packFocusIndex,
            packPhase,
            revealedPackIndices,
            openedPackResult,
          );
          const launched = packFlyInCount;
          if (launched <= 0) return;
          const newIndex =
            (((packFocusIndex + delta) % launched) + launched) % launched;
          packFocusIndex = newIndex;
          // Reveal the card we're navigating to as well
          packMachine.revealCard(
            newIndex,
            packPhase,
            revealedPackIndices,
            openedPackResult,
          );
          // If all cards are now revealed, clear inspect so summary animation plays
          const totalCards = openedPackResult?.cards?.length || 0;
          if (totalCards > 0 && revealedPackIndices.size >= totalCards) {
            if (inspectedCard) {
              const actor = zoneManager?.getActor(inspectedCard.id);
              if (actor) actor.setInspected(false);
            }
            inspectedCard = null;
            return;
          }
          // Update inspected card to the new focused card
          const card = packResultCards[newIndex];
          if (card) {
            if (inspectedCard) {
              const actor = zoneManager?.getActor(inspectedCard.id);
              if (actor) actor.setInspected(false);
            }
            inspectedCard = card;
            inspectTilt = { x: -0.18, y: 0 };
          }
        },
        confirmSelection: () => {
          if (deckSelectMode && selectedDeckId) {
            if (selectedDeckId === "__create__") {
              submitAction("games.cardgame.builder_create_deck", {});
            } else if (selectedDeckId === "__collection__") {
              submitAction("games.cardgame.open_card_browser", {});
            } else {
              submitAction("games.cardgame.select_deck", {
                deck_id: selectedDeckId,
              });
            }
          }
        },
        inspectSelected: () => {
          // Toggle inspect on the currently selected card
          if (inspectedCard) {
            const actor = zoneManager?.getActor(inspectedCard.id);
            if (actor) actor.setInspected(false);
            inspectedCard = null;
          } else {
            const card = deckSelectMode
              ? selectedDeckCard
              : builderSelectedCard;
            if (card) {
              inspectedCard = card;
              inspectTilt = { x: -0.18, y: 0 };
            }
          }
        },
        inspectNavigate: (delta) => {
          rotateRing(delta);
          // ringCards hasn't recomputed yet, so derive the new center card directly
          const lib = activeRingLibrary;
          if (lib.length === 0) return;
          const centerOffset = Math.floor(
            (Math.min(ringWindowSize, lib.length) - 1) / 2,
          );
          const centerIdx = pagedRingMode
            ? Math.min(ringStartIndex, lib.length - 1)
            : (((ringStartIndex + centerOffset) % lib.length) + lib.length) %
              lib.length;
          const newCenter = lib[centerIdx];
          if (newCenter) {
            if (inspectedCard) {
              const actor = zoneManager?.getActor(inspectedCard.id);
              if (actor) actor.setInspected(false);
            }
            inspectedCard = newCenter;
            inspectTilt = { x: -0.18, y: 0 };
          }
        },
      },
    );
    keyboardController.attach();

    const handleFullscreenChange = () => {
      isFullscreen = !!document.fullscreenElement;
    };
    document.addEventListener("fullscreenchange", handleFullscreenChange);

    const fadeInterval = window.setInterval(() => {
      const now = performance.now();
      if (fadingCards.length > 0)
        fadingCards = fadingCards.filter((e) => e.until > now);
      if (hiddenRingCards.length > 0)
        hiddenRingCards = hiddenRingCards.filter((e) => e.until > now);
    }, 200);

    // 8. Render loop
    let animFrameId: number;
    const animate = () => {
      animFrameId = requestAnimationFrame(animate);
      if (!sceneManager || !interactionController) return;

      const delta = sceneManager.getDelta();
      const time = sceneManager.getTime();

      interactionController.update();
      if (zoneManager) zoneManager.tick(delta, time);
      tableManager?.tick(delta);

      // Modifier highlight sync
      if (zoneManager && highlightedCardId !== prevHighlightedId) {
        if (prevHighlightedId)
          zoneManager.getActor(prevHighlightedId)?.setTargetHint(false);
        if (highlightedCardId)
          zoneManager.getActor(highlightedCardId)?.setTargetHint(true);
        prevHighlightedId = highlightedCardId;
      }

      const isLobby =
        gameState?.stage === "lobby" ||
        (gameState?.stage === "draft_building" &&
          !deckBuilderMode &&
          !cardBrowserMode);
      const isDeckBuilding =
        deckBuilderMode || deckSelectMode || cardBrowserMode || isAwardsMode;
      const isPackOpening = showPacks;

      // Button manager update
      {
        const activeFilters = new Set<string>(
          (deckBuilder?.filter_kind || "")
            .split(",")
            .map((e: string) => e.trim())
            .filter(Boolean),
        );
        buttonManager.update({
          isPackOpening,
          isLobby,
          isDraftMode,
          isDeckBuilding,
          operatorSelectMode,
          deckBuilderMode,
          cardBrowserMode,
          deckSelectMode,
          packPhase,
          userPackCount: userPacks.length,
          selectedDeckId,
          selectedDeckCard,
          selectSelectedIndex,
          ringCards,
          deckSelectLibrary,
          builderSelectedId,
          builderSelectedCard,
          selectedQty:
            deckBuilderMode && builderSelectedId
              ? (deckBuilder?.cards?.find(
                  (e: any) => e?.card?.id === builderSelectedId,
                )?.quantity ?? 0)
              : 0,
          deckBuilderTotalCards,
          deckBuilder,
          operatorCard,
          manaOptions:
            deckBuilderMode || cardBrowserMode
              ? (gameState?.mana_type_options ?? [])
              : [],
          kindOptions:
            deckBuilderMode || cardBrowserMode
              ? (gameState?.kind_options ?? [])
              : [],
          kindDragOffset,
          activeFilters,
          sortOrder: deckBuilder?.sort_order ?? null,
          gameState,
          viewer,
          pendingDiscard,
          discardSelected,
          discardCommitted,
        });
        if (
          isDeckBuilding ||
          isLobby ||
          isPackOpening ||
          isDraftMode ||
          (pendingDiscard && !discardCommitted)
        )
          buttonManager.tick(delta, time);
        hudManager.update(sceneManager.camera.aspect);
      }

      if (draftPyramidZone)
        draftPyramidZone.inspectedCardId = inspectedCard?.id ?? null;

      pongManager?.setVisible(
        isLobby &&
          !isDeckBuilding &&
          !isPackOpening &&
          !isDraftMode &&
          !isAwardsMode,
      );
      if (
        isLobby &&
        !isDeckBuilding &&
        !isPackOpening &&
        !isDraftMode &&
        !isAwardsMode &&
        pongManager
      ) {
        pongManager.update(
          delta,
          players,
          viewer?.id,
          keyboardController.moveState.left,
          keyboardController.moveState.right,
        );
      }

      // Actor visibility
      if (zoneManager) {
        for (const actor of zoneManager.getAllActors()) {
          actor.group.visible =
            isDeckBuilding || isPackOpening || isDraftMode || isAwardsMode
              ? true
              : !isLobby;
        }
        const scryZone = zoneManager.get("scry") as SelectionZone | undefined;
        scryZone
          ?.getSceneObjects()
          .forEach(
            (obj) =>
              (obj.visible =
                !isLobby &&
                !isDeckBuilding &&
                !isPackOpening &&
                !isDraftMode &&
                !isAwardsMode),
          );
        for (const actor of zoneManager.getAllActors()) {
          if (actor.getZone() === "ring") {
            actor.setTargetHint(isAwardsMode && awardsSelected.has(actor.id));
          }
        }
      }

      zoneManager?.updateLayouts();

      // Hand hover
      if (
        hand.length > 0 &&
        scry.length === 0 &&
        !deckBuilderMode &&
        !isPackOpening &&
        !deckSelectMode &&
        !isAwardsMode &&
        !searchActive
      ) {
        if (!handHoverActive) {
          if (inputManager.pointer.y < -0.8) handHoverActive = true;
        } else {
          if (inputManager.pointer.y > -0.3) handHoverActive = false;
        }
        const handZone = zoneManager?.get("hand") as HandZone | undefined;
        handZone?.setHovered(handHoverActive);
        handZone?.setPointer(inputManager.pointer.x, inputManager.pointer.y);
      } else {
        handHoverActive = false;
        const handZone = zoneManager?.get("hand") as HandZone | undefined;
        handZone?.setHovered(false);
        handZone?.setPointer(inputManager.pointer.x, inputManager.pointer.y);
      }

      // Camera
      cameraController.update(delta, {
        deckBuilderMode: deckBuilderMode || cardBrowserMode,
        isDraftMode,
        isPackOpening,
        packPhase,
        packRevealZoomed,
        packTearProgress,
        deckSelectMode: deckSelectMode || isAwardsMode,
        searchActive,
        searchCameraTarget,
        searchCameraLook,
        scryCameraTarget,
        scryCameraLook,
        isLobby,
        players,
        viewerId: viewer?.id ?? null,
        moveState: keyboardController.moveState,
        manualCamera,
        packCameraTarget,
        packCameraLook,
      });

      // Inspect placement
      interactionController.isInspecting = !!inspectedCard;
      if (inspectedCard && zoneManager && cameraController) {
        const actor = zoneManager.getActor(inspectedCard.id);
        if (actor) {
          const { position, rotation, scale } =
            cameraController.getInspectPlacement();
          const baseQ = new THREE.Quaternion().setFromEuler(rotation);
          const tiltQ = new THREE.Quaternion().setFromEuler(
            new THREE.Euler(inspectTilt.x, inspectTilt.y, 0),
          );
          baseQ.multiply(tiltQ);
          const tiltedRot = new THREE.Euler().setFromQuaternion(baseQ, "XYZ");
          actor.setTarget(position, tiltedRot, scale, 0.25);
          actor.setInspected(true);
        }
      }

      sceneManager.render();

      // Pointer HUD
      pointerPos.x = inputManager.pointer.x;
      pointerPos.y = inputManager.pointer.y;
      if (canvas) {
        const rect = canvas.getBoundingClientRect();
        mousePixelPos.x = inputManager.pointer.rawX - rect.left;
        mousePixelPos.y = inputManager.pointer.rawY - rect.top;
      }

      if (inspectPanel && sceneManager?.camera) {
        const panelAllowed =
          showGameElements && !inspectedCard && !isPackOpening;
        inspectPanel.setCard(panelAllowed ? hoveredCard : null);
        inspectPanel.updateAnchor(
          sceneManager.camera as THREE.PerspectiveCamera,
        );
        inspectPanel.tick(delta, time);
      }

      // Render targeting beams (source → picked/hovered targets + stack beams).
      if (targetingBeamGroup) {
        targetingBeamGroup.clear();
        if (targetingSource && zoneManager && interactionController) {
          const sourceActor = zoneManager.getActor(targetingSource.id);
          if (sourceActor) {
            const srcWorld = new THREE.Vector3();
            sourceActor.group.getWorldPosition(srcWorld);
            srcWorld.y += 0.25;

            const endpoints: Array<{
              pos: THREE.Vector3;
              locked: boolean;
            }> = [];
            const picked = interactionController.getPendingTargets?.() ?? [];
            for (const t of picked) {
              if (t.kind === "card") {
                const a = zoneManager.getActor(t.id);
                if (a) {
                  const p = new THREE.Vector3();
                  a.group.getWorldPosition(p);
                  p.y += 0.25;
                  endpoints.push({ pos: p, locked: true });
                }
              } else {
                const pa = (battlefieldZone as any)
                  ?.getPlayerActors?.()
                  ?.find((x: any) => x.id === t.id);
                if (pa) {
                  const p = new THREE.Vector3();
                  pa.group.getWorldPosition(p);
                  p.y += 0.5;
                  endpoints.push({ pos: p, locked: true });
                }
              }
            }
            const hoveredId =
              interactionController.getHoveredActorId?.() ?? null;
            if (
              hoveredId &&
              hoveredId !== targetingSource.id &&
              !picked.some((t) => t.id === hoveredId)
            ) {
              const a = zoneManager.getActor(hoveredId);
              if (a) {
                const p = new THREE.Vector3();
                a.group.getWorldPosition(p);
                p.y += 0.25;
                endpoints.push({ pos: p, locked: false });
              }
            }
            for (const { pos, locked } of endpoints) {
              targetingBeamGroup.add(
                buildTargetingBeam(
                  srcWorld,
                  pos,
                  locked,
                  time,
                  targetingSource.name,
                ),
              );
            }
          }
        }

        // Stack beams: for every spell/ability on the stack with targets,
        // draw a cyan beam from the stack card to each target.
        const stackItems = (gameState?.stack || []) as any[];
        for (const item of stackItems) {
          const card = item?.card || item;
          if (!card?.id) continue;
          const srcActor = zoneManager?.getActor(card.id);
          if (!srcActor) continue;
          const srcWorld = new THREE.Vector3();
          srcActor.group.getWorldPosition(srcWorld);
          srcWorld.y += 0.25;
          for (const target of item?.targets || []) {
            if (!target?.id) continue;
            let endPos: THREE.Vector3 | null = null;
            if (target.kind === "player") {
              const pa = (battlefieldZone as any)
                ?.getPlayerActors?.()
                ?.find((x: any) => x.id === target.id);
              if (pa) {
                endPos = new THREE.Vector3();
                pa.group.getWorldPosition(endPos);
                endPos.y += 0.5;
              }
            } else {
              const a = zoneManager?.getActor(target.id);
              if (a) {
                endPos = new THREE.Vector3();
                a.group.getWorldPosition(endPos);
                endPos.y += 0.25;
              }
            }
            if (endPos) {
              targetingBeamGroup.add(
                buildTargetingBeam(srcWorld, endPos, true, time, card.name),
              );
            }
          }
        }
      }

      // Project the targeting source card's world position to screen space
      // so the dialog can float above it. Nulled when targeting ends.
      if (targetingSource && sceneManager?.camera && canvas && zoneManager) {
        const actor = zoneManager.getActor(targetingSource.id);
        if (actor) {
          const worldPos = new THREE.Vector3();
          actor.group.getWorldPosition(worldPos);
          // Lift the anchor just above the card's top edge so the dialog
          // sits close to the card, not floating far above it.
          worldPos.y += 0.3;
          const projected = worldPos.clone().project(sceneManager.camera);
          const rect = canvas.getBoundingClientRect();
          const sx = (projected.x * 0.5 + 0.5) * rect.width;
          const sy = (-projected.y * 0.5 + 0.5) * rect.height;
          // If behind camera (projected.z > 1), hide.
          if (projected.z < 1) {
            targetingScreenPos = { x: sx, y: sy };
          } else {
            targetingScreenPos = null;
          }
        } else {
          targetingScreenPos = null;
        }
      } else if (targetingScreenPos) {
        targetingScreenPos = null;
      }

      if (sceneManager?.camera) {
        const pos = sceneManager.camera.position;
        const instantFps = delta > 0 ? 1 / delta : 0;
        fps = fps <= 0 ? instantFps : fps * 0.9 + instantFps * 0.1;
        const orbit = cameraController.getOrbitState();
        const builderCounts =
          deckBuilderMode || isAwardsMode
            ? ` | lib ${deckBuilderLibrary.length} line ${ringCards.length} deck ${deckBuilderTableCards.length}`
            : "";
        cameraHud = `cam ${pos.x.toFixed(2)}, ${pos.y.toFixed(2)}, ${pos.z.toFixed(2)} | r ${orbit.radius.toFixed(2)} h ${orbit.height.toFixed(2)} | fov ${sceneManager.camera.fov.toFixed(0)} | fps ${fps.toFixed(0)}${deckBuilderMode ? " | deck_builder" : isAwardsMode ? " | awards" : ""}${manualCamera ? " | manual" : ""}${builderCounts}`;
      }
    };
    animFrameId = requestAnimationFrame(animate);

    return () => {
      cancelAnimationFrame(animFrameId);
      sceneManager.dispose();
      inputManager.dispose();
      keyboardController.dispose();
      packMachine.dispose();
      buttonManager.dispose();
      hudManager.dispose();
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
      window.clearInterval(fadeInterval);
    };
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return;
    if (['q', 'w', 'e', 'r'].includes(e.key.toLowerCase())) {
      interactionController?.handleActionHotkey(e.key);
    }
    if (e.key.toLowerCase() === 'f') {
      interactionController?.handleInspectHotkey();
    }
  }}
/>

<div
  bind:this={container}
  class="absolute inset-0 h-full w-full overflow-hidden bg-slate-950"
>
  <canvas bind:this={canvas} class="h-full w-full cursor-default"></canvas>

  {#if !preloadDone && preloadTotal > 0}
    <div
      class="absolute inset-0 z-[200] flex flex-col items-center justify-center bg-slate-950/95 backdrop-blur-sm transition-opacity duration-500"
    >
      <div
        class="text-xs uppercase tracking-[0.3em] text-cyan-400 font-semibold mb-4"
      >
        Loading Assets
      </div>
      <div class="w-64 h-1.5 rounded-full bg-slate-800 overflow-hidden">
        <div
          class="h-full rounded-full bg-gradient-to-r from-cyan-500 to-cyan-300 transition-all duration-150 ease-out"
          style="width: {preloadTotal > 0
            ? (preloadProgress / preloadTotal) * 100
            : 0}%"
        ></div>
      </div>
      <div class="mt-2 text-[10px] font-mono text-slate-500">
        {preloadProgress} / {preloadTotal}
      </div>
    </div>
  {/if}

  <div
    class="pointer-events-none absolute bottom-4 right-4 z-30 rounded border border-slate-700/60 bg-black/70 px-3 py-2 text-[11px] font-mono text-slate-200"
  >
    {cameraHud}
  </div>

  <!-- Toolbar Buttons -->
  <div class="pointer-events-auto absolute top-4 right-4 z-[60] flex gap-2">
    {#if showGameElements}
      <button
        class="flex h-10 w-10 items-center justify-center rounded-lg border bg-slate-950/90 shadow-[0_0_15px_rgba(100,116,139,0.2)] transition-all hover:scale-110 active:scale-95 {showGameLog
          ? 'border-emerald-400/60 text-emerald-300 shadow-[0_0_25px_rgba(16,185,129,0.3)]'
          : 'border-slate-500/50 text-slate-300 hover:border-emerald-400/60 hover:text-emerald-300 hover:shadow-[0_0_25px_rgba(16,185,129,0.3)]'}"
        onclick={() => (showGameLog = !showGameLog)}
        title="Game Log"
      >
        <svg
          class="h-5 w-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9h8m-8 4h5"
          />
        </svg>
      </button>
    {/if}
    <button
      class="flex h-10 w-10 items-center justify-center rounded-lg border border-slate-500/50 bg-slate-950/90 text-slate-300 shadow-[0_0_15px_rgba(100,116,139,0.2)] transition-all hover:scale-110 hover:border-amber-400/60 hover:text-amber-300 hover:shadow-[0_0_25px_rgba(245,158,11,0.3)] active:scale-95"
      onclick={() => {
        showPacks = packMachine?.toggle(showPacks, submitAction);
      }}
      title="Booster Packs"
    >
      <svg
        class="h-5 w-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
        />
      </svg>
    </button>
    <button
      class="flex h-10 w-10 items-center justify-center rounded-lg border border-slate-500/50 bg-slate-950/90 text-slate-300 shadow-[0_0_15px_rgba(100,116,139,0.2)] transition-all hover:scale-110 hover:border-cyan-400/60 hover:text-cyan-300 hover:shadow-[0_0_25px_rgba(6,182,212,0.3)] active:scale-95"
      onclick={() => (showHelp = true)}
      title="How to Play"
    >
      <svg
        class="h-5 w-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
    </button>
    <button
      class="flex h-10 w-10 items-center justify-center rounded-lg border border-slate-500/50 bg-slate-950/90 text-slate-300 shadow-[0_0_15px_rgba(100,116,139,0.2)] transition-all hover:scale-110 hover:border-slate-400 hover:text-slate-100 hover:shadow-[0_0_25px_rgba(100,116,139,0.3)] active:scale-95"
      onclick={() => {
        if (!document.fullscreenElement) {
          container?.requestFullscreen();
          isFullscreen = true;
        } else {
          document.exitFullscreen();
          isFullscreen = false;
        }
      }}
      title="Fullscreen (Alt+Enter)"
    >
      {#if isFullscreen}
        <svg
          class="h-5 w-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M9 9V4.5M9 9H4.5M9 9L3.75 3.75M9 15v4.5M9 15H4.5M9 15l-5.25 5.25M15 9h4.5M15 9V4.5M15 9l5.25-5.25M15 15h4.5M15 15v4.5m0-4.5l5.25 5.25"
          ></path>
        </svg>
      {:else}
        <svg
          class="h-5 w-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 8V4m0 0h4M4 4l5.5 5.5M20 8V4m0 0h-4m4 0l-5.5 5.5M4 16v4m0 0h4m-4 0l5.5-5.5M20 16v4m0 0h-4m4 0l-5.5-5.5"
          ></path>
        </svg>
      {/if}
    </button>
  </div>

  {#if showPacks && (packPhase === "inventory" || packPhase === "grab" || packPhase === "tear")}
    <div
      class="pointer-events-none absolute bottom-6 left-1/2 z-40 -translate-x-1/2"
    >
      <div
        class="rounded-xl border border-cyan-400/40 bg-slate-950/80 px-4 py-2 text-center shadow-lg shadow-cyan-500/10"
      >
        <div class="text-[10px] uppercase tracking-[0.14em] text-cyan-300/90">
          Drag Across Pack Seam To Tear
        </div>
        <div class="mt-1 h-1.5 w-56 overflow-hidden rounded bg-slate-800/80">
          <div
            class="h-full bg-gradient-to-r from-cyan-500 to-emerald-400 transition-all duration-75"
            style={`width: ${Math.round(packTearProgress * 100)}%`}
          ></div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Hand Trigger Indicator -->
  {#if !deckBuilderMode && !isAwardsMode && hand.length > 0 && scry.length === 0}
    <div
      class="pointer-events-none absolute bottom-0 left-1/2 flex h-1 w-48 -translate-x-1/2 transition-all duration-300"
      style="
        opacity: {pointerPos.y < -0.7
        ? pointerPos.y < -0.82
          ? 1.0
          : 0.4
        : 0.15};
        background: linear-gradient(90deg, transparent, #00e5ff, transparent);
        box-shadow: 0 -2px 12px #00e5ff;
        transform: translate(-50%, 0) scaleX({pointerPos.y < -0.82
        ? 1.2
        : 1.0});
      "
    ></div>
  {/if}

  <!-- Pending Trigger Target Banner -->
  {#if pendingTriggerTarget}
    <div
      class="{pendingTriggerTarget.optional
        ? 'pointer-events-auto'
        : 'pointer-events-none'} absolute top-6 left-1/2 -translate-x-1/2 z-50"
    >
      <div
        class="rounded-2xl border-2 border-cyan-500/50 bg-gradient-to-b from-cyan-950/90 to-black/90 backdrop-blur-md px-6 py-4 space-y-3 text-center shadow-xl shadow-cyan-500/20"
      >
        <div class="flex items-center justify-center gap-2">
          <svg
            class="w-5 h-5 text-cyan-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13 10V3L4 14h7v7l9-11h-7z"
            />
          </svg>
          <div
            class="text-[10px] uppercase tracking-widest text-cyan-400 font-semibold"
          >
            Select Target
          </div>
        </div>
        <div
          class="text-xs text-cyan-300/90 italic border-t border-cyan-500/20 pt-2"
        >
          {pendingTriggerTarget.source_card_name}
        </div>
        <div class="text-sm text-white font-medium">
          {pendingTriggerTarget.effect_text}
        </div>
        <div class="text-xs text-slate-400 bg-black/30 rounded-lg px-3 py-1.5">
          {#if pendingTriggerTarget.target_kind === "player"}
            Click an opponent to target them
          {:else if pendingTriggerTarget.target_kind === "card"}
            Click a permanent on the battlefield
          {:else if pendingTriggerTarget.target_kind === "unit"}
            Click a unit on the battlefield
          {:else}
            Click a player or unit on the battlefield
          {/if}
        </div>
        {#if pendingTriggerTarget.optional}
          <button
            class="px-4 py-1.5 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
            onclick={() => submitAction("games.cardgame.trigger_target_cancel", {})}
          >
            Skip (Esc)
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Pending Trigger Choice Banner -->
  {#if pendingTriggerChoice}
    <div
      class="pointer-events-auto absolute top-6 left-1/2 -translate-x-1/2 z-50"
    >
      <div
        class="rounded-2xl border-2 border-amber-500/50 bg-gradient-to-b from-amber-950/90 to-black/90 backdrop-blur-md px-6 py-4 space-y-3 text-center shadow-xl shadow-amber-500/20"
      >
        <div class="flex items-center justify-center gap-2">
          <svg
            class="w-5 h-5 text-amber-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01"
            />
          </svg>
          <div
            class="text-[10px] uppercase tracking-widest text-amber-400 font-semibold"
          >
            Trigger Choice
          </div>
        </div>
        <div
          class="text-xs text-amber-300/90 italic border-t border-amber-500/20 pt-2"
        >
          {pendingTriggerChoice.source_card_name}
        </div>
        <div class="text-sm text-white font-medium">
          {pendingTriggerChoice.effect_text}
        </div>
        {#if pendingTriggerChoice.cost_text}
          <div class="text-xs text-slate-400 bg-black/30 rounded-lg px-3 py-1.5">
            Cost: {pendingTriggerChoice.cost_text}
          </div>
        {/if}
        <div class="flex gap-3 justify-center pt-1">
          <button
            class="px-4 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-500 text-white text-sm font-medium transition-colors"
            onclick={() => submitAction("games.cardgame.trigger_choice", { accept: true })}
          >
            Accept
          </button>
          <button
            class="px-4 py-1.5 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
            onclick={() => submitAction("games.cardgame.trigger_choice", { accept: false })}
          >
            Decline
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Pending Sacrifice Banner -->
  {#if pendingSacrifice}
    <div
      class="pointer-events-auto absolute top-6 left-1/2 -translate-x-1/2 z-50"
    >
      <div
        class="rounded-2xl border-2 border-orange-500/50 bg-gradient-to-b from-orange-950/90 to-black/90 backdrop-blur-md px-6 py-4 space-y-3 text-center shadow-xl shadow-orange-500/20"
      >
        <div class="flex items-center justify-center gap-2">
          <svg
            class="w-5 h-5 text-orange-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M17.657 18.657A8 8 0 016.343 7.343S7 9 9 10c0-2 .5-5 2.986-7C14 5 16.09 5.777 17.656 7.343A7.975 7.975 0 0120 13a7.975 7.975 0 01-2.343 5.657z"
            />
          </svg>
          <div
            class="text-[10px] uppercase tracking-widest text-orange-400 font-semibold"
          >
            Sacrifice Required
          </div>
        </div>
        <div
          class="text-xs text-orange-300/90 italic border-t border-orange-500/20 pt-2"
        >
          {pendingSacrifice.source_card_name}
        </div>
        <div class="text-sm text-white font-medium">
          Sacrifice {pendingSacrifice.remaining} unit{pendingSacrifice.remaining !==
          1
            ? "s"
            : ""}
        </div>
        <div class="text-xs text-slate-400 bg-black/30 rounded-lg px-3 py-1.5">
          Drag a unit to the sacrifice zone
        </div>
      </div>
    </div>
  {/if}

  <!-- Discard prompt is rendered inside the 3D scene via ButtonManager.
       The old HTML overlay was retired. -->
  {#if false && pendingDiscard && !discardCommitted}
    <div
      class="pointer-events-auto absolute top-6 left-1/2 -translate-x-1/2 z-50 flex items-start gap-4"
    >
      <div
        class="rounded-2xl border-2 border-purple-500/50 bg-gradient-to-b from-purple-950/90 to-black/90 backdrop-blur-md px-6 py-4 space-y-3 text-center shadow-xl shadow-purple-500/20 max-w-sm"
      >
        <div class="flex items-center justify-center gap-2">
          <svg
            class="w-5 h-5 text-purple-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <div
            class="text-[10px] uppercase tracking-widest text-purple-400 font-semibold"
          >
            Required Discard
          </div>
        </div>
        {#if !pendingDiscard.source_card?.image_url && pendingDiscard.source_description}
          <div
            class="text-xs text-purple-300/90 italic border-t border-purple-500/20 pt-2"
          >
            {pendingDiscard.source_description}
          </div>
        {/if}
        <div class="text-sm text-white font-medium">
          Select {pendingDiscard.amount - discardSelected.size} more card{pendingDiscard.amount -
            discardSelected.size !==
          1
            ? "s"
            : ""} to discard
        </div>
        <div class="text-xs text-slate-400 bg-black/30 rounded-lg px-3 py-1.5">
          {#if isDragging}
            Release over the discard zone to discard the dragged card
          {:else}
            Click cards in your hand to select them, or drag to the discard zone
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <ModifierTooltip
    card={hoveredCard}
    actionHints={hoveredActionHints}
    pointerPos={mousePixelPos}
    onHighlightCard={(id) => {
      highlightedCardId = id;
    }}
  />

  {#if showGameElements && revealedCards.length > 0}
    <div class="pointer-events-none absolute inset-0 z-40 flex items-center justify-center">
      <div class="flex flex-col items-center gap-6">
        {#each revealedCards as rc (rc.card.id)}
          <div class="flex flex-col items-center gap-3 animate-reveal-pop">
            <div class="rounded-md border border-amber-400/60 bg-black/80 px-5 py-2 font-mono text-sm text-white shadow-[0_0_28px_rgba(251,191,36,0.35)] backdrop-blur">
              <span class="uppercase tracking-[0.18em] text-amber-300/90">{rc.owner_name}</span>
              <span class="mx-2 text-slate-400">reveals</span>
              <span class="font-semibold">{rc.card.name}</span>
            </div>
            {#if rc.card.image_url}
              <img
                src={rc.card.image_url}
                alt={rc.card.name}
                class="h-64 w-auto rounded-lg border-2 border-amber-400/60 shadow-[0_0_48px_rgba(251,191,36,0.4)]"
              />
            {/if}
            <div class="rounded-md border border-slate-500/50 bg-black/80 px-4 py-1 font-mono text-xs text-slate-200 backdrop-blur">
              {rc.outcome_label}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if showGameLog && showGameElements && gameState?.game_log && gameState.game_log.length > 0}
    <div class="pointer-events-auto absolute left-4 top-4 z-40 w-80 max-w-[calc(100vw-2rem)] max-h-[65vh] overflow-y-auto rounded-xl border border-cyan-500/20 bg-slate-950/85 backdrop-blur-md shadow-2xl">
      <div class="sticky top-0 bg-slate-950/95 px-3 py-2 border-b border-cyan-500/20 space-y-2">
        <div class="flex items-center justify-between gap-3">
          <div>
            <div class="font-mono text-[10px] uppercase tracking-[0.18em] text-cyan-300/80">
              Game Log
            </div>
            <div class="font-mono text-[10px] text-slate-400">
              {#if selectedLogRound != null}
                Round {selectedLogRound}
                {#if currentLogRound != null && selectedLogRound === currentLogRound}
                  <span class="text-emerald-300/80">LIVE</span>
                {/if}
              {/if}
            </div>
          </div>
          <div class="flex items-center gap-1">
            <button
              class="rounded border border-white/10 bg-white/5 px-2 py-1 font-mono text-[10px] text-slate-200 transition hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-40"
              onclick={() => jumpLogRound(-1)}
              disabled={
                selectedLogRound == null ||
                availableLogRounds.indexOf(selectedLogRound) <= 0
              }
            >
              Prev
            </button>
            <button
              class="rounded border border-cyan-500/20 bg-cyan-500/10 px-2 py-1 font-mono text-[10px] text-cyan-100 transition hover:bg-cyan-500/15 disabled:cursor-not-allowed disabled:opacity-40"
              onclick={resetLogRound}
              disabled={selectedLogRound == null || selectedLogRound === currentLogRound}
            >
              Current
            </button>
            <button
              class="rounded border border-white/10 bg-white/5 px-2 py-1 font-mono text-[10px] text-slate-200 transition hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-40"
              onclick={() => jumpLogRound(1)}
              disabled={
                selectedLogRound == null ||
                availableLogRounds.indexOf(selectedLogRound) === availableLogRounds.length - 1
              }
            >
              Next
            </button>
          </div>
        </div>
      </div>
      <div class="px-3 py-2 space-y-2">
        {#if visibleGameLogEntries.length === 0}
          <div class="rounded-lg border border-dashed border-white/10 bg-white/5 px-3 py-4 text-center font-mono text-[10px] text-slate-400">
            No log entries for this round yet.
          </div>
        {:else}
          {#each visibleGameLogEntries as entry, i (entry.round + '-' + i)}
            <div class="rounded-lg border border-white/8 bg-black/30 px-2.5 py-2">
              <div class="mb-1 flex items-center gap-2 font-mono text-[10px]">
                <span class="text-slate-500">R{entry.round}</span>
                <span
                  class={`rounded border px-1.5 py-0.5 tracking-[0.16em] ${gameLogKindTone(entry.kind)}`}
                >
                  {gameLogKindLabel(entry.kind)}
                </span>
                <span class="text-cyan-200 font-semibold">{entry.player_name}</span>
              </div>
              <div class="font-mono text-[10px] leading-relaxed text-slate-200">
                {entry.message}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  {#if (showGameElements && displayInspectedCard) || (cardBrowserMode && (displayInspectedCard || hoveredCard))}
    <InspectOverlay
      inspectedCard={displayInspectedCard ?? (cardBrowserMode ? hoveredCard : null)}
      hoveredCard={null}
      {pointerPos}
      onHighlightCard={(id) => {
        highlightedCardId = id;
      }}
    />
  {/if}

  {#if showGameElements}
    <GameHudOverlay
      {players}
      viewerId={viewer?.id}
      ruleset={gameState?.ruleset ?? 'mana_clash'}
      onOpenDiscard={(playerId) => submitAction('games.cardgame.open_card_browser', { target_player_id: playerId })}
      canTargetPlayer={(playerId) =>
        !!targetingSource && interactionController?.isValidTargetPlayer?.(
          {
            source: targetingSource,
            mode: targetingMode,
            abilityId: targetingAbilityId,
          },
          playerId,
        ) === true
      }
      onTargetPlayer={(playerId) =>
        interactionController?.targetPlayer?.(playerId) ?? false
      }
    />
  {/if}

  {#if !deckBuilderMode && !deckSelectMode && !isAwardsMode}
    <TargetingOverlay
      {targetingSource}
      {targetingMode}
      {targetingAbility}
      {hoveredCard}
      {targetingScreenPos}
      combatPairs={gameState?.combat_pairs ?? []}
      onCancel={() => {
        clearTargeting();
      }}
    />

    {#if targetingError}
      <div
        class="pointer-events-none absolute inset-x-0 bottom-48 z-50 flex justify-center"
      >
        <div
          class="rounded-full border border-rose-400/40 bg-rose-950/80 px-5 py-1.5 text-xs font-semibold text-rose-200 shadow-lg backdrop-blur-sm"
        >
          {targetingError}
        </div>
      </div>
    {/if}

    <ActionMenu
      card={actionMenu?.card}
      actions={actionMenu?.actions ?? []}
      x={actionMenu?.x ?? 0}
      y={actionMenu?.y ?? 0}
      onSelect={(action) => {
        if (!actionMenu) return;
        action.execute(actionMenu.card, {
          submitAction,
          setTargeting: (source, mode, abilityId) => {
            targetingSource = source;
            targetingMode = mode;
            targetingAbilityId = abilityId ?? null;
          },
        });
        actionMenu = null;
      }}
      onClose={() => {
        actionMenu = null;
      }}
    />

    <ActionPanel
      workflowId={workflow.instance_id}
      stage={gameState?.stage}
      combatPhase={gameState?.combat_phase}
      stackLength={(gameState?.stack || []).length}
      stackTop={(gameState?.stack || [])[0] || null}
      viewerId={viewer?.id}
      isActive={!!viewer?.is_active}
      isCurrentBlocker={gameState?.combat_phase === "declare_blockers" &&
        gameState?.combat_blocker_id === viewer?.id}
      hide={handHoverActive && pointerPos.y < -0.85}
      show={!!(
        viewer?.is_active ||
        viewer?.pending_discard ||
        gameState?.stage === "lobby" ||
        gameState?.stage === "draft_building" ||
        (gameState?.stack || []).length > 0 ||
        (gameState?.combat_phase === "declare_blockers" &&
          gameState?.combat_blocker_id === viewer?.id)
      )}
      pendingDiscard={pendingDiscard && !discardCommitted ? pendingDiscard : null}
      discardSelectedCount={discardSelected.size}
      onDiscardCommit={submitDiscardCommit}
    />
  {/if}

  {#if isDraftMode && draftView}
    <DraftOverlay
      {draftView}
      canAutoDraft={!!viewer}
      onAutoDraft={() => submitAction("games.cardgame.draft_auto_pick", {})}
    />
  {/if}

  {#if isAwardsMode && awardsView}
    <AwardsOverlay
      {awardsView}
      selectedCount={awardsSelected.size}
      focusedCard={builderSelectedCard}
      selectedIds={awardsSelected}
      onToggleFocused={() => {
        if (builderSelectedCard) toggleAwardsSelection(builderSelectedCard.id);
      }}
      onSubmit={submitAwards}
    />
  {/if}

  {#if gameState?.match_state && gameState.stage !== 'lobby' && gameState.stage !== 'sideboard_swap'}
    <MatchScoreBanner
      matchState={gameState.match_state}
      players={gameState.players || []}
      viewerId={viewer?.id ?? null}
    />
  {/if}

  {#if deckBuilderMode && !operatorSelectMode && gameState?.deck_builder?.deck_id}
    <DeckBuilderSideboardPanel
      sideboard={gameState.deck_builder.sideboard || []}
      deckId={gameState.deck_builder.deck_id}
      sideboardMode={builderSideboardMode}
      onToggleMode={(active) => (builderSideboardMode = active)}
      onAdd={(cardId) =>
        submitAction('games.cardgame.builder_add_sideboard_card', {
          deck_id: gameState.deck_builder.deck_id,
          card_id: cardId,
        })}
      onRemove={(cardId) =>
        submitAction('games.cardgame.builder_remove_sideboard_card', {
          deck_id: gameState.deck_builder.deck_id,
          card_id: cardId,
        })}
    />
  {/if}

  {#if gameState?.stage === 'sideboard_swap' && viewer}
    {#key gameState.match_state?.current_game ?? 0}
      {#if gameState.match_state}
        <InterGameBanner
          matchState={gameState.match_state}
          players={gameState.players || []}
          viewerId={viewer.id ?? null}
        />
      {/if}
    {/key}
    <SideboardOverlay
      mainDeck={gameState.viewer_main_deck || []}
      sideboard={gameState.viewer_sideboard || []}
      matchState={gameState.match_state}
      viewerId={viewer.id ?? null}
      onSubmit={(swaps: { main_out: string; side_in: string }[]) =>
        submitAction('games.cardgame.submit_sideboard', { swaps })}
    />
  {/if}

  {#if displayInspectedCard}
    {@const spec = displayInspectedCard.visual_spec}
    {@const fallbackSpec = fallbackVisualSpec(displayInspectedCard)}
    {@const isHolo = spec?.holo ?? false}
    {@const isOwnedCard = displayInspectedCard.owned !== false}
    {@const seedValue = spec?.seed ?? fallbackSpec.seed}
    {@const wearValue = spec?.wear ?? fallbackSpec.wear}
    <div
      class="absolute bottom-4 left-4 z-50 pointer-events-none text-xs font-mono bg-black/60 border border-slate-600/40 rounded px-2.5 py-1.5 text-slate-400 flex gap-3"
    >
      {#if displayInspectedCard.owner_name}<span
          >{displayInspectedCard.owner_name}</span
        >{/if}
      {#if isHolo}<span class="text-yellow-300">HOLO</span>{/if}
      <span>wear {(wearValue * 100).toFixed(0)}%</span>
      <span
        >{isOwnedCard
          ? `seed ${seedValue}`
          : `unowned, seed ${seedValue}`}</span
      >
    </div>
  {/if}

  <!-- Trigger Log Feed -->
  {#if visibleTriggers.length > 0}
    {@const groupedTriggers = visibleTriggers.reduce((acc: any[], entry) => {
      const last = acc[acc.length - 1];
      if (
        last &&
        last.source_card_id === entry.source_card_id &&
        last.effect_description === entry.effect_description
      ) {
        last.count = (last.count ?? 1) + 1;
      } else {
        acc.push({ ...entry, count: 1 });
      }
      return acc;
    }, [])}
    <div class="pointer-events-none absolute bottom-16 left-4 z-40 flex flex-col gap-1.5">
      {#each groupedTriggers as entry}
        <div class="flex items-center gap-2 rounded-lg border border-amber-500/30 bg-slate-950/90 px-3 py-1.5 shadow-[0_0_12px_rgba(245,158,11,0.1)] backdrop-blur-md animate-in slide-in-from-left-2 fade-in-0 duration-300">
          <div class="h-1.5 w-1.5 rounded-full bg-amber-400 animate-pulse"></div>
          <span class="font-mono text-[11px] font-semibold text-amber-200">{entry.source_card_name}</span>
          <span class="font-mono text-[11px] text-slate-400">&mdash;</span>
          <span class="font-mono text-[11px] text-slate-300">{entry.effect_description}</span>
          {#if entry.count > 1}
            <span class="ml-1 rounded-full bg-amber-500/20 px-1.5 py-0.5 font-mono text-[9px] font-semibold text-amber-200">×{entry.count}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if showHelp}
    <HelpOverlay onClose={() => (showHelp = false)} />
  {/if}

  {#if flashRarity}
    <div
      class="pointer-events-none fixed inset-0 z-[100]"
      style="background: {flashRarity === 'singularity'
        ? '#facc15'
        : '#f43f5e'}; opacity: 0.3; animation: flash-fade 400ms ease-out forwards;"
    ></div>
  {/if}
</div>

<style>
  @keyframes flash-fade {
    0% {
      opacity: 0.3;
    }
    100% {
      opacity: 0;
    }
  }
  @keyframes reveal-pop {
    0% { opacity: 0; transform: scale(0.6); }
    12% { opacity: 1; transform: scale(1.05); }
    22% { transform: scale(1); }
    85% { opacity: 1; transform: scale(1); }
    100% { opacity: 0; transform: scale(0.95); }
  }
  :global(.animate-reveal-pop) {
    animation: reveal-pop 4s ease-out forwards;
  }
</style>

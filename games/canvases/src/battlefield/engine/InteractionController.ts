import * as THREE from 'three';
import type { InputManager } from './InputManager';
import type { ZoneManager } from './ZoneManager';
import type { CardActor } from '../entities/CardActor';
import type { PlayerActor } from '../entities/PlayerActor';

import type { CardView } from '../types';
import {
  getApplicableActions,
  type CardAction,
  type InteractionContext,
} from '../actions';

export type ActionHint = { key: string; label: string; disabled?: boolean };

export class InteractionController {
  public isInspecting = false;
  public onInspectDrag?: (dx: number, dy: number) => void;

  private draggingActor: CardActor | null = null;
  private dragPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  private hoveredActor: CardActor | null = null;
  private hoveredPlayerActor: PlayerActor | null = null;
  private hoveredButton: THREE.Object3D | null = null;
  private draggingScryId: string | null = null;
  private targetHintActor: CardActor | null = null;
  private targetHintPlayerActor: PlayerActor | null = null;
  // All actors currently highlighted as valid targets for the in-progress
  // targeting action. Updated every tick while targeting is active so new /
  // removed valid targets stay in sync.
  private validTargetActors = new Set<CardActor>();
  private validTargetPlayerActors = new Set<PlayerActor>();
  // Actors currently highlighted as the target of an item on the stack
  // (e.g. an opponent's in-flight spell). Separate from the in-progress
  // targeting sets so the two systems don't stomp each other.
  private stackTargetActors = new Set<CardActor>();
  private stackTargetPlayerActors = new Set<PlayerActor>();
  // The actor being highlighted as the targeting "source" (the card using
  // the ability / casting the spell). Local-only: opponents don't see it.
  private targetSourceActor: CardActor | null = null;
  private updateCounter = 0;
  private ringDragActive = false;
  private ringDragLastX = 0;
  private ringDragLastY = 0;
  private ringDragCarry = 0;
  private ringDragStepPx = 16;
  private ringDragMode: 'ring' | 'kind' = 'ring';
  private packTearActive = false;
  private packTearTypeId: string | null = null;
  private packTearStartX = 0;
  private packTearStartY = 0;
  private packTearProgress = 0;
  private packTearDirection: -1 | 1 = 1;
  private packTearLastLoggedPct = -1;
  private pendingTargets: Array<{ kind: 'card' | 'player'; id: string }> = [];

  public clearPendingTargets() {
    this.pendingTargets = [];
  }

  /** Returns a human-readable reason why this card can't be targeted in
   *  the given mode, or null if it can. Matches backend rules:
   *  - Stealth: only prevents blocking, NOT spell/ability targeting.
   *  - Camouflage: prevents targeting by opponents' spells/abilities.
   */
  private whyCantTarget(
    card: CardView,
    mode?: 'cast' | 'attack' | 'block' | 'activate' | null,
  ): string | null {
    const keywords: any[] = card.keywords ?? [];
    const has = (kw: string) =>
      keywords.some((k: any) => {
        const s = typeof k === 'string' ? k : (k?.key ?? k?.text ?? '');
        return String(s).toLowerCase() === kw.toLowerCase();
      });
    // Stealth only blocks… blocking.
    if (mode === 'block' && has('stealth')) {
      return `${card.name} has Stealth and can't be blocked.`;
    }
    // Camouflage prevents opponent targeting (spells & abilities).
    if (mode !== 'block' && has('camouflage')) {
      return `${card.name} has Camouflage and can't be targeted.`;
    }
    return null;
  }

  public getPendingTargets(): ReadonlyArray<{ kind: 'card' | 'player'; id: string }> {
    return this.pendingTargets;
  }

  public getHoveredActorId(): string | null {
    return this.hoveredActor?.id ?? null;
  }

  constructor(
    private input: InputManager,
    private zones: ZoneManager,
    private onSubmitAction: (action: string, payload: any) => void,
    private onHover?: (card: CardView | null, actionHints?: ActionHint[] | null) => void,
    private onRipple?: (x: number, z: number) => void,
    private getTargeting?: () => {
      source: CardView | null;
      mode: 'cast' | 'attack' | 'block' | 'activate' | null;
      abilityId?: string | null;
    },
    private setTargeting?: (
      source: CardView | null,
      mode: 'cast' | 'attack' | 'block' | 'activate' | null,
      abilityId?: string | null,
    ) => void,
    private isScryActive?: () => boolean,
    private getScryTarget?: (
      cardId: string,
    ) => { zone: 'pool' | 'top' | 'bottom'; index: number } | null,
    private moveScryCard?: (
      cardId: string,
      zone: 'pool' | 'top' | 'bottom',
      index?: number,
    ) => void,
    private endScryDrag?: () => void,
    private getViewerId?: () => string | null,
    private onScryDrop?: (
      cardId: string,
      zone: 'top' | 'bottom' | 'pool',
      beforeId?: string | null,
    ) => void,
    private onActionMenu?: (
      card: CardView,
      actions: CardAction[],
      screenPos: { x: number; y: number },
    ) => void,
    private getDeckBuilderContext?: () => {
      enabled: boolean;
      deckId: string | null;
    },
    private onDeckBuilderDrop?: (cardId: string, deckId: string) => void,
    private getDeckSelectContext?: () => { enabled: boolean },
    private onDeckSelectDrop?: (
      deckId: string,
      mode: 'as_is' | 'customize',
    ) => void,
    private getExtraInteractables?: () => THREE.Object3D[],
    private onRingDragStep?: (step: -1 | 1) => void,
    private onFilterDragStep?: (step: -1 | 1) => void,
    private onDragStateChange?: (
      dragging: boolean,
      cardId: string | null,
    ) => void,
    private onDangerDrop?: (cardId: string) => void,
    private onDangerHover?: (hovered: boolean) => void,
    private onDiscardHover?: (hovered: boolean) => void,
    private onSacrificeHover?: (hovered: boolean) => void,
    private onInspect?: (card: CardView | null) => void,
    private onPackReveal?: (index: number) => void,
    private onPackTearStart?: (packTypeId: string) => void,
    private onPackTearProgress?: (progress: number) => void,
    private onPackTearCommit?: (packTypeId: string) => void,
    private onPackTearCancel?: () => void,
    private getPendingDiscard?: () => {
      active: boolean;
      handIds: Set<string>;
    },
    private onDiscardToggle?: (cardId: string) => void,
    private getPendingSacrifice?: () => boolean,
    private onTargetingRejected?: (reason: string) => void,
    // Returns IDs of cards/players currently being targeted by any item
    // on the stack so they can be highlighted — lets the player see what
    // an opponent's in-flight spell is aiming at before it resolves.
    private getStackTargets?: () => Array<{ kind: 'card' | 'player'; id: string }>,
  ) {
    // Hook into input events
    // We'll poll in update() for hovers, but use events for clicks/drags
    this.input.onClick = this.handleClick;
    this.input.onDragStart = this.handleDragStart;
    this.input.onDragMove = this.handleDragMove;
    this.input.onDragEnd = this.handleDragEnd;
  }

  public update() {
    this.updateCounter++;
    // Hover Logic - Throttle to every 3 frames (~20Hz)
    // Only perform hover checks if NOT actively dragging or ring-scrolling
    if (
      !this.draggingActor &&
      !this.ringDragActive &&
      this.updateCounter % 3 === 0
    ) {
      const extraInteractables = this.getExtraInteractables?.() ?? [];
      const buttonHit =
        extraInteractables.length > 0
          ? this.input.getIntersection(extraInteractables)
          : null;
      if (buttonHit) {
        let buttonObj: THREE.Object3D | null = buttonHit.object;
        while (buttonObj && !buttonObj.userData.isButton) {
          buttonObj = buttonObj.parent;
        }
        if (buttonObj?.userData?.isButton) {
          if (buttonObj?.userData?.disableHover) {
            this.hoveredActor?.setHovered(false);
            this.hoveredActor = null;
            this.hoveredPlayerActor?.setHovered(false);
            this.hoveredPlayerActor = null;
            this.onHover?.(null);
            this.setDeckDropHover(null);
            this.hoveredButton = buttonObj;
            return;
          }
          if (this.hoveredButton && this.hoveredButton !== buttonObj) {
            this.setButtonHover(this.hoveredButton, false);
          }
          if (this.hoveredButton !== buttonObj) {
            this.setButtonHover(buttonObj, true);
          }
          this.hoveredButton = buttonObj;
          this.hoveredActor?.setHovered(false);
          this.hoveredActor = null;
          this.hoveredPlayerActor?.setHovered(false);
          this.hoveredPlayerActor = null;
          this.onHover?.(null);
          this.setDeckDropHover(null);
          return;
        }
      }

      if (this.hoveredButton) {
        this.setButtonHover(this.hoveredButton, false);
        this.hoveredButton = null;
      }

      const interactables = [
        ...this.zones.getInteractables(),
        ...extraInteractables,
      ];
      const hit = this.input.getIntersection(interactables);

      if (hit) {
        let obj: THREE.Object3D | null = hit.object;

        // 1. Check for Buttons First
        let buttonObj: THREE.Object3D | null = obj;
        while (buttonObj && !buttonObj.userData.isButton) {
          buttonObj = buttonObj.parent;
        }

        if (buttonObj?.userData?.isButton) {
          this.setButtonHover(buttonObj, true);
          if (this.hoveredButton && this.hoveredButton !== buttonObj) {
            this.setButtonHover(this.hoveredButton, false);
          }
          this.hoveredButton = buttonObj;
          // Clear card/player hover
          this.hoveredActor?.setHovered(false);
          this.hoveredActor = null;
          this.hoveredPlayerActor?.setHovered(false);
          this.hoveredPlayerActor = null;
          this.onHover?.(null);
          this.setDeckDropHover(null);
          return;
        }
        if (this.hoveredButton) {
          this.setButtonHover(this.hoveredButton, false);
          this.hoveredButton = null;
        }

        // 2. Check for Deck Select Drops
        let dropObj: THREE.Object3D | null = obj;
        while (dropObj && !dropObj.userData?.deckSelectDrop) {
          dropObj = dropObj.parent ?? null;
        }
        if (dropObj?.userData?.deckSelectDrop) {
          const deckSelect = this.getDeckSelectContext?.();
          if (deckSelect?.enabled) {
            const kind = dropObj.userData.deckSelectDrop as
              | 'as_is'
              | 'customize';
            this.setDeckDropHover(kind);
            this.hoveredActor?.setHovered(false);
            this.hoveredActor = null;
            this.hoveredPlayerActor?.setHovered(false);
            this.hoveredPlayerActor = null;
            this.onHover?.(null);
            return;
          }
        }
        this.setDeckDropHover(null);

        // 3. Traversal for Actors
        while (obj && !obj.userData.actor && !obj.userData.playerActor) {
          obj = obj.parent;
        }

        const actor = obj?.userData?.actor as CardActor | undefined;
        const playerActor = obj?.userData?.playerActor as PlayerActor | undefined;

        if (actor !== this.hoveredActor) {
          this.hoveredActor?.setHovered(false);
          this.hoveredActor = actor || null;
          this.hoveredActor?.setHovered(true);
          const card = this.hoveredActor ? this.hoveredActor.getCard() : null;
          this.onHover?.(card, this.getActionHints(card));
        }

        if (playerActor !== this.hoveredPlayerActor) {
          this.hoveredPlayerActor?.setHovered(false);
          this.hoveredPlayerActor = playerActor || null;
          this.hoveredPlayerActor?.setHovered(true);
          // If we hover a player, clear card info
          if (this.hoveredPlayerActor) {
              this.onHover?.(null);
          }
        }
      } else {
        if (this.hoveredButton) {
          this.setButtonHover(this.hoveredButton, false);
          this.hoveredButton = null;
        }
        this.setDeckDropHover(null);
        this.hoveredActor?.setHovered(false);
        this.hoveredActor = null;
        this.hoveredPlayerActor?.setHovered(false);
        this.hoveredPlayerActor = null;
        this.onHover?.(null);
      }
    } else if (this.ringDragActive && this.hoveredButton) {
      // Immediate clear if dragging starts
      this.setButtonHover(this.hoveredButton, false);
      this.hoveredButton = null;
    }

    const targeting = this.getTargeting?.();

    // Clear old hover-specific target hints (keep the pre-highlighted pool
    // below — we only clear the single hovered-hint here because it may move).
    if (this.targetHintActor) {
      this.targetHintActor.setTargetHint(false);
      this.targetHintActor = null;
    }
    if (this.targetHintPlayerActor) {
      this.targetHintPlayerActor.setTargetHint(false);
      this.targetHintPlayerActor = null;
    }

    if (targeting?.source) {
      // Amber highlight on the source card (the one using the ability /
      // spell). Find its actor and update on every tick so the highlight
      // follows the card if it moves between zones.
      const sourceId = targeting.source.id;
      let nextSourceActor: CardActor | null = null;
      for (const actor of this.zones.getAllActors()) {
        const c = actor.getCard();
        if (c && c.id === sourceId) {
          nextSourceActor = actor;
          break;
        }
      }
      if (this.targetSourceActor && this.targetSourceActor !== nextSourceActor) {
        this.targetSourceActor.setTargetingSource(false);
      }
      if (nextSourceActor) {
        nextSourceActor.setTargetingSource(true);
      }
      this.targetSourceActor = nextSourceActor;

      // Highlight every valid target on the board (not only the hovered
      // one) so the player can see at a glance where they can click.
      const newValidActors = new Set<CardActor>();
      const newValidPlayers = new Set<PlayerActor>();

      // Use the backend's valid_target_ids list (computed with full game
      // rules — Camouflage, Hexproof, scope, etc.) instead of guessing
      // client-side. For cast/activate the list is on the source card or
      // ability; for attack/block we fall back to type-based checks.
      const backendValidIds = new Set<string>();
      if (targeting.mode === 'cast' || targeting.mode === 'activate') {
        let ids: string[] = [];
        if (targeting.mode === 'activate' && targeting.abilityId) {
          const ability = (targeting.source.abilities ?? []).find(
            (a: any) => a.id === targeting.abilityId,
          );
          ids = (ability as any)?.valid_target_ids ?? [];
        } else {
          ids = (targeting.source as any).valid_target_ids ?? [];
        }
        for (const id of ids) backendValidIds.add(id);
      }

      for (const actor of this.zones.getAllActors()) {
        const card = actor.getCard();
        if (!card) continue;
        const zone = actor.getZone?.();
        if (zone !== 'battlefield') continue;
        if (card.id === sourceId) continue;

        let valid = false;
        if (backendValidIds.size > 0) {
          // Backend-authoritative: only light up IDs the server says are valid.
          valid = backendValidIds.has(card.id);
        } else {
          // Fallback for attack/block (no backend list).
          valid = this.isValidTargetCard(targeting, card);
        }
        if (valid) {
          actor.setTargetHint(true);
          newValidActors.add(actor);
        }
      }

      const bfZone: any = this.zones.get('battlefield');
      const playerActors: PlayerActor[] =
        bfZone && typeof bfZone.getPlayerActors === 'function'
          ? bfZone.getPlayerActors()
          : [];
      for (const pa of playerActors) {
        const pid = (pa as any).id as string;
        if (this.isValidTargetPlayer(targeting, pid)) {
          pa.setTargetHint(true);
          newValidPlayers.add(pa);
        }
      }

      // Turn off any actors that are no longer valid (e.g. a card just left
      // the battlefield) so lingering highlights don't pile up.
      for (const actor of this.validTargetActors) {
        if (!newValidActors.has(actor)) {
          actor.setTargetHint(false);
        }
      }
      for (const pa of this.validTargetPlayerActors) {
        if (!newValidPlayers.has(pa)) {
          pa.setTargetHint(false);
        }
      }
      this.validTargetActors = newValidActors;
      this.validTargetPlayerActors = newValidPlayers;
    } else if (
      this.validTargetActors.size > 0 ||
      this.validTargetPlayerActors.size > 0 ||
      this.targetSourceActor
    ) {
      // Targeting ended — clear every highlight we applied.
      for (const actor of this.validTargetActors) {
        actor.setTargetHint(false);
      }
      for (const pa of this.validTargetPlayerActors) {
        pa.setTargetHint(false);
      }
      if (this.targetSourceActor) {
        this.targetSourceActor.setTargetingSource(false);
        this.targetSourceActor = null;
      }
      this.validTargetActors.clear();
      this.validTargetPlayerActors.clear();
    }

    // Independent pass: highlight the targets of whatever is on the stack
    // so the player can see what an opponent's in-flight spell is aiming at.
    const stackTargets = this.getStackTargets?.() ?? [];
    if (stackTargets.length > 0 || this.stackTargetActors.size > 0 || this.stackTargetPlayerActors.size > 0) {
      const wantCards = new Set<string>();
      const wantPlayers = new Set<string>();
      for (const t of stackTargets) {
        if (t.kind === 'card') wantCards.add(t.id);
        else if (t.kind === 'player') wantPlayers.add(t.id);
      }

      const nextCardActors = new Set<CardActor>();
      for (const actor of this.zones.getAllActors()) {
        const card = actor.getCard();
        if (!card) continue;
        if (!wantCards.has(card.id)) continue;
        const zone = actor.getZone?.();
        // Only highlight the on-battlefield copy (avoid lighting up graveyard/hand dupes).
        if (zone !== 'battlefield') continue;
        // Don't collide with the in-progress-targeting highlight — it already shows.
        if (this.validTargetActors.has(actor)) continue;
        actor.setTargetHint(true);
        nextCardActors.add(actor);
      }
      for (const actor of this.stackTargetActors) {
        if (!nextCardActors.has(actor) && !this.validTargetActors.has(actor)) {
          actor.setTargetHint(false);
        }
      }
      this.stackTargetActors = nextCardActors;

      const bfZone: any = this.zones.get('battlefield');
      const playerActors: PlayerActor[] =
        bfZone && typeof bfZone.getPlayerActors === 'function'
          ? bfZone.getPlayerActors()
          : [];
      const nextPlayerActors = new Set<PlayerActor>();
      for (const pa of playerActors) {
        const pid = (pa as any).id as string;
        if (!wantPlayers.has(pid)) continue;
        if (this.validTargetPlayerActors.has(pa)) continue;
        pa.setTargetHint(true);
        nextPlayerActors.add(pa);
      }
      for (const pa of this.stackTargetPlayerActors) {
        if (!nextPlayerActors.has(pa) && !this.validTargetPlayerActors.has(pa)) {
          pa.setTargetHint(false);
        }
      }
      this.stackTargetPlayerActors = nextPlayerActors;
    }
  }

  public isValidTargetPlayer(
    targeting: {
      source: CardView | null;
      mode: 'cast' | 'attack' | 'block' | 'activate' | null;
      abilityId?: string | null;
    },
    playerId: string,
  ) {
    if (targeting.mode === 'attack') {
      // Can't attack yourself.
      const viewerId = this.getViewerId?.();
      return !!viewerId && viewerId !== playerId;
    }
    if (targeting.mode === 'block') {
      return false;
    }

    if (!targeting.source) return false;

    let spec = targeting.source.target_spec;
    if (targeting.mode === 'activate' && targeting.abilityId) {
      const ability = targeting.source.abilities?.find(
        (a) => a.id === targeting.abilityId,
      );
      if (ability) {
        spec = ability.target_spec as any;
      }
    }

    if (!spec) return false;
    const kind = String(spec.kind || '').toLowerCase();
    if (kind !== 'player' && kind !== 'player_or_unit') return false;

    const viewerId = this.getViewerId?.();
    const scope = String((spec as any).scope || '').toLowerCase();
    if (scope === 'self_player' || scope === 'selfplayer') {
      return !!viewerId && playerId === viewerId;
    }
    if (scope === 'opponent') {
      return !!viewerId && playerId !== viewerId;
    }
    return true;
  }

  private isValidTargetCard(
    targeting: {
      source: CardView | null;
      mode: 'cast' | 'attack' | 'block' | 'activate' | null;
      abilityId?: string | null;
    },
    card: CardView,
  ) {
    const viewerId = this.getViewerId?.();
    const owner = (card as any)._owner as string | undefined;

    if (targeting.mode === 'attack') {
      // Can only attack enemy units/agents, not your own.
      if (!viewerId || !owner || owner === viewerId) return false;
      return card.kind_tag === 'unit' || card.kind_tag === 'agent';
    }
    if (targeting.mode === 'block') {
      // Can only block enemy attackers.
      if (!viewerId || !owner || owner === viewerId) return false;
      return card.kind_tag === 'unit';
    }

    if (!targeting.source) return false;

    let spec = targeting.source.target_spec;
    if (targeting.mode === 'activate' && targeting.abilityId) {
      const ability = targeting.source.abilities?.find(
        (a) => a.id === targeting.abilityId,
      );
      if (ability) {
        spec = ability.target_spec as any;
      }
    }

    if (!spec) return false;
    const kind = String(spec.kind || '').toLowerCase();
    const scope = String((spec as any).scope || '').toLowerCase();

    const isUnitish = card.kind_tag === 'unit' || card.kind_tag === 'agent';
    if (kind === 'unit' && !isUnitish) return false;
    if (kind === 'player_or_unit' && !isUnitish) return false;
    if (kind === 'player') return false;

    // Scope filter for units.
    if (viewerId && owner) {
      if (scope === 'self_player' || scope === 'selfplayer') {
        return owner === viewerId;
      }
      if (scope === 'opponent') {
        return owner !== viewerId;
      }
    }
    return true;
  }

  public getActionHints(card: CardView | null): ActionHint[] | null {
    if (!card) return null;

    // Forced discard: only the discard action is valid, so replace any
    // other hints with a single discard hint for cards in the hand pool.
    const pendingDiscard = this.getPendingDiscard?.();
    if (pendingDiscard?.active) {
      if (pendingDiscard.handIds.has(card.id)) {
        return [{ key: 'Q', label: 'Discard' }, { key: 'LMB', label: 'Discard' }];
      }
      return null;
    }

    const targeting = this.getTargeting?.();
    if (targeting?.source && this.setTargeting) {
      const label = card.id === targeting.source.id ? "Cancel Target" : "Select Target";
      return [{ key: 'LMB', label }];
    }

    let actions = getApplicableActions(card, {
      sacrificeRequired: !!this.getPendingSacrifice?.(),
    });
    if (actions.length === 0) return null;

    const zone = this.hoveredActor?.getZone?.() || null;
    const isHandCard = zone === 'hand';
    const isOperatorZoneCard = zone === 'operator';
    const isBattlefieldCard = zone === 'battlefield';

    if (isHandCard) {
      actions = actions.filter(
        (action) => action.id === 'play_land' || action.id === 'cast',
      );
    } else if (isOperatorZoneCard) {
      actions = actions.filter((action) => action.id === 'cast');
    } else if (isBattlefieldCard) {
      actions = actions.filter(
        (action) => action.id !== 'play_land' && action.id !== 'cast',
      );
    } else {
      actions = actions.filter((action) => action.id === 'cast');
    }

    if (actions.length === 0) return null;

    const keys = ['Q', 'W', 'E', 'R'];
    const hints: ActionHint[] = [];

    if (actions.length > 1 && this.onActionMenu) {
      hints.push({ key: 'LMB', label: 'Open Context Menu' });
    }

    for (let i = 0; i < actions.length && i < keys.length; i++) {
      hints.push({ key: keys[i], label: actions[i].label, disabled: actions[i].disabled });
    }

    return hints;
  }

  public handleActionHotkey(key: string) {
    if (!this.hoveredActor) return;
    const card = this.hoveredActor.getCard();
    if (!card) return;

    // Forced discard: Q (only valid hotkey) triggers discard selection.
    const pendingDiscard = this.getPendingDiscard?.();
    if (pendingDiscard?.active) {
      if (
        key.toLowerCase() === 'q' &&
        pendingDiscard.handIds.has(card.id) &&
        this.onDiscardToggle
      ) {
        this.onDiscardToggle(card.id);
      }
      return;
    }

    let actions = getApplicableActions(card, {
      sacrificeRequired: !!this.getPendingSacrifice?.(),
    });
    if (actions.length === 0) return;

    const zone = this.hoveredActor.getZone?.() || null;
    const isHandCard = zone === 'hand';
    const isOperatorZoneCard = zone === 'operator';
    const isBattlefieldCard = zone === 'battlefield';

    if (isHandCard) {
      actions = actions.filter(
        (action) => action.id === 'play_land' || action.id === 'cast',
      );
    } else if (isOperatorZoneCard) {
      actions = actions.filter((action) => action.id === 'cast');
    } else if (isBattlefieldCard) {
      actions = actions.filter(
        (action) => action.id !== 'play_land' && action.id !== 'cast',
      );
    } else {
      actions = actions.filter((action) => action.id === 'cast');
    }

    const enabledActions = actions.filter((a) => !a.disabled);
    if (enabledActions.length === 0) return;

    const keyMap: Record<string, number> = { 'q': 0, 'w': 1, 'e': 2, 'r': 3 };
    const index = keyMap[key.toLowerCase()];
    if (index !== undefined && index < enabledActions.length) {
      enabledActions[index].execute(card, {
        submitAction: this.onSubmitAction,
        setTargeting: (source, mode, abilityId) =>
          this.setTargeting?.(source, mode, abilityId),
      });
    }
  }

  /**
   * Select a player as a target from outside the 3D scene (e.g. clicking an
   * opponent's HUD entry). Runs the same targeting flow as clicking the
   * player's avatar in the battlefield. Returns true if the click was
   * consumed as a target selection.
   */
  public targetPlayer(playerId: string): boolean {
    const targeting = this.getTargeting?.();
    if (!targeting?.source || !this.setTargeting) return false;
    if (!this.isValidTargetPlayer(targeting, playerId)) return false;

    if (targeting.mode === 'attack') {
      this.onSubmitAction('games.cardgame.attack', {
        card_id: targeting.source.id,
        target: { player: playerId },
      });
      this.pendingTargets = [];
      this.setTargeting(null, null);
      return true;
    }
    if (targeting.mode === 'block') {
      return false; // Blocks only apply to units
    }

    let spec: any = targeting.source.target_spec;
    if (targeting.mode === 'activate' && targeting.abilityId) {
      const ability = targeting.source.abilities?.find(
        (a) => a.id === targeting.abilityId,
      );
      if (ability) spec = (ability as any).target_spec;
    }
    const maxTargets = spec?.max_targets ?? 1;

    const alreadyPicked = this.pendingTargets.some(
      (t) => t.kind === 'player' && t.id === playerId,
    );
    if (alreadyPicked) return false;

    this.pendingTargets.push({ kind: 'player', id: playerId });

    if (this.pendingTargets.length >= maxTargets) {
      const targets = this.pendingTargets.map((t) => ({ [t.kind]: t.id }));
      if (targeting.mode === 'activate') {
        if (!targeting.abilityId) {
          // Guard: activate requires a valid ability_id. Without it the
          // server rejects the payload. Reset targeting rather than
          // sending a broken action.
          this.pendingTargets = [];
          this.setTargeting(null, null);
          return false;
        }
        this.onSubmitAction('games.cardgame.activate', {
          card_id: targeting.source.id,
          ability_id: targeting.abilityId,
          targets,
        });
      } else {
        this.onSubmitAction('games.cardgame.cast', {
          card_id: targeting.source.id,
          targets,
        });
      }
      this.pendingTargets = [];
      this.setTargeting(null, null);
    }
    return true;
  }

  public handleInspectHotkey() {
    // If already inspecting something, exit inspect mode
    if (this.isInspecting) {
      this.onInspect?.(null);
      return;
    }
    // Otherwise inspect the hovered card if any
    if (!this.hoveredActor) return;
    const card = this.hoveredActor.getCard();
    if (!card) return;
    this.onInspect?.(card);
  }

  public handleClick = (hit: THREE.Intersection | null, button: number = 0) => {
    const extraInteractables = this.getExtraInteractables?.() ?? [];
    const buttonHit =
      extraInteractables.length > 0
        ? this.input.getIntersection(extraInteractables)
        : null;
    if (buttonHit && button === 0) {
      let btnObj: THREE.Object3D | null = buttonHit.object;
      while (btnObj && !btnObj.userData.isButton) {
        btnObj = btnObj.parent;
      }
      if (btnObj?.userData?.isButton) {
        if (btnObj.userData.disabled) return;
        const actionId = btnObj.userData.actionId;
        const payload =
          btnObj.userData.payload ??
          (actionId === 'games.cardgame.list_cards'
            ? { pagination: { first: 40 } }
            : {});
        this.onSubmitAction(actionId, payload);
        return;
      }
    }
    if (!hit) {
      // Cancel targeting if clicking empty space
      const targeting = this.getTargeting?.();
      if (targeting?.source && this.setTargeting) {
        this.setTargeting(null, null);
        return;
      }
      if (button === 0) {
        // Table Ping fallback when clicking empty space
        const ray = this.input.raycaster.ray;
        const target = new THREE.Vector3();
        if (ray.intersectPlane(this.dragPlane, target)) {
          this.onRipple?.(target.x, target.z);
          this.onSubmitAction('games.cardgame.ping', {
            x: target.x,
            z: target.z,
          });
        }
      }
      this.onInspect?.(null);
      return;
    }

    // 1. Check for 3D Buttons
    let obj: THREE.Object3D | null = hit.object;
    while (obj && !obj.userData.isButton) {
      obj = obj.parent;
    }
    if (obj?.userData?.isButton && button === 0) {
      if (obj.userData.disabled) return;
      const actionId = obj.userData.actionId;
      const payload =
        obj.userData.payload ??
        (actionId === 'games.cardgame.list_cards'
          ? { pagination: { first: 40 } }
          : {});
      this.onSubmitAction(actionId, payload);
      this.onInspect?.(null);
      return;
    }

    let clickedCard: CardView | null = null;
    let clickedPlayerId: string | null = null;
    let actor: CardActor | null = null;
    if (hit.object?.userData?.playerId) {
      clickedPlayerId = hit.object.userData.playerId as string;
    } else {
      let actorObj: THREE.Object3D | null = hit.object;
      while (actorObj && !actorObj.userData.actor) {
        actorObj = actorObj.parent;
      }
      actor = actorObj?.userData?.actor as CardActor | undefined || null;
      if (actor) {
        clickedCard = actor.getCard();
      }
    }

    const zone = actor?.getZone?.() || null;
    const isPackInteraction = zone === 'pack_inventory' || zone === 'pack_reveal';

    if (button === 2 && !isPackInteraction) { // Right click
      this.onInspect?.(clickedCard ?? null);
      if (clickedCard) return;
    }

    if (zone === 'pack_inventory') {
      if (button === 0) {
        const typeId = clickedCard?.id?.replace('pack-type-', '');
        if (typeId) {
          this.onPackTearStart?.(typeId);
          return;
        }
      }
    } else if (zone === 'pack_reveal') {
      if (button === 0) {
        const index = parseInt(
          clickedCard?.id?.replace('pack-result-', '') || '-1',
        );
        if (index >= 0) {
          this.onPackReveal?.(index);
          // If we just clicked to reveal, don't also inspect immediately
          // to avoid jarring camera jumps.
          return; 
        }
      }
    } else if (zone === 'pack_summary') {
      if (button === 0) {
        this.onInspect?.(clickedCard ?? null);
        return;
      }
    }

    const bfZone = this.zones.get('battlefield');
    const bfObjs = bfZone?.getSceneObjects() ?? [];
    const showGameElements =
      bfObjs.length > 0 ? (bfObjs[0].userData?.showGameElements ?? false) : false;

    // Clicking something that isn't a card clears any active inspect.
    // (Inspect is triggered by right-click at the top of this handler.)
    if (!clickedCard) {
      this.onInspect?.(null);
    }

    const deckBuilder = this.getDeckBuilderContext?.();
    if (deckBuilder?.enabled) {
      return;
    }

    const targeting = this.getTargeting?.();
    if (targeting?.source && this.setTargeting) {
      // Build the backend-authoritative valid target set for cast/activate.
      const backendValidIds = new Set<string>();
      if (targeting.mode === 'cast' || targeting.mode === 'activate') {
        let ids: string[] = [];
        if (targeting.mode === 'activate' && targeting.abilityId) {
          const ability = (targeting.source.abilities ?? []).find(
            (a: any) => a.id === targeting.abilityId,
          );
          ids = (ability as any)?.valid_target_ids ?? [];
        } else {
          ids = (targeting.source as any).valid_target_ids ?? [];
        }
        for (const id of ids) backendValidIds.add(id);
      }

      if (clickedCard && clickedCard.id === targeting.source.id) {
        this.pendingTargets = [];
        this.setTargeting(null, null);
        return;
      }
      // If clicked card is castable from hand and has its own target_spec,
      // switch targeting source to that card instead of selecting it as a target
      if (clickedCard && clickedCard.target_spec && clickedCard.can_cast) {
        const clickedActor = (() => {
          let actorObj: THREE.Object3D | null = hit?.object ?? null;
          while (actorObj && !actorObj.userData.actor) actorObj = actorObj.parent;
          return actorObj?.userData?.actor as CardActor | undefined;
        })();
        const clickedZone = clickedActor?.getZone?.();
        if (clickedZone === 'hand' || clickedZone === 'operator') {
          this.setTargeting(clickedCard, 'cast');
          return;
        }
      }
      if (clickedCard || clickedPlayerId) {
        const targetId = clickedCard?.id || clickedPlayerId;
        const targetKind: 'card' | 'player' = clickedCard ? 'card' : 'player';

        if (targetId) {
          if (targeting.mode === 'attack') {
            this.onSubmitAction('games.cardgame.attack', {
              card_id: targeting.source.id,
              target: { [targetKind]: targetId },
            });
            this.pendingTargets = [];
            this.setTargeting(null, null);
            return;
          }
          if (targeting.mode === 'block') {
            if (targetKind === 'card' && clickedCard) {
              const reason = this.whyCantTarget(clickedCard, 'block');
              if (reason) {
                this.onTargetingRejected?.(reason);
                return;
              }
              this.onSubmitAction('games.cardgame.block', {
                attacker_id: targetId,
                blocker_id: targeting.source.id,
              });
            }
            this.pendingTargets = [];
            this.setTargeting(null, null);
            return;
          }

          // Cast / activate: accumulate targets until we have max_targets
          let spec: any = targeting.source.target_spec;
          if (targeting.mode === 'activate' && targeting.abilityId) {
            const ability = targeting.source.abilities?.find(
              (a) => a.id === targeting.abilityId,
            );
            if (ability) spec = (ability as any).target_spec;
          }

          const maxTargets = spec?.max_targets ?? 1;

          // Validate against the backend's valid_target_ids list.
          if (targetKind === 'card' && clickedCard && backendValidIds.size > 0) {
            if (!backendValidIds.has(clickedCard.id)) {
              this.onTargetingRejected?.(
                `${clickedCard.name} is not a valid target.`,
              );
              return;
            }
          }

          // Prevent duplicate target selection
          const alreadyPicked = this.pendingTargets.some(
            (t) => t.kind === targetKind && t.id === targetId,
          );
          if (alreadyPicked) return;

          this.pendingTargets.push({ kind: targetKind, id: targetId });

          if (this.pendingTargets.length >= maxTargets) {
            const targets = this.pendingTargets.map((t) => ({ [t.kind]: t.id }));
            if (targeting.mode === 'activate') {
              if (!targeting.abilityId) {
                this.pendingTargets = [];
                this.setTargeting(null, null);
                return;
              }
              this.onSubmitAction('games.cardgame.activate', {
                card_id: targeting.source.id,
                ability_id: targeting.abilityId,
                targets,
              });
            } else {
              this.onSubmitAction('games.cardgame.cast', {
                card_id: targeting.source.id,
                targets,
              });
            }
            this.pendingTargets = [];
            this.setTargeting(null, null);
          }
          return;
        }
      }
      this.setTargeting(null, null);
      return;
    }

    if (clickedCard) {
      let actions = getApplicableActions(clickedCard, {
        sacrificeRequired: !!this.getPendingSacrifice?.(),
      });
      console.log('[InteractionController] Clicked card:', clickedCard.name, {
        zone: this.hoveredActor?.getZone?.() || null,
        can_tap: clickedCard.can_tap,
        can_attack: clickedCard.can_attack,
        can_block: clickedCard.can_block,
        can_cast: clickedCard.can_cast,
        abilities: clickedCard.abilities?.map((a) => ({
          name: a.name,
          can_activate: a.can_activate,
        })),
        availableActions: actions.map((a) => a.id),
      });

      if (actions.length > 0) {
        const context: InteractionContext = {
          submitAction: this.onSubmitAction,
          setTargeting: (source, mode, abilityId) =>
            this.setTargeting?.(source, mode, abilityId),
        };
        const zone = this.hoveredActor?.getZone?.() || null;
        const isHandCard = zone === 'hand';
        const isOperatorZoneCard = zone === 'operator';
        const isBattlefieldCard = zone === 'battlefield';

        const playActions = actions.filter(
          (action) => action.id === 'play_land' || action.id === 'cast',
        );

        // Filter actions based on zone
        if (isHandCard) {
          // Hand cards can cast or play land (discard is via drag-and-drop)
          actions = actions.filter(
            (action) => action.id === 'play_land' || action.id === 'cast',
          );
        } else if (isOperatorZoneCard) {
          // Operator zone cards can only cast
          actions = actions.filter((action) => action.id === 'cast');
        } else if (isBattlefieldCard) {
          // Battlefield cards can activate, attack, block, etc. (handled by backend can_X flags mostly, but we filter out cast/play_land)
          actions = actions.filter(
            (action) => action.id !== 'play_land' && action.id !== 'cast',
          );
          console.log(
            '[InteractionController] Battlefield card actions after filter:',
            actions.map((a) => a.id),
          );
        } else {
          // Other zones (Library, Graveyard, etc.) - usually no actions unless specifically allowed
          actions = actions.filter((action) => action.id === 'cast'); // Some graveyard effects might allow casting
        }

        if (actions.length === 0) {
          console.log(
            '[InteractionController] No actions available after filtering',
          );
          return;
        }
        if ((isHandCard || isOperatorZoneCard) && playActions.length > 0) {
          // Auto-execute if it's the only primary action from hand/operator
          if (playActions.length === 1 && actions.length === 1) {
            console.log(
              '[InteractionController] Auto-executing play action:',
              playActions[0].id,
            );
            playActions[0].execute(clickedCard, context);
            return;
          }
        }
        const enabledActions = actions.filter((a) => !a.disabled);
        // Auto-execute only if there's exactly 1 total action (not just 1 enabled).
        // Cards with multiple abilities (e.g. agents) always show a menu.
        if ((enabledActions.length === 1 && actions.length === 1) || !this.onActionMenu) {
          if (enabledActions.length > 0) {
            console.log(
              '[InteractionController] Executing single action:',
              enabledActions[0].id,
              enabledActions[0].label,
            );
            enabledActions[0].execute(clickedCard, context);
            return;
          }
        }
        console.log(
          '[InteractionController] Showing action menu with',
          actions.length,
          'actions',
        );
        this.onActionMenu?.(clickedCard, actions, {
          x: this.input.pointer.rawX,
          y: this.input.pointer.rawY,
        });
        return;
      }
    }

    if (!clickedCard && !clickedPlayerId) {
      // Table Ping
      // Raycast to floor
      const ray = this.input.raycaster.ray;
      const target = new THREE.Vector3();
      if (ray.intersectPlane(this.dragPlane, target)) {
        this.onRipple?.(target.x, target.z);
        this.onSubmitAction('games.cardgame.ping', {
          x: target.x,
          z: target.z,
        });
      }
    }
  };

  private handleDragStart = (hit: THREE.Intersection | null) => {
    if (this.isInspecting) return;

    const dragStartActor = (() => {
      if (this.hoveredActor) return this.hoveredActor;
      if (!hit) return null;
      let obj: THREE.Object3D | null = hit.object;
      while (obj && !obj.userData.actor) {
        obj = obj.parent ?? null;
      }
      return (obj?.userData?.actor as CardActor | undefined) || null;
    })();

    if (dragStartActor) {
      const zone = dragStartActor.getZone?.() || null;
      if (zone === 'pack_inventory' || zone === 'pack_opening') {
        const packId = dragStartActor.getCard()?.id || '';
        const typeId = packId.replace('pack-type-', '');
        if (typeId) {
          const uv = hit?.uv ?? null;
          this.packTearActive = true;
          this.packTearTypeId = typeId;
          this.packTearStartX = this.input.pointer.rawX;
          this.packTearStartY = this.input.pointer.rawY;
          this.packTearProgress = 0;
          this.packTearDirection = uv ? (uv.x < 0.5 ? 1 : -1) : 1;
          this.packTearLastLoggedPct = -1;
          console.log('[PackTear] start', {
            typeId,
            pointer: { x: this.packTearStartX, y: this.packTearStartY },
            uv,
            direction: this.packTearDirection,
          });
          if (zone === 'pack_inventory') {
            this.onPackTearStart?.(typeId);
          }
          this.onPackTearProgress?.(0);
          return;
        }
      }
    }
    if (this.isScryActive?.() && this.hoveredActor) {
      const card = this.hoveredActor.getCard();
      const scryTarget = this.getScryTarget?.(card.id);
      if (scryTarget) {
        this.draggingScryId = card.id;
        this.draggingActor = this.hoveredActor;
        this.draggingActor.setDragging(true);
        const dragY = Math.max(0.2, this.draggingActor.group.position.y);
        this.dragPlane.constant = -dragY;
        return;
      }
    }
    const deckBuilder = this.getDeckBuilderContext?.();
    const deckSelect = this.getDeckSelectContext?.();
    const ringMode = Boolean(deckBuilder?.enabled || deckSelect?.enabled);
    if (ringMode && !this.hoveredActor) {
      let obj: THREE.Object3D | null = hit?.object ?? null;
      while (obj && !obj.userData?.isButton) {
        obj = obj.parent ?? null;
      }
      const isButtonHit = Boolean(obj?.userData?.isButton);
      if (
        isButtonHit &&
        obj?.userData?.actionId === 'games.cardgame.builder_set_filter' &&
        obj?.userData?.filterGroup === 'kind'
      ) {
        this.ringDragMode = 'kind';
        this.ringDragActive = true;
        this.ringDragLastX = this.input.pointer.rawX;
        this.ringDragLastY = this.input.pointer.rawY;
        this.ringDragCarry = 0;
      } else if (!isButtonHit) {
        this.ringDragMode = 'ring';
        this.ringDragActive = true;
        this.ringDragLastX = this.input.pointer.rawX;
        this.ringDragLastY = this.input.pointer.rawY;
        this.ringDragCarry = 0;
      }

      if (this.ringDragActive && this.hoveredButton) {
        this.setButtonHover(this.hoveredButton, false);
        this.hoveredButton = null;
      }
      return;
    }
    if (this.hoveredActor) {
      this.draggingActor = this.hoveredActor;
      this.draggingActor.setDragging(true);
      this.onDragStateChange?.(true, this.draggingActor.id);

      // Update drag plane to the current height of the actor for consistent dragging
      // Use a minimum height of 0.2 for flat cards to ensure they are visible above the table surface
      const dragY = Math.max(0.2, this.draggingActor.group.position.y);
      this.dragPlane.constant = -dragY;
    }
  };

  private handleDragMove = (hit: THREE.Intersection | null) => {
    if (this.isInspecting) {
      this.onInspectDrag?.(this.input.pointer.movementX, this.input.pointer.movementY);
      return;
    }

    if (this.packTearActive) {
      const dx = this.input.pointer.rawX - this.packTearStartX;
      const dy = this.input.pointer.rawY - this.packTearStartY;
      // Use absolute horizontal pull for visible feedback in either swipe direction.
      const directional = Math.max(0, Math.abs(dx) - Math.abs(dy) * 0.15);
      const progress = THREE.MathUtils.clamp(directional / 220, 0, 1);
      this.packTearProgress = progress;
      const pct = Math.round(progress * 100);
      if (pct !== this.packTearLastLoggedPct && (pct % 5 === 0 || pct === 100)) {
        this.packTearLastLoggedPct = pct;
        console.log('[PackTear] progress', {
          pct,
          dx: Math.round(dx),
          dy: Math.round(dy),
          directional: Math.round(directional),
        });
      }
      this.onPackTearProgress?.(progress);
      return;
    }
    if (this.ringDragActive) {
      const currentX = this.input.pointer.rawX;
      const currentY = this.input.pointer.rawY;
      const dx = currentX - this.ringDragLastX;
      const dy = currentY - this.ringDragLastY;
      this.ringDragLastX = currentX;
      this.ringDragLastY = currentY;
      const delta = this.ringDragMode === 'kind' ? dx : dx;
      this.ringDragCarry += delta;
      while (this.ringDragCarry <= -this.ringDragStepPx) {
        if (this.ringDragMode === 'kind') {
          this.onFilterDragStep?.(-1);
        } else {
          this.onRingDragStep?.(-1);
        }
        this.ringDragCarry += this.ringDragStepPx;
      }
      while (this.ringDragCarry >= this.ringDragStepPx) {
        if (this.ringDragMode === 'kind') {
          this.onFilterDragStep?.(1);
        } else {
          this.onRingDragStep?.(1);
        }
        this.ringDragCarry -= this.ringDragStepPx;
      }
      return;
    }
    if (this.draggingScryId && this.draggingActor) {
      // Move card with cursor
      const ray = this.input.raycaster.ray;
      const target = new THREE.Vector3();
      if (ray.intersectPlane(this.dragPlane, target)) {
        this.draggingActor.group.position.copy(target);
      }

      // Check drop zone hover
      let hoveredZone: 'top' | 'bottom' | null = null;
      const scryZone = this.zones.get('scry') as any;
      const scryInteractables = scryZone?.getExtraInteractables?.() ?? [];
      const scryHit = this.input.getIntersection(scryInteractables);
      if (scryHit) {
        let obj: THREE.Object3D | null = scryHit.object;
        while (obj && !obj.userData?.scryDropZone) {
          obj = obj.parent ?? null;
        }
        if (obj?.userData?.scryDropZone) {
          hoveredZone = obj.userData.scryDropZone as 'top' | 'bottom';
        }
      }
      if (scryZone && typeof scryZone.setDropHover === 'function') {
        scryZone.setDropHover(hoveredZone);
      }
      return;
    }
    if (!this.draggingActor) return;

    // Raycast against infinite plane for smooth dragging
    const ray = this.input.raycaster.ray;
    const target = new THREE.Vector3();
    if (ray.intersectPlane(this.dragPlane, target)) {
      // Update dragging actor position visually
      this.draggingActor.group.position.copy(target);

      // Rotate to face center (simple lookAt for now)
      this.draggingActor.group.lookAt(
        0,
        this.draggingActor.group.position.y,
        0,
      );

      // If flat layout, maintain the flat orientation while dragging
      if (
        this.draggingActor.isInHand() &&
        (this.draggingActor as any).flatLayout
      ) {
        this.draggingActor.group.rotation.set(
          -Math.PI / 2,
          0,
          this.draggingActor.group.rotation.y + Math.PI / 2,
        );
      }

      const bfZone = this.zones.get('battlefield') as any;
      const viewerId = this.getViewerId?.() ?? null;

      // Check for Drop Zone Hover (Danger or Sacrifice — discard-zone drop retired)
      const extra = this.getExtraInteractables?.() ?? [];
      const hoverHit = this.input.getIntersection(extra);
      let isOverDanger = false;
      let isOverSacrifice = false;
      if (hoverHit) {
        let obj: THREE.Object3D | null = hoverHit.object;
        while (
          obj &&
          !obj.userData?.dangerDrop &&
          !obj.userData?.sacrificeDrop
        ) {
          obj = obj.parent ?? null;
        }
        if (obj?.userData?.dangerDrop) isOverDanger = true;
        if (obj?.userData?.sacrificeDrop) isOverSacrifice = true;
      }
      this.onDangerHover?.(isOverDanger);
      this.onDiscardHover?.(false);
      this.onSacrificeHover?.(isOverSacrifice);

      if (this.draggingActor) {
        this.draggingActor.setDragOverDiscard(isOverSacrifice);
      }

      if (
        bfZone &&
        typeof bfZone.setPreviewOrder === 'function' &&
        typeof bfZone.getDropIndex === 'function'
      ) {
        const currentIds = bfZone.getCardIds(viewerId);
        const draggingId = this.draggingActor.id;
        if (currentIds.includes(draggingId)) {
          const newIndex = bfZone.getDropIndex(target, viewerId, draggingId);
          const newOrder = currentIds.filter((id: string) => id !== draggingId);
          newOrder.splice(newIndex, 0, draggingId);
          bfZone.setPreviewOrder(newOrder);
        }
      }
    }
  };

  private handleDragEnd = () => {
    if (this.packTearActive) {
      const typeId = this.packTearTypeId;
      const committed = this.packTearProgress >= 0.98;
      console.log('[PackTear] end', {
        typeId,
        progress: Number(this.packTearProgress.toFixed(3)),
        committed,
      });
      this.packTearActive = false;
      this.packTearTypeId = null;
      this.packTearProgress = 0;
      this.packTearDirection = 1;
      this.packTearLastLoggedPct = -1;
      if (committed && typeId) {
        this.onPackTearCommit?.(typeId);
      } else {
        this.onPackTearCancel?.();
      }
      return;
    }
    if (this.ringDragActive) {
      this.ringDragActive = false;
      this.ringDragCarry = 0;
      return;
    }
    if (this.draggingScryId) {
      const cardId = this.draggingScryId;
      this.draggingScryId = null;

      // Clean up dragging actor
      if (this.draggingActor) {
        this.draggingActor.setDragging(false);
        this.draggingActor = null;
      }

      const scryZone = this.zones.get('scry') as any;
      if (scryZone?.setDropHover) scryZone.setDropHover(null);

      const scryInteractables = scryZone?.getExtraInteractables?.() ?? [];
      const scryHit = this.input.getIntersection(scryInteractables);
      if (scryHit) {
        let obj: THREE.Object3D | null = scryHit.object;
        while (obj && !obj.userData?.scryDropZone) {
          obj = obj.parent ?? null;
        }
        if (obj?.userData?.scryDropZone) {
          const zone = obj.userData.scryDropZone as 'top' | 'bottom';
          this.onScryDrop?.(cardId, zone);
        }
      }
      return;
    }
    if (this.draggingActor) {
      const actor = this.draggingActor;
      this.draggingActor = null; // Clear early

      actor.setDragging(false);
      actor.setDragOverDiscard(false);
      this.onDragStateChange?.(false, null);

      // Set target to current position so it doesn't lerp back to hand while waiting for server response
      actor.setTarget(
        actor.group.position.clone(),
        actor.group.rotation.clone(),
        actor.group.scale.x,
      );

      const draggingId = actor.id;
      const dragMeshes = new Set(actor.getInteractables());

      const deckSelect = this.getDeckSelectContext?.();
      if (deckSelect?.enabled) {
        let droppedMode: 'as_is' | 'customize' | null = null;
        const interactables = this.zones
          .getInteractables()
          .filter((obj) => !dragMeshes.has(obj));
        const hit = this.input.getIntersection(interactables);
        if (hit) {
          let obj: THREE.Object3D | null = hit.object;
          while (obj && !obj.userData?.deckSelectDrop) {
            obj = obj.parent ?? null;
          }
          if (obj?.userData?.deckSelectDrop) {
            droppedMode = obj.userData.deckSelectDrop as 'as_is' | 'customize';
          }
        }
        if (droppedMode) {
          this.onDeckSelectDrop?.(draggingId, droppedMode);
        }
        this.draggingActor = null;
        return;
      }

      const deckBuilder = this.getDeckBuilderContext?.();
      if (deckBuilder?.enabled) {
        let droppedOnDanger = false;
        const interactables = this.zones
          .getInteractables()
          .filter((obj) => !dragMeshes.has(obj));
        const extra = this.getExtraInteractables?.() ?? [];
        const hit = this.input.getIntersection([...interactables, ...extra]);
        if (hit) {
          let obj: THREE.Object3D | null = hit.object;
          while (obj && !obj.userData?.dangerDrop) {
            obj = obj.parent ?? null;
          }
          if (obj?.userData?.dangerDrop) {
            droppedOnDanger = true;
          }
        }

        if (droppedOnDanger) {
          this.onDangerDrop?.(draggingId);
        }

        this.draggingActor = null;
        return;
      }

      // Check for Discard Drop (voluntary discard via drag-and-drop from hand)
      const extra = this.getExtraInteractables?.() ?? [];
      const interactables = this.zones
        .getInteractables()
        .filter((obj) => !dragMeshes.has(obj));
      const dropHit = this.input.getIntersection([...interactables, ...extra]);
      if (dropHit) {
        // Discard-zone drop retired; forced discards use the in-scene prompt
        // and voluntary discards are gone. Only sacrifice remains.
        let obj: THREE.Object3D | null = dropHit.object;
        while (obj && !obj.userData?.sacrificeDrop) {
          obj = obj.parent ?? null;
        }
        if (obj?.userData?.sacrificeDrop) {
          this.onSubmitAction('games.cardgame.sacrifice', {
            card_id: draggingId,
          });
          this.draggingActor = null;
          return;
        }
      }

      // 1. Check for Scry Drop
      const scryZone = this.zones.get('scry');
      if (scryZone && (scryZone as any).getCardIds().length > 0) {
        const interactables = (scryZone as any).getExtraInteractables?.() ?? [];
        const otherCards = this.zones
          .getAllActors()
          .filter(
            (a) =>
              a.id !== draggingId &&
              (scryZone as any).getCardIds().includes(a.id),
          )
          .flatMap((a) => a.getInteractables());

        const hit = this.input.getIntersection([
          ...interactables,
          ...otherCards,
        ]);
        if (hit) {
          let targetZone: 'top' | 'bottom' | 'pool' | null = null;
          let beforeId: string | null = null;

          if (hit.object.userData.scryDropZone) {
            targetZone = hit.object.userData.scryDropZone;
          } else {
            // Hit another card
            let obj = hit.object;
            while (obj && !obj.userData.actor) obj = obj.parent!;
            const targetActor = obj?.userData?.actor;
            if (targetActor) {
              beforeId = targetActor.id;
              targetZone = (scryZone as any).getZoneForCard(beforeId);
            }
          }

          if (targetZone) {
            this.onScryDrop?.(draggingId, targetZone, beforeId);
            const bfZone = this.zones.get('battlefield') as any;
            if (bfZone?.setPreviewOrder) bfZone.setPreviewOrder(null);
            this.draggingActor = null;
            return;
          }
        }
      }

      // 2. Default Battlefield Reorder
      const ray = this.input.raycaster.ray;
      const target = new THREE.Vector3();
      ray.intersectPlane(this.dragPlane, target);

      const bfZone = this.zones.get('battlefield');
      const viewerId = this.getViewerId?.() ?? null;

      if (bfZone) {
        const newIndex = bfZone.getDropIndex(target, viewerId);
        const currentIds = bfZone.getCardIds(viewerId);

        const newOrder = currentIds.filter((id) => id !== draggingId);
        newOrder.splice(newIndex, 0, draggingId);

        this.onSubmitAction('games.cardgame.reorder', {
          card_ids: newOrder,
        });
        if (typeof bfZone.setPreviewOrder === 'function') {
          bfZone.setPreviewOrder(null);
        }
      }

      this.draggingActor = null;
    }
  };

  private setButtonHover(button: THREE.Object3D, hovered: boolean) {
    if (button.userData.isHovered === hovered) return;

    const isHexButton =
      !!button.userData?.hoverTarget || !!button.userData?.hoverHalo;
    button.userData.isHovered = hovered;
    const baseScale = button.userData?.baseScale ?? 1.0;
    if (button.userData?.disableHover) {
      button.scale.setScalar(baseScale);
      return;
    }
    const hoverScale =
      typeof button.userData?.hoverScale === 'number'
        ? button.userData.hoverScale
        : isHexButton
          ? 1.18
          : 1.06;
    const targetScale = hovered ? baseScale * hoverScale : baseScale;
    button.scale.setScalar(targetScale);

    const glowColor = button.userData?.hoverGlow as string | undefined;
    const hoverEmissive =
      typeof button.userData?.hoverEmissive === 'number'
        ? button.userData.hoverEmissive
        : 1.5;
    if (glowColor) {
      const applyGlow = (obj: THREE.Object3D) => {
        const mat: any = (obj as any).material;
        if (mat && mat.emissive) {
          if (!mat.userData) mat.userData = {};
          if (!mat.userData.baseEmissive) {
            mat.userData.baseEmissive = mat.emissive.clone();
          }
          const baseEmissive = mat.userData.baseEmissive as THREE.Color;
          const target = new THREE.Color(glowColor);
          mat.emissive.copy(hovered ? target : baseEmissive);
          mat.emissiveIntensity = hovered ? hoverEmissive : 0.6;
        }
      };
      button.traverse(applyGlow);
    }

    const hoverTarget =
      (button.userData?.hoverTarget as THREE.Object3D | undefined) ?? button;
    const hoverEdge = button.userData?.hoverEdge as any;
    const hoverHalo = button.userData?.hoverHalo as any;

    const applyMaterial = (mat: any) => {
      if (!mat || typeof mat.emissiveIntensity !== 'number') return;
      if (!mat.userData) mat.userData = {};
      if (typeof mat.userData.baseEmissiveIntensity !== 'number') {
        mat.userData.baseEmissiveIntensity = mat.emissiveIntensity;
      }
      const base = mat.userData.baseEmissiveIntensity as number;
      mat.emissiveIntensity = hovered ? base * 1.8 : base;
    };
    const applyBasicMaterial = (mat: any) => {
      if (!mat || !mat.color) return;
      if (!mat.userData) mat.userData = {};
      if (!mat.userData.baseColor) {
        mat.userData.baseColor = mat.color.clone();
      }
      const baseColor = mat.userData.baseColor;
      if (hovered) {
        mat.color.copy(baseColor).lerp(new THREE.Color(0xffffff), 0.2);
      } else {
        mat.color.copy(baseColor);
      }
      if (typeof mat.opacity === 'number') {
        if (typeof mat.userData.baseOpacity !== 'number') {
          mat.userData.baseOpacity = mat.opacity;
        }
        const baseOpacity = mat.userData.baseOpacity as number;
        mat.opacity = hovered ? Math.min(1, baseOpacity + 0.2) : baseOpacity;
      }
    };
    const material = (hoverTarget as any).material;
    if (Array.isArray(material)) {
      material.forEach((mat) => {
        applyMaterial(mat);
        applyBasicMaterial(mat);
      });
    } else {
      applyMaterial(material);
      applyBasicMaterial(material);
    }

    if (isHexButton) {
      const edgeMat = hoverEdge?.material;
      if (edgeMat && typeof edgeMat.opacity === 'number') {
        if (!edgeMat.userData) edgeMat.userData = {};
        if (typeof edgeMat.userData.baseOpacity !== 'number') {
          edgeMat.userData.baseOpacity = edgeMat.opacity;
        }
        const baseOpacity = edgeMat.userData.baseOpacity as number;
        edgeMat.opacity = hovered
          ? Math.min(1, baseOpacity + 0.3)
          : baseOpacity;
      }

      const haloMat = hoverHalo?.material;
      if (haloMat && typeof haloMat.opacity === 'number') {
        haloMat.opacity = hovered ? 0.7 : 0.0;
      }
    }
  }

  private setDeckDropHover(kind: 'as_is' | 'customize' | null) {
    const dropZone = this.zones.get('deck_select_drop') as any;
    if (dropZone && typeof dropZone.setHover === 'function') {
      dropZone.setHover(kind);
    }
  }
}

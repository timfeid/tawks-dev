import type { CardView } from './types';

export type InteractionContext = {
  submitAction: (action: string, payload: any) => void;
  setTargeting: (source: CardView, mode: 'cast' | 'attack' | 'block' | 'activate', abilityId?: string | null) => void;
};

/** Transient board state that influences which actions are applicable. */
export type CardActionEnv = {
  /** True when the viewer currently owes a sacrifice (pending_sacrifice). */
  sacrificeRequired?: boolean;
};

export interface CardAction {
  id: string;
  label: string;
  priority: number;
  isApplicable: (card: CardView, env?: CardActionEnv) => boolean;
  execute: (card: CardView, context: InteractionContext) => void;
  disabled?: boolean;
  disabledReason?: string;
}

export const PlayLandAction: CardAction = {
  id: 'play_land',
  label: 'Play Land',
  priority: 100,
  isApplicable: (card) => card.can_play_land,
  execute: (card, { submitAction }) => {
    submitAction('games.cardgame.play_land', { card_id: card.id });
  },
};

export const CastAction: CardAction = {
  id: 'cast',
  label: 'Cast',
  priority: 90,
  isApplicable: (card) => card.can_cast,
  execute: (card, { submitAction, setTargeting }) => {
    if (card.target_spec) {
      setTargeting(card, 'cast');
    } else {
      submitAction('games.cardgame.cast', { card_id: card.id, targets: [] });
    }
  },
};

export const AttackAction: CardAction = {
  id: 'attack',
  label: 'Attack',
  priority: 80,
  isApplicable: (card) => card.can_attack,
  execute: (card, { setTargeting }) => {
    console.log('[AttackAction] Setting targeting for attack:', card.name, card.id);
    setTargeting(card, 'attack');
  },
};

export const BlockAction: CardAction = {
  id: 'block',
  label: 'Block',
  priority: 85,
  isApplicable: (card) => card.can_block,
  execute: (card, { setTargeting }) => {
    console.log('[BlockAction] Entering block targeting mode with:', card.name, card.id);
    setTargeting(card, 'block');
  },
};

export const TapAbilityAction: CardAction = {
  id: 'tap',
  label: 'Activate',
  priority: 70,
  isApplicable: (card) => card.can_tap,
  execute: (card, { submitAction }) => {
    console.log('[TapAbilityAction] Executing tap action for card:', card.name, card.id);
    submitAction('games.cardgame.tap', { card_id: card.id });
  },
};

export const SacrificeAction: CardAction = {
  id: 'sacrifice',
  label: 'Sacrifice',
  // High priority so it surfaces at the top of the menu while a sacrifice
  // is owed — the player can just click to fulfill the requirement.
  priority: 95,
  isApplicable: (card, env) =>
    !!env?.sacrificeRequired &&
    ((card as any)._zone === 'bf_units' ||
      (card as any)._zone === 'battlefield'),
  execute: (card, { submitAction }) => {
    submitAction('games.cardgame.sacrifice', { card_id: card.id });
  },
};

// Note: Discard is still drag-and-drop; Sacrifice is now a menu action.
export const AllActions = [
  PlayLandAction,
  CastAction,
  AttackAction,
  BlockAction,
  TapAbilityAction,
  SacrificeAction,
];

export const getApplicableActions = (
  card: CardView,
  env?: CardActionEnv,
): CardAction[] => {
  const hasActivatableAbilities = (card.abilities || []).some((a) => a.can_activate);
  const actions = AllActions.filter((action) => {
    // Suppress generic tap action when specific activated abilities exist
    if (action.id === 'tap' && hasActivatableAbilities) return false;
    return action.isApplicable(card, env);
  });

  // Add Dynamic Activated Abilities
  (card.abilities || []).forEach((ability) => {
    if (ability.can_activate) {
      actions.push({
        id: `activate_${ability.id}`,
        label: ability.name,
        priority: 75,
        isApplicable: () => true,
        execute: (c, { submitAction, setTargeting }) => {
          console.log('[ActivateAbility] Executing ability:', ability.name, ability.id, 'for card:', c.name);
          if (ability.target_spec) {
            console.log('[ActivateAbility] Setting targeting for ability with target_spec');
            setTargeting(c, 'activate', ability.id);
          } else {
            console.log('[ActivateAbility] Submitting activate action without targets');
            submitAction('games.cardgame.activate', {
              card_id: c.id,
              ability_id: ability.id,
              targets: [],
            });
          }
        },
      });
    } else if (ability.inactive_reason) {
      actions.push({
        id: `activate_${ability.id}`,
        label: ability.name,
        priority: 10,
        isApplicable: () => true,
        execute: () => {},
        disabled: true,
        disabledReason: ability.inactive_reason,
      });
    }
  });

  return actions.sort((a, b) => b.priority - a.priority);
};

use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};
use tawks_data::repository::Repository;

use crate::action::CardGameAction;
use crate::card::*;
use crate::deck::DeckTemplateEntry;
use crate::sim::action_gen::legal_actions_for_player;
use crate::sim::agent::{Agent, ArchetypeAgent, profile_for_deck};
use crate::sim::runner::next_actor_id;
use crate::state::*;
use crate::types::*;
use crate::view::*;
use game_framework::{GameEffect, GameLogic};

use crate::repository::CardRepository;
use std::sync::Arc;
use ulid::Ulid;

// Import helper utilities
use crate::helpers::*;

// Import action handlers
use crate::handlers::*;

impl CardGameState {
    pub(crate) fn format_allows_deck(&self, format: &DeckFormat) -> bool {
        *format == deck_format_for_game(&self.format)
    }

    fn ensure_card_options_loaded(&mut self) {
        if self.card_options_loaded {
            return;
        }
        let repo = self
            .repository
            .as_ref()
            .expect("Cardgame repository not initialized")
            .clone();

        let result: Result<(
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
        )> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let kind_values = repo.list_enum_values("card_kind").await?;
                let supertype_values = repo
                    .list_enum_values("card_supertype")
                    .await
                    .unwrap_or_default();
                let unit_values = repo.list_enum_values("unit_type").await?;
                let artifact_values = repo.list_enum_values("artifact_type").await?;
                let effect_values = repo.list_enum_values("card_effect_kind").await?;
                let mana_values = repo.list_enum_values("mana_type").await?;
                let trigger_values = repo.list_enum_values("trigger_event").await?;
                let stat_values = repo.list_enum_values("stat_type").await.unwrap_or_default();
                let expire_values = repo
                    .list_enum_values("expire_contract")
                    .await
                    .unwrap_or_default();
                let target_values = repo
                    .list_enum_values("effect_target_spec_kind")
                    .await
                    .unwrap_or_default();
                Ok((
                    kind_values,
                    supertype_values,
                    unit_values,
                    artifact_values,
                    effect_values,
                    mana_values,
                    trigger_values,
                    stat_values,
                    expire_values,
                    target_values,
                ))
            })
        });

        let (
            kind_values,
            supertype_values,
            unit_values,
            artifact_values,
            effect_values,
            mana_values,
            trigger_values,
            stat_values,
            expire_values,
            target_values,
        ) = result.expect("Failed to load card enum options");
        self.kind_options = build_select_options(kind_values, &[]);
        self.supertype_options = build_select_options(supertype_values, &[]);
        self.unit_type_options =
            build_select_options(unit_values, &[("bio_organism", "Bio-Organism")]);
        self.artifact_type_options = build_select_options(artifact_values, &[]);
        let supported_effects = filter_effect_kinds(
            effect_values,
            &[
                "add_mana",
                "draw_cards",
                "damage_player",
                "damage_unit",
                "damage_target",
                "gain_life",
                "freeze",
                "discard",
                "scry",
                "loot",
                "apply_stat_modifier",
                "bounce",
                "board_wipe",
                "tap_target",
                "destroy_target",
                "exile_target",
                "equip",
                "purify",
            ],
        );
        self.effect_kind_options = build_select_options(
            supported_effects,
            &[
                ("add_mana", "Add Mana"),
                ("draw_cards", "Draw Cards"),
                ("damage_player", "Damage Player"),
                ("damage_unit", "Damage Unit"),
                ("damage_target", "Damage Target"),
                ("gain_life", "Gain Life"),
                ("freeze", "Freeze"),
                ("discard", "Discard"),
                ("scry", "Scry"),
                ("loot", "Loot"),
                ("apply_stat_modifier", "Stat Modifier"),
                ("bounce", "Bounce"),
                ("board_wipe", "Board Wipe"),
                ("tap_target", "Tap Target"),
                ("destroy_target", "Destroy Target"),
                ("exile_target", "Exile Target"),
                ("equip", "Equip"),
                ("purify", "Purify"),
            ],
        );
        self.mana_type_options = build_select_options(mana_values, &[]);
        self.trigger_event_options = build_select_options(trigger_values, &[]);
        self.stat_type_options = build_select_options(stat_values, &[]);
        self.expire_contract_options = build_select_options(expire_values, &[]);
        self.effect_target_options = build_select_options(target_values, &[]);

        if self.effect_kind_options.is_empty() {
            panic!("No supported effect kinds found in card_effect_kind enum");
        }
        self.card_options_loaded = true;
    }

    /// Compute deck colors from card mana costs
    fn compute_deck_colors(templates: &[DeckTemplateEntry]) -> Vec<String> {
        let mut mana_types = HashSet::new();

        for entry in templates {
            let cost = &entry.card.cost;
            if cost.neon > 0 {
                mana_types.insert(ManaType::Neon);
            }
            if cost.bio > 0 {
                mana_types.insert(ManaType::Bio);
            }
            if cost.slag > 0 {
                mana_types.insert(ManaType::Slag);
            }
            if cost.glitch > 0 {
                mana_types.insert(ManaType::Glitch);
            }
            if cost.chrome > 0 {
                mana_types.insert(ManaType::Chrome);
            }
        }

        // Convert to sorted hex colors for consistent ordering
        let mut colors: Vec<String> = mana_types
            .iter()
            .map(|mt| mana_color_for_type(mt).to_string())
            .collect();
        colors.sort();

        // Fallback to colorless if no colors found
        if colors.is_empty() {
            colors.push("#64748b".to_string());
        }

        colors
    }

    pub(crate) fn ensure_deck_choices_loaded_for(&mut self, player_id: &str) {
        if self.deck_choices_loaded.contains(player_id) {
            return;
        }

        let choices = if let Some(repo) = self.repository.as_ref() {
            let repo = repo.clone();
            let result: Result<Vec<DeckChoiceView>> = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                let player_id = player_id.to_string();
                let format = self.format.clone();
                handle.block_on(async move {
                    let decks = repo.list_decks_for_user(&player_id).await?;

                    let target_format = deck_format_for_game(&format);

                    let mut choices = Vec::new();
                    for deck in decks.into_iter() {
                        let matches = deck.format == target_format;
                        if !matches {
                            continue;
                        }

                        // Load deck cards to compute colors
                        let templates =
                            repo.load_deck_templates(&deck.id).await.unwrap_or_default();
                        let colors = Self::compute_deck_colors(&templates);

                        choices.push(DeckChoiceView {
                            id: deck.id.clone(),
                            name: deck.name.clone(),
                            description: deck.description.clone(),
                            colors,
                            image_url: deck.image_url.clone(),
                        });
                    }

                    Ok(choices)
                })
            });
            result.unwrap_or_else(|err| {
                eprintln!(
                    "ERROR loading deck choices for format {:?}: {}",
                    self.format, err
                );
                if self.format == GameFormat::Operator {
                    Vec::new()
                } else {
                    Self::deck_choices()
                }
            })
        } else {
            if self.format == GameFormat::Operator {
                Vec::new()
            } else {
                Self::deck_choices()
            }
        };

        self.deck_choices_by_player
            .insert(player_id.to_string(), choices);
        self.deck_choices_loaded.insert(player_id.to_string());
    }

    pub(crate) fn refresh_deck_choices_for(&mut self, player_id: &str) {
        if let Some(repo) = self.repository.as_ref() {
            let repo = repo.clone();
            let result: Result<Vec<DeckChoiceView>> = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                let player_id = player_id.to_string();
                let format = self.format.clone();
                handle.block_on(async move {
                    let decks = repo.list_decks_for_user(&player_id).await?;

                    let mut choices = Vec::new();
                    for deck in decks
                        .into_iter()
                        .filter(|d| d.format == deck_format_for_game(&format))
                    {
                        // Load deck cards to compute colors
                        let templates =
                            repo.load_deck_templates(&deck.id).await.unwrap_or_default();
                        let colors = Self::compute_deck_colors(&templates);

                        choices.push(DeckChoiceView {
                            id: deck.id,
                            name: deck.name,
                            description: deck.description,
                            colors,
                            image_url: deck.image_url,
                        });
                    }

                    Ok(choices)
                })
            });

            if let Ok(choices) = result {
                self.deck_choices_by_player
                    .insert(player_id.to_string(), choices);
                self.deck_choices_loaded.insert(player_id.to_string());
            }
        }
    }

    pub(crate) fn load_deck_builder_for_player(
        &mut self,
        player_id: &str,
        deck_id: &str,
    ) -> Result<()> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let deck_id = deck_id.to_string();
        let player_id_str = player_id.to_string();
        let result: Result<(
            crate::repository::DeckRecord,
            Vec<DeckTemplateEntry>,
            Vec<DeckTemplateEntry>,
        )> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            let deck_id = deck_id.clone();
            let player_id_task = player_id_str.clone();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id).await?;
                let resolved = if deck.owner_id.is_none() {
                    if let Some(existing) = repo
                        .get_user_deck_for_base(&player_id_task, &deck.id)
                        .await?
                    {
                        existing
                    } else {
                        let new_id = format!("{}-{}", deck.id, Ulid::new());
                        let new_name = format!("{} (Custom)", deck.name);
                        repo.clone_deck_with_owner(
                            &deck.id,
                            &new_id,
                            &new_name,
                            &deck.description,
                            Some(&player_id_task),
                            Some(&deck.id),
                        )
                        .await?
                    }
                } else if deck.owner_id.as_deref() == Some(&player_id_task) {
                    deck
                } else {
                    return Err(anyhow!("Deck does not belong to player"));
                };
                let cards = repo.load_deck_templates(&resolved.id).await?;
                let sideboard = repo.load_sideboard_templates(&resolved.id).await?;
                Ok((resolved, cards, sideboard))
            })
        });

        let (deck, cards, sideboard) = result?;
        let Some(player) = self.players.get_mut(&player_id_str) else {
            return Err(anyhow!("Player not found"));
        };
        player.deck_builder_deck_id = Some(deck.id);
        player.deck_builder_name = Some(deck.name);
        player.deck_builder_description = Some(deck.description);
        player.deck_builder_format = Some(deck.format);
        player.deck_builder_operator_card_id = deck.operator_card_id.clone();
        player.deck_builder_autobuild_colors = None;
        let mut operator = None;
        player.deck_builder_cards = cards
            .into_iter()
            .filter_map(|entry| {
                if Some(&entry.card.id) == deck.operator_card_id.as_ref() {
                    operator = Some(entry.card);
                    None
                } else {
                    Some(DeckBuilderCard {
                        card: entry.card,
                        quantity: entry.quantity,
                    })
                }
            })
            .collect();
        player.deck_builder_sideboard_cards = sideboard
            .into_iter()
            .map(|entry| DeckBuilderCard {
                card: entry.card,
                quantity: entry.quantity,
            })
            .collect();
        player.deck_builder_operator = operator;
        Ok(())
    }

    /// Ends the game and remembers who was watching, so a later visit from a
    /// new workflow instance can start the next game.
    pub(crate) fn finish_table(&mut self) {
        self.stage = CardGameStage::Finished;
        self.finished_instances = self.viewer_instances.values().cloned().collect();
    }

    /// Back to an empty lobby with the same settings. Seats, decks, the
    /// draft and the board all go; caches and who is watching stay.
    pub(crate) fn reset_table(&mut self) {
        let mut fresh = Self::new(self.init.clone());
        fresh.lobby_id = self.lobby_id.clone();
        fresh.format = self.format.clone();
        fresh.mode = self.mode.clone();
        fresh.viewer_instances = std::mem::take(&mut self.viewer_instances);
        fresh.pack_types_cache = std::mem::take(&mut self.pack_types_cache);
        fresh.kind_options = std::mem::take(&mut self.kind_options);
        fresh.supertype_options = std::mem::take(&mut self.supertype_options);
        fresh.unit_type_options = std::mem::take(&mut self.unit_type_options);
        fresh.artifact_type_options = std::mem::take(&mut self.artifact_type_options);
        fresh.effect_kind_options = std::mem::take(&mut self.effect_kind_options);
        fresh.mana_type_options = std::mem::take(&mut self.mana_type_options);
        fresh.trigger_event_options = std::mem::take(&mut self.trigger_event_options);
        fresh.stat_type_options = std::mem::take(&mut self.stat_type_options);
        fresh.expire_contract_options = std::mem::take(&mut self.expire_contract_options);
        fresh.effect_target_options = std::mem::take(&mut self.effect_target_options);
        fresh.card_options_loaded = self.card_options_loaded;
        *self = fresh;
    }

    /// A finished table resets for anyone; a live one only for its host.
    fn handle_reset(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        let host = self.host_id.as_deref() == Some(player_id);
        if self.stage != CardGameStage::Finished && !host && !self.players.is_empty() {
            return Err(anyhow!("Only the host can reset the table"));
        }
        self.reset_table();
        Ok(vec![])
    }

    pub(crate) fn record_instance_id(&mut self, player_id: &str, instance_id: &str) {
        if player_id != "system" {
            self.viewer_instances
                .insert(player_id.to_string(), instance_id.to_string());

            // Also update the player state if it exists, for consistency
            if let Some(player) = self.players.get_mut(player_id) {
                player.instance_id = instance_id.to_string();
            }
        }
    }

    fn emit_card_game_end_milestones(&mut self) {
        let format_str = match self.format {
            GameFormat::Classic => "classic",
            GameFormat::Operator => "operator",
        };

        // Winner is the remaining player in turn_order
        if let Some(winner_id) = self.turn_order.first().cloned() {
            // Practice AI seats are not users; a score ledger row for one fails.
            if self.is_ai_player(&winner_id) {
                return;
            }
            if let Some(winner) = self.players.get(&winner_id) {
                let mut meta = HashMap::new();
                meta.insert("won".into(), serde_json::Value::Bool(true));
                meta.insert(
                    "format".into(),
                    serde_json::Value::String(format_str.into()),
                );
                meta.insert("life_remaining".into(), serde_json::json!(winner.life));
                meta.insert("rounds".into(), serde_json::json!(self.round));
                meta.insert(
                    "player_count".into(),
                    serde_json::json!(self.initial_player_count),
                );

                if self.format == GameFormat::Operator {
                    meta.insert(
                        "operator_cast_count".into(),
                        serde_json::json!(winner.operator_cast_count),
                    );
                }

                self.deferred_effects.push(GameEffect::MilestoneEvent {
                    game_id: "card_game".into(),
                    event_key: "game_completed".into(),
                    user_id: winner_id,
                    lobby_id: String::new(),
                    meta,
                });
            }
        }
    }

    fn update_inner(&mut self, player_id: &str, action: CardGameAction) -> Result<Vec<GameEffect>> {
        tracing::info!(
            player = %player_id,
            action = %format!("{:?}", std::mem::discriminant(&action)),
            stage = %format!("{:?}", self.stage),
            "update_inner"
        );
        if matches!(action, CardGameAction::Reset) {
            return self.handle_reset(player_id);
        }

        // If the game is over (Awards stage), only allow read-only actions.
        // Anything else would trigger reconcile/fanout loops on zombie workflows.
        // Note: winner_info() returns turn_order[0] which is always set once
        // someone joins, so we check stage instead.
        if self.stage == crate::types::CardGameStage::Awards {
            match &action {
                CardGameAction::View { .. }
                | CardGameAction::ListCards { .. }
                | CardGameAction::Ping { .. }
                | CardGameAction::ListPackTypes
                | CardGameAction::GetUserPacks
                | CardGameAction::GetPackOpeningResults { .. }
                // The award claim is the whole point of this stage, and the
                // AI's claim arrives wrapped in AiExecute.
                | CardGameAction::DraftKeepCards { .. }
                | CardGameAction::AiExecute { .. }
                | CardGameAction::Reset => {}
                other => {
                    tracing::debug!(
                        player = %player_id,
                        action = ?std::mem::discriminant(other),
                        "Ignoring action on finished game"
                    );
                    return Ok(vec![]);
                }
            }
        }

        // Clear trigger log from previous action
        self.trigger_log.clear();
        // Cleanup old pings (older than 2 seconds)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.pings.retain(|p| now - p.timestamp < 2000);

        if !matches!(
            &action,
            CardGameAction::View { .. }
                | CardGameAction::ScryCommit { .. }
                | CardGameAction::SearchCommit { .. }
                | CardGameAction::SearchCancel
                | CardGameAction::TriggerChoice { .. }
                | CardGameAction::TriggerTargetSelect { .. }
                | CardGameAction::TriggerTargetCancel
                | CardGameAction::DiscardCommit { .. }
                | CardGameAction::Discard { .. }
                | CardGameAction::Sacrifice { .. }
                | CardGameAction::Ping { .. }
                | CardGameAction::ListPackTypes
                | CardGameAction::GetUserPacks
                | CardGameAction::OpenPack { .. }
                | CardGameAction::GetPackOpeningResults { .. }
                | CardGameAction::AdminGrantPack { .. }
                | CardGameAction::AddAi
                | CardGameAction::AiExecute { .. }
                | CardGameAction::DraftStart
                | CardGameAction::DraftPick { .. }
                | CardGameAction::DraftConfirmPool
                | CardGameAction::DraftKeepCards { .. }
        ) {
            // Check if ANY player has pending discard (discard blocks everyone)
            for (pid, player) in &self.players {
                if player.has_pending_discard() {
                    return Err(anyhow!(
                        "Player {} must resolve discard before continuing",
                        pid
                    ));
                }
            }

            // Check the acting player's pending actions
            if let Some(player) = self.players.get(player_id) {
                if player.has_pending_sacrifice() {
                    return Err(anyhow!("You must sacrifice a unit before continuing"));
                }
                if !player.scry.is_empty() {
                    return Err(anyhow!("Resolve scry before continuing"));
                }
                if player.has_pending_trigger_choice() {
                    return Err(anyhow!("Resolve trigger choice before continuing"));
                }
                if player.has_pending_trigger_target() {
                    return Err(anyhow!("Select a target for trigger before continuing"));
                }
            }
        }

        if matches!(&action, CardGameAction::View { .. }) {
            self.ensure_card_options_loaded();
            self.ensure_deck_choices_loaded_for(player_id);
        }

        match action {
            CardGameAction::View { instance_id, .. } => {
                if self.stage == CardGameStage::Finished
                    && !self.finished_instances.contains(&instance_id)
                {
                    self.reset_table();
                }
                return self.handle_view(player_id, instance_id);
            }
            CardGameAction::AddAi => {
                return self.handle_add_ai(player_id);
            }
            CardGameAction::Join {
                name,
                avatar,
                instance_id,
            } => {
                return self.handle_join(player_id, name, avatar, instance_id);
            }
            CardGameAction::AiExecute {
                player_id: ai_player_id,
                action,
            } => {
                if !self.is_ai_player(&ai_player_id) {
                    return Err(anyhow!("AI player not found"));
                }
                let deferred = std::mem::take(&mut self.deferred_effects);
                let mut effects = self.update_inner(&ai_player_id, *action)?;
                effects.extend(deferred);
                return Ok(effects);
            }
            CardGameAction::SetGameFormat { format } => {
                return self.handle_set_game_format(player_id, format);
            }
            CardGameAction::SetGameMode { mode } => {
                return self.handle_set_game_mode(player_id, mode);
            }
            CardGameAction::SetMatchFormat { format } => {
                return self.handle_set_match_format(player_id, format);
            }
            CardGameAction::SubmitSideboard { swaps } => {
                return self.handle_submit_sideboard(player_id, swaps);
            }
            CardGameAction::SelectDeck { deck_id } => {
                return self.handle_select_deck(player_id, deck_id);
            }
            CardGameAction::StartGame => {
                return self.handle_start_game(player_id);
            }
            CardGameAction::PlayLand { card_id } => {
                return self.handle_play_land(player_id, card_id);
            }
            CardGameAction::DeployNode { mana_type } => {
                if matches!(mana_type, ManaType::Colorless) {
                    return Err(anyhow!("Cannot deploy colorless nodes"));
                }
                return self.handle_deploy_node(player_id, mana_type);
            }
            CardGameAction::TapCard { card_id, .. } => {
                return self.handle_tap_card(player_id, card_id);
            }
            CardGameAction::CastSpell { card_id, targets } => {
                return self.handle_cast_spell(player_id, card_id, targets);
            }
            CardGameAction::ActivateAbility {
                card_id,
                ability_id,
                targets,
            } => {
                return self.handle_activate_ability(player_id, card_id, ability_id, targets);
            }
            CardGameAction::DeclareAttack { card_id, target } => {
                return self.handle_declare_attack(player_id, card_id, target);
            }
            CardGameAction::DeclareBlock {
                attacker_id,
                blocker_id,
            } => {
                return self.handle_declare_block(player_id, attacker_id, blocker_id);
            }
            CardGameAction::ScryCommit { top, bottom } => {
                return self.handle_scry_commit(player_id, top, bottom);
            }
            CardGameAction::SearchCommit { card_id } => {
                return self.handle_search_commit(player_id, card_id);
            }
            CardGameAction::SearchCancel => {
                return self.handle_search_cancel(player_id);
            }
            CardGameAction::TriggerChoice { accept } => {
                return self.handle_trigger_choice(player_id, accept);
            }
            CardGameAction::TriggerTargetSelect { target } => {
                return self.handle_trigger_target_select(player_id, target);
            }
            CardGameAction::TriggerTargetCancel => {
                return self.handle_trigger_target_cancel(player_id);
            }
            CardGameAction::DiscardCommit { card_ids } => {
                return self.handle_discard_commit(player_id, card_ids);
            }
            CardGameAction::Discard { card_id } => {
                return self.handle_discard(player_id, card_id);
            }
            CardGameAction::Sacrifice { card_id } => {
                return self.handle_sacrifice(player_id, card_id);
            }
            CardGameAction::ResolveStackTop => {
                return self.handle_resolve_stack_top(player_id);
            }
            CardGameAction::CancelStackTop => {
                return self.handle_cancel_stack_top(player_id);
            }
            CardGameAction::PassPriority => {
                return self.handle_pass_priority(player_id);
            }
            CardGameAction::NextStage => {
                return self.handle_next_stage(player_id);
            }
            CardGameAction::ConfirmAttackers => {
                return self.handle_confirm_attackers(player_id);
            }
            CardGameAction::ConfirmBlockers => {
                return self.handle_confirm_blockers(player_id);
            }
            CardGameAction::Ping { x, z } => {
                return self.handle_ping(player_id, x, z);
            }
            CardGameAction::ReorderBattlefield { card_ids } => {
                return self.handle_reorder_battlefield(player_id, card_ids);
            }
            CardGameAction::SaveCard { card } => {
                return self.handle_save_card(card);
            }
            CardGameAction::ListCards { pagination } => {
                return self.handle_list_cards(player_id, pagination);
            }
            CardGameAction::BuilderOpen => {
                return self.handle_builder_open(player_id);
            }
            CardGameAction::BuilderClose => {
                return self.handle_builder_close(player_id);
            }
            CardGameAction::BuilderLoadDeck { deck_id } => {
                return self.handle_builder_load_deck(player_id, deck_id);
            }
            CardGameAction::BuilderAddCard { deck_id, card_id } => {
                return self.handle_builder_add_card(player_id, deck_id, card_id);
            }
            CardGameAction::BuilderRemoveCard { deck_id, card_id } => {
                return self.handle_builder_remove_card(player_id, deck_id, card_id);
            }
            CardGameAction::BuilderAddSideboardCard { deck_id, card_id } => {
                return self.handle_builder_add_sideboard_card(player_id, deck_id, card_id);
            }
            CardGameAction::BuilderRemoveSideboardCard { deck_id, card_id } => {
                return self.handle_builder_remove_sideboard_card(player_id, deck_id, card_id);
            }
            CardGameAction::BuilderSetOperator { deck_id, card_id } => {
                return self.handle_builder_set_operator(player_id, deck_id, card_id);
            }
            CardGameAction::BuilderCloneDeck { deck_id } => {
                return self.handle_builder_clone_deck(player_id, deck_id);
            }
            CardGameAction::BuilderSetFilter { kind } => {
                return self.handle_builder_set_filter(player_id, kind);
            }
            CardGameAction::BuilderSetSort { sort } => {
                return self.handle_builder_set_sort(player_id, sort);
            }
            CardGameAction::BuilderCreateDeck => {
                return self.handle_builder_create_deck(player_id);
            }
            CardGameAction::BuilderAutobuild => {
                return self.handle_builder_autobuild(player_id);
            }
            CardGameAction::BrowserOpen { target_player_id } => {
                return self.handle_browser_open(player_id, target_player_id);
            }
            CardGameAction::BrowserClose => {
                return self.handle_browser_close(player_id);
            }
            CardGameAction::BrowserSetFilter { kind } => {
                return self.handle_browser_set_filter(player_id, kind);
            }
            CardGameAction::BrowserSetSort { sort } => {
                return self.handle_browser_set_sort(player_id, sort);
            }
            CardGameAction::BrowserPage { delta } => {
                return self.handle_browser_page(player_id, delta);
            }
            CardGameAction::DebugSetup => {
                return self.handle_debug_setup(player_id);
            }
            CardGameAction::DebugAiConcede => {
                return self.handle_debug_ai_concede(player_id);
            }
            // Pack actions
            CardGameAction::ListPackTypes => {
                return self.handle_list_pack_types(player_id);
            }
            CardGameAction::GetUserPacks => {
                return self.handle_get_user_packs(player_id);
            }
            CardGameAction::OpenPack { pack_instance_id } => {
                return self.handle_open_pack(player_id, pack_instance_id);
            }
            CardGameAction::GetPackOpeningResults { pack_instance_id } => {
                return self.handle_get_pack_opening_results(player_id, pack_instance_id);
            }
            CardGameAction::AdminGrantPack {
                user_id,
                pack_type_id,
            } => {
                return self.handle_admin_grant_pack(player_id, user_id, pack_type_id);
            }
            // Draft actions
            CardGameAction::Reset => {
                return self.handle_reset(player_id);
            }
            CardGameAction::DraftStart => {
                return self.handle_draft_start(player_id);
            }
            CardGameAction::DraftPick { index } => {
                return self.handle_draft_pick(player_id, index);
            }
            CardGameAction::DraftAutoPick => {
                return self.handle_draft_auto_pick(player_id);
            }
            CardGameAction::DraftConfirmPool => {
                return self.handle_draft_confirm_pool(player_id);
            }
            CardGameAction::DraftKeepCards { card_ids } => {
                return self.handle_draft_keep_cards(player_id, card_ids);
            }
            CardGameAction::SetMatchFormat { .. } | CardGameAction::SubmitSideboard { .. } => {
                return Ok(vec![]);
            }
        }
    }
}

impl GameLogic for CardGameState {
    type Action = CardGameAction;
    type View = CardGameView;
    type Effect = GameEffect;
    type InitArgs = CardGameInitArgs;

    fn new(args: Self::InitArgs) -> Self {
        let repository = args
            .pool
            .as_ref()
            .map(|pool| Arc::new(CardRepository::new(pool.clone())));
        let format = args.format.clone();
        let ruleset = args.ruleset.clone();
        Self {
            init: args,
            repository,
            format,
            ruleset,
            ..Default::default()
        }
    }

    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.viewer_instances.get(player_id).cloned()
    }

    fn update(&mut self, player_id: &str, action: Self::Action) -> Result<Vec<Self::Effect>> {
        let deferred = std::mem::take(&mut self.deferred_effects);
        let mut effects = self.update_inner(player_id, action)?;
        effects.extend(deferred);
        self.apply_passive_state_updates();
        Ok(effects)
    }

    fn view_for(&self, player_id: &str) -> Self::View {
        let pings = self.pings.clone();
        let active_player_name = self
            .active_player_id
            .as_ref()
            .and_then(|id| self.players.get(id))
            .map(|player| player.name.clone());

        let players = self
            .turn_order
            .iter()
            .filter_map(|id| self.players.get(id))
            .map(|player| {
                let (battlefield_infrastructure, battlefield_units): (Vec<_>, Vec<_>) = player
                    .battlefield
                    .iter()
                    .partition(|card| matches!(card.kind, CardKind::Infrastructure));

                let battlefield_infrastructure = battlefield_infrastructure
                    .into_iter()
                    .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Battlefield)))
                    .collect();

                let battlefield_units = battlefield_units
                    .into_iter()
                    .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Battlefield)))
                    .collect();

                let operator_zone = player
                    .operator_zone
                    .iter()
                    .map(|card| self.card_view_for(&player.id, card, Some(CardZone::OperatorZone)))
                    .collect();

                let grid = player
                    .grid
                    .iter()
                    .map(|node| GridNodeView {
                        mana_type: node.mana_type.clone(),
                        color: mana_color_for_type(&node.mana_type).to_string(),
                    })
                    .collect();

                PlayerSummaryView {
                    id: player.id.clone(),
                    name: player.name.clone(),
                    avatar_url: player.avatar_url.clone(),
                    is_ai: player.is_ai,
                    life: player.life,
                    hand_size: player.hand.len(),
                    battlefield_count: player.battlefield.len(),
                    battlefield_units,
                    battlefield_infrastructure,
                    operator_zone,
                    grid,
                    nodes_deployed_this_turn: player.nodes_deployed_this_turn,
                    infrastructures: Self::infrastructure_count(player),
                    mana_pool: player.mana_pool.clone(),
                    heat: player.heat,
                    is_active: self.is_active_player(&player.id),
                    deck_name: player.deck_name.clone(),
                    deck_colors: player.deck_colors.clone(),
                    library_size: player.deck.len(),
                    graveyard_size: player.graveyard.len(),
                    operator_damage: player.operator_damage.clone(),
                }
            })
            .collect::<Vec<_>>();

        let viewer = self.players.get(player_id).map(|player| {
            let (battlefield_infrastructure, battlefield_units): (Vec<_>, Vec<_>) = player
                .battlefield
                .iter()
                .partition(|card| matches!(card.kind, CardKind::Infrastructure));

            let battlefield_infrastructure = battlefield_infrastructure
                .into_iter()
                .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Battlefield)))
                .collect();

            let battlefield_units = battlefield_units
                .into_iter()
                .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Battlefield)))
                .collect();

            let scry = player
                .scry
                .iter()
                .map(|card| {
                    let mut view = self.card_view_for(&player.id, card, Some(CardZone::Library));
                    view.can_cast = false;
                    view.can_play_land = false;
                    view.can_attack = false;
                    view.can_tap = false;
                    view
                })
                .collect();

            let pending_trigger_choice = player.pending_trigger_choice().map(|choice| {
                if let PendingAction::TriggerChoice {
                    source_card_id,
                    source_card_name,
                    effect,
                    optional_cost,
                    ..
                } = choice
                {
                    TriggerChoiceView {
                        source_card_id: source_card_id.clone(),
                        source_card_name: source_card_name.clone(),
                        effect_text: Self::effect_text(effect, None),
                        cost_text: optional_cost.as_ref().and_then(format_ability_cost),
                    }
                } else {
                    unreachable!()
                }
            });

            let pending_trigger_target = player.pending_trigger_target().map(|pt| {
                if let PendingAction::TriggerTargetSelection {
                    source_card_id,
                    source_card_name,
                    effect,
                    target_kind,
                    optional,
                } = pt
                {
                    TriggerTargetSelectionView {
                        source_card_id: source_card_id.clone(),
                        source_card_name: source_card_name.clone(),
                        effect_text: Self::effect_text(effect, None),
                        target_kind: target_kind.clone(),
                        optional: *optional,
                    }
                } else {
                    unreachable!()
                }
            });

            let pending_discard = player.pending_discard().map(|pd| {
                if let PendingAction::Discard {
                    amount,
                    source_description,
                    source_card_id,
                } = pd
                {
                    PendingDiscardView {
                        amount: *amount,
                        source_description: source_description.clone(),
                        source_card: source_card_id
                            .as_ref()
                            .and_then(|card_id| self.find_card_with_owner_and_zone(card_id))
                            .map(|(owner_id, card, zone)| {
                                self.card_view_for(&owner_id, &card, Some(zone))
                            }),
                        hand: player
                            .hand
                            .iter()
                            .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Hand)))
                            .collect(),
                    }
                } else {
                    unreachable!()
                }
            });

            let pending_sacrifice = player.pending_sacrifice().map(|ps| {
                if let PendingAction::Sacrifice {
                    remaining,
                    source_card_id,
                    ability,
                    ..
                } = ps
                {
                    let source_name = player
                        .battlefield
                        .iter()
                        .find(|c| &c.id == source_card_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| ability.name.clone());
                    PendingSacrificeView {
                        remaining: *remaining,
                        source_card_name: source_name,
                    }
                } else {
                    unreachable!()
                }
            });

            let can_deploy_node = self.is_active_player(&player.id)
                && (self.stage == CardGameStage::Main
                    || self.stage == CardGameStage::PostCombatMain)
                && player.nodes_deployed_this_turn < 1
                // Deploying a node is sorcery-speed — not allowed while
                // anything is pending resolution on the stack or showcase.
                && self.stack.is_empty()
                && self.showcase.is_none();
            let deployable_node_types: Vec<DeployableNodeView> = if can_deploy_node {
                player
                    .color_identity
                    .iter()
                    .filter(|mt| !matches!(mt, ManaType::Colorless))
                    .map(|mt| {
                        let (label, image_url) = match mt {
                            ManaType::Neon => (
                                "Neon",
                                Some("https://cdn.tawks.app/cards/neon_relay_1775254809.png"),
                            ),
                            ManaType::Glitch => (
                                "Glitch",
                                Some("https://cdn.tawks.app/cards/glitch_conduit_1775254826.png"),
                            ),
                            ManaType::Chrome => (
                                "Chrome",
                                Some("https://cdn.tawks.app/cards/chrome_foundry_1775254839.png"),
                            ),
                            ManaType::Slag => (
                                "Slag",
                                Some("https://cdn.tawks.app/cards/slag_furnace_1775254853.png"),
                            ),
                            ManaType::Bio => (
                                "Bio",
                                Some("https://cdn.tawks.app/cards/bio_garden_1775254873.png"),
                            ),
                            ManaType::Colorless => ("Colorless", None),
                        };
                        DeployableNodeView {
                            value: mt.clone(),
                            label: label.to_string(),
                            color: mana_color_for_type(mt).to_string(),
                            image_url: image_url.map(|s| s.to_string()),
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            let grid = player
                .grid
                .iter()
                .map(|node| GridNodeView {
                    mana_type: node.mana_type.clone(),
                    color: mana_color_for_type(&node.mana_type).to_string(),
                })
                .collect();

            ViewerView {
                id: player.id.clone(),
                name: player.name.clone(),
                avatar_url: player.avatar_url.clone(),
                life: player.life,
                mana_pool: player.mana_pool.clone(),
                heat: player.heat,
                infrastructures_played_this_turn: player.infrastructures_played_this_turn,
                nodes_deployed_this_turn: player.nodes_deployed_this_turn,
                grid,
                can_deploy_node,
                deployable_node_types,
                is_active: self.is_active_player(&player.id),
                is_host: self.host_id.as_deref() == Some(player.id.as_str()),
                deck_id: player.deck_id.clone(),
                deck_name: player.deck_name.clone(),
                deck_colors: player.deck_colors.clone(),
                hand: player
                    .hand
                    .iter()
                    .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Hand)))
                    .chain(
                        player
                            .graveyard
                            .iter()
                            .filter(|card| {
                                card.gravecast_limit()
                                    .is_some_and(|limit| card.gravecast_uses < limit)
                            })
                            .map(|card| {
                                self.card_view_for(&player.id, card, Some(CardZone::Graveyard))
                            }),
                    )
                    .collect(),
                scry,
                battlefield_units,
                battlefield_infrastructure,
                operator_zone: player
                    .operator_zone
                    .iter()
                    .map(|card| self.card_view_for(&player.id, card, Some(CardZone::OperatorZone)))
                    .collect(),
                pending_trigger_choice,
                pending_trigger_target,
                pending_discard,
                pending_sacrifice,
                library_search_active: player.library_search_active,
                can_respond: self.effective_priority_holder() == Some(player.id.as_str()),
            }
        });

        let mut stack = self
            .stack
            .iter()
            .map(|item| {
                let targets = item
                    .targets
                    .iter()
                    .filter_map(|t| match t {
                        TargetRef::Player(pid) => self.players.get(pid).map(|p| StackTargetView {
                            id: pid.clone(),
                            name: p.name.clone(),
                            kind: "player".to_string(),
                        }),
                        TargetRef::Card(cid) => {
                            self.find_card_name(cid).map(|name| StackTargetView {
                                id: cid.clone(),
                                name,
                                kind: "card".to_string(),
                            })
                        }
                    })
                    .collect();
                StackItemView {
                    card: self.stack_item_card_view(item),
                    owner_id: item.owner_id.clone(),
                    owner_name: self
                        .players
                        .get(&item.owner_id)
                        .map(|player| player.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    targets,
                }
            })
            .collect::<Vec<_>>();

        // Include the showcase entry in the stack view so the UI displays it
        // in the spotlight zone alongside any stack items.
        if let Some(entry) = &self.showcase {
            let targets = entry
                .targets
                .iter()
                .filter_map(|t| match t {
                    TargetRef::Player(pid) => self.players.get(pid).map(|p| StackTargetView {
                        id: pid.clone(),
                        name: p.name.clone(),
                        kind: "player".to_string(),
                    }),
                    TargetRef::Card(cid) => self.find_card_name(cid).map(|name| StackTargetView {
                        id: cid.clone(),
                        name,
                        kind: "card".to_string(),
                    }),
                })
                .collect();
            stack.push(StackItemView {
                card: self.card_view_for(&entry.owner_id, &entry.card, Some(CardZone::Stack)),
                owner_id: entry.owner_id.clone(),
                owner_name: self
                    .players
                    .get(&entry.owner_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
                targets,
            });
        }

        let revealed_cards: Vec<RevealedCardView> = self
            .revealed_cards
            .iter()
            .map(|r| RevealedCardView {
                card: self.card_view_for(&r.owner_id, &r.card, Some(CardZone::Stack)),
                owner_id: r.owner_id.clone(),
                owner_name: self
                    .players
                    .get(&r.owner_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
                outcome_label: r.outcome_label.clone(),
            })
            .collect();

        let combat_attackers = self
            .active_player_id
            .as_ref()
            .and_then(|id| self.players.get(id))
            .map(|player| {
                player
                    .battlefield
                    .iter()
                    .filter(|card| self.combat_attackers.contains(&card.id))
                    .map(|card| self.card_view_for(&player.id, card, Some(CardZone::Battlefield)))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let combat_pairs = self
            .active_player_id
            .as_ref()
            .and_then(|active_id| {
                self.players
                    .get(active_id)
                    .map(|active| (active_id, active))
            })
            .map(|(active_id, active_player)| {
                self.combat_attackers
                    .iter()
                    .filter_map(|attacker_id| {
                        let attacker = active_player
                            .battlefield
                            .iter()
                            .find(|card| &card.id == attacker_id)?;
                        let target =
                            self.combat_targets.get(attacker_id).cloned().or_else(|| {
                                self.default_defender_id(active_id).map(TargetRef::Player)
                            });
                        let blockers = self
                            .combat_blockers
                            .get(attacker_id)
                            .map(|blocker_ids| {
                                let defender_player = match target {
                                    Some(TargetRef::Player(ref defender_id)) => {
                                        self.players.get(defender_id)
                                    }
                                    _ => None,
                                };
                                blocker_ids
                                    .iter()
                                    .filter_map(|blocker_id| {
                                        defender_player.and_then(|player| {
                                            player
                                                .battlefield
                                                .iter()
                                                .find(|card| &card.id == blocker_id)
                                        })
                                    })
                                    .map(|card| {
                                        self.card_view_for(
                                            active_id,
                                            card,
                                            Some(CardZone::Battlefield),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        let is_blocked = !blockers.is_empty();

                        let (target_kind, target_label, target_player_id, target_card_id) =
                            match &target {
                                Some(TargetRef::Player(defender_id)) => {
                                    let defender_player = self.players.get(defender_id.as_str());
                                    let label = defender_player
                                        .map(|player| player.name.clone())
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    ("player".to_string(), label, Some(defender_id.clone()), None)
                                }
                                Some(TargetRef::Card(card_id)) => {
                                    let owner_id = self
                                        .find_battlefield_card_owner(card_id)
                                        .map(|(owner_id, _)| owner_id);
                                    let (label, oid) = owner_id
                                        .and_then(|oid| {
                                            let owner = self.players.get(&oid)?;
                                            let card = owner
                                                .battlefield
                                                .iter()
                                                .find(|card| card.id == card_id.as_str())?;
                                            Some((format!("{} ({})", card.name, owner.name), oid))
                                        })
                                        .unwrap_or_else(|| {
                                            ("Unknown unit".to_string(), String::new())
                                        });
                                    (
                                        "unit".to_string(),
                                        label,
                                        if oid.is_empty() { None } else { Some(oid) },
                                        Some(card_id.clone()),
                                    )
                                }
                                None => ("unknown".to_string(), "Unknown".to_string(), None, None),
                            };
                        // Predict combat deaths for visual cues.
                        let (attacker_dies, blocker_dies) = if is_blocked {
                            let (a_power, a_tough, a_armor, a_shield, a_dt, a_ind, _) =
                                self.card_combat_stats(attacker);
                            let mut total_blocker_power = 0i32;
                            let mut any_blocker_dt = false;
                            let mut remaining_damage = a_power;
                            let mut b_dies = Vec::new();

                            let blocker_ids = self.combat_blockers.get(attacker_id);
                            let defender_id_for_blockers = match &target {
                                Some(TargetRef::Player(did)) => Some(did.as_str()),
                                _ => None,
                            };
                            let defender_player =
                                defender_id_for_blockers.and_then(|did| self.players.get(did));

                            if let Some(bids) = blocker_ids {
                                for bid in bids {
                                    let blocker_card = defender_player
                                        .and_then(|p| p.battlefield.iter().find(|c| &c.id == bid));
                                    if let Some(bc) = blocker_card {
                                        let (b_pow, b_tough, b_arm, b_shd, b_dt, b_ind, _) =
                                            self.card_combat_stats(bc);
                                        total_blocker_power += b_pow;
                                        if b_dt && b_pow > 0 {
                                            any_blocker_dt = true;
                                        }
                                        let lethal = b_tough + b_arm + b_shd;
                                        let dmg = if a_dt && remaining_damage > 0 {
                                            1
                                        } else {
                                            remaining_damage.min(lethal)
                                        };
                                        let dies = !b_ind
                                            && ((a_dt && dmg > 0) || (dmg + bc.damage) >= lethal);
                                        b_dies.push(dies);
                                        remaining_damage = (remaining_damage - dmg).max(0);
                                    } else {
                                        b_dies.push(false);
                                    }
                                }
                            }

                            let a_lethal = a_tough + a_armor + a_shield;
                            let a_dies = !a_ind
                                && ((any_blocker_dt && total_blocker_power > 0)
                                    || (total_blocker_power + attacker.damage) >= a_lethal);
                            (a_dies, b_dies)
                        } else {
                            (false, vec![])
                        };

                        Some(CombatPairView {
                            attacker: self.card_view_for(
                                active_id,
                                attacker,
                                Some(CardZone::Battlefield),
                            ),
                            blockers,
                            target_kind,
                            target_label,
                            target_player_id,
                            target_card_id,
                            is_blocked,
                            attacker_dies,
                            blocker_dies,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let viewer_player = self.players.get(player_id);
        let is_browser = viewer_player
            .map(|p| p.card_browser_active)
            .unwrap_or(false);
        let is_deck_building = viewer_player
            .map(|p| p.deck_builder_active && !p.card_browser_active)
            .unwrap_or(false);

        let targetable_units = if is_deck_building {
            self.players
                .values()
                .flat_map(|player| {
                    player
                        .battlefield
                        .iter()
                        .filter(|card| matches!(card.kind, CardKind::Unit(_)))
                        .map(|card| UnitTargetView {
                            id: card.id.clone(),
                            name: card.name.clone(),
                            owner_id: player.id.clone(),
                            owner_name: player.name.clone(),
                            power: card.power.map(|base| {
                                base + self.modifier_amount_for_card(&card.id, StatType::Power)
                            }),
                            toughness: card.toughness.map(|base| {
                                base + self.modifier_amount_for_card(&card.id, StatType::Toughness)
                            }),
                            armor: self.modifier_amount_for_card(&card.id, StatType::Armor),
                            shield: self.modifier_amount_for_card(&card.id, StatType::Shield),
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        let browser_page_cards = viewer_player
            .map(|p| {
                p.library_search_results
                    .iter()
                    .map(|card| {
                        if is_browser {
                            self.card_browser_view_for(card)
                        } else {
                            self.card_view_for(player_id, card, Some(CardZone::Library))
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let browser_page_size = browser_page_cards.len() as i32;
        let mut deck_builder_source_cards = browser_page_cards.clone();

        // Ensure operator is in the cards list for the UI
        if !is_browser {
            if let Some(p) = viewer_player {
                if let Some(operator) = &p.deck_builder_operator {
                    if !deck_builder_source_cards
                        .iter()
                        .any(|c| c.id == operator.id)
                    {
                        deck_builder_source_cards.push(self.card_view_for(
                            player_id,
                            operator,
                            Some(CardZone::Lobby),
                        ));
                    }
                }
            }
        }

        let deck_builder_library = if let Some(filter) =
            viewer_player.and_then(|p| p.deck_builder_filter_kind.as_deref())
        {
            let filters: Vec<String> = filter
                .split(|ch| ch == ',' || ch == '|')
                .map(|entry| entry.trim().to_ascii_lowercase())
                .filter(|entry| !entry.is_empty())
                .collect();
            let mana_filters: Vec<&str> = filters
                .iter()
                .map(|value| value.as_str())
                .filter(|value| {
                    matches!(
                        *value,
                        "neon" | "glitch" | "chrome" | "slag" | "bio" | "colorless"
                    )
                })
                .collect();
            let kind_filters: Vec<&str> = filters
                .iter()
                .map(|value| value.as_str())
                .filter(|value| {
                    !matches!(
                        *value,
                        "neon" | "glitch" | "chrome" | "slag" | "bio" | "colorless" | "owned"
                    )
                })
                .collect();
            deck_builder_source_cards
                .iter()
                .cloned()
                .filter(|card| {
                    if filters.is_empty() {
                        return true;
                    }
                    let kind_tag = card.kind_tag.to_ascii_lowercase();
                    let kind_ok = if kind_filters.is_empty() {
                        true
                    } else {
                        kind_filters.iter().any(|kind| *kind == kind_tag)
                    };
                    let mana_ok = if mana_filters.is_empty() {
                        true
                    } else {
                        mana_cost_matches_any_filter(&card.cost, &mana_filters)
                            || mana_filters.iter().any(|f| {
                                let filter_mana = mana_type_for_color_value(f);
                                card.colors.iter().any(|hex| {
                                    filter_mana.as_ref() == mana_type_for_color_value(hex).as_ref()
                                })
                            })
                    };
                    kind_ok && mana_ok
                })
                .collect::<Vec<_>>()
        } else {
            deck_builder_source_cards.clone()
        };

        // Apply "owned" filter if present
        let deck_builder_library = if let Some(filter) =
            viewer_player.and_then(|p| p.deck_builder_filter_kind.as_deref())
        {
            let has_owned = filter
                .split(|ch| ch == ',' || ch == '|')
                .any(|entry| entry.trim().eq_ignore_ascii_case("owned"));
            if has_owned {
                deck_builder_library
                    .into_iter()
                    .filter(|card| card.owned)
                    .collect()
            } else {
                deck_builder_library
            }
        } else {
            deck_builder_library
        };

        // Apply sort order
        let mut deck_builder_library = deck_builder_library;
        if let Some(sort) = viewer_player.and_then(|p| p.deck_builder_sort_order.as_deref()) {
            let desc = sort.ends_with("_desc");
            let base = sort.trim_end_matches("_desc");
            match base {
                "name" => deck_builder_library.sort_by(|a, b| a.name.cmp(&b.name)),
                "color" => deck_builder_library.sort_by(|a, b| {
                    let a_color = a.colors.first().map(String::as_str).unwrap_or("");
                    let b_color = b.colors.first().map(String::as_str).unwrap_or("");
                    a_color.cmp(b_color)
                }),
                "obtained" => deck_builder_library.sort_by(|a, b| {
                    let a_ts = a.obtained_at.as_deref().unwrap_or("~");
                    let b_ts = b.obtained_at.as_deref().unwrap_or("~");
                    a_ts.cmp(b_ts)
                }),
                "created_at" => deck_builder_library.sort_by(|a, b| {
                    let a_ts = a.created_at.as_deref().unwrap_or("~");
                    let b_ts = b.created_at.as_deref().unwrap_or("~");
                    a_ts.cmp(b_ts)
                }),
                "rarity" => deck_builder_library.sort_by(|a, b| {
                    let rank = |r: &CardRarity| match r {
                        CardRarity::Singularity => 0,
                        CardRarity::Classified => 1,
                        CardRarity::Restricted => 2,
                        CardRarity::Private => 3,
                        CardRarity::Public => 4,
                    };
                    rank(&a.rarity).cmp(&rank(&b.rarity))
                }),
                "value" => deck_builder_library.sort_by(|a, b| {
                    let a_val = a.value.unwrap_or(0.0);
                    let b_val = b.value.unwrap_or(0.0);
                    b_val
                        .partial_cmp(&a_val)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                _ => {}
            }
            if desc {
                deck_builder_library.reverse();
            }
        }

        let deck_builder_cards = if is_deck_building {
            viewer_player
                .map(|p| {
                    p.deck_builder_cards
                        .iter()
                        .map(|entry| DeckCardView {
                            card: self.card_view_for(player_id, &entry.card, Some(CardZone::Lobby)),
                            quantity: entry.quantity,
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };

        let deck_builder_sideboard = if is_deck_building {
            viewer_player
                .map(|p| {
                    p.deck_builder_sideboard_cards
                        .iter()
                        .map(|entry| DeckCardView {
                            card: self.card_view_for(player_id, &entry.card, Some(CardZone::Lobby)),
                            quantity: entry.quantity,
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };

        let cards = if is_browser || is_deck_building {
            vec![]
        } else {
            browser_page_cards
        };

        let deck_builder = if let Some(p) = viewer_player {
            DeckBuilderView {
                active: p.deck_builder_active,
                deck_id: p.deck_builder_deck_id.clone(),
                name: p.deck_builder_name.clone(),
                description: p.deck_builder_description.clone(),
                format: p.deck_builder_format.clone(),
                operator_card_id: p.deck_builder_operator_card_id.clone(),
                autobuild_colors: p.deck_builder_autobuild_colors.clone(),
                operator: if is_browser {
                    None
                } else {
                    p.deck_builder_operator
                        .as_ref()
                        .map(|c| self.card_view_for(player_id, c, Some(CardZone::Lobby)))
                },
                cards: deck_builder_cards,
                sideboard: deck_builder_sideboard,
                filter_kind: p.deck_builder_filter_kind.clone(),
                sort_order: p.deck_builder_sort_order.clone(),
                library: deck_builder_library,
                browser_active: p.card_browser_active,
                browser_source: p
                    .card_browser_source
                    .as_ref()
                    .map(|source| source.key().to_string()),
                browser_title: p.card_browser_title.clone(),
                browser_subtitle: p.card_browser_subtitle.clone(),
                collection_page: p.collection_page,
                collection_total: p.collection_total,
                collection_has_prev: p.collection_page > 0,
                collection_has_next: (p.collection_page + 1)
                    * crate::handlers::COLLECTION_PAGE_SIZE
                    < p.collection_total.max(browser_page_size),
            }
        } else {
            DeckBuilderView {
                active: false,
                deck_id: None,
                name: None,
                description: None,
                format: None,
                operator_card_id: None,
                autobuild_colors: None,
                operator: None,
                cards: vec![],
                sideboard: vec![],
                filter_kind: None,
                sort_order: None,
                library: vec![],
                browser_active: false,
                browser_source: None,
                browser_title: None,
                browser_subtitle: None,
                collection_page: 0,
                collection_total: 0,
                collection_has_prev: false,
                collection_has_next: false,
            }
        };

        CardGameView {
            stage: self.stage.clone(),
            format: self.format.clone(),
            ruleset: self.ruleset.clone(),
            mode: self.mode.clone(),
            active_player_id: self.active_player_id.clone(),
            active_player_name,
            priority_player_id: self.effective_priority_holder().map(str::to_string),
            round: self.round,
            players,
            viewer,
            deck_choices: self
                .deck_choices_by_player
                .get(player_id)
                .cloned()
                .unwrap_or_default(),
            deck_builder,
            stack,
            revealed_cards,
            combat_phase: match self.combat_phase {
                CombatPhase::Idle => "idle".to_string(),
                CombatPhase::DeclareAttackers => "declare_attackers".to_string(),
                CombatPhase::DeclareBlockers => "declare_blockers".to_string(),
            },
            combat_blocker_id: self.combat_blocker_id.clone(),
            combat_attackers,
            combat_pairs,
            targetable_units,
            pings,
            cards,
            kind_options: self.kind_options.clone(),
            supertype_options: self.supertype_options.clone(),
            unit_type_options: self.unit_type_options.clone(),
            artifact_type_options: self.artifact_type_options.clone(),
            effect_kind_options: self.effect_kind_options.clone(),
            mana_type_options: self.mana_type_options.clone(),
            trigger_event_options: self.trigger_event_options.clone(),
            stat_type_options: self.stat_type_options.clone(),
            expire_contract_options: self.expire_contract_options.clone(),
            effect_target_options: self.effect_target_options.clone(),
            winner_player_id: self.winner_info().0,
            winner_player_name: self.winner_info().1,
            match_state: self.match_state.as_ref().map(|m| MatchStateView {
                format: m.format,
                current_game: m.current_game,
                wins_required: m.format.wins_required(),
                wins: m.wins.clone(),
                series_winner: m.series_winner.clone(),
                pending_sideboard: m.pending_sideboard.clone(),
                game_winners: m.game_winners.clone(),
            }),
            viewer_sideboard: self
                .players
                .get(player_id)
                .map(|p| {
                    p.sideboard
                        .iter()
                        .map(|c| self.card_view_for(player_id, c, None))
                        .collect()
                })
                .unwrap_or_default(),
            viewer_main_deck: self
                .players
                .get(player_id)
                .map(|p| {
                    let is_real = |c: &crate::card::Card| {
                        !c.id.starts_with("token-") && !c.id.starts_with("temp-")
                    };
                    let mut out: Vec<CardView> = Vec::new();
                    for zone in [&p.deck, &p.hand, &p.battlefield, &p.graveyard, &p.exile] {
                        for c in zone.iter().filter(|c| is_real(c)) {
                            out.push(self.card_view_for(player_id, c, None));
                        }
                    }
                    out
                })
                .unwrap_or_default(),
            draft: self.draft_view_for(player_id),
            draft_awards: self.draft_awards_view_for(player_id),
            pack_types: self.pack_types_cache.clone(),
            user_packs: self
                .user_packs_cache
                .get(player_id)
                .cloned()
                .unwrap_or_default(),
            pack_open_result: self
                .pack_open_result_cache
                .get(player_id)
                .cloned()
                .unwrap_or(None),
            pack_opening_results: self
                .pack_opening_results_cache
                .get(player_id)
                .cloned()
                .unwrap_or_default(),
            trigger_log: self.trigger_log.clone(),
            game_log: self.game_log.clone(),
        }
    }

    fn tick(&mut self) -> Vec<Self::Action> {
        self.apply_passive_state_updates();
        // Expire revealed-card displays after 4 seconds so they don't
        // linger forever on the client.
        let now = chrono::Utc::now();
        self.revealed_cards
            .retain(|r| (now - r.revealed_at).num_seconds() < 4);
        let mut actions = self.collect_display_timer_actions();
        actions.extend(self.maybe_auto_confirm_blockers());
        actions.extend(self.collect_ai_tick_actions());
        actions
    }

    fn get_player_ids(&self) -> Vec<String> {
        let mut ids: HashSet<String> = self.players.keys().cloned().collect();
        ids.extend(self.viewer_instances.keys().cloned());
        ids.into_iter().collect()
    }

    fn should_fanout(&self, action: &Self::Action) -> bool {
        !matches!(
            action,
            CardGameAction::View { .. } | CardGameAction::ListCards { .. }
        )
    }
}

impl CardGameState {
    pub(crate) fn apply_passive_state_updates(&mut self) {
        let mut cards_to_remove = Vec::new();
        for (pid, player) in &self.players {
            for card in &player.battlefield {
                match &card.kind {
                    CardKind::Agent => {
                        if let Some(loyalty) = card.loyalty {
                            if loyalty <= 0 {
                                cards_to_remove.push((pid.clone(), card.id.clone()));
                            }
                        }
                    }
                    CardKind::Unit(_) => {
                        let (_, toughness, _, _, _, is_indestructible, _) =
                            self.card_combat_stats(card);
                        // Unit dies when accumulated damage meets or exceeds its
                        // effective toughness. Damage persists until end of turn.
                        if !is_indestructible && (toughness <= 0 || card.damage >= toughness) {
                            cards_to_remove.push((pid.clone(), card.id.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }

        for (pid, cid) in cards_to_remove {
            self.remove_from_battlefield_and_attachments(&pid, &cid);
        }

        // 2. Player Elimination
        let dead_player_ids: Vec<String> = self
            .players
            .iter()
            .filter(|(_, p)| p.life <= 0)
            .map(|(id, _)| id.clone())
            .collect();

        if !dead_player_ids.is_empty() {
            for pid in &dead_player_ids {
                // Clear their battlefield
                let card_ids: Vec<String> = self
                    .players
                    .get(pid)
                    .map(|p| p.battlefield.iter().map(|c| c.id.clone()).collect())
                    .unwrap_or_default();
                for cid in card_ids {
                    self.remove_from_battlefield_and_attachments(pid, &cid);
                }

                self.players.remove(pid);
                self.turn_order.retain(|id| id != pid);

                if self.active_player_id.as_ref() == Some(pid) {
                    // Active player eliminated! Move to next player in turn order
                    if self.turn_order.is_empty() {
                        self.active_player_id = None;
                    } else {
                        // Find where they were and pick the next one (already rotated because we retained)
                        // This is a simplification, might need more robust turn passing logic
                        self.active_player_id = Some(self.turn_order[0].clone());
                    }
                }
            }

            // Check Game End
            if self.turn_order.len() <= 1
                && self.stage != CardGameStage::Lobby
                && self.stage != CardGameStage::SideboardSwap
            {
                let was_finished =
                    matches!(self.stage, CardGameStage::Awards | CardGameStage::Finished);
                let winner_id = self.turn_order.first().cloned();

                // For Bo3/multi-game matches, each game end tallies a win.
                // If the series is still undecided, we stage into
                // `SideboardSwap` to collect swaps for the next game. Only
                // when someone hits `wins_required` do we actually mark the
                // match Finished and emit CompleteWorkflow.
                let match_undecided = if let (Some(_), Some(winner)) =
                    (self.match_state.as_ref(), winner_id.as_ref())
                    && self.draft.is_none()
                {
                    let pending: Vec<String> = {
                        let match_state = self.match_state.as_ref().unwrap();
                        match_state
                            .wins
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .filter(|pid| !self.is_ai_player(pid))
                            .collect()
                    };
                    let match_state = self.match_state.as_mut().unwrap();
                    match_state.game_winners.push(winner.clone());
                    let entry = match_state.wins.entry(winner.clone()).or_insert(0);
                    *entry += 1;
                    if *entry >= match_state.format.wins_required() {
                        match_state.series_winner = Some(winner.clone());
                        false
                    } else {
                        match_state.pending_sideboard = pending;
                        true
                    }
                } else {
                    false
                };

                // Draft games always flow into Awards at the end — they
                // never enter the Bo3 sideboard path even if somehow
                // match_state slipped in.
                self.stage = if self.draft.is_some() {
                    CardGameStage::Awards
                } else if match_undecided {
                    CardGameStage::SideboardSwap
                } else {
                    CardGameStage::Finished
                };

                // AIs pre-apply their matchup-specific swaps so their
                // decks adapt between games. Humans swap through the UI.
                if match_undecided {
                    self.apply_ai_sideboards_for_next_game();
                }

                // If every player left in the match is an AI (or the game
                // was undecided with no humans for any reason),
                // pending_sideboard is empty and no one will ever submit —
                // auto-advance to the next game so AI-vs-AI Bo3 matches
                // don't softlock.
                if match_undecided
                    && self
                        .match_state
                        .as_ref()
                        .map(|m| m.pending_sideboard.is_empty())
                        .unwrap_or(false)
                {
                    if let Err(err) = self.start_next_game_in_match() {
                        tracing::warn!(
                            "start_next_game_in_match failed during auto-advance: {err}"
                        );
                    }
                }

                if !was_finished && !match_undecided {
                    self.emit_card_game_end_milestones();
                    // Non-draft games have no awards screen — complete the
                    // workflow for every connected human so the UI shows a
                    // game-over message instead of sitting on a frozen board.
                    if self.draft.is_none() {
                        let winner_name = winner_id
                            .as_ref()
                            .and_then(|id| self.players.get(id).map(|p| p.name.clone()))
                            .unwrap_or_else(|| "Winner".to_string());
                        let recipients: Vec<(String, bool)> = self
                            .viewer_instances
                            .keys()
                            .cloned()
                            .filter_map(|pid| {
                                if self.is_ai_player(&pid) {
                                    return None;
                                }
                                let is_winner = winner_id.as_ref() == Some(&pid);
                                Some((pid, is_winner))
                            })
                            .collect();
                        for (pid, is_winner) in recipients {
                            let message = if is_winner {
                                "You win!".to_string()
                            } else {
                                format!("{} wins.", winner_name)
                            };
                            self.deferred_effects.push(GameEffect::CompleteWorkflow {
                                user_id: pid,
                                message,
                                responses: HashMap::new(),
                                close_active_modal: true,
                            });
                        }
                    }
                }
            }
        }

        // Auto-pass priority only while the holder is a non-human (AI)
        // player. This keeps the showcase/stack visible for humans (who
        // must explicitly pass or wait for the display timer) while
        // avoiding a 1-second stall when the next priority holder is an
        // AI that can't respond anyway.
        self.auto_pass_ai_holders();
    }

    /// Walk the priority chain and auto-pass for every AI player at the
    /// front that can't respond (or is the caster). Stops as soon as a
    /// human holds priority, so humans always see their response window.
    fn auto_pass_ai_holders(&mut self) {
        const LIMIT: usize = 64;
        let mut iterations = 0;
        while !self.stack.is_empty() || self.showcase.is_some() {
            if iterations >= LIMIT {
                break;
            }
            iterations += 1;

            let Some(holder_id) = self.priority_holder.clone() else {
                break;
            };
            let is_human = self
                .players
                .get(&holder_id)
                .map(|p| !p.is_ai)
                .unwrap_or(true);
            if is_human {
                // Human's turn to respond — stop auto-passing.
                break;
            }
            let is_caster = if !self.stack.is_empty() {
                self.stack
                    .last()
                    .map(|item| item.owner_id == holder_id)
                    .unwrap_or(false)
            } else {
                self.showcase
                    .as_ref()
                    .map(|s| s.owner_id == holder_id)
                    .unwrap_or(false)
            };
            let can_respond = self
                .players
                .get(&holder_id)
                .map(|p| self.player_has_response_available(p))
                .unwrap_or(false);

            if !can_respond || is_caster {
                self.consecutive_priority_passes += 1;
                if self.consecutive_priority_passes >= self.turn_order.len() {
                    if !self.stack.is_empty() {
                        let active_id = self
                            .active_player_id
                            .clone()
                            .unwrap_or_else(|| holder_id.clone());
                        let _ = self.handle_resolve_stack_top(&active_id);
                    }
                    if self.stack.is_empty() {
                        if let Some(entry) = self.showcase.take() {
                            self.resolve_showcase(entry);
                        }
                    }
                    self.reset_priority();
                    if !self.stack.is_empty() || self.showcase.is_some() {
                        self.priority_holder = self.active_player_id.clone();
                        self.consecutive_priority_passes = 0;
                    }
                } else {
                    self.priority_holder = self.next_player_after(&holder_id);
                }
            } else {
                // AI has a possible response; let its agent decide on the
                // next tick rather than force a pass here.
                break;
            }
        }
    }

    /// Force-resolve the entire stack and any showcase entry, ignoring
    /// opponent response availability. Used by the sim/AI probe so scoring
    /// can see the effect of a spell/ability — otherwise both targets of
    /// e.g. a bounce would leave the probe state identical (spell sitting
    /// on the stack) and the AI couldn't distinguish self vs opponent
    /// targets. Opponent responses are already modeled at deeper search
    /// depth, so it's safe (and necessary) to just resolve here.
    pub fn sim_force_resolve_pending(&mut self) {
        use crate::handlers::StackHandlers;
        const LIMIT: usize = 256;
        let mut iterations = 0;
        while (!self.stack.is_empty() || self.showcase.is_some()) && iterations < LIMIT {
            iterations += 1;
            let active_id = self
                .active_player_id
                .clone()
                .or_else(|| self.turn_order.first().cloned());
            let Some(active_id) = active_id else { break };
            if !self.stack.is_empty() {
                if self.handle_resolve_stack_top(&active_id).is_err() {
                    break;
                }
            } else if let Some(entry) = self.showcase.take() {
                self.resolve_showcase(entry);
            }
            self.apply_passive_state_updates();
        }
    }

    /// Auto-pass priority for players who can't respond or who own the top
    /// stack/showcase item. When `respect_display_timer` is true, waits at
    /// least 5 seconds after the last stack push before auto-resolving,
    /// giving human players time to read what was played.
    pub(crate) fn auto_pass_priority(&mut self, respect_display_timer: bool) {
        if respect_display_timer {
            const MIN_DISPLAY_SECS: i64 = 5;
            let has_pending = !self.stack.is_empty() || self.showcase.is_some();
            if has_pending {
                if let Some(push_at) = self.last_stack_push_at {
                    let elapsed = (chrono::Utc::now() - push_at).num_seconds();
                    if elapsed < MIN_DISPLAY_SECS {
                        return; // wait for the display window
                    }
                }
            }
        }

        const AUTO_PRIORITY_PASS_LIMIT: usize = 256;
        let mut auto_pass_iterations = 0usize;
        while !self.stack.is_empty() || self.showcase.is_some() {
            if auto_pass_iterations >= AUTO_PRIORITY_PASS_LIMIT {
                let top_summary = self
                    .stack
                    .last()
                    .map(|item| match &item.kind {
                        StackItemKind::Spell(card) => {
                            format!("spell:{}({})", card.name, card.id)
                        }
                        StackItemKind::Ability {
                            source_card_id,
                            ability,
                        } => format!("ability:{}:{}", source_card_id, ability.id),
                    })
                    .unwrap_or_else(|| "<empty>".to_string());
                tracing::error!(
                    stack_len = self.stack.len(),
                    top = %top_summary,
                    "auto-priority-pass loop exceeded {AUTO_PRIORITY_PASS_LIMIT} iterations; clearing stack to recover"
                );
                self.stack.clear();
                self.showcase = None;
                self.reset_priority();
                break;
            }
            auto_pass_iterations += 1;

            let Some(holder_id) = self.priority_holder.clone() else {
                break;
            };

            let is_caster = if !self.stack.is_empty() {
                self.stack
                    .last()
                    .map(|item| item.owner_id == holder_id)
                    .unwrap_or(false)
            } else {
                // Showcase: the caster also auto-passes
                self.showcase
                    .as_ref()
                    .map(|s| s.owner_id == holder_id)
                    .unwrap_or(false)
            };

            let can_respond = self
                .players
                .get(&holder_id)
                .map(|p| self.player_has_response_available(p))
                .unwrap_or(false);

            if !can_respond || is_caster {
                self.consecutive_priority_passes += 1;
                if self.consecutive_priority_passes >= self.turn_order.len() {
                    if !self.stack.is_empty() {
                        let active_id = self
                            .active_player_id
                            .clone()
                            .unwrap_or_else(|| holder_id.clone());
                        let _ = self.handle_resolve_stack_top(&active_id);
                    }
                    // If the stack is now empty and there's a showcase, resolve it
                    if self.stack.is_empty() {
                        if let Some(entry) = self.showcase.take() {
                            self.resolve_showcase(entry);
                        }
                    }
                    self.reset_priority();
                    // If more items remain on the stack, re-assign priority
                    if !self.stack.is_empty() || self.showcase.is_some() {
                        self.priority_holder = self.active_player_id.clone();
                        self.consecutive_priority_passes = 0;
                    }
                } else {
                    self.priority_holder = self.next_player_after(&holder_id);
                }
            } else {
                break;
            }
        }
    }

    fn collect_ai_tick_actions(&self) -> Vec<CardGameAction> {
        const MAX_AI_ACTIONS_PER_TICK: usize = 24;

        let mut probe = self.clone();
        let mut actions = Vec::new();

        for _ in 0..MAX_AI_ACTIONS_PER_TICK {
            let Some((player_id, action)) = probe.next_ai_action() else {
                break;
            };
            let execute = CardGameAction::AiExecute {
                player_id: player_id.clone(),
                action: Box::new(action.clone()),
            };

            // Claiming awards writes card instances through the shared repository,
            // so running it on the probe would grant them once per tick.
            if probe.stage == CardGameStage::Awards {
                actions.push(execute);
                break;
            }
            if probe.update(&player_id, action).is_err() {
                break;
            }
            actions.push(execute);
            // The real game leaves a spell on the stack for its display
            // window before resolving it, so plans made past that point
            // would arrive while it is still there. Replan next tick.
            if !probe.stack.is_empty() || probe.showcase.is_some() {
                break;
            }
        }

        actions
    }

    /// After the minimum display window (5 seconds) has elapsed, auto-pass
    /// priority for any human player who is holding it AND has no legal
    /// response available (so the game doesn't stall). If the human COULD
    /// do something — cast an instant, activate a tap ability, etc. — we
    /// leave priority with them and let them choose to act or manually
    /// pass. This is only called from the real game tick; simulations
    /// skip this so they don't block on wall-clock time.
    fn collect_display_timer_actions(&mut self) -> Vec<CardGameAction> {
        const MIN_DISPLAY_SECS: i64 = 5;
        let has_pending = !self.stack.is_empty() || self.showcase.is_some();
        if !has_pending {
            return vec![];
        }
        let Some(push_at) = self.last_stack_push_at else {
            return vec![];
        };
        let elapsed = (chrono::Utc::now() - push_at).num_seconds();
        if elapsed < MIN_DISPLAY_SECS {
            return vec![];
        }

        // If the current priority holder is a human WITH a legal response
        // available, don't auto-resolve — it's their turn to decide.
        if let Some(holder_id) = self.priority_holder.clone() {
            if let Some(holder) = self.players.get(&holder_id) {
                let is_human = !holder.is_ai;
                if is_human && self.player_has_response_available(holder) {
                    return vec![];
                }
            }
        }

        self.auto_pass_priority(false);
        vec![]
    }

    /// Auto-confirm blockers for a human defender who has no untapped units
    /// to block with and no instant-speed responses available. Runs in the
    /// same tick path as collect_display_timer_actions so the UI doesn't
    /// freeze on an empty declare-blockers window.
    fn maybe_auto_confirm_blockers(&mut self) -> Vec<CardGameAction> {
        if self.stage != CardGameStage::Combat || self.combat_phase != CombatPhase::DeclareBlockers
        {
            return vec![];
        }
        let Some(blocker_id) = self.combat_blocker_id.clone() else {
            return vec![];
        };
        let Some(player) = self.players.get(&blocker_id) else {
            return vec![];
        };
        if player.is_ai {
            return vec![];
        }
        if self.player_has_potential_blockers(&blocker_id) {
            return vec![];
        }
        let _ = self.handle_confirm_blockers(&blocker_id);
        vec![]
    }

    fn next_ai_action(&self) -> Option<(String, CardGameAction)> {
        match self.stage {
            CardGameStage::Lobby => self.next_ai_lobby_action(),
            CardGameStage::Draft => self.next_ai_draft_action(),
            CardGameStage::DraftBuilding => self.next_ai_draft_build_action(),
            CardGameStage::Awards => self.next_ai_awards_action(),
            CardGameStage::Main
            | CardGameStage::Combat
            | CardGameStage::PostCombatMain
            | CardGameStage::End => self.next_ai_gameplay_action(),
            CardGameStage::SideboardSwap => None,
            CardGameStage::Finished => None,
        }
    }

    fn next_ai_lobby_action(&self) -> Option<(String, CardGameAction)> {
        self.turn_order.iter().find_map(|player_id| {
            let player = self.players.get(player_id)?;
            if !player.is_ai || player.deck_id.is_some() {
                return None;
            }
            let deck_id = self.default_ai_deck_id(player_id)?;
            Some((player_id.clone(), CardGameAction::SelectDeck { deck_id }))
        })
    }

    fn next_ai_draft_action(&self) -> Option<(String, CardGameAction)> {
        let draft = self.draft.as_ref()?;
        let player_id = draft.current_picker.clone()?;
        if !self.is_ai_player(&player_id) {
            return None;
        }
        let index = crate::handlers::choose_auto_draft_pick(draft, &player_id)?;
        Some((player_id, CardGameAction::DraftPick { index }))
    }

    fn next_ai_draft_build_action(&self) -> Option<(String, CardGameAction)> {
        let draft = self.draft.as_ref()?;
        self.turn_order.iter().find_map(|player_id| {
            if !self.is_ai_player(player_id) || draft.confirmed.contains(player_id) {
                return None;
            }
            let player = self.players.get(player_id)?;
            let needs_build =
                player.deck_builder_cards.is_empty() && player.deck_builder_operator.is_none();
            Some((
                player_id.clone(),
                if needs_build {
                    CardGameAction::BuilderAutobuild
                } else {
                    CardGameAction::DraftConfirmPool
                },
            ))
        })
    }

    fn next_ai_awards_action(&self) -> Option<(String, CardGameAction)> {
        let winner_id = self.turn_order.first()?.clone();
        if !self.is_ai_player(&winner_id) {
            return None;
        }
        let draft = self.draft.as_ref()?;
        if draft.award_selections.contains_key(&winner_id) {
            return None;
        }

        let mut cards = draft
            .drafted
            .values()
            .flat_map(|cards| cards.iter().cloned())
            .collect::<Vec<_>>();
        cards.sort_by(|a, b| {
            crate::handlers::draft_card_base_score(b)
                .partial_cmp(&crate::handlers::draft_card_base_score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        let card_ids = cards
            .into_iter()
            .take(crate::handlers::DRAFT_WINNER_PICK_COUNT)
            .map(|card| card.id)
            .collect::<Vec<_>>();
        Some((winner_id, CardGameAction::DraftKeepCards { card_ids }))
    }

    /// An AI seat with no legal move would freeze the table for everyone, so
    /// say what it is stuck on and try the ways out of a prompt or a phase.
    fn ai_stall_fallback(&self, actor_id: &str) -> Option<(String, CardGameAction)> {
        if let Some(player) = self.players.get(actor_id) {
            tracing::warn!(
                player = %actor_id,
                stage = ?self.stage,
                combat = ?self.combat_phase,
                stack = self.stack.len(),
                discard = player.has_pending_discard(),
                sacrifice = player.pending_sacrifice().is_some(),
                trigger_choice = player.has_pending_trigger_choice(),
                trigger_target = player.pending_trigger_target().is_some(),
                search = player.library_search_active,
                search_results = player.library_search_results.len(),
                scry = player.scry.len(),
                showcase = self.showcase.is_some(),
                priority = ?self.priority_holder,
                "AI has no legal action; trying fallbacks"
            );
        }
        let fallbacks = [
            CardGameAction::SearchCancel,
            CardGameAction::TriggerTargetCancel,
            CardGameAction::TriggerChoice { accept: false },
            CardGameAction::PassPriority,
            CardGameAction::NextStage,
        ];
        for action in fallbacks {
            let mut probe = self.clone();
            if probe.update(actor_id, action.clone()).is_ok() {
                return Some((actor_id.to_string(), action));
            }
        }
        None
    }

    fn next_ai_gameplay_action(&self) -> Option<(String, CardGameAction)> {
        let actor_id = next_actor_id(self)?;
        if !self.is_ai_player(&actor_id) {
            return None;
        }

        let legal_actions = legal_actions_for_player(self, &actor_id);
        if legal_actions.is_empty() {
            return self.ai_stall_fallback(&actor_id);
        }

        let deck_id = self
            .players
            .get(&actor_id)
            .and_then(|player| player.deck_id.as_deref())
            .unwrap_or("generalist");
        let agent = ArchetypeAgent::new(profile_for_deck(deck_id));
        let action = agent
            .choose_action(self, &actor_id, &legal_actions)
            .or_else(|| legal_actions.first().cloned())?;
        Some((actor_id, action))
    }
}

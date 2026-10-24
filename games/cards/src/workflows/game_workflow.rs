use std::collections::HashMap;

use serde_json::json;
use workflow::{CreateWorkflowDefinition, WorkflowDisplay};
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};

use crate::CardGameActionIds;

pub fn definition(actions: &CardGameActionIds) -> CreateWorkflowDefinition {
    table_definition(
        actions,
        "cardgame_table",
        WorkflowDisplayBuilder::new("w-full h-full absolute inset-0").battlefield_canvas().build(),
    )
}

/// The same table with the battlefield served as an activity from `client_url`.
pub fn activity_definition(actions: &CardGameActionIds) -> CreateWorkflowDefinition {
    table_definition(
        actions,
        "cardgame_activity",
        WorkflowDisplayBuilder::new("w-full h-full absolute inset-0")
            .activity("game", "games.cardgame.view")
            .build(),
    )
}

fn table_definition(actions: &CardGameActionIds, workflow_id: &str, canvas: WorkflowDisplay) -> CreateWorkflowDefinition {
    let view_action = WorkflowActionBuilder::new("games.cardgame.view")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.view)
        .build();

    let join_action = WorkflowActionBuilder::new("games.cardgame.join")
        .run_server_action()
        .target(&actions.join)
        .build();

    let add_ai_action = WorkflowActionBuilder::new("games.cardgame.add_ai")
        .run_server_action()
        .target(&actions.add_ai)
        .build();

    let set_format_action = WorkflowActionBuilder::new("games.cardgame.set_format")
        .run_server_action()
        .target(&actions.set_format)
        .build();

    let start_action = WorkflowActionBuilder::new("games.cardgame.start")
        .run_server_action()
        .target(&actions.start)
        .build();

    let select_deck_action = WorkflowActionBuilder::new("games.cardgame.select_deck")
        .run_server_action()
        .target(&actions.select_deck)
        .build();

    let play_land_action = WorkflowActionBuilder::new("games.cardgame.play_land")
        .run_server_action()
        .target(&actions.play_land)
        .build();

    let deploy_node_action = WorkflowActionBuilder::new("games.cardgame.deploy_node")
        .run_server_action()
        .target(&actions.deploy_node)
        .build();

    let tap_card_action = WorkflowActionBuilder::new("games.cardgame.tap")
        .run_server_action()
        .target(&actions.tap_card)
        .build();

    let cast_action = WorkflowActionBuilder::new("games.cardgame.cast")
        .run_server_action()
        .target(&actions.cast_spell)
        .build();

    let activate_action = WorkflowActionBuilder::new("games.cardgame.activate")
        .run_server_action()
        .target(&actions.activate_ability)
        .build();

    let resolve_stack_action = WorkflowActionBuilder::new("games.cardgame.resolve_stack")
        .run_server_action()
        .target(&actions.resolve_stack)
        .build();

    let cancel_stack_action = WorkflowActionBuilder::new("games.cardgame.cancel_stack")
        .run_server_action()
        .target(&actions.cancel_stack)
        .build();

    let pass_priority_action = WorkflowActionBuilder::new("games.cardgame.pass_priority")
        .run_server_action()
        .target(&actions.pass_priority)
        .build();

    let trigger_choice_action = WorkflowActionBuilder::new("games.cardgame.trigger_choice")
        .run_server_action()
        .target(&actions.trigger_choice)
        .build();

    let trigger_target_select_action =
        WorkflowActionBuilder::new("games.cardgame.trigger_target_select")
            .run_server_action()
            .target(&actions.trigger_target_select)
            .build();

    let trigger_target_cancel_action =
        WorkflowActionBuilder::new("games.cardgame.trigger_target_cancel")
            .run_server_action()
            .target(&actions.trigger_target_cancel)
            .build();

    let discard_commit_action = WorkflowActionBuilder::new("games.cardgame.discard_commit")
        .run_server_action()
        .target(&actions.discard_commit)
        .build();

    let discard_action = WorkflowActionBuilder::new("games.cardgame.discard")
        .run_server_action()
        .target(&actions.discard)
        .build();

    let sacrifice_action = WorkflowActionBuilder::new("games.cardgame.sacrifice")
        .run_server_action()
        .target(&actions.sacrifice)
        .build();

    let declare_attack_action = WorkflowActionBuilder::new("games.cardgame.attack")
        .run_server_action()
        .target(&actions.declare_attack)
        .build();

    let declare_block_action = WorkflowActionBuilder::new("games.cardgame.block")
        .run_server_action()
        .target(&actions.declare_block)
        .build();

    let next_stage_action = WorkflowActionBuilder::new("games.cardgame.next")
        .run_server_action()
        .target(&actions.next_stage)
        .build();

    let confirm_attackers_action = WorkflowActionBuilder::new("games.cardgame.confirm_attackers")
        .run_server_action()
        .target(&actions.confirm_attackers)
        .build();

    let confirm_blockers_action = WorkflowActionBuilder::new("games.cardgame.confirm_blockers")
        .run_server_action()
        .target(&actions.confirm_blockers)
        .build();

    let scry_commit_action = WorkflowActionBuilder::new("games.cardgame.scry_commit")
        .run_server_action()
        .target(&actions.scry_commit)
        .build();

    let search_commit_action = WorkflowActionBuilder::new("games.cardgame.search_commit")
        .run_server_action()
        .target(&actions.search_commit)
        .build();

    let search_cancel_action = WorkflowActionBuilder::new("games.cardgame.search_cancel")
        .run_server_action()
        .target(&actions.search_cancel)
        .build();

    let ping_action = WorkflowActionBuilder::new("games.cardgame.ping")
        .run_server_action()
        .target(&actions.ping)
        .build();

    let reorder_action = WorkflowActionBuilder::new("games.cardgame.reorder")
        .run_server_action()
        .target(&actions.reorder_battlefield)
        .build();

    let save_card_action = WorkflowActionBuilder::new("games.cardgame.save_card")
        .run_server_action()
        .target(&actions.save_card)
        .build();

    let list_cards_action = WorkflowActionBuilder::new("games.cardgame.list_cards")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.list_cards)
        .build();

    let builder_load_deck_action = WorkflowActionBuilder::new("games.cardgame.builder_load_deck")
        .run_server_action()
        .target(&actions.builder_load_deck)
        .build();

    let builder_add_card_action = WorkflowActionBuilder::new("games.cardgame.builder_add_card")
        .run_server_action()
        .target(&actions.builder_add_card)
        .build();

    let builder_remove_card_action =
        WorkflowActionBuilder::new("games.cardgame.builder_remove_card")
            .run_server_action()
            .target(&actions.builder_remove_card)
            .build();

    let builder_set_operator_action =
        WorkflowActionBuilder::new("games.cardgame.builder_set_operator")
            .run_server_action()
            .target(&actions.builder_set_operator)
            .build();

    let builder_clone_deck_action = WorkflowActionBuilder::new("games.cardgame.builder_clone_deck")
        .run_server_action()
        .target(&actions.builder_clone_deck)
        .build();

    let builder_set_filter_action = WorkflowActionBuilder::new("games.cardgame.builder_set_filter")
        .run_server_action()
        .target(&actions.builder_set_filter)
        .build();

    let builder_set_sort_action = WorkflowActionBuilder::new("games.cardgame.builder_set_sort")
        .run_server_action()
        .target(&actions.builder_set_sort)
        .build();

    let builder_create_deck_action =
        WorkflowActionBuilder::new("games.cardgame.builder_create_deck")
            .run_server_action()
            .target(&actions.builder_create_deck)
            .build();

    let builder_autobuild_action = WorkflowActionBuilder::new("games.cardgame.builder_autobuild")
        .run_server_action()
        .target(&actions.builder_autobuild)
        .build();

    let builder_close_action = WorkflowActionBuilder::new("games.cardgame.builder_close")
        .run_server_action()
        .target(&actions.builder_close)
        .build();

    let debug_setup_action = WorkflowActionBuilder::new("games.cardgame.debug_setup")
        .run_server_action()
        .target(&actions.debug_setup)
        .build();

    let debug_ai_concede_action = WorkflowActionBuilder::new("games.cardgame.debug_ai_concede")
        .run_server_action()
        .target(&actions.debug_ai_concede)
        .build();

    let list_pack_types_action = WorkflowActionBuilder::new("games.cardgame.list_pack_types")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.list_pack_types)
        .build();

    let get_user_packs_action = WorkflowActionBuilder::new("games.cardgame.get_user_packs")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.get_user_packs)
        .build();

    let open_pack_action = WorkflowActionBuilder::new("games.cardgame.open_pack")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.open_pack)
        .build();

    let get_pack_opening_results_action =
        WorkflowActionBuilder::new("games.cardgame.get_pack_opening_results")
            .run_server_action()
            .allow_while_loading()
            .target(&actions.get_pack_opening_results)
            .build();

    let reset_action = WorkflowActionBuilder::new("games.cardgame.reset")
        .run_server_action()
        .target(&actions.reset)
        .build();

    // Draft actions
    let draft_start_action = WorkflowActionBuilder::new("games.cardgame.draft_start")
        .run_server_action()
        .target(&actions.draft_start)
        .build();

    let draft_pick_action = WorkflowActionBuilder::new("games.cardgame.draft_pick")
        .run_server_action()
        .target(&actions.draft_pick)
        .build();

    let draft_auto_pick_action = WorkflowActionBuilder::new("games.cardgame.draft_auto_pick")
        .run_server_action()
        .target(&actions.draft_auto_pick)
        .build();

    let draft_confirm_pool_action = WorkflowActionBuilder::new("games.cardgame.draft_confirm_pool")
        .run_server_action()
        .target(&actions.draft_confirm_pool)
        .build();

    let draft_keep_cards_action = WorkflowActionBuilder::new("games.cardgame.draft_keep_cards")
        .run_server_action()
        .target(&actions.draft_keep_cards)
        .build();

    let open_deck_builder_action = WorkflowActionBuilder::new("games.cardgame.open_deck_builder")
        .run_server_action()
        .build();

    let deck_builder_back_action = WorkflowActionBuilder::new("games.cardgame.deck_builder_back")
        .run_server_action()
        .build();

    let open_card_browser_action = WorkflowActionBuilder::new("games.cardgame.open_card_browser")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.browser_open)
        .build();

    let card_browser_back_action = WorkflowActionBuilder::new("games.cardgame.card_browser_back")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.browser_close)
        .build();

    let browser_set_filter_action = WorkflowActionBuilder::new("games.cardgame.browser_set_filter")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.browser_set_filter)
        .build();

    let browser_set_sort_action = WorkflowActionBuilder::new("games.cardgame.browser_set_sort")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.browser_set_sort)
        .build();

    let browser_page_action = WorkflowActionBuilder::new("games.cardgame.browser_page")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.browser_page)
        .build();

    let _player_card = WorkflowDisplayBuilder::new(
        "rounded-xl border border-white/10 bg-white/5 p-4 flex flex-col gap-2",
    )
    .div(vec![
        WorkflowDisplayBuilder::new("flex items-center justify-between")
            .div(vec![
                WorkflowDisplayBuilder::new("text-sm font-semibold text-white")
                    .text("{player.name}")
                    .build(),
                WorkflowDisplayBuilder::new("text-xs text-emerald-300")
                    .condition(
                        Cond::response_equals("player.is_active", json!(true)),
                        vec![
                            WorkflowDisplayBuilder::new("text-xs text-emerald-300")
                                .text("Active")
                                .build(),
                        ],
                    )
                    .build(),
            ])
            .build(),
        WorkflowDisplayBuilder::new("text-xs text-slate-400")
            .text("Life {player.life} • Hand {player.hand_size}")
            .build(),
        WorkflowDisplayBuilder::new("text-xs text-slate-400")
            .text("Grid {player.nodes_deployed_this_turn} • Mana {player.mana_pool}")
            .build(),
        WorkflowDisplayBuilder::new("text-xs text-slate-500")
            .text("Battlefield {player.battlefield_count}")
            .build(),
    ])
    .build();

    let _hand_card = WorkflowDisplayBuilder::new(
        "rounded-xl border border-white/10 bg-white/5 p-3 flex flex-col gap-2",
    )
    .div(vec![
        WorkflowDisplayBuilder::new("flex items-center justify-between")
            .div(vec![
                WorkflowDisplayBuilder::new("text-sm font-semibold text-white")
                    .text("{card.name}")
                    .build(),
                WorkflowDisplayBuilder::new("text-xs text-slate-400")
                    .text("Cost {card.cost}")
                    .build(),
            ])
            .build(),
        WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-500")
            .text("{card.kind_label}")
            .build(),
        WorkflowDisplayBuilder::new("text-xs text-slate-400")
            .text("{card.description}")
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::response_equals("card.kind_tag", json!("unit")),
                vec![WorkflowDisplayBuilder::new("text-xs text-slate-400")
                    .text("{card.power}/{card.toughness}")
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::not(Cond::response_equals("card.effect_text", json!(null))),
                vec![WorkflowDisplayBuilder::new("text-xs text-emerald-300")
                    .text("{card.effect_text}")
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("flex gap-2")
            .condition(
                Cond::and(vec![
                    Cond::response_equals("card.kind_tag", json!("land")),
                    Cond::response_equals("card.can_play_land", json!(true)),
                ]),
                vec![WorkflowDisplayBuilder::new("")
                    .button(
                        vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                            .text("Play Land")
                            .build()],
                        Some(HashMap::from([(
                            "card_id".into(),
                            json!("{card.id}"),
                        )])),
                        "games.cardgame.play_land",
                        "outline",
                        "sm",
                    )
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("flex flex-col gap-2")
            .condition(
                Cond::and(vec![
                    Cond::not(Cond::response_equals("card.kind_tag", json!("land"))),
                    Cond::response_equals("card.can_cast", json!(true)),
                ]),
                vec![
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::response_equals("card.target_spec", json!(null)),
                            vec![WorkflowDisplayBuilder::new("")
                                .button(
                                    vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                        .text("Cast")
                                        .build()],
                                    Some(HashMap::from([
                                        ("card_id".into(), json!("{card.id}")),
                                        ("targets".into(), json!([])),
                                    ])),
                                    "games.cardgame.cast",
                                    "solid",
                                    "sm",
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::not(Cond::response_equals("card.target_spec", json!(null))),
                                Cond::response_equals("card.target_spec.optional", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("")
                                .button(
                                    vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                        .text("Cast (No Target)")
                                        .build()],
                                    Some(HashMap::from([
                                        ("card_id".into(), json!("{card.id}")),
                                        ("targets".into(), json!([])),
                                    ])),
                                    "games.cardgame.cast",
                                    "outline",
                                    "sm",
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::not(Cond::response_equals("card.target_spec", json!(null))),
                                Cond::response_equals("card.target_spec.kind", json!("player")),
                                Cond::response_equals(
                                    "card.target_spec.scope",
                                    json!("self_player"),
                                ),
                                Cond::response_equals("card.can_cast", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.players",
                                    "player_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .condition(
                                            Cond::and(vec![
                                                Cond::response_equals(
                                                    "card.target_spec.scope",
                                                    json!("self_player"),
                                                ),
                                                Cond::response_equals(
                                                    "player_target.id",
                                                    json!("{game.viewer.id}"),
                                                ),
                                            ]),
                                            vec![WorkflowDisplayBuilder::new("")
                                                .button(
                                                    vec![WorkflowDisplayBuilder::new(
                                                        "text-xs font-semibold",
                                                    )
                                                    .text("Cast on {player_target.name}")
                                                    .build()],
                                                    Some(HashMap::from([
                                                        ("card_id".into(), json!("{card.id}")),
                                                        (
                                                            "targets".into(),
                                                            json!([{"player": "{player_target.id}"}]),
                                                        ),
                                                    ])),
                                                    "games.cardgame.cast",
                                                    "solid",
                                                    "sm",
                                                )
                                                .build()],
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::response_equals("card.target_spec.kind", json!("player")),
                                Cond::response_equals(
                                    "card.target_spec.scope",
                                    json!("opponent"),
                                ),
                                Cond::response_equals("card.can_cast", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.players",
                                    "player_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .condition(
                                            Cond::not(Cond::response_equals(
                                                "player_target.id",
                                                json!("{game.viewer.id}"),
                                            )),
                                            vec![WorkflowDisplayBuilder::new("")
                                                .button(
                                                    vec![WorkflowDisplayBuilder::new(
                                                        "text-xs font-semibold",
                                                    )
                                                    .text("Cast on {player_target.name}")
                                                    .build()],
                                                    Some(HashMap::from([
                                                        ("card_id".into(), json!("{card.id}")),
                                                        (
                                                            "targets".into(),
                                                            json!([{"player": "{player_target.id}"}]),
                                                        ),
                                                    ])),
                                                    "games.cardgame.cast",
                                                    "solid",
                                                    "sm",
                                                )
                                                .build()],
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::response_equals("card.target_spec.kind", json!("player")),
                                Cond::response_equals("card.target_spec.scope", json!("any")),
                                Cond::response_equals("card.can_cast", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.players",
                                    "player_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new(
                                                "text-xs font-semibold",
                                            )
                                            .text("Cast on {player_target.name}")
                                            .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                (
                                                    "targets".into(),
                                                    json!([{"player": "{player_target.id}"}]),
                                                ),
                                            ])),
                                            "games.cardgame.cast",
                                            "solid",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::response_equals(
                                "card.target_spec.kind",
                                json!("player_or_unit"),
                            ),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.players",
                                    "player_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new(
                                                "text-xs font-semibold",
                                            )
                                            .text("Cast on {player_target.name}")
                                            .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                (
                                                    "targets".into(),
                                                    json!([{"player": "{player_target.id}"}]),
                                                ),
                                            ])),
                                            "games.cardgame.cast",
                                            "solid",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.targetable_units",
                                    "unit_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new(
                                                "text-xs font-semibold",
                                            )
                                            .text(
                                                "Cast on {unit_target.name} ({unit_target.owner_name})",
                                            )
                                            .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                (
                                                    "targets".into(),
                                                    json!([{"card": "{unit_target.id}"}]),
                                                ),
                                            ])),
                                            "games.cardgame.cast",
                                            "solid",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::response_equals("card.target_spec.kind", json!("unit")),
                                Cond::response_equals(
                                    "card.target_spec.scope",
                                    json!("self_player"),
                                ),
                                Cond::response_equals("card.can_cast", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.targetable_units",
                                    "unit_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .condition(
                                            Cond::and(vec![
                                                Cond::response_equals(
                                                    "card.target_spec.scope",
                                                    json!("self_player"),
                                                ),
                                                Cond::response_equals(
                                                    "unit_target.owner_id",
                                                    json!("{game.viewer.id}"),
                                                ),
                                            ]),
                                            vec![WorkflowDisplayBuilder::new("")
                                                .button(
                                                    vec![WorkflowDisplayBuilder::new(
                                                        "text-xs font-semibold",
                                                    )
                                                    .text(
                                                        "Cast on {unit_target.name} ({unit_target.owner_name})",
                                                    )
                                                    .build()],
                                                    Some(HashMap::from([
                                                        ("card_id".into(), json!("{card.id}")),
                                                        (
                                                            "targets".into(),
                                                            json!([{"card": "{unit_target.id}"}]),
                                                        ),
                                                    ])),
                                                    "games.cardgame.cast",
                                                    "solid",
                                                    "sm",
                                                )
                                                .build()],
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::response_equals(
                                    "card.target_spec.kind",
                                    json!("unit"),
                                ),
                                Cond::response_equals(
                                    "card.target_spec.scope",
                                    json!("opponent"),
                                ),
                                Cond::response_equals("card.can_cast", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.targetable_units",
                                    "unit_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .condition(
                                            Cond::not(Cond::response_equals(
                                                "unit_target.owner_id",
                                                json!("{game.viewer.id}"),
                                            )),
                                            vec![WorkflowDisplayBuilder::new("")
                                                .button(
                                                    vec![WorkflowDisplayBuilder::new(
                                                        "text-xs font-semibold",
                                                    )
                                                    .text(
                                                        "Cast on {unit_target.name} ({unit_target.owner_name})",
                                                    )
                                                    .build()],
                                                    Some(HashMap::from([
                                                        ("card_id".into(), json!("{card.id}")),
                                                        (
                                                            "targets".into(),
                                                            json!([{"card": "{unit_target.id}"}]),
                                                        ),
                                                    ])),
                                                    "games.cardgame.cast",
                                                    "solid",
                                                    "sm",
                                                )
                                                .build()],
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .condition(
                            Cond::and(vec![
                                Cond::response_equals(
                                    "card.target_spec.kind",
                                    json!("unit"),
                                ),
                                Cond::response_equals("card.target_spec.scope", json!("any")),
                                Cond::response_equals("card.can_cast", json!(true)),
                            ]),
                            vec![WorkflowDisplayBuilder::new("grid gap-2")
                                .list(
                                    "game.targetable_units",
                                    "unit_target",
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new(
                                                "text-xs font-semibold",
                                            )
                                            .text(
                                                "Cast on {unit_target.name} ({unit_target.owner_name})",
                                            )
                                            .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                (
                                                    "targets".into(),
                                                    json!([{"card": "{unit_target.id}"}]),
                                                ),
                                            ])),
                                            "games.cardgame.cast",
                                            "solid",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                ],
            )
            .build(),
    ])
    .build();

    let _battlefield_card = WorkflowDisplayBuilder::new(
        "rounded-lg border border-white/10 bg-white/5 p-2 text-xs text-slate-200",
    )
    .div(vec![
        WorkflowDisplayBuilder::new("font-semibold")
            .text("{card.name}")
            .build(),
        WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-500")
            .text("{card.kind_label}")
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::not(Cond::response_equals("card.attached_to_name", json!(null))),
                vec![WorkflowDisplayBuilder::new("text-[10px] uppercase text-slate-400")
                    .text("Attached to {card.attached_to_name}")
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::not(Cond::response_equals("card.remembered_player_name", json!(null))),
                vec![WorkflowDisplayBuilder::new("text-[10px] uppercase text-cyan-300")
                    .text("Chosen player {card.remembered_player_name}")
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::response_equals("card.tapped", json!(true)),
                vec![WorkflowDisplayBuilder::new("text-[10px] uppercase text-amber-300")
                    .text("Tapped")
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::and(vec![
                    Cond::response_equals("card.kind_tag", json!("unit")),
                    Cond::response_equals("card.can_attack", json!(true)),
                ]),
                vec![
                    WorkflowDisplayBuilder::new("grid gap-2")
                        .list(
                            "game.players",
                            "player_target",
                            vec![WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::not(Cond::response_equals(
                                        "player_target.id",
                                        json!("{game.viewer.id}"),
                                    )),
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                                .text("Attack {player_target.name}")
                                                .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                (
                                                    "target".into(),
                                                    json!({"player": "{player_target.id}"}),
                                                ),
                                            ])),
                                            "games.cardgame.attack",
                                            "outline",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("grid gap-2")
                        .list(
                            "game.targetable_units",
                            "unit_target",
                            vec![WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::not(Cond::response_equals(
                                        "unit_target.owner_id",
                                        json!("{game.viewer.id}"),
                                    )),
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                                .text(
                                                    "Attack {unit_target.name} ({unit_target.owner_name})",
                                                )
                                                .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                (
                                                    "target".into(),
                                                    json!({"card": "{unit_target.id}"}),
                                                ),
                                            ])),
                                            "games.cardgame.attack",
                                            "outline",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build()],
                        )
                        .build(),
                ],
            )
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::and(vec![
                    Cond::response_equals("card.kind_tag", json!("land")),
                    Cond::response_equals("card.can_tap", json!(true)),
                ]),
                vec![WorkflowDisplayBuilder::new("")
                    .button(
                        vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                            .text("Tap for Mana")
                            .build()],
                        Some(HashMap::from([(
                            "card_id".into(),
                            json!("{card.id}"),
                        )])),
                        "games.cardgame.tap",
                        "outline",
                        "sm",
                    )
                    .build()],
            )
            .build(),
        WorkflowDisplayBuilder::new("flex flex-col gap-2")
            .condition(
                Cond::and(vec![
                    Cond::response_equals("game.stage", json!("main")),
                    Cond::response_equals("game.viewer.is_active", json!(true)),
                    Cond::not(Cond::response_equals("card.abilities", json!([]))),
                ]),
                vec![WorkflowDisplayBuilder::new("grid gap-2")
                    .list(
                        "card.abilities",
                        "ability",
                        vec![
                            WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-400")
                                .text("Ability")
                                .build(),
                            WorkflowDisplayBuilder::new("text-xs text-slate-300")
                                .text("{ability.name}")
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::and(vec![
                                        Cond::response_equals("ability.target_spec", json!(null)),
                                        Cond::response_equals("ability.can_activate", json!(true)),
                                    ]),
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                                .text("Activate")
                                                .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                ("ability_id".into(), json!("{ability.id}")),
                                                ("targets".into(), json!([])),
                                            ])),
                                            "games.cardgame.activate",
                                            "outline",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::and(vec![
                                        Cond::not(Cond::response_equals("ability.target_spec", json!(null))),
                                        Cond::response_equals("ability.target_spec.optional", json!(true)),
                                        Cond::response_equals("ability.can_activate", json!(true)),
                                    ]),
                                    vec![WorkflowDisplayBuilder::new("")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                                .text("Skip Target")
                                                .build()],
                                            Some(HashMap::from([
                                                ("card_id".into(), json!("{card.id}")),
                                                ("ability_id".into(), json!("{ability.id}")),
                                                ("targets".into(), json!([])),
                                            ])),
                                            "games.cardgame.activate",
                                            "outline",
                                            "sm",
                                        )
                                        .build()],
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::response_equals("ability.target_spec.kind", json!("player")),
                                    vec![WorkflowDisplayBuilder::new("grid gap-2")
                                        .list(
                                            "game.players",
                                            "player_target",
                                            vec![WorkflowDisplayBuilder::new("")
                                                .condition(
                                                    Cond::and(vec![
                                                        Cond::response_equals(
                                                            "ability.can_activate",
                                                            json!(true),
                                                        ),
                                                        Cond::response_equals(
                                                            "ability.target_spec.scope",
                                                            json!("self_player"),
                                                        ),
                                                        Cond::response_equals(
                                                            "player_target.id",
                                                            json!("{game.viewer.id}"),
                                                        ),
                                                    ]),
                                                    vec![WorkflowDisplayBuilder::new("")
                                                        .button(
                                                            vec![WorkflowDisplayBuilder::new(
                                                                "text-xs font-semibold",
                                                            )
                                                            .text("Activate on {player_target.name}")
                                                            .build()],
                                                            Some(HashMap::from([
                                                                ("card_id".into(), json!("{card.id}")),
                                                                ("ability_id".into(), json!("{ability.id}")),
                                                                (
                                                                    "targets".into(),
                                                                    json!([{"player": "{player_target.id}"}]),
                                                                ),
                                                            ])),
                                                            "games.cardgame.activate",
                                                            "outline",
                                                            "sm",
                                                        )
                                                        .build()],
                                                )
                                                .build(),
                                                WorkflowDisplayBuilder::new("")
                                                    .condition(
                                                        Cond::and(vec![
                                                            Cond::response_equals(
                                                                "ability.can_activate",
                                                                json!(true),
                                                            ),
                                                            Cond::response_equals(
                                                                "ability.target_spec.scope",
                                                                json!("opponent"),
                                                            ),
                                                            Cond::not(Cond::response_equals(
                                                                "player_target.id",
                                                                json!("{game.viewer.id}"),
                                                            )),
                                                        ]),
                                                        vec![WorkflowDisplayBuilder::new("")
                                                            .button(
                                                                vec![WorkflowDisplayBuilder::new(
                                                                    "text-xs font-semibold",
                                                                )
                                                                .text("Activate on {player_target.name}")
                                                                .build()],
                                                                Some(HashMap::from([
                                                                    ("card_id".into(), json!("{card.id}")),
                                                                    ("ability_id".into(), json!("{ability.id}")),
                                                                    (
                                                                        "targets".into(),
                                                                        json!([{"player": "{player_target.id}"}]),
                                                                    ),
                                                                ])),
                                                                "games.cardgame.activate",
                                                                "outline",
                                                                "sm",
                                                            )
                                                            .build()],
                                                    )
                                                    .build(),
                                                WorkflowDisplayBuilder::new("")
                                                    .condition(
                                                        Cond::and(vec![
                                                            Cond::response_equals(
                                                                "ability.target_spec.scope",
                                                                json!("any"),
                                                            ),
                                                            Cond::response_equals(
                                                                "ability.can_activate",
                                                                json!(true),
                                                            ),
                                                        ]),
                                                        vec![WorkflowDisplayBuilder::new("")
                                                            .button(
                                                                vec![WorkflowDisplayBuilder::new(
                                                                    "text-xs font-semibold",
                                                                )
                                                                .text("Activate on {player_target.name}")
                                                                .build()],
                                                                Some(HashMap::from([
                                                                    ("card_id".into(), json!("{card.id}")),
                                                                    ("ability_id".into(), json!("{ability.id}")),
                                                                    (
                                                                        "targets".into(),
                                                                        json!([{"player": "{player_target.id}"}]),
                                                                    ),
                                                                ])),
                                                                "games.cardgame.activate",
                                                                "outline",
                                                                "sm",
                                                            )
                                                            .build()],
                                                    )
                                                    .build(),
                                            ],
                                        )
                                        .build()],
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::response_equals(
                                        "ability.target_spec.kind",
                                        json!("player_or_unit"),
                                    ),
                                    vec![WorkflowDisplayBuilder::new("grid gap-2")
                                        .list(
                                            "game.players",
                                            "player_target",
                                            vec![WorkflowDisplayBuilder::new("")
                                                .condition(
                                                    Cond::response_equals(
                                                        "ability.can_activate",
                                                        json!(true),
                                                    ),
                                                    vec![WorkflowDisplayBuilder::new("")
                                                        .button(
                                                            vec![WorkflowDisplayBuilder::new(
                                                                "text-xs font-semibold",
                                                            )
                                                            .text("Activate on {player_target.name}")
                                                            .build()],
                                                            Some(HashMap::from([
                                                                ("card_id".into(), json!("{card.id}")),
                                                                ("ability_id".into(), json!("{ability.id}")),
                                                                (
                                                                    "targets".into(),
                                                                    json!([{"player": "{player_target.id}"}]),
                                                                ),
                                                            ])),
                                                            "games.cardgame.activate",
                                                            "outline",
                                                            "sm",
                                                        )
                                                        .build()],
                                                )
                                                .build()],
                                        )
                                        .build(),
                                    WorkflowDisplayBuilder::new("grid gap-2")
                                        .list(
                                            "game.targetable_units",
                                            "unit_target",
                                            vec![WorkflowDisplayBuilder::new("")
                                                .condition(
                                                    Cond::response_equals(
                                                        "ability.can_activate",
                                                        json!(true),
                                                    ),
                                                    vec![WorkflowDisplayBuilder::new("")
                                                        .button(
                                                            vec![WorkflowDisplayBuilder::new(
                                                                "text-xs font-semibold",
                                                            )
                                                            .text(
                                                                "Activate on {unit_target.name} ({unit_target.owner_name})",
                                                            )
                                                            .build()],
                                                            Some(HashMap::from([
                                                                ("card_id".into(), json!("{card.id}")),
                                                                ("ability_id".into(), json!("{ability.id}")),
                                                                (
                                                                    "targets".into(),
                                                                    json!([{"card": "{unit_target.id}"}]),
                                                                ),
                                                            ])),
                                                            "games.cardgame.activate",
                                                            "outline",
                                                            "sm",
                                                        )
                                                        .build()],
                                                )
                                                .build()],
                                        )
                                        .build()],
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .condition(
                                    Cond::response_equals(
                                        "ability.target_spec.kind",
                                        json!("unit"),
                                    ),
                                    vec![WorkflowDisplayBuilder::new("grid gap-2")
                                        .list(
                                            "game.targetable_units",
                                            "unit_target",
                                            vec![WorkflowDisplayBuilder::new("")
                                                .condition(
                                                    Cond::response_equals(
                                                        "ability.can_activate",
                                                        json!(true),
                                                    ),
                                                    vec![WorkflowDisplayBuilder::new("")
                                                        .button(
                                                            vec![WorkflowDisplayBuilder::new(
                                                                "text-xs font-semibold",
                                                            )
                                                            .text(
                                                                "Activate on {unit_target.name} ({unit_target.owner_name})",
                                                            )
                                                            .build()],
                                                            Some(HashMap::from([
                                                                ("card_id".into(), json!("{card.id}")),
                                                                ("ability_id".into(), json!("{ability.id}")),
                                                                (
                                                                    "targets".into(),
                                                                    json!([{"card": "{unit_target.id}"}]),
                                                                ),
                                                            ])),
                                                            "games.cardgame.activate",
                                                            "outline",
                                                            "sm",
                                                        )
                                                        .build()],
                                                )
                                                .build()],
                                        )
                                        .build()],
                                )
                                .build(),
                        ],
                    )
                    .build()],
            )
            .build(),
    ])
    .build();

    let node = WorkflowNodeBuilder::new("Mana Clash")
        .layout("min-h-screen w-full bg-slate-950 text-slate-100 flex flex-col items-center px-4 py-8")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("table")
                .tile(vec![
                    canvas,
                    WorkflowDisplayBuilder::new("w-full max-w-6xl h-full space-y-6 relative z-10 pointer-events-none")
                        .div(vec![
                            WorkflowDisplayBuilder::new("fixed bottom-8 left-8 w-80 pointer-events-auto")
                                .condition(
                                    Cond::and(vec![
                                        Cond::not(Cond::response_equals("game.viewer", json!(null))),
                                        Cond::not(Cond::response_equals("game.viewer.pending_trigger_choice", json!(null))),
                                    ]),
                                    vec![
                                        WorkflowDisplayBuilder::new("rounded-2xl border border-white/10 bg-black/70 p-4 space-y-3")
                                            .div(vec![
                                                WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-400")
                                                    .text("Optional Trigger")
                                                    .build(),
                                                WorkflowDisplayBuilder::new("text-sm font-semibold text-white")
                                                    .text("{game.viewer.pending_trigger_choice.source_card_name}")
                                                    .build(),
                                                WorkflowDisplayBuilder::new("text-xs text-slate-300")
                                                    .text("{game.viewer.pending_trigger_choice.effect_text}")
                                                    .build(),
                                                WorkflowDisplayBuilder::new("")
                                                    .condition(
                                                        Cond::not(Cond::response_equals("game.viewer.pending_trigger_choice.cost_text", json!(null))),
                                                        vec![
                                                            WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-500")
                                                                .text("Cost: {game.viewer.pending_trigger_choice.cost_text}")
                                                                .build(),
                                                        ],
                                                    )
                                                    .build(),
                                                WorkflowDisplayBuilder::new("flex items-center gap-2")
                                                    .div(vec![
                                                        WorkflowDisplayBuilder::new("")
                                                            .button(
                                                                vec![WorkflowDisplayBuilder::new("text-sm font-semibold")
                                                                    .text("Resolve")
                                                                    .build()],
                                                                Some(HashMap::from([("accept".into(), json!(true))])),
                                                                "games.cardgame.trigger_choice",
                                                                "solid",
                                                                "sm",
                                                            )
                                                            .build(),
                                                        WorkflowDisplayBuilder::new("")
                                                            .button(
                                                                vec![WorkflowDisplayBuilder::new("text-sm font-semibold")
                                                                    .text("Skip")
                                                                    .build()],
                                                                Some(HashMap::from([("accept".into(), json!(false))])),
                                                                "games.cardgame.trigger_choice",
                                                                "outline",
                                                                "sm",
                                                            )
                                                            .build(),
                                                    ])
                                                    .build(),
                                            ])
                                            .build(),
                                    ],
                                )
                                .build(),
                        ])
                        .build(),
                ])
                .build(),
        )
        .build();

    WorkflowDefinitionBuilder::new(workflow_id, "Mana Clash")
        .description("Prototype mana-based card battle")
        .starting_server_action(&actions.view)
        .add_actions(vec![
            join_action,
            add_ai_action,
            set_format_action,
            select_deck_action,
            start_action,
            play_land_action,
            deploy_node_action,
            tap_card_action,
            cast_action,
            activate_action,
            resolve_stack_action,
            cancel_stack_action,
            pass_priority_action,
            declare_attack_action,
            declare_block_action,
            scry_commit_action,
            search_commit_action,
            search_cancel_action,
            trigger_choice_action,
            trigger_target_select_action,
            trigger_target_cancel_action,
            discard_commit_action,
            discard_action,
            sacrifice_action,
            next_stage_action,
            confirm_attackers_action,
            confirm_blockers_action,
            ping_action,
            reorder_action,
            save_card_action,
            list_cards_action,
            builder_load_deck_action,
            builder_add_card_action,
            builder_remove_card_action,
            builder_set_operator_action,
            builder_clone_deck_action,
            builder_set_filter_action,
            builder_set_sort_action,
            builder_create_deck_action,
            builder_autobuild_action,
            builder_close_action,
            open_deck_builder_action,
            deck_builder_back_action,
            open_card_browser_action,
            card_browser_back_action,
            browser_set_filter_action,
            browser_set_sort_action,
            browser_page_action,
            debug_setup_action,
            debug_ai_concede_action,
            list_pack_types_action,
            get_user_packs_action,
            open_pack_action,
            get_pack_opening_results_action,
            reset_action,
            draft_start_action,
            draft_pick_action,
            draft_auto_pick_action,
            draft_confirm_pool_action,
            draft_keep_cards_action,
            view_action,
        ])
        .add_node("table", node)
        .build()
}

use std::collections::HashMap;

use serde_json::json;
use workflow::builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder};

pub(super) fn deck_builder_node() -> workflow::WorkflowNode {
    let _deck_card = WorkflowDisplayBuilder::new(
        "rounded-lg border border-white/10 bg-white/5 p-3 flex items-center justify-between gap-3 text-xs",
    )
    .div(vec![
        WorkflowDisplayBuilder::new("space-y-1")
            .div(vec![
                WorkflowDisplayBuilder::new("font-semibold")
                    .text("{deck_card.card.name}")
                    .build(),
                WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-500")
                    .text("{deck_card.card.kind_label}")
                    .build(),
            ])
            .build(),
        WorkflowDisplayBuilder::new("flex items-center gap-2")
            .div(vec![
                WorkflowDisplayBuilder::new("rounded-full bg-slate-800 px-2 py-1 text-[10px] font-semibold text-slate-200")
                    .text("x{deck_card.quantity}")
                    .build(),
                WorkflowDisplayBuilder::new("flex items-center gap-1")
                    .div(vec![
                        WorkflowDisplayBuilder::new("")
                            .button(
                                vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                    .text("-")
                                    .build()],
                                Some(HashMap::from([
                                    ("deck_id".into(), json!("{game.deck_builder.deck_id}")),
                                    ("card_id".into(), json!("{deck_card.card.id}")),
                                ])),
                                "games.cardgame.builder_remove_card",
                                "outline",
                                "sm",
                            )
                            .build(),
                        WorkflowDisplayBuilder::new("")
                            .button(
                                vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                    .text("+")
                                    .build()],
                                Some(HashMap::from([
                                    ("deck_id".into(), json!("{game.deck_builder.deck_id}")),
                                    ("card_id".into(), json!("{deck_card.card.id}")),
                                ])),
                                "games.cardgame.builder_add_card",
                                "outline",
                                "sm",
                            )
                            .build(),
                    ])
                    .build(),
            ])
            .build(),
    ])
    .build();

    WorkflowNodeBuilder::new("Deck Builder")
        .layout("min-h-screen w-full bg-slate-950 text-slate-100 flex flex-col items-center px-4 py-8")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("deck_builder")
                .tile(vec![
                    WorkflowDisplayBuilder::new("w-full h-full absolute inset-0")
                        .battlefield_canvas()
                        .build(),
                    WorkflowDisplayBuilder::new("w-full max-w-6xl h-full space-y-6 relative z-10 pointer-events-none")
                        .div(vec![
                            WorkflowDisplayBuilder::new(
                                "flex items-center justify-between gap-4 rounded-xl border border-white/10 bg-black/60 px-4 py-3 pointer-events-auto",
                            )
                            .div(vec![
                                WorkflowDisplayBuilder::new("space-y-1")
                                    .div(vec![
                                        WorkflowDisplayBuilder::new("text-xl font-black tracking-tight text-white")
                                            .text("Deck Builder")
                                            .build(),
                                        WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-400")
                                            .text("Mana Clash • 3D Mode")
                                            .build(),
                                    ])
                                    .build(),
                                WorkflowDisplayBuilder::new("flex items-center gap-2")
                                    .div(vec![
                                        WorkflowDisplayBuilder::new("")
                                            .button(
                                                vec![WorkflowDisplayBuilder::new("text-xs font-semibold")
                                                    .text("Back to Table")
                                                    .build()],
                                                None,
                                                "games.cardgame.deck_builder_back",
                                                "outline",
                                                "sm",
                                            )
                                            .build(),
                                    ])
                                    .build(),
                            ])
                            .build(),
                        ])
                        .build(),
                ])
                .build(),
        )
        .build()
}

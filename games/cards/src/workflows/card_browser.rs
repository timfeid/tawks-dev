use workflow::builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder};

pub(super) fn card_browser_node() -> workflow::WorkflowNode {
    WorkflowNodeBuilder::new("Card Collection")
        .layout("min-h-screen w-full bg-slate-950 text-slate-100 flex flex-col items-center px-4 py-8")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("card_browser")
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
                                            .text("{game.deck_builder.browser_title}")
                                            .build(),
                                        WorkflowDisplayBuilder::new("text-[10px] uppercase tracking-widest text-slate-400")
                                            .text("{game.deck_builder.browser_subtitle}")
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
                                                "games.cardgame.card_browser_back",
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

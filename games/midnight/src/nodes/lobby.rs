use std::collections::HashMap;
use workflow::builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder};
use workflow::{DisplayType, StickyPosition, WorkflowDisplay, WorkflowNode};

// ─── HELPER: PLAYER PORTRAIT ───────────────────────────────────────────────
fn player_display(var: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new(format!(
        "player-{{{var}.id}} flex flex-col items-center group relative p-2"
    ))
    .div(vec![
        // 1. The Frame & Avatar
        WorkflowDisplayBuilder::new(
            "relative size-20 md:size-24 rounded-full p-1 bg-gradient-to-b from-[#8B7355] to-[#4A3B2A] shadow-xl group-hover:scale-105 transition-transform duration-300",
        )
        .div(vec![
            WorkflowDisplayBuilder::new(
                "w-full h-full rounded-full border-2 border-black object-cover bg-black",
            )
            .image(format!("{{{var}.avatar_url}}"), format!("{{{var}.name}}"))
            .build(),
        ])
        .build(),
        // 2. The Name Plate
        WorkflowDisplayBuilder::new(
            "absolute -bottom-1 bg-black/80 border border-[#8B7355]/50 px-3 py-0.5 rounded-sm backdrop-blur-md shadow-lg",
        )
        .text(format!("{{{var}.name}}"))
        .build(),
        // 3. Hover Status
        WorkflowDisplayBuilder::new(
            "absolute -top-6 opacity-0 group-hover:opacity-100 transition-opacity text-[10px] tracking-widest text-[#BCAA87] uppercase font-bold",
        )
        .text("PRESENT")
        .build(),
    ])
    .build()
}

// ─── HELPER: ROLE CARD VISUAL ──────────────────────────────────────────────
fn card_visual(var: &str, interactive: bool) -> WorkflowDisplay {
    // 1. Interactive Hover State
    let overlay_hover_class = if interactive {
        "group-hover:bg-white/10"
    } else {
        ""
    };
    let hover_class = if interactive {
        "group-hover:-translate-y-2 group-hover:shadow-2xl group-hover:bg-white/10 group-hover:z-10 cursor-pointer"
    } else {
        "cursor-default"
    };

    WorkflowDisplayBuilder::new(format!(
        "relative w-full aspect-[2/3] max-w-[180px] rounded-xl overflow-hidden transition-all duration-300 border border-white/10 bg-black/60 {}",
        hover_class
    ))
    .div(vec![
        // ── IMAGE LAYER ──
        WorkflowDisplayBuilder::new("absolute inset-0 flex items-center justify-center p-4")
        .div(vec![
             // Tint applied here
             WorkflowDisplayBuilder::new(format!(
                 "max-w-[84px] max-h-[84px] w-auto h-auto object-contain midnight-tint-{{{var}.alliance}} opacity-90"
             ))
             .image(format!("{{{var}.image_url}}"), format!("{{{var}.name}}"))
             .build()
        ])
        .build(),

        // ── GRADIENTS (Visual Depth) ──
        // Top gradient for Name readability
        WorkflowDisplayBuilder::new("absolute top-0 inset-x-0 h-24 bg-gradient-to-b from-black/90 via-black/40 to-transparent pointer-events-none")
        .build(),

        // Bottom gradient for Alliance/Info readability
        WorkflowDisplayBuilder::new("absolute bottom-0 inset-x-0 h-24 bg-gradient-to-t from-black/90 via-black/40 to-transparent pointer-events-none")
        .build(),

        // ── STATE A: ENABLED (Active Glow) ──
        WorkflowDisplayBuilder::new(format!(
            "absolute inset-0 w-full h-full transition-all duration-500 border-2 midnight-border-{{{var}.alliance}} bg-white/10 shadow-[inset_0_0_30px_rgba(0,0,0,0.5)]"
        ))
        .condition(
            Cond::response_equals(format!("{var}.enabled"), serde_json::json!(true)),
            vec![
                // Active Indicator (Top right)
                WorkflowDisplayBuilder::new("absolute top-2 right-2 shadow-primary/15 bg-white/25 backdrop-blur-sm size-3 border border-white/10 rounded-full shadow-lg z-10")
                    .div(vec![
                    ])
                    .build(),
            ],
        )
        .build(),

        // ── STATE B: DISABLED (Dimmed) ──
        WorkflowDisplayBuilder::new(format!(" absolute inset-0 w-full h-full bg-black/20 transition-all duration-500 {overlay_hover_class}"))
        .condition(
            Cond::response_equals(format!("{var}.enabled"), serde_json::json!(false)),
            vec![
                WorkflowDisplayBuilder::new("absolute inset-0 bg-black/20").build(),
            ],
        )
        .build(),

        // ── CONTENT OVERLAYS ──

        // 1. Role Name (TOP)
        WorkflowDisplayBuilder::new("absolute top-12 inset-x-0 text-center z-10 px-1 pointer-events-none")
            .div(vec![
                WorkflowDisplayBuilder::new(format!(
                    "font-halloween text-xl leading-none drop-shadow-xl midnight-text-{{{var}.alliance}}",
                ))
                .text(format!("{{{var}.name}}"))
                .build(),
            ])
            .build(),


        // 3. Bottom Info (Alliance Label + Icon)
        WorkflowDisplayBuilder::new("absolute bottom-3 inset-x-0 flex flex-col items-center gap-1 z-10")
            .div(vec![
                // Alliance Name (Grayscale, small)
                WorkflowDisplayBuilder::new("text-[9px] font-mono uppercase tracking-[0.25em] text-white/40 drop-shadow-md")
                    .text(format!("{{{var}.alliance}}"))
                    .build(),

                // Info Tooltip
                WorkflowDisplayBuilder::new("pointer-events-auto opacity-40 hover:opacity-100 transition-opacity hover:scale-110 duration-200")
                    .tooltip(
                        format!("{{{var}.description}}"),
                        "cursor-help relative p-2",
                        vec![
                            WorkflowDisplayBuilder::new(
                                "size-4 flex items-center justify-center text-[10px] font-mono border border-white/20 rounded-full text-white/80 bg-black/40",
                            )
                            .text("?")
                            .build(),
                        ],
                    )
                    .build(),
            ])
            .build(),
    ])
    .build()
}

// ─── MAIN CREATE FUNCTION ──────────────────────────────────────────────────
pub fn create() -> WorkflowNode {
    let mut meta = HashMap::new();
    meta.insert("duration".into(), serde_json::json!(5000));

    WorkflowNodeBuilder::new("Lobby State")
        .condition(Cond::and(vec![
            Cond::response_exists("game"),
            Cond::response_equals("game.stage", serde_json::json!("Lobby")),
        ]))
        // ── LAYER 0: BACKGROUND ──
        .visual(
            WorkflowVisualBuilder::new("bg_layer")
                .tile(vec![
                    // Fixed background
                    WorkflowDisplay {
                        class: "fixed left-0 md:left-[var(--sidebar-width)] top-[25rem] flex right-0 md:right-[var(--userlist-width)] grayscale opacity-5 pointer-events-none".into(),
                        r#type: DisplayType::Div {
                            content: vec![
                                WorkflowDisplay {
                                    r#type: DisplayType::Image {
                                        src: "https://cdn.tawks.app/midnight/midnight.webp".into(),
                                        alt_text: "midnight".into(),
                                    },
                                    class: "midnightbg w-full max-w-3xl aspect-square mx-auto".into(),
                                }
                            ],
                        },
                    },
                    // Vignette
                    WorkflowDisplayBuilder::new(
                        "fixed inset-0 w-full h-full -z-10 bg-[radial-gradient(circle_at_center,transparent_0%,#000000_100%)] pointer-events-none",
                    )
                    .build(),
                ])
                .build(),
        )
        // ── LAYER 1: ATMOSPHERE ──
        .visual(
            WorkflowVisualBuilder::new("fog_anim")
                .animation(".wf-shell", "fog", Some("on_appear".into()), None, meta)
                .build(),
        )
        // ── LAYER 2: MAIN CONTENT ──
        .visual(
            WorkflowVisualBuilder::new("main_content")
                .tile(vec![WorkflowDisplayBuilder::new(
                    "",
                )
                .div(vec![
                    // ── SECTION: PLAYERS ──
                    WorkflowDisplayBuilder::new("w-full flex flex-col items-center mt-10 mb-8")
                        .div(vec![
                            // Header
                            WorkflowDisplayBuilder::new("text-center mb-6")
                                .div(vec![
                                    WorkflowDisplayBuilder::new(
                                        "text-4xl font-halloween text-gray-200 drop-shadow-md",
                                    )
                                    .text("The Gathering")
                                    .build(),
                                    WorkflowDisplayBuilder::new(
                                        "text-xs font-mono text-amber-500/80 tracking-[0.3em] uppercase mt-2",
                                    )
                                    .text(
                                        "{game.players_count} / {game.players_required} SOULS PRESENT",
                                    )
                                    .build(),
                                ])
                                .build(),
                            // Grid
                            WorkflowDisplayBuilder::new(
                                "flex flex-wrap justify-center gap-6 max-w-5xl mx-auto px-4",
                            )
                            .list("game.players", "player", vec![player_display("player")])
                            .build(),
                        ])
                        .build(),

                    // ── SECTION: ROLES ──
                    WorkflowDisplayBuilder::new("w-full max-w-7xl mx-auto px-4")
                        .div(vec![
                            // Header
                            WorkflowDisplayBuilder::new("flex items-center justify-center gap-4 mb-8")
                                .div(vec![
                                    WorkflowDisplayBuilder::new(
                                        "h-px w-16 bg-gradient-to-r from-transparent to-white/20",
                                    )
                                    .build(),
                                    WorkflowDisplayBuilder::new(
                                        "text-2xl font-halloween text-gray-300 drop-shadow-lg",
                                    )
                                    .text("The Grimoire")
                                    .build(),
                                    WorkflowDisplayBuilder::new(
                                        "h-px w-16 bg-gradient-to-l from-transparent to-white/20",
                                    )
                                    .build(),
                                ])
                                .build(),

                            // ── INTERACTIVE GRID ──
                            WorkflowDisplayBuilder::new(
                                "grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 justify-items-center perspective-1000",
                            )
                            .list(
                                "game.all_roles",
                                "role",
                                vec![WorkflowDisplayBuilder::new("w-full h-full")
                                    .div(vec![
                                        // ── OWNER VIEW (CLICKABLE) ──
                                        WorkflowDisplayBuilder::new("w-full h-full")
                                            .condition(
                                                Cond::response_equals(
                                                    "$user_id",
                                                    serde_json::json!("{game.owner_user_id}"),
                                                ),
                                                vec![WorkflowDisplayBuilder::new(
                                                    "group w-full h-full focus:outline-none p-0 bg-transparent border-0 hover:bg-transparent",
                                                )
                                                .button(
                                                    vec![card_visual("role", true)],
                                                    Some(HashMap::from([(
                                                        "role_id".into(),
                                                        serde_json::json!("{role.id}"),
                                                    )])),
                                                    "games.midnight.enable_role",
                                                    "ghost",
                                                    "default",
                                                )
                                                .build()],
                                            )
                                            .build(),
                                        // ── GUEST VIEW (STATIC) ──
                                        WorkflowDisplayBuilder::new("w-full h-full")
                                            .condition(
                                                Cond::not(Cond::response_equals(
                                                    "$user_id",
                                                    serde_json::json!("{game.owner_user_id}"),
                                                )),
                                                vec![WorkflowDisplayBuilder::new(
                                                    "w-full h-full cursor-default",
                                                )
                                                .div(vec![card_visual("role", false)])
                                                .build()],
                                            )
                                            .build(),
                                    ])
                                    .build()],
                            )
                            .build(),
                        ])
                        .build(),
                ])
                .build()])
                .build(),
        )
        // ── LAYER 3: STICKY ACTIONS ──
        .visual(
            WorkflowVisualBuilder::new("sticky_footer")
                .sticky(
                    StickyPosition::Bottom,
                    vec![WorkflowDisplayBuilder::new(
                        "w-full py-8 px-4 bg-gradient-to-t shadow-xl from-black to-transparent flex justify-center pointer-events-none",
                    )
                    .div(vec![WorkflowDisplayBuilder::new(
                        "pointer-events-auto relative",
                    )
                    .div(vec![
                        // 1. JOIN BUTTON
                        WorkflowDisplayBuilder::new("")
                            .condition(
                                Cond::not(Cond::some_in_list(
                                    "game.players",
                                    "player",
                                    Cond::response_equals(
                                        "$user_id",
                                        serde_json::json!("{player.id}"),
                                    ),
                                )),
                                vec![WorkflowDisplayBuilder::new("group")
                                    .button(
                                        vec![WorkflowDisplayBuilder::new(
                                            "px-12 py-4 bg-[#BCAA87] text-black font-halloween text-3xl rounded shadow-[0_0_20px_rgba(188,170,135,0.2)] hover:bg-[#d8cba8] hover:shadow-[0_0_40px_rgba(188,170,135,0.5)] hover:-translate-y-1 transition-all duration-300 clip-path-polygon",
                                        )
                                        .text("ENTER THE CIRCLE")
                                        .build()],
                                        None,
                                        "games.midnight.join",
                                        "ghost",
                                        "default",
                                    )
                                    .build()],
                            )
                            .build(),
                        // 2. START BUTTON
                        WorkflowDisplayBuilder::new("")
                            .condition(
                                Cond::and(vec![
                                    Cond::response_equals(
                                        "$user_id",
                                        serde_json::json!("{game.owner_user_id}"),
                                    ),
                                    Cond::gte(
                                        "game.players_count",
                                        serde_json::json!("{game.players_required}"),
                                    ),
                                ]),
                                vec![WorkflowDisplayBuilder::new("group")
                                    .button(
                                        vec![WorkflowDisplayBuilder::new(
                                            "px-16 py-5 bg-gradient-to-r from-emerald-900 to-emerald-700 border border-emerald-500/30 text-emerald-100 font-halloween text-4xl rounded-lg shadow-[0_0_30px_rgba(16,185,129,0.2)] hover:scale-105 hover:shadow-[0_0_60px_rgba(16,185,129,0.5)] transition-all duration-300",
                                        )
                                        .text("BEGIN RITUAL")
                                        .build()],
                                        None,
                                        "games.midnight.start",
                                        "ghost",
                                        "default",
                                    )
                                    .build()],
                            )
                            .build(),
                        // 3. WAITING TEXT
                        WorkflowDisplayBuilder::new("")
                            .condition(
                                Cond::and(vec![
                                    Cond::response_equals(
                                        "$user_id",
                                        serde_json::json!("{game.owner_user_id}"),
                                    ),
                                    Cond::not(Cond::gte(
                                        "game.players_count",
                                        serde_json::json!("{game.players_required}"),
                                    )),
                                ]),
                                vec![WorkflowDisplayBuilder::new(
                                    "font-halloween text-xl text-white/40 animate-pulse",
                                )
                                .text("Waiting for souls...")
                                .build()],
                            )
                            .build(),
                    ])
                    .build()])
                    .build()],
                )
                .build(),
        )
        .build()
}

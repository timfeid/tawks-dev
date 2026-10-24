use workflow::{
    WorkflowDisplay, WorkflowNode,
    builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder},
};

use std::collections::HashMap;

// ─── VISUAL HELPER: THE REVEAL CARD ─────────────────────────────────────────
pub fn reveal_card_display(var: &str) -> WorkflowDisplay {
    // 1. Container: Pure Black (bg-black) to avoid the "blue-gray" tint.
    //    We add a subtle white border and a deep shadow.
    WorkflowDisplayBuilder::new("card p-6 relative items-center justify-center w-80 rounded-2xl overflow-hidden border border-white/10 bg-black shadow-[0_0_60px_rgba(0,0,0,1)] flex flex-col")
        .div(vec![

            // ── IMAGE LAYER (Centered & Constrained) ──
            // We use flex center to hold the image in the middle.
            // strictly capped at 96px (max-w-[96px]) and object-contain to prevent stretching.
            WorkflowDisplayBuilder::new("")
                .div(vec![
                    WorkflowDisplayBuilder::new(format!(
                        "max-w-[96px] max-h-[96px] w-auto h-auto object-contain midnight-tint-{{{var}.alliance}} opacity-90 transition-opacity duration-1000"
                    ))
                    .image(format!("{{{var}.image_url}}"), format!("{{{var}.name}}"))
                    .build()
                ])
                .build(),

            // ── GRADIENTS (Text Readability) ──
            // These ensure the white text pops against the black background/image.

            // Top Gradient (for Name)
            WorkflowDisplayBuilder::new("absolute top-0 inset-x-0 h-32 bg-gradient-to-b from-black via-black/60 to-transparent pointer-events-none")
                .build(),

            // Bottom Gradient (for Description/Alliance)
            WorkflowDisplayBuilder::new("absolute bottom-0 inset-x-0 h-1/2 bg-gradient-to-t from-black via-black/80 to-transparent pointer-events-none")
                .build(),

            // ── CONTENT LAYER ──
            WorkflowDisplayBuilder::new("")
                .div(vec![
                    // ── TOP: Role Name ──
                    WorkflowDisplayBuilder::new("flex flex-col items-center gap-1 mt-2")
                        .div(vec![
                            WorkflowDisplayBuilder::new(format!(
                                "font-halloween text-4xl text-center tracking-wider drop-shadow-2xl midnight-text-{{{var}.alliance}}"
                            ))
                            .text(format!("{{{var}.name}}"))
                            .build(),
                        ])
                        .build(),

                    // ── BOTTOM: Lore & Alliance ──
                    WorkflowDisplayBuilder::new("flex flex-col items-center gap-4 mb-2")
                        .div(vec![
                            // Description Text (Lore)
                            // Using text-white/70 for a neutral gray that isn't blue
                            WorkflowDisplayBuilder::new("text-center")
                                .div(vec![
                                    // Decorative separator
                                    WorkflowDisplayBuilder::new("h-px w-8 bg-white/20 mx-auto mb-3").build(),

                                    // Descriptions are HTML, so they render rather than show as text.
                                    WorkflowDisplayBuilder::new("text-sm font-serif text-white/70 leading-relaxed italic drop-shadow-md")
                                        .wysiyg(format!("{var}.description"))
                                        .build(),
                                ])
                                .build(),

                            // Alliance Label (Pill style)
                            WorkflowDisplayBuilder::new("px-3 py-1 rounded-full border border-white/10 bg-white/5 backdrop-blur-md")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-[10px] font-mono uppercase tracking-[0.3em] text-white/50")
                                        .text(format!("{{{var}.alliance}}"))
                                        .build()
                                ])
                                .build(),
                        ])
                        .build(),
                ])
                .build(),


            // Bottom Left (optional balance)
            WorkflowDisplayBuilder::new(format!(
                "absolute bottom-4 left-4 size-1.5 rounded-full border border-white/10 midnight-bg-{{{var}.alliance}} z-20 opacity-40"
            )).build(),
        ])
        .build()
}

// ─── MAIN WORKFLOW NODE ─────────────────────────────────────────────────────
pub fn create() -> WorkflowNode {
    WorkflowNodeBuilder::new("Role reveal")
        .description("The moment of truth")

        // ── VISUAL: THE REVEAL SCREEN ──
        .visual(
            WorkflowVisualBuilder::new("waiting")
                .tile(vec![
                    // Main Container (Centered)
                    WorkflowDisplayBuilder::new("min-h-screen w-full flex flex-col items-center justify-center py-12 gap-8")
                        .div(vec![

                            // 1. Header Text
                            WorkflowDisplayBuilder::new("flex flex-col items-center gap-2 animate-pulse")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xl font-serif italic text-white/50")
                                        .text("The fog lifts. You are...")
                                        .build(),
                                ])
                                .build(),

                            // 2. The Card (Using the helper)
                            // This helper now uses pure black and constrained images
                            reveal_card_display("game.role"),

                            // 3. Footer / Timer
                            WorkflowDisplayBuilder::new("mt-4 flex flex-col items-center gap-2")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs font-mono text-white/30 uppercase tracking-[0.2em]")
                                        .text("NIGHT FALLS IN")
                                        .build(),

                                    WorkflowDisplayBuilder::new("font-halloween text-5xl text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.5)] transition-all duration-300 shake:text-red-500 shake:animate-pulse shake:scale-110")
                                        .countdown(
                                            "{game.role_reveal_until}",
                                            "timeleft",
                                            "{timeleft.total_seconds}",
                                            Some(10_000)
                                        )
                                        .build(),
                                ])
                                .build(),
                        ])
                        .build()
                ])
                .build()
        )

        // ── ANIMATION ──
        // Triggers the "reveal" animation (likely a flip or fade-in) on the .card class
        .visual(
            WorkflowVisualBuilder::new("animation")
                .animation(".card", "reveal", Some("on_appear".into()), None, HashMap::new())
                .build()
        )

        // ── CONDITIONS ──
        .condition(Cond::and(vec![
            Cond::response_equals("game.is_role_reveal", true.into()),
            Cond::response_equals("game.is_live", true.into()),
            Cond::response_equals("game.is_complete", false.into()),
        ]))
        .build()
}

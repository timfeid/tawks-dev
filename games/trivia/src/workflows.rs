use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tawks_bot::{
    TawksBot,
    workflow::{IconLibrary, InputType, builder::*},
};
use tawks_data::pagination::PaginationArgs;
use workflow::transform::TransformOp;
use workflow::service::InjectWorkflowPlace;
use workflow::{CreateWorkflowDefinition, StickyPosition};

use crate::multiplayer::TriviaMultiplayerActionIds;
use crate::server_actions::TriviaServerActions;
use crate::state::TriviaState;

pub fn build_trivia_creator(sa: &TriviaServerActions) -> CreateWorkflowDefinition {
    let card = "w-full relative mt-16 mb-6 p-6 max-w-xl mx-auto flex flex-col";

    let start_generate = WorkflowActionBuilder::new("start_generate")
        .run_server_action()
        .target(&sa.gen_start)
        .build();

    let poll_generate = WorkflowActionBuilder::new("poll_generate")
        .run_server_action()
        .target(&sa.gen_poll)
        .build();

    let run_generate = WorkflowActionBuilder::new(sa.wf_action_run_generate.clone())
        .run_server_action()
        .target(&sa.gen_run)
        .build();

    let complete = WorkflowActionBuilder::new("complete").cancel().build();

    let input_node = WorkflowNodeBuilder::new("Create Trivia")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::and(vec![
            Cond::not(Cond::response_equals("gen_state", json!("running"))),
            Cond::not(Cond::response_equals("gen_state", json!("ready"))),
        ]))
        .visual(
            WorkflowVisualBuilder::new("creator")
                .modal( vec![WorkflowDisplayBuilder::new(card).form(
                    "start_generate",
                    vec![
                        WorkflowDisplayBuilder::new("text-2xl font-black text-white text-center mb-2")
                            .text("Create Trivia")
                            .build(),
                        WorkflowDisplayBuilder::new("text-sm text-zinc-400 text-center mb-6")
                            .text("Type a topic like “basic math”, and we’ll generate a quiz.")
                            .build(),
                        WorkflowDisplayBuilder::new("w-full")
                            .input(
                                "topic",
                                WorkflowInputBuilder::new()
                                    .r#type(InputType::Text { placeholder: None })
                                    .required(true)
                                    .build(),
                            )
                            .build(),
                        WorkflowDisplayBuilder::new("w-full mt-4")
                            .condition(
                                Cond::response_equals("gen_state", json!("error")),
                                vec![WorkflowDisplayBuilder::new(
                                    "w-full rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-rose-200 text-sm font-bold",
                                )
                                .text("{gen_error}")
                                .build()],
                            )
                            .build(),
            WorkflowDisplayBuilder::new("mx-auto py-6").div(vec![

                        WorkflowDisplayBuilder::new("")
                            .submit(
                                vec![WorkflowDisplayBuilder::new("text-white font-bold")
                                    .text("Generate")
                                    .build()],
                                None,
                                "start_generate",
                                "default",
                                "default",
                            )
                            .build(),
            ]).build()
                    ],
                    workflow::SubmitWhen::OnSubmit,
                ).build()])
                .build(),
        )

        .build();

    let loading_node = WorkflowNodeBuilder::new("Generating")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::response_equals("gen_state", json!("running")))
        .visual(
            WorkflowVisualBuilder::new("loading")
                .modal( vec![WorkflowDisplayBuilder::new(card).div(vec![
                    WorkflowDisplayBuilder::new("text-2xl font-black text-white text-center mb-2")
                        .text("Generating…")
                        .build(),
                    WorkflowDisplayBuilder::new("text-sm text-zinc-400 text-center mb-6")
                        .text("Cooking up questions for: “{topic}”")
                        .build(),
                    WorkflowDisplayBuilder::new("w-full flex items-center justify-center py-12 text-zinc-300 font-bold animate-pulse")
                        .text("Please wait")
                        .build(),
                    WorkflowDisplayBuilder::new("hidden")
                        .input(
                            "poll_loader",
                            WorkflowInputBuilder::new()
                                .r#type(InputType::ServerActionLoader { target: "poll_generate".into() })
                                .build(),
                        )
                        .build(),
                ]).build()])
                .build(),
        )
        .build();

    let ready_node = WorkflowNodeBuilder::new("Ready")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::response_equals("gen_state", json!("ready")))
        .visual(
            WorkflowVisualBuilder::new("ready")
                .modal( vec![WorkflowDisplayBuilder::new(card).div(vec![
                    WorkflowDisplayBuilder::new("text-2xl font-black text-white text-center mb-2")
                        .text("Trivia Ready")
                        .build(),
                    WorkflowDisplayBuilder::new("text-sm text-zinc-400 text-center mb-6")
                        .text("Topic: “{topic}”")
                        .build(),
                    WorkflowDisplayBuilder::new("w-full rounded-xl border border-white/10 bg-white/[0.02] p-4 mb-6")
                        .div(vec![
                            WorkflowDisplayBuilder::new("text-xs font-bold text-zinc-500 uppercase tracking-widest mb-2")
                                .text("Preview")
                                .build(),
                            WorkflowDisplayBuilder::new("text-white font-bold")
                                .text("{preview_question_0.text}")
                                .build(),
                        ])
                        .build(),
                    WorkflowDisplayBuilder::new("w-full")
                        .button(
                            vec![WorkflowDisplayBuilder::new("text-white font-bold").text("Finish").build()],
                                Some(HashMap::from([("trivia_id".into(), "{trivia_id}".into())])),
                            "complete",
                            "solid",
                            "lg",
                        )
                        .build(),
                ]).build()])
                .build(),
        )
        .build();

    WorkflowDefinitionBuilder::new("trivia_creator", "Create Trivia")
        .add_action(start_generate)
        .add_action(poll_generate)
        .add_action(run_generate)
        .add_action(complete)
        .add_node("input", input_node)
        .add_node("loading", loading_node)
        .add_node("ready", ready_node)
        .build()
}

pub fn build_trivia_player(sa: &TriviaServerActions) -> CreateWorkflowDefinition {
    let record_db_score = WorkflowActionBuilder::new(sa.wf_action_db_added.clone())
        .run_server_action()
        .progress_node(false)
        .target(&sa.wf_action_db_added)
        .allow_while_loading()
        .build();

    let record_final_score_action =
        WorkflowActionBuilder::new(sa.wf_action_record_final_score.clone())
            .run_server_action()
            .progress_node(false)
            .target(&sa.record_final_score)
            .build();

    let advance_action = WorkflowActionBuilder::new("advance_answer")
        .run_server_action()
        .target(&sa.advance)
        .build();

    let submit_action = WorkflowActionBuilder::new(sa.wf_action_submit_answer.clone())
        .run_server_action()
        .target(&sa.reveal)
        .build();

    let finish_action = WorkflowActionBuilder::new("exit_quiz").cancel().build();

    let card_style = [
        "w-full",
        "relative",
        "lg:bg-white/5",
        "lg:backdrop-blur-md",
        "lg:border",
        "border-white/10",
        "mt-16",
        "mb-6",
        "lg:rounded-2xl",
        "p-6",
        "lg:shadow-2xl",
        "max-w-xl",
        "mx-auto",
        "min-h-[600px]",
        "flex",
        "flex-col",
    ]
    .join(" ");

    let question_card = WorkflowDisplayBuilder::new(&card_style)
        .form(
            "submit_answer",
            vec![
                WorkflowDisplayBuilder::new(
                    ["w-full", "h-1.5", "bg-white/5", "rounded-full", "mb-6"].join(" "),
                )
                .progress("progress_val", "bg-indigo-500 transition-all duration-500")
                .build(),
                WorkflowDisplayBuilder::new(
                    ["flex", "items-center", "justify-between", "mb-8"].join(" "),
                )
                .div(vec![
                    WorkflowDisplayBuilder::new(
                        [
                            "text-xs",
                            "font-bold",
                            "text-zinc-500",
                            "uppercase",
                            "tracking-widest",
                        ]
                        .join(" "),
                    )
                    .text("Question {current_question_number} / {total_questions}")
                    .build(),
                    WorkflowDisplayBuilder::new(
                        ["text-xs", "font-black", "text-indigo-300", "tabular-nums"].join(" "),
                    )
                    .condition(
                        Cond::not(Cond::response_equals("revealed", json!(true))),
                        vec![
                            WorkflowDisplayBuilder::new("")
                                .countdown(
                                    "{question_ends_at}",
                                    "timeleft",
                                    "{timeleft.total_seconds}s",
                                    Some(5_000),
                                )
                                .build(),
                        ],
                    )
                    .build(),
                    WorkflowDisplayBuilder::new("")
                        .score_ticker(
                            "score",
                            "activity",
                            "size-5 text-primary/55",
                            None::<String>,
                            None::<String>,
                            None::<String>,
                        )
                        .build(),
                ])
                .build(),
                WorkflowDisplayBuilder::new(
                    ["flex-1", "flex", "flex-col", "justify-center"].join(" "),
                )
                .div(vec![
                    WorkflowDisplayBuilder::new(
                        ["text-2xl", "font-bold", "text-white", "text-center", "mb-8"].join(" "),
                    )
                    .text("{current_question.text}")
                    .build(),
                    WorkflowDisplayBuilder::new(["grid", "grid-cols-1", "gap-3"].join(" "))
                        .input(
                            "user_answer",
                            WorkflowInputBuilder::new()
                                .r#type(InputTypeBuilder::select_from_list(
                                    "current_question.options",
                                    "opt",
                                    "{opt.value}",
                                    [
                                        "relative",
                                        "w-full",
                                        "rounded-xl",
                                        "border",
                                        "border-white/10",
                                        "bg-white/[0.02]",
                                        "p-4",
                                        "text-left",
                                        "transition-all",
                                        "hover:bg-white/[0.08]",
                                        "hover:border-white/20",
                                        "cursor-pointer",
                                        "focus:outline-none",
                                    ]
                                    .join(" "),
                                    [
                                        "border-indigo-500",
                                        "bg-indigo-500/20",
                                        "shadow-[0_0_15px_rgba(99,102,241,0.3)]",
                                        "hover:border-indigo-500",
                                        "hover:bg-indigo-500/20",
                                    ]
                                    .join(" "),
                                    vec![
                                        WorkflowDisplayBuilder::new(
                                            ["w-full", "h-full", "block"].join(" "),
                                        )
                                        .div(vec![
                                            WorkflowDisplayBuilder::new(
                                                ["flex", "items-center", "gap-3"].join(" "),
                                            )
                                            .div(vec![
                                                WorkflowDisplayBuilder::new(
                                                    [
                                                        "flex",
                                                        "items-center",
                                                        "justify-center",
                                                        "size-6",
                                                        "rounded-full",
                                                        "bg-white/5",
                                                        "text-zinc-400",
                                                    ]
                                                    .join(" "),
                                                )
                                                .icon(
                                                    IconLibrary::Lucide,
                                                    "{opt.icon}",
                                                    None,
                                                    Some("size-4".into()),
                                                )
                                                .build(),
                                                WorkflowDisplayBuilder::new(
                                                    ["font-bold", "text-zinc-200"].join(" "),
                                                )
                                                .text("{opt.label}")
                                                .build(),
                                            ])
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "absolute",
                                                    "inset-0",
                                                    "z-20",
                                                    "bg-zinc-900",
                                                    "border-2",
                                                    "border-emerald-500/50",
                                                    "rounded-xl",
                                                    "flex",
                                                    "items-center",
                                                    "gap-3",
                                                    "p-4",
                                                    "js-anim-winner",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::and(vec![
                                                    Cond::response_equals("revealed", json!(true)),
                                                    Cond::response_equals(
                                                        "opt.status",
                                                        json!("correct"),
                                                    ),
                                                ]),
                                                vec![
                                                    WorkflowDisplayBuilder::new(
                                                        [
                                                            "flex",
                                                            "items-center",
                                                            "justify-center",
                                                            "size-6",
                                                            "rounded-full",
                                                            "bg-emerald-500/20",
                                                            "text-emerald-400",
                                                        ]
                                                        .join(" "),
                                                    )
                                                    .lucide_icon("check", Some("size-4".into()))
                                                    .build(),
                                                    WorkflowDisplayBuilder::new(
                                                        ["font-bold", "text-emerald-100"].join(" "),
                                                    )
                                                    .text("{opt.label}")
                                                    .build(),
                                                    WorkflowDisplayBuilder::new(
                                                        [
                                                            "ml-auto",
                                                            "text-[10px]",
                                                            "font-black",
                                                            "uppercase",
                                                            "text-emerald-500",
                                                            "tracking-wider",
                                                        ]
                                                        .join(" "),
                                                    )
                                                    .condition(
                                                        Cond::response_equals(
                                                            "opt.selected",
                                                            json!(true),
                                                        ),
                                                        vec![
                                                            WorkflowDisplayBuilder::new("")
                                                                .text("YOU")
                                                                .build(),
                                                        ],
                                                    )
                                                    .build(),
                                                ],
                                            )
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "absolute",
                                                    "inset-0",
                                                    "z-20",
                                                    "bg-zinc-900",
                                                    "border-2",
                                                    "border-rose-500/50",
                                                    "rounded-xl",
                                                    "flex",
                                                    "items-center",
                                                    "gap-3",
                                                    "p-4",
                                                    "js-anim-loser",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::and(vec![
                                                    Cond::response_equals("revealed", json!(true)),
                                                    Cond::response_equals(
                                                        "opt.status",
                                                        json!("wrong"),
                                                    ),
                                                ]),
                                                vec![
                                                    WorkflowDisplayBuilder::new(
                                                        [
                                                            "flex",
                                                            "items-center",
                                                            "justify-center",
                                                            "size-6",
                                                            "rounded-full",
                                                            "bg-rose-500/20",
                                                            "text-rose-400",
                                                        ]
                                                        .join(" "),
                                                    )
                                                    .lucide_icon("x", Some("size-4".into()))
                                                    .build(),
                                                    WorkflowDisplayBuilder::new(
                                                        ["font-bold", "text-rose-100"].join(" "),
                                                    )
                                                    .text("{opt.label}")
                                                    .build(),
                                                    WorkflowDisplayBuilder::new(
                                                        [
                                                            "ml-auto",
                                                            "text-[10px]",
                                                            "font-black",
                                                            "uppercase",
                                                            "text-rose-500",
                                                            "tracking-wider",
                                                        ]
                                                        .join(" "),
                                                    )
                                                    .text("YOU")
                                                    .build(),
                                                ],
                                            )
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "absolute",
                                                    "inset-0",
                                                    "z-20",
                                                    "bg-zinc-900/95",
                                                    "border",
                                                    "border-white/5",
                                                    "rounded-xl",
                                                    "flex",
                                                    "items-center",
                                                    "gap-3",
                                                    "p-4",
                                                    "opacity-60",
                                                    "grayscale",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::and(vec![
                                                    Cond::response_equals("revealed", json!(true)),
                                                    Cond::response_equals(
                                                        "opt.status",
                                                        json!("neutral"),
                                                    ),
                                                ]),
                                                vec![
                                                    WorkflowDisplayBuilder::new(
                                                        [
                                                            "flex",
                                                            "items-center",
                                                            "justify-center",
                                                            "size-6",
                                                            "rounded-full",
                                                            "bg-white/5",
                                                            "text-zinc-400",
                                                        ]
                                                        .join(" "),
                                                    )
                                                    .icon(
                                                        IconLibrary::Lucide,
                                                        "{opt.icon}",
                                                        None,
                                                        Some("size-4".into()),
                                                    )
                                                    .build(),
                                                    WorkflowDisplayBuilder::new(
                                                        ["font-bold", "text-zinc-400"].join(" "),
                                                    )
                                                    .text("{opt.label}")
                                                    .build(),
                                                ],
                                            )
                                            .build(),
                                        ])
                                        .build(),
                                    ],
                                ))
                                .required(true)
                                .build(),
                        )
                        .build(),
                ])
                .build(),
                WorkflowDisplayBuilder::new("hidden")
                    .condition(
                        Cond::response_equals("revealed", json!(true)),
                        vec![
                            WorkflowDisplayBuilder::new("")
                                .input(
                                    "loader",
                                    WorkflowInputBuilder::new()
                                        .r#type(InputType::ServerActionLoader {
                                            target: "advance_answer".into(),
                                        })
                                        .build(),
                                )
                                .build(),
                        ],
                    )
                    .build(),
                WorkflowDisplayBuilder::new(
                    [
                        "w-full",
                        "mt-8",
                        "text-center",
                        "text-sm",
                        "font-bold",
                        "text-zinc-500",
                        "animate-pulse",
                        "uppercase",
                        "tracking-widest",
                    ]
                    .join(" "),
                )
                .condition(
                    Cond::response_equals("revealed", json!(true)),
                    vec![
                        WorkflowDisplayBuilder::new("text-xs text-zinc-400 text-center mb-2")
                            .condition(
                                Cond::not(Cond::response_equals(
                                    "current_question.annotation_url",
                                    json!(null),
                                )),
                                vec![
                                    WorkflowDisplayBuilder::new("")
                                        .text("{current_question.annotation_url}")
                                        .build(),
                                ],
                            )
                            .build(),
                        WorkflowDisplayBuilder::new(["w-full", "mt-4"].join(" "))
                            .condition(
                                Cond::and(vec![
                                    Cond::response_equals("revealed", json!(true)),
                                    Cond::response_equals("no_answer", json!(true)),
                                ]),
                                vec![
                                    WorkflowDisplayBuilder::new(
                                        [
                                            "w-full",
                                            "rounded-xl",
                                            "border",
                                            "border-amber-500/30",
                                            "bg-amber-500/10",
                                            "px-4",
                                            "py-3",
                                            "text-center",
                                        ]
                                        .join(" "),
                                    )
                                    .div(vec![
                                        WorkflowDisplayBuilder::new(
                                            [
                                                "text-xs",
                                                "font-black",
                                                "uppercase",
                                                "tracking-widest",
                                                "text-amber-300",
                                            ]
                                            .join(" "),
                                        )
                                        .text("Time’s up")
                                        .build(),
                                        WorkflowDisplayBuilder::new(
                                            ["text-sm", "font-bold", "text-amber-100", "mt-1"]
                                                .join(" "),
                                        )
                                        .text("No answer recorded")
                                        .build(),
                                    ])
                                    .build(),
                                ],
                            )
                            .build(),
                        WorkflowDisplayBuilder::new("")
                            .text("Next round starting...")
                            .build(),
                    ],
                )
                .build(),
            ],
            workflow::SubmitWhen::OnChange,
        )
        .build();

    let result_card = WorkflowDisplayBuilder::new(&card_style)
        .div(vec![
            WorkflowDisplayBuilder::new("text-center mb-8")
                .div(vec![
                    WorkflowDisplayBuilder::new(["text-6xl", "mb-4", "animate-bounce"].join(" "))
                        .text("🏆")
                        .build(),
                    WorkflowDisplayBuilder::new(
                        ["text-3xl", "font-black", "text-white", "mb-2"].join(" "),
                    )
                    .text("Trivia Complete")
                    .build(),
                    WorkflowDisplayBuilder::new("flex flex-col items-center justify-center")
                        .div(vec![
                            WorkflowDisplayBuilder::new(
                                [
                                    "text-xs",
                                    "font-bold",
                                    "text-primary/55",
                                    "uppercase",
                                    "tracking-widest",
                                    "mb-1",
                                ]
                                .join(" "),
                            )
                            .text("Final Score")
                            .build(),
                            WorkflowDisplayBuilder::new("score-ticker-trivia")
                                .score_ticker(
                                    "score",
                                    "activity",
                                    "size-6 text-primary/55",
                                    None::<String>,
                                    None::<String>,
                                    None::<String>,
                                )
                                .build(),
                        ])
                        .build(),
                ])
                .build(),
            WorkflowDisplayBuilder::new(
                [
                    "flex-1",
                    "w-full",
                    "text-left",
                    "mb-8",
                    "space-y-4",
                    "overflow-y-auto",
                    "pr-2",
                    "custom-scrollbar",
                ]
                .join(" "),
            )
            .list(
                "audit",
                "entry",
                vec![
                    WorkflowDisplayBuilder::new(
                        [
                            "p-4",
                            "rounded-xl",
                            "bg-zinc-900/50",
                            "border",
                            "border-white/5",
                        ]
                        .join(" "),
                    )
                    .div(vec![
                        WorkflowDisplayBuilder::new(
                            ["flex", "items-start", "justify-between", "gap-4", "mb-4"].join(" "),
                        )
                        .div(vec![
                            WorkflowDisplayBuilder::new(
                                ["text-sm", "font-bold", "text-zinc-100", "leading-snug"].join(" "),
                            )
                            .text("{entry.question}")
                            .build(),
                            WorkflowDisplayBuilder::new(
                                ["shrink-0", "flex", "flex-col", "items-end", "gap-1"].join(" "),
                            )
                            .div(vec![
                                WorkflowDisplayBuilder::new(
                                    [
                                        "text-[10px]",
                                        "font-bold",
                                        "font-mono",
                                        "text-emerald-400",
                                        "bg-emerald-500/10",
                                        "px-2",
                                        "py-1",
                                        "rounded",
                                        "border",
                                        "border-emerald-500/20",
                                    ]
                                    .join(" "),
                                )
                                .condition(
                                    Cond::response_equals("entry.is_correct", json!(true)),
                                    vec![
                                        WorkflowDisplayBuilder::new("")
                                            .text("+{entry.points_awarded}")
                                            .build(),
                                    ],
                                )
                                .build(),
                                WorkflowDisplayBuilder::new(
                                    [
                                        "text-[10px]",
                                        "font-bold",
                                        "font-mono",
                                        "text-zinc-500",
                                        "bg-white/5",
                                        "px-2",
                                        "py-1",
                                        "rounded",
                                        "border",
                                        "border-white/5",
                                    ]
                                    .join(" "),
                                )
                                .condition(
                                    Cond::response_equals("entry.is_correct", json!(false)),
                                    vec![WorkflowDisplayBuilder::new("").text("0").build()],
                                )
                                .build(),
                            ])
                            .build(),
                        ])
                        .build(),
                        WorkflowDisplayBuilder::new(["grid", "grid-cols-1", "gap-2"].join(" "))
                            .list(
                                "entry.options",
                                "opt",
                                vec![
                                    WorkflowDisplayBuilder::new(
                                        [
                                            "flex",
                                            "items-center",
                                            "gap-3",
                                            "p-2",
                                            "rounded-lg",
                                            "bg-emerald-500/10",
                                            "border",
                                            "border-emerald-500/30",
                                            "transition-all",
                                        ]
                                        .join(" "),
                                    )
                                    .condition(
                                        Cond::response_equals("opt.status", json!("correct")),
                                        vec![
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "flex",
                                                    "items-center",
                                                    "justify-center",
                                                    "size-6",
                                                    "rounded-full",
                                                    "bg-emerald-500",
                                                    "text-emerald-950",
                                                    "shrink-0",
                                                ]
                                                .join(" "),
                                            )
                                            .lucide_icon("check", Some("size-3.5".into()))
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "flex-1",
                                                    "text-xs",
                                                    "font-bold",
                                                    "text-emerald-100",
                                                ]
                                                .join(" "),
                                            )
                                            .text("{opt.label}")
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "text-[10px]",
                                                    "font-black",
                                                    "uppercase",
                                                    "tracking-widest",
                                                    "text-emerald-400",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::and(vec![
                                                    Cond::response_equals(
                                                        "opt.selected",
                                                        json!(true),
                                                    ),
                                                    Cond::not(Cond::response_equals(
                                                        "entry.no_answer",
                                                        json!(true),
                                                    )),
                                                ]),
                                                vec![
                                                    WorkflowDisplayBuilder::new("")
                                                        .text("YOU")
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "text-[10px]",
                                                    "font-black",
                                                    "uppercase",
                                                    "tracking-widest",
                                                    "text-rose-400",
                                                    "bg-rose-500/10",
                                                    "px-2",
                                                    "py-1",
                                                    "rounded",
                                                    "border",
                                                    "border-rose-500/20",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::response_equals(
                                                    "entry.no_answer",
                                                    json!(true),
                                                ),
                                                vec![
                                                    WorkflowDisplayBuilder::new("")
                                                        .text("NO ANSWER")
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                        ],
                                    )
                                    .build(),
                                    WorkflowDisplayBuilder::new(
                                        [
                                            "flex",
                                            "items-center",
                                            "gap-3",
                                            "p-2",
                                            "rounded-lg",
                                            "bg-rose-500/10",
                                            "border",
                                            "border-rose-500/30",
                                            "transition-all",
                                        ]
                                        .join(" "),
                                    )
                                    .condition(
                                        Cond::response_equals("opt.status", json!("wrong")),
                                        vec![
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "flex",
                                                    "items-center",
                                                    "justify-center",
                                                    "size-6",
                                                    "rounded-full",
                                                    "bg-rose-500",
                                                    "text-rose-950",
                                                    "shrink-0",
                                                ]
                                                .join(" "),
                                            )
                                            .lucide_icon("x", Some("size-3.5".into()))
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                ["flex-1", "text-xs", "font-bold", "text-rose-100"]
                                                    .join(" "),
                                            )
                                            .text("{opt.label}")
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "text-[10px]",
                                                    "font-black",
                                                    "uppercase",
                                                    "text-rose-400",
                                                    "tracking-wider",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::and(vec![
                                                    Cond::response_equals(
                                                        "opt.selected",
                                                        json!(true),
                                                    ),
                                                    Cond::not(Cond::response_equals(
                                                        "entry.no_answer",
                                                        json!(true),
                                                    )),
                                                ]),
                                                vec![
                                                    WorkflowDisplayBuilder::new("")
                                                        .text("YOU")
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "text-[10px]",
                                                    "font-black",
                                                    "uppercase",
                                                    "text-amber-400",
                                                    "tracking-wider",
                                                ]
                                                .join(" "),
                                            )
                                            .condition(
                                                Cond::and(vec![
                                                    Cond::response_equals(
                                                        "entry.no_answer",
                                                        json!(true),
                                                    ),
                                                    Cond::response_equals(
                                                        "opt.selected",
                                                        json!(false),
                                                    ),
                                                ]),
                                                vec![
                                                    WorkflowDisplayBuilder::new("")
                                                        .text("TIME OUT")
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                        ],
                                    )
                                    .build(),
                                    WorkflowDisplayBuilder::new(
                                        [
                                            "flex",
                                            "items-center",
                                            "gap-3",
                                            "p-2",
                                            "rounded-lg",
                                            "bg-white/[0.02]",
                                            "border",
                                            "border-white/5",
                                            "transition-all",
                                            "opacity-60",
                                        ]
                                        .join(" "),
                                    )
                                    .condition(
                                        Cond::response_equals("opt.status", json!("neutral")),
                                        vec![
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "size-6",
                                                    "rounded-full",
                                                    "border",
                                                    "border-white/10",
                                                    "shrink-0",
                                                ]
                                                .join(" "),
                                            )
                                            .build(),
                                            WorkflowDisplayBuilder::new(
                                                [
                                                    "flex-1",
                                                    "text-xs",
                                                    "font-medium",
                                                    "text-zinc-400",
                                                ]
                                                .join(" "),
                                            )
                                            .text("{opt.label}")
                                            .build(),
                                        ],
                                    )
                                    .build(),
                                ],
                            )
                            .build(),
                    ])
                    .build(),
                ],
            )
            .build(),
            WorkflowDisplayBuilder::new("w-full pt-4")
                .share_result("trivia_share", "Share your result")
                .build(),
            WorkflowDisplayBuilder::new(["w-full", "mt-auto", "pt-4"].join(" "))
                .button(
                    vec![
                        WorkflowDisplayBuilder::new("text-white font-bold")
                            .text("Return to Lobby")
                            .build(),
                    ],
                    None,
                    "exit_quiz",
                    "solid",
                    "lg",
                )
                .build(),
        ])
        .build();

    let mut meta = HashMap::new();
    meta.insert("from_id".into(), ".score-ticker-trivia".into());
    meta.insert("amount".into(), "{db_delta}".into());
    meta.insert("unit".into(), "DB".into());
    meta.insert("once_guard_path".into(), "db_anim_should_run".into());
    meta.insert("once_set_path".into(), "db_anim_should_run".into());
    meta.insert("once_set_value".into(), "false".into());
    meta.insert("key".into(), "{db_anim_key}".into());
    let db_score_anim_visual = WorkflowVisualBuilder::new("db_score_anim").animation(
        "#db-score-{$user_id}",
        "score_add",
        Some("on_appear".into()),
        Some(250),
        meta,
    );

    let winner_anim = WorkflowVisualBuilder::new("win_anim").animation(
        ".js-anim-winner",
        "winner",
        Some("on_appear".into()),
        None,
        HashMap::new(),
    );
    let loser_anim = WorkflowVisualBuilder::new("lose_anim").animation(
        ".js-anim-loser",
        "loser",
        Some("on_appear".into()),
        None,
        HashMap::new(),
    );

    let game_node = WorkflowNodeBuilder::new("Game Loop")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::not(Cond::response_equals("is_finished", json!(true))))
        .visual(
            WorkflowVisualBuilder::new("card")
                .block("", vec![question_card])
                .build(),
        )
        .visual(winner_anim.build())
        .visual(loser_anim.build())
        .build();

    let result_node = WorkflowNodeBuilder::new("Score Screen")
        .parent_id("game_loop")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::response_equals("is_finished", json!(true)))
        .visual(
            WorkflowVisualBuilder::new("res")
                .block("", vec![result_card])
                .build(),
        )
        .visual(db_score_anim_visual.build())
        .build();

    WorkflowDefinitionBuilder::new("daily_mind_bender", "DBZ Trivia")
        .starting_server_action(&sa.start)
        .context("quiz", json!(null))
        .response("current_index", json!(0))
        .response("score", json!(0))
        .response("timed_out", json!(false))
        .response("no_answer", json!(false))
        .response("total_questions", json!(0))
        .response("is_finished", json!(false))
        .response("current_question", json!(null))
        .response("progress_val", json!(0))
        .response("audit", json!([]))
        .response("trivia_share", json!(null))
        .response("results_recorded", json!(false))
        .response("revealed", json!(false))
        .add_action(record_db_score)
        .add_action(submit_action)
        .add_action(record_final_score_action)
        .add_action(advance_action)
        .add_action(finish_action)
        .add_node("game_loop", game_node)
        .add_node("results", result_node)
        .response("db_delta", json!(0))
        .response("db_anim_should_run", json!(false))
        .response("db_anim_key", json!(""))
        .build()
}

pub fn build_trivia_multiplayer(
    sa: &TriviaServerActions,
    actions: &TriviaMultiplayerActionIds,
) -> CreateWorkflowDefinition {
    // --- ACTIONS (Unchanged) ---
    let load_list = WorkflowActionBuilder::new("load_list")
        .run_server_action()
        .target(&sa.list)
        .allow_while_loading()
        .build();

    let load_trivia = WorkflowActionBuilder::new("trivia_load")
        .run_server_action()
        .target(&sa.mp_load)
        .allow_while_loading()
        .build();

    let reset_action = WorkflowActionBuilder::new("games.trivia.reset")
        .run_server_action()
        .target(&sa.mp_reset)
        .build();

    let join_action = WorkflowActionBuilder::new("games.trivia.join")
        .run_server_action()
        .target(&actions.join)
        .build();

    let start_action = WorkflowActionBuilder::new("games.trivia.start")
        .run_server_action()
        .target(&actions.start)
        .build();

    let answer_action = WorkflowActionBuilder::new("games.trivia.answer")
        .run_server_action()
        .target(&actions.answer)
        .build();

    let record_db_score = WorkflowActionBuilder::new(sa.wf_action_db_added.clone())
        .run_server_action()
        .progress_node(false)
        .target(&sa.wf_action_db_added)
        .allow_while_loading()
        .build();

    let start_generate = WorkflowActionBuilder::new("start_generate")
        .run_server_action()
        .target(&sa.gen_start)
        .build();
    let poll_generate = WorkflowActionBuilder::new("poll_generate")
        .run_server_action()
        .target(&sa.gen_poll)
        .build();
    let run_generate = WorkflowActionBuilder::new(sa.wf_action_run_generate.clone())
        .run_server_action()
        .target(&sa.mp_gen_run)
        .build();
    let generate_form = WorkflowDisplayBuilder::new(
        "max-w-6xl mx-auto mb-8 rounded-2xl border border-primary/30 bg-primary/5 p-5",
    )
    .form(
        "start_generate",
        vec![
            WorkflowDisplayBuilder::new("text-xs font-bold text-primary uppercase tracking-widest")
                .text("Make your own")
                .build(),
            WorkflowDisplayBuilder::new("text-xl font-black text-white mb-1")
                .text("Generate a quiz with AI")
                .build(),
            WorkflowDisplayBuilder::new("text-sm text-zinc-400 mb-4")
                .text("Type a topic and the lobby gets a fresh set of questions in a moment.")
                .build(),
            WorkflowDisplayBuilder::new("flex flex-col gap-3 sm:flex-row sm:items-start")
                .div(vec![
                    WorkflowDisplayBuilder::new("flex-1")
                        .input(
                            "topic",
                            WorkflowInputBuilder::new()
                                .r#type(InputType::Text {
                                    placeholder: Some("e.g. 2003 Stanley Cup Final".into()),
                                })
                                .required(true)
                                .build(),
                        )
                        .build(),
                    WorkflowDisplayBuilder::new("shrink-0")
                        .submit(
                            vec![WorkflowDisplayBuilder::new("font-bold")
                                .text("Generate")
                                .build()],
                            None,
                            "start_generate",
                            "default",
                            "default",
                        )
                        .build(),
                ])
                .build(),
            WorkflowDisplayBuilder::new("mt-3")
                .condition(
                    Cond::response_equals("gen_state", json!("error")),
                    vec![WorkflowDisplayBuilder::new(
                        "rounded-xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm font-bold text-rose-200",
                    )
                    .text("{gen_error}")
                    .build()],
                )
                .build(),
        ],
        workflow::SubmitWhen::OnSubmit,
    )
    .build();
    let generating_node = WorkflowNodeBuilder::new("Generating")
        .layout("w-full")
        .condition(Cond::and(vec![
            Cond::response_equals("gen_state", json!("running")),
            Cond::or(vec![
                Cond::response_equals("game", json!(null)),
                Cond::response_equals("game.trivia", json!(null)),
            ]),
        ]))
        .visual(
            WorkflowVisualBuilder::new("generating")
                .block(
                    "mx-auto flex w-full max-w-xl flex-col items-center gap-3 px-6 py-16 text-center",
                    vec![
                        WorkflowDisplayBuilder::new("text-2xl font-black text-white")
                            .text("Generating…")
                            .build(),
                        WorkflowDisplayBuilder::new("text-sm text-zinc-400")
                            .text("Cooking up questions for “{topic}”")
                            .build(),
                        WorkflowDisplayBuilder::new("py-8 font-bold text-zinc-300 animate-pulse")
                            .text("Please wait")
                            .build(),
                        WorkflowDisplayBuilder::new("hidden")
                            .input(
                                "poll_loader",
                                WorkflowInputBuilder::new()
                                    .r#type(InputType::ServerActionLoader {
                                        target: "poll_generate".into(),
                                    })
                                    .build(),
                            )
                            .build(),
                    ],
                )
                .build(),
        )
        .build();
    let list_start_action = WorkflowActionBuilder::new("start")
        .run_server_action()
        .target(&sa.mp_load)
        .build();

    // --- ANIMATIONS ---
    let mut winner_meta = HashMap::new();
    winner_meta.insert("repeat".into(), json!("always"));
    let winner_anim = WorkflowVisualBuilder::new("win_anim").animation(
        ".js-anim-winner",
        "winner",
        Some("on_appear".into()),
        None,
        winner_meta,
    );
    let mut loser_meta = HashMap::new();
    loser_meta.insert("repeat".into(), json!("always"));
    let loser_anim = WorkflowVisualBuilder::new("lose_anim").animation(
        ".js-anim-loser",
        "loser",
        Some("on_appear".into()),
        None,
        loser_meta,
    );

    // --- SHARED STYLING (Matches Single Player) ---
    let card_style = [
        "w-full",
        "relative",
        "lg:bg-white/5",
        "lg:backdrop-blur-md",
        "lg:border",
        "border-white/10",
        "mt-16",
        "mb-6",
        "lg:rounded-2xl",
        "p-6",
        "lg:shadow-2xl",
        "max-w-2xl", // Slightly wider for MP list
        "mx-auto",
        "min-h-[600px]",
        "flex",
        "flex-col",
    ]
    .join(" ");

    // --- VISUAL COMPONENTS ---

    let list_loader = WorkflowVisualBuilder::new("load_list")
        .block(
            "hidden",
            vec![
                WorkflowDisplayBuilder::new("")
                    .input(
                        "list_loader",
                        WorkflowInputBuilder::new()
                            .r#type(InputType::ServerActionLoader {
                                target: "load_list".into(),
                            })
                            .build(),
                    )
                    .build(),
            ],
        )
        .build();

    let auto_join_visual = WorkflowVisualBuilder::new("auto_join")
        .block(
            "hidden",
            vec![
                WorkflowDisplayBuilder::new("")
                    .condition(
                        Cond::or(vec![
                            Cond::response_equals("game", json!(null)),
                            Cond::response_equals("game.viewer_joined", json!(false)),
                        ]),
                        vec![
                            WorkflowDisplayBuilder::new("")
                                .input(
                                    "auto_join_pick",
                                    WorkflowInputBuilder::new()
                                        .r#type(InputType::ServerActionLoader {
                                            target: "games.trivia.join".into(),
                                        })
                                        .build(),
                                )
                                .build(),
                        ],
                    )
                    .build(),
            ],
        )
        .build();

    // Reusing your existing list_node() logic elsewhere, just applying loader/autojoin
    let list_node = list_node_with(vec![generate_form])
        .condition(Cond::and(vec![
            Cond::not(Cond::response_equals("gen_state", json!("running"))),
            Cond::or(vec![
                Cond::response_equals("game", json!(null)),
                Cond::response_equals("game.trivia", json!(null)),
            ]),
        ]))
        .visual(list_loader)
        .visual(auto_join_visual);

    // 1. LOBBY VISUALS
    let lobby_player_list = WorkflowDisplayBuilder::new(
        "grid gap-2 mt-6 max-h-64 overflow-y-auto pr-2 custom-scrollbar",
    )
    .list(
        "game.players",
        "player",
        vec![
            WorkflowDisplayBuilder::new(
                "flex items-center gap-3 rounded-xl bg-white/5 border border-white/5 px-3 py-2",
            )
            .div(vec![
                WorkflowDisplayBuilder::new("size-8 rounded-full overflow-hidden")
                    .image("{player.avatar_url}", "{player.name}")
                    .build(),
                WorkflowDisplayBuilder::new("flex-1 text-sm font-bold text-white")
                    .text("{player.name}")
                    .build(),
                WorkflowDisplayBuilder::new("text-xs font-bold text-emerald-300")
                    .condition(
                        Cond::response_equals("player.answered", json!(true)),
                        vec![WorkflowDisplayBuilder::new("").text("Ready").build()],
                    )
                    .build(),
                WorkflowDisplayBuilder::new("text-xs font-bold text-zinc-500")
                    .condition(
                        Cond::response_equals("player.answered", json!(false)),
                        vec![WorkflowDisplayBuilder::new("").text("...").build()],
                    )
                    .build(),
            ])
            .build(),
        ],
    )
    .build();

    let lobby_node = WorkflowNodeBuilder::new("Lobby")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::and(vec![
            Cond::or(vec![
                Cond::response_equals("game.stage", json!("lobby")),
                Cond::response_equals("game", json!(null)),
            ]),
            Cond::not(Cond::response_equals("game.trivia", json!(null))),
        ]))
        .visual(
            WorkflowVisualBuilder::new("card")
                .block(
                    "",
                    vec![WorkflowDisplayBuilder::new(&card_style)
                        .div(vec![
                            WorkflowDisplayBuilder::new("hidden")
                                .condition(
                                    Cond::and(vec![
                                        Cond::response_equals("game.viewer_joined", json!(false)),
                                        Cond::not(Cond::response_equals(
                                            "game.trivia",
                                            json!(null),
                                        )),
                                    ]),
                                    vec![WorkflowDisplayBuilder::new("")
                                        .input(
                                            "auto_join",
                                            WorkflowInputBuilder::new()
                                                .r#type(InputType::ServerActionLoader {
                                                    target: "games.trivia.join".into(),
                                                })
                                                .build(),
                                        )
                                        .build()],
                                )
                                .build(),
                            WorkflowDisplayBuilder::new(
                                "text-3xl font-black text-white text-center mb-2",
                            )
                            .text("{game.trivia.title}")
                            .build(),
                            WorkflowDisplayBuilder::new(
                                "text-sm text-zinc-400 text-center mb-6",
                            )
                            .text("{game.trivia.description}")
                            .build(),
                            // Start/Join Button Area
                            WorkflowDisplayBuilder::new(
                                "flex items-center justify-center gap-3 mb-6",
                            )
                            .condition(
                                Cond::not(Cond::response_equals(
                                    "game.viewer_joined",
                                    json!(true),
                                )),
                                vec![WorkflowDisplayBuilder::new("w-full max-w-xs")
                                    .button(
                                        vec![WorkflowDisplayBuilder::new(
                                            "text-white font-bold",
                                        )
                                        .text("Join Game")
                                        .build()],
                                        None,
                                        "games.trivia.join",
                                        "solid",
                                        "lg",
                                    )
                                    .build()],
                            )
                            .build(),
                            WorkflowDisplayBuilder::new(
                                "flex items-center justify-center gap-3 mb-6",
                            )
                            .condition(
                                Cond::and(vec![
                                    Cond::response_equals("game.viewer_joined", json!(true)),
                                    Cond::not(Cond::response_equals(
                                        "game.trivia",
                                        json!(null),
                                    )),
                                ]),
                                vec![WorkflowDisplayBuilder::new("w-full max-w-xs")
                                    .button(
                                        vec![WorkflowDisplayBuilder::new(
                                            "text-white font-bold",
                                        )
                                        .text("Start Trivia")
                                        .build()],
                                        None,
                                        "games.trivia.start",
                                        "solid",
                                        "lg",
                                    )
                                    .build()],
                            )
                            .build(),
                            // Divider
                            WorkflowDisplayBuilder::new("w-full h-px bg-white/10 mb-4").div(vec![]).build(),
                            // Player List Header
                            WorkflowDisplayBuilder::new("text-xs font-bold text-zinc-500 uppercase tracking-widest text-center")
                                .text("Players in Lobby")
                                .build(),
                            lobby_player_list,
                        ])
                        .build()],
                )
                .build(),
        )
        .build();

    // 2. QUESTION OPTIONS
    // Styled to match single player "select_from_list" look but using manual buttons
    let question_options = WorkflowDisplayBuilder::new("grid gap-3 mt-6")
        .list(
            "game.current_question.options",
            "opt",
            vec![WorkflowDisplayBuilder::new("w-full")
                .button(
                    vec![WorkflowDisplayBuilder::new("flex items-center gap-3 text-left w-full")
                        .div(vec![
                            WorkflowDisplayBuilder::new("size-8 rounded-full bg-white/10 border border-white/10 flex items-center justify-center text-xs font-bold uppercase text-zinc-300 group-hover:bg-white/20 transition-colors")
                                .text("{opt.icon}")
                                .build(),
                            WorkflowDisplayBuilder::new("flex-1 text-zinc-200 font-bold group-hover:text-white transition-colors")
                                .text("{opt.label}")
                                .build(),
                        ])
                        .build()],
                    Some({
                        let mut map = HashMap::new();
                        map.insert("answer_id".into(), "{opt.value}".into());
                        map
                    }),
                    "games.trivia.answer",
                    // Custom classes to match the single player card look
                    "relative w-full rounded-xl border border-white/10 bg-white/[0.02] p-4 hover:bg-white/[0.08] hover:border-white/20 cursor-pointer focus:outline-none transition-all group",
                    "default", // Size ignored due to custom class
                )
                .build()],
        )
        .build();

    // 2b. Player Progress List (Small version for bottom of question card)
    let player_progress_list = WorkflowDisplayBuilder::new("flex items-center justify-center gap-2 mt-8 flex-wrap")
        .list(
            "game.players",
            "player",
            vec![
                WorkflowDisplayBuilder::new("relative")
                    .div(vec![
                        WorkflowDisplayBuilder::new("size-8 rounded-full overflow-hidden ring-2 ring-zinc-900 transition-all")
                            .image("{player.avatar_url}", "{player.name}")
                            .build(),
                        // Status Indicator Dot
                        WorkflowDisplayBuilder::new("absolute -bottom-1 -right-1 size-3 rounded-full border-2 border-zinc-900 bg-emerald-500")
                            .condition(
                                Cond::response_equals("player.answered", json!(true)),
                                vec![WorkflowDisplayBuilder::new("").build()],
                            )
                            .build(),
                         WorkflowDisplayBuilder::new("absolute -bottom-1 -right-1 size-3 rounded-full border-2 border-zinc-900 bg-zinc-600")
                            .condition(
                                Cond::response_equals("player.answered", json!(false)),
                                vec![WorkflowDisplayBuilder::new("").build()],
                            )
                            .build(),
                    ])
                    .build(),
            ],
        )
        .build();

    let question_node = WorkflowNodeBuilder::new("Question")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::response_equals("game.stage", json!("question")))
        .visual(
            WorkflowVisualBuilder::new("card")
                .block(
                    "",
                    vec![WorkflowDisplayBuilder::new(&card_style)
                        .div(vec![
                            // Top Meta Row
                            WorkflowDisplayBuilder::new("flex items-center justify-between text-xs font-bold text-zinc-500 uppercase tracking-widest mb-8")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("")
                                        .text("Question {game.question_number} / {game.total_questions}")
                                        .build(),
                                    WorkflowDisplayBuilder::new("text-indigo-300 tabular-nums")
                                        .countdown(
                                            "{game.question_ends_at}",
                                            "timeleft",
                                            "{timeleft.total_seconds}s",
                                            Some(5_000),
                                        )
                                        .build(),
                                ])
                                .build(),

                            // Question Text
                            WorkflowDisplayBuilder::new("text-2xl font-black text-white text-center min-h-[8rem] flex items-center justify-center")
                                .text("{game.current_question.text}")
                                .build(),

                            // Join Button (If late join)
                            WorkflowDisplayBuilder::new("mt-6")
                                .condition(
                                    Cond::not(Cond::response_equals("game.viewer_joined", json!(true))),
                                    vec![WorkflowDisplayBuilder::new("w-full max-w-xs mx-auto")
                                        .button(
                                            vec![WorkflowDisplayBuilder::new("text-white font-bold")
                                                .text("Join to Answer")
                                                .build()],
                                            None,
                                            "games.trivia.join",
                                            "solid",
                                            "lg",
                                        )
                                        .build()],
                                )
                                .build(),

                            // Options (If joined and not answered)
                            WorkflowDisplayBuilder::new("mt-2")
                                .condition(
                                    Cond::and(vec![
                                        Cond::response_equals("game.viewer_joined", json!(true)),
                                        Cond::not(Cond::response_equals("game.viewer_answered", json!(true))),
                                    ]),
                                    vec![question_options],
                                )
                                .build(),

                            // "Locked In" State
                            WorkflowDisplayBuilder::new("mt-12 flex flex-col items-center justify-center text-center animate-pulse")
                                .condition(
                                    Cond::response_equals("game.viewer_answered", json!(true)),
                                    vec![
                                        WorkflowDisplayBuilder::new("bg-emerald-500/10 text-emerald-300 px-4 py-2 rounded-full border border-emerald-500/20 font-bold uppercase tracking-widest text-xs mb-2")
                                            .text("Answer Locked In")
                                            .build(),
                                        WorkflowDisplayBuilder::new("text-zinc-400 text-sm")
                                            .text("Waiting for other players...")
                                            .build()
                                    ],
                                )
                                .build(),

                            // Bottom Player Progress
                            WorkflowDisplayBuilder::new("mt-auto pt-8 border-t border-white/5")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-center text-[10px] font-bold uppercase tracking-widest text-zinc-600 mb-2")
                                        .text("{game.answered_count} Players Answered")
                                        .build(),
                                    player_progress_list
                                ])
                                .build(),
                        ])
                        .build()],
                )
                .build(),
        )
        .build();

    // 3. REVEAL VISUALS (With Animation Triggers)
    let reveal_list = WorkflowDisplayBuilder::new(
        "grid gap-2 mt-6 pr-2",
    )
    .list(
        "game.reveal_answers",
        "entry",
        vec![
            WorkflowDisplayBuilder::new("")
                .condition(
                    Cond::response_equals("entry.is_correct", json!(true)),
                    vec![WorkflowDisplayBuilder::new(
                        "flex items-center gap-3 rounded-xl bg-emerald-500/10 px-3 py-2 border border-emerald-500/30 js-anim-winner",
                    )
                    .div(vec![
                        WorkflowDisplayBuilder::new("size-8 rounded-full overflow-hidden")
                            .image("{entry.avatar_url}", "{entry.name}")
                            .build(),
                        WorkflowDisplayBuilder::new("flex-1")
                            .div(vec![
                                WorkflowDisplayBuilder::new("text-sm font-bold text-white")
                                    .text("{entry.name}")
                                    .build(),
                                WorkflowDisplayBuilder::new("text-xs text-emerald-200/70")
                                    .text("{entry.answer_label}")
                                    .build(),
                            ])
                            .build(),
                        WorkflowDisplayBuilder::new(
                            "text-[10px] font-black uppercase tracking-widest text-emerald-300 bg-emerald-500/20 px-2 py-1 rounded border border-emerald-500/40",
                        )
                        .text("Correct")
                        .build(),
                        WorkflowDisplayBuilder::new("text-sm font-black text-emerald-200")
                            .text("+{entry.points_awarded}")
                            .build(),
                    ])
                    .build()],
                )
                .build(),
            WorkflowDisplayBuilder::new("")
                .condition(
                    Cond::response_equals("entry.is_correct", json!(false)),
                    vec![WorkflowDisplayBuilder::new(
                        "flex items-center gap-3 rounded-xl bg-rose-500/10 px-3 py-2 border border-rose-500/30 js-anim-loser",
                    )
                    .div(vec![
                        WorkflowDisplayBuilder::new("size-8 rounded-full overflow-hidden")
                            .image("{entry.avatar_url}", "{entry.name}")
                            .build(),
                        WorkflowDisplayBuilder::new("flex-1")
                            .div(vec![
                                WorkflowDisplayBuilder::new("text-sm font-bold text-white")
                                    .text("{entry.name}")
                                    .build(),
                                WorkflowDisplayBuilder::new("text-xs text-rose-200/70")
                                    .text("{entry.answer_label}")
                                    .build(),
                            ])
                            .build(),
                        WorkflowDisplayBuilder::new(
                            "text-[10px] font-black uppercase tracking-widest text-rose-300 bg-rose-500/20 px-2 py-1 rounded border border-rose-500/40",
                        )
                        .text("Wrong")
                        .build(),
                        WorkflowDisplayBuilder::new("text-sm font-black text-rose-200")
                            .text("+{entry.points_awarded}")
                            .build(),
                    ])
                    .build()],
                )
                .build(),
        ],
    )
    .build();

    let reveal_node = WorkflowNodeBuilder::new("Reveal")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::response_equals("game.stage", json!("reveal")))
        .visual(
            WorkflowVisualBuilder::new("card")
                .block(
                    "",
                    vec![WorkflowDisplayBuilder::new(&card_style)
                        .div(vec![
                            WorkflowDisplayBuilder::new("text-xs font-bold text-zinc-500 uppercase tracking-widest text-center mb-6")
                                .text("Round Complete")
                                .build(),

                            WorkflowDisplayBuilder::new("text-xl font-black text-white text-center mb-4")
                                .text("{game.current_question.text}")
                                .build(),

                            // The Correct Answer Hero
                            WorkflowDisplayBuilder::new("flex flex-col items-center justify-center py-8 rounded-2xl bg-emerald-500/10 border border-emerald-500/20 mb-6")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs font-bold text-emerald-400 uppercase tracking-widest mb-2")
                                        .text("Correct Answer")
                                        .build(),
                                    WorkflowDisplayBuilder::new("text-3xl font-black text-white text-center px-4")
                                        .text("{game.correct_answer_label}")
                                        .build(),
                                ])
                                .build(),

                            reveal_list,
                        ])
                        .build()],
                )
                .build(),
        )
        // Attach animations to the node so they can be triggered
        .visual(winner_anim.build())
        .visual(loser_anim.build())
        .build();

    // 4. FINISHED VISUALS
    let finished_node = WorkflowNodeBuilder::new("Finished")
        .layout("min-h-screen flex flex-col justify-center items-center py-12 px-4 w-full")
        .condition(Cond::response_equals("game.stage", json!("finished")))
        .visual(
            WorkflowVisualBuilder::new("card")
                .block(
                    "",
                    vec![WorkflowDisplayBuilder::new(&card_style)
                        .div(vec![
                            WorkflowDisplayBuilder::new("text-center mb-8")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-6xl mb-4 animate-bounce")
                                        .text("🏆")
                                        .build(),
                                    WorkflowDisplayBuilder::new("text-3xl font-black text-white mb-2")
                                        .text("Final Standings")
                                        .build(),
                                    WorkflowDisplayBuilder::new("text-sm text-zinc-400")
                                        .text("{game.trivia.title}")
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("grid gap-2 overflow-y-auto custom-scrollbar flex-1")
                                .list(
                                    "game.leaderboard",
                                    "player",
                                    vec![WorkflowDisplayBuilder::new("flex items-center gap-3 rounded-xl bg-white/5 border border-white/5 px-4 py-3")
                                        .div(vec![
                                            WorkflowDisplayBuilder::new("font-mono font-bold text-zinc-500 text-lg w-6")
                                                .text("#{player.rank}")
                                                .build(),
                                            WorkflowDisplayBuilder::new("size-10 rounded-full overflow-hidden")
                                                .image("{player.avatar_url}", "{player.name}")
                                                .build(),
                                            WorkflowDisplayBuilder::new("flex-1 text-base font-bold text-white")
                                                .text("{player.name}")
                                                .build(),
                                            WorkflowDisplayBuilder::new("text-lg font-black text-emerald-400 tabular-nums")
                                                .text("{player.total_score}")
                                                .build(),
                                        ])
                                        .build()],
                                )
                                .build(),

                            WorkflowDisplayBuilder::new("mt-6")
                                .button(
                                    vec![WorkflowDisplayBuilder::new("text-white font-bold")
                                        .text("Return to Lobby")
                                        .build()],
                                    None,
                                    "games.trivia.reset",
                                    "ghost",
                                    "lg",
                                )
                                .build(),
                        ])
                        .build()],
                )
                .build(),
        )
        .build();

    WorkflowDefinitionBuilder::new("trivia_multiplayer", "Trivia Multiplayer")
        .starting_server_action(&actions.view)
        .response(
            "data",
            json!({
                "page_info": { "has_next_page": false, "has_prev_page": false, "total_count": 0 },
                "edges": [],
                "meta": {}
            }),
        )
        .response("trivia_loaded", json!(false))
        .response("trivia", json!(null))
        .response("game", json!(null))
        .add_action(load_list)
        .add_action(load_trivia)
        .add_action(reset_action)
        .add_action(join_action)
        .add_action(start_action)
        .add_action(answer_action)
        .add_action(record_db_score)
        .add_action(list_start_action)
        .add_action(start_generate)
        .add_action(poll_generate)
        .add_action(run_generate)
        .add_node("generating", generating_node)
        .add_node("pick", list_node.build())
        .add_node("lobby", lobby_node)
        .add_node("question", question_node)
        .add_node("reveal", reveal_node)
        .add_node("finished", finished_node)
        .build()
}

pub fn list_node() -> WorkflowNodeBuilder {
    list_node_with(Vec::new())
}

/// The trivia library with extra displays rendered between the header and
/// the list, e.g. the lobby's generate form.
pub fn list_node_with(top: Vec<workflow::WorkflowDisplay>) -> WorkflowNodeBuilder {
    let take = PaginationArgs::default_take();

    // --- DATA MAPPINGS ---
    let next_input_map: HashMap<String, Value> = HashMap::from([
        ("niche_id".into(), "{niche_id}".into()),
        ("after".into(), "{data.page_info.end_cursor}".into()),
        ("before".into(), json!(null)),
        ("first".into(), take.into()),
        ("last".into(), json!(null)),
    ]);

    let prev_input_map: HashMap<String, Value> = HashMap::from([
        ("niche_id".into(), "{niche_id}".into()),
        ("before".into(), "{data.page_info.start_cursor}".into()),
        ("after".into(), json!(null)),
        ("last".into(), take.into()),
        ("first".into(), json!(null)),
    ]);

    let card_click_payload: HashMap<String, Value> =
        HashMap::from([("trivia_id".into(), "{edge.node.id}".into())]);

    // --- 1. HEADER ---
    let header = WorkflowDisplayBuilder::new(
        "flex items-end justify-between max-w-6xl mx-auto mb-8 shrink-0",
    )
    .div(vec![
        WorkflowDisplayBuilder::new("flex flex-col gap-1")
            .div(vec![
                WorkflowDisplayBuilder::new(
                    "text-xs font-bold text-white/40 uppercase tracking-widest",
                )
                .text("Arcade Mode")
                .build(),
                WorkflowDisplayBuilder::new("text-3xl font-black text-white tracking-tight")
                    .text("Trivia Library")
                    .build(),
            ])
            .build(),
        WorkflowDisplayBuilder::new("")
            .badge("secondary", "{data.page_info.total_count} Quizzes")
            .build(),
    ])
    .build();

    // --- 2. THE LIST (Content Area) ---
    let content_area = WorkflowDisplayBuilder::new("flex-1 mb-8 max-w-6xl mx-auto w-full")
        .div(vec![
            WorkflowDisplayBuilder::new("flex flex-col gap-6")
                .list(
                    "data.edges",
                    "edge",
                    vec![
                        // === CARD ROW START ===
                        // FIX: Changed from 'flex' to 'grid' to strictly enforce column widths
                        WorkflowDisplayBuilder::new("group relative w-full min-h-[16rem] !px-0 !whitespace-wrap !text-wrap grid grid-cols-1 lg:grid-cols-[1fr_24rem] rounded-3xl border border-white/5 bg-white/[0.02] hover:bg-white/[0.04] hover:border-white/10 hover:-translate-y-1 hover:shadow-2xl transition-all duration-300 overflow-hidden text-left items-stretch")
                            .button(
                                vec![
                                    // A. Background Glow
                                    WorkflowDisplayBuilder::new("absolute -left-20 top-0 bottom-0 w-64 bg-primary/5 blur-[80px] group-hover:bg-primary/10 transition-all pointer-events-none").div(vec![]).build(),

                                    // === LEFT PANEL: QUIZ INFO (1fr) ===
                                    // 'min-w-0' combined with grid column forces text wrapping
                                    WorkflowDisplayBuilder::new("flex flex-col justify-between p-8 relative z-10 min-w-0")
                                        .div(vec![
                                            // Top: Badge & Title
                                            WorkflowDisplayBuilder::new("flex flex-col gap-4")
                                                .div(vec![
                                                    WorkflowDisplayBuilder::new("w-fit")
                                                        .badge("glass", "{edge.node.topic}")
                                                        .build(),
                                                    WorkflowDisplayBuilder::new("flex flex-col gap-2")
                                                        .div(vec![
                                                            WorkflowDisplayBuilder::new("text-2xl font-bold text-white leading-tight break-words group-hover:text-primary/90 transition-colors pr-4")
                                                                .text("{edge.node.title}")
                                                                .build(),
                                                            WorkflowDisplayBuilder::new("text-sm font-medium text-zinc-500 leading-relaxed break-words max-w-xl")
                                                                .text("{edge.node.description}")
                                                                .build(),
                                                        ])
                                                        .build(),
                                                ])
                                                .build(),

                                            // Bottom: CTA
                                            WorkflowDisplayBuilder::new("mt-6 flex items-center gap-2 text-xs font-bold text-white/40 uppercase tracking-widest group-hover:text-white transition-colors")
                                                .div(vec![
                                                    WorkflowDisplayBuilder::new("").text("Start Quiz").build(),
                                                    WorkflowDisplayBuilder::new("").lucide_icon("arrow-right", Some("size-3".into())).build(),
                                                ])
                                                .build(),
                                        ])
                                        .build(),

                                    // === RIGHT PANEL: LEADERBOARD (24rem fixed) ===
                                    // Border logic: Top border on mobile, Left border on desktop
                                    WorkflowDisplayBuilder::new("flex flex-col h-full border-t lg:border-t-0 lg:border-l border-white/5 bg-black/20 p-6 relative z-10")
                                        .div(vec![
                                            // Header
                                            WorkflowDisplayBuilder::new("flex items-center justify-between mb-4")
                                                .div(vec![
                                                    WorkflowDisplayBuilder::new("text-[10px] font-bold text-white/30 uppercase tracking-widest")
                                                        .text("Top Agents")
                                                        .build(),
                                                    WorkflowDisplayBuilder::new("text-white/20")
                                                        .lucide_icon("trophy", Some("size-3".into()))
                                                        .build(),
                                                ])
                                                .build(),

                                            // The List (Unconditional)
                                            WorkflowDisplayBuilder::new("flex-1 flex flex-col gap-2")
                                                .list(
                                                    "edge.node.top_players",
                                                    "p",
                                                    vec![
                                                        WorkflowDisplayBuilder::new("flex items-center gap-3 p-3 rounded-xl bg-white/5 border border-white/5")
                                                            .div(vec![
                                                                // Avatar
                                                                WorkflowDisplayBuilder::new("size-8 rounded-full ring-1 ring-white/10 overflow-hidden shrink-0 bg-zinc-900")
                                                                    .user_avatar("p.user")
                                                                    .build(),

                                                                // Name
                                                                WorkflowDisplayBuilder::new("flex flex-col min-w-0")
                                                                    .div(vec![
                                                                        WorkflowDisplayBuilder::new("text-xs font-bold text-white/90 truncate")
                                                                            .text("{p.user.username}")
                                                                            .build(),
                                                                    ])
                                                                    .build(),

                                                                // Score
                                                                WorkflowDisplayBuilder::new("ml-auto text-sm font-mono font-bold text-primary tabular-nums")
                                                                    .text("{p.score}")
                                                                    .build(),
                                                            ])
                                                            .build()
                                                    ]
                                                )
                                                .build(),
                                        ])
                                        .build(),
                                ],
                                Some(card_click_payload),
                                "start",
                                "ghost",
                                "lg"
                            )
                            .build()
                        // === CARD ROW END ===
                    ]
                )
                .build(),

            // Main List Empty State
            WorkflowDisplayBuilder::new("h-full flex flex-col items-center justify-center text-white/30 border border-dashed border-white/10 rounded-2xl bg-white/[0.01] py-24")
                .condition(
                    Cond::not(Cond::response_list_not_empty("data.edges")),
                    vec![
                        WorkflowDisplayBuilder::new("mb-4").lucide_icon("ghost", Some("size-8 opacity-50".into())).build(),
                        WorkflowDisplayBuilder::new("text-sm font-medium").text("No quizzes found.").build(),
                    ],
                )
                .build(),
        ])
        .build();

    // --- 3. FOOTER (Same) ---
    let footer = WorkflowDisplayBuilder::new("flex items-center justify-center shrink-0 w-full py-4")
        .div(vec![
            WorkflowDisplayBuilder::new("flex items-center mx-auto gap-1 p-1 rounded-full bg-zinc-900/90 border border-white/10 backdrop-blur-md shadow-2xl")
                .div(vec![
                    // Prev
                    WorkflowDisplayBuilder::new("").condition(
                        Cond::response_equals("data.page_info.has_prev_page", json!(true)),
                        vec![WorkflowDisplayBuilder::new("h-9 w-12 rounded-full hover:bg-white/10 transition-colors flex items-center justify-center").button(
                            vec![WorkflowDisplayBuilder::new("text-zinc-400 hover:text-white transition-colors").lucide_icon("chevron-left", Some("size-5".into())).build()],
                            Some(prev_input_map), "load_list", "ghost", "icon_sm"
                        ).build()]
                    ).build(),
                    // Disabled Prev
                    WorkflowDisplayBuilder::new("").condition(
                        Cond::response_equals("data.page_info.has_prev_page", json!(false)),
                        vec![WorkflowDisplayBuilder::new("h-9 w-12 rounded-full cursor-not-allowed flex items-center justify-center").button(
                            vec![WorkflowDisplayBuilder::new("text-zinc-700").lucide_icon("chevron-left", Some("size-5".into())).build()],
                            None, "noop", "ghost", "icon_sm"
                        ).build()]
                    ).build(),

                    WorkflowDisplayBuilder::new("w-px h-4 bg-white/10 mx-1").div(vec![]).build(),

                    // Next
                    WorkflowDisplayBuilder::new("").condition(
                        Cond::response_equals("data.page_info.has_next_page", json!(true)),
                        vec![WorkflowDisplayBuilder::new("h-9 w-12 rounded-full hover:bg-white/10 transition-colors flex items-center justify-center").button(
                            vec![WorkflowDisplayBuilder::new("text-zinc-400 hover:text-white transition-colors").lucide_icon("chevron-right", Some("size-5".into())).build()],
                            Some(next_input_map), "load_list", "ghost", "icon_sm"
                        ).build()]
                    ).build(),
                    // Disabled Next
                    WorkflowDisplayBuilder::new("").condition(
                        Cond::response_equals("data.page_info.has_next_page", json!(false)),
                        vec![WorkflowDisplayBuilder::new("h-9 w-12 rounded-full cursor-not-allowed flex items-center justify-center").button(
                            vec![WorkflowDisplayBuilder::new("text-zinc-700").lucide_icon("chevron-right", Some("size-5".into())).build()],
                            None, "noop", "ghost", "icon_sm"
                        ).build()]
                    ).build(),
                ])
                .build()
        ])
        .build();

    // --- ROOT ---
    let mut root_children = vec![
        WorkflowDisplayBuilder::new("absolute top-0 left-0 w-full h-full bg-gradient-to-br from-primary/[0.01] to-transparent pointer-events-none").div(vec![]).build(),
        header,
    ];
    root_children.extend(top);
    root_children.push(content_area);
    let root = WorkflowDisplayBuilder::new("pt-12 px-6 pb-24 max-w-[1920px] mx-auto w-full")
        .div(root_children)
        .build();

    WorkflowNodeBuilder::new("Trivia List")
        .layout("w-full h-full relative overflow-y-auto")
        .visual(
            WorkflowVisualBuilder::new("buttons")
                .sticky(
                    workflow::StickyPosition::Toolbar,
                    vec![
                        WorkflowDisplayBuilder::new("ml-auto")
                            .button(
                                vec![
                                    WorkflowDisplayBuilder::new("")
                                        .lucide_icon("plus", None)
                                        .build(),
                                ],
                                None,
                                "create_trivia",
                                "ghost",
                                "icon",
                            )
                            .build(),
                    ],
                )
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("list")
                .block("", vec![root])
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("footer")
                .sticky(workflow::StickyPosition::Bottom, vec![footer])
                .build(),
        )
}

pub fn build_trivia_list(
    sa: &TriviaServerActions,
    trivia_creator_workflow_id: String,
    trivia_player_workflow_id: String,
) -> CreateWorkflowDefinition {
    // --- ACTIONS ---
    let load_action = WorkflowActionBuilder::new("load_list")
        .run_server_action()
        .target(&sa.list)
        .allow_while_loading()
        .build();

    let refresh = vec![
        TransformOp::Set {
            path: "after".into(),
            value: json!(null),
        },
        TransformOp::Set {
            path: "before".into(),
            value: json!(null),
        },
        TransformOp::Set {
            path: "first".into(),
            value: PaginationArgs::default_take().into(),
        },
        TransformOp::Set {
            path: "last".into(),
            value: json!(null),
        },
        TransformOp::EmitRunServerAction {
            action_id: sa.list.clone(),
            input_map: None,
            reconcile_node: true,
        },
    ];
    // let refresh_action = WorkflowActionBuilder::new("refresh_list")
    //     .transform_responses(refresh.clone(), false)
    //     .allow_while_loading()
    //     .build();

    let create_action = WorkflowActionBuilder::new("create_trivia")
        .start_injected_workflow(
            None,
            HashMap::new(),
            HashMap::new(),
            false,
            InjectWorkflowPlace::Modal {
                title: "Create trivia".into(),
            },
            refresh.clone(),
        )
        .target(&trivia_creator_workflow_id)
        .build();

    let start_action = WorkflowActionBuilder::new("start")
        .start_injected_workflow(
            Some(HashMap::from([("trivia_id".into(), "trivia_id".into())])),
            HashMap::new(),
            HashMap::new(),
            false,
            InjectWorkflowPlace::This,
            refresh.clone(),
        )
        .target(&trivia_player_workflow_id)
        .build();

    // Removed: exit_action
    let noop_action = WorkflowActionBuilder::new("noop").noop().build();

    // --- VISUALS ---

    let node = list_node().build();

    WorkflowDefinitionBuilder::new("trivia_list", "Trivia List")
        .description("Browse generated trivia quizzes")
        .starting_server_action(&sa.list)
        .context(
            "data",
            json!({
                "page_info": { "has_next_page": false, "has_prev_page": false, "total_count": 0 },
                "edges": [],
                "meta": {}
            }),
        )
        // Removed exit_action from the list
        .add_actions(vec![load_action, noop_action, start_action, create_action])
        .add_node("main", node)
        .build()
}

#[derive(Clone, Debug)]
pub struct TriviaWorkflows {
    pub creator: String,
    pub player: String,
    pub lister: String,
}

pub async fn register_workflows(state: Arc<TriviaState>) -> Result<TriviaWorkflows> {
    let sa = state
        .server_actions
        .get()
        .ok_or_else(|| anyhow!("server actions must be registered before workflows"))?
        .clone();

    let client = state.tawks_client.as_ref().expect("Not using sdk");

    let trivia_player = client
        .register_workflow(build_trivia_player(&sa))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    let trivia_creator = client
        .register_workflow(build_trivia_creator(&sa))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    let trivia_list = client
        .register_workflow(build_trivia_list(
            &sa,
            trivia_creator.clone(),
            trivia_player.clone(),
        ))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(TriviaWorkflows {
        creator: trivia_creator,
        player: trivia_player,
        lister: trivia_list,
    })
}

pub async fn register_multiplayer_workflow(
    bot: &TawksBot,
    sa: &TriviaServerActions,
    actions: &TriviaMultiplayerActionIds,
) -> Result<String> {
    bot.register_workflow(build_trivia_multiplayer(sa, actions))
        .await
        .map_err(|e| anyhow!(e.to_string()))
}

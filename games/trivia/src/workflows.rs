use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tawks_bot::{
    TawksBot,
    workflow::{InputType, builder::*},
};
use tawks_data::pagination::PaginationArgs;
use workflow::service::InjectWorkflowPlace;
use workflow::transform::TransformOp;
use workflow::ui::*;
use workflow::{
    CreateWorkflowDefinition, DisplayVisual, NodeCondition, WorkflowDisplay, WorkflowVisual,
};

use crate::multiplayer::TriviaMultiplayerActionIds;
use crate::server_actions::TriviaServerActions;
use crate::state::TriviaState;

/// A game screen: one narrow column, centred up and down.
fn game_page(id: &str, content: Vec<WorkflowDisplay>) -> WorkflowVisual {
    WorkflowVisualBuilder::new(id)
        .display(DisplayVisual::Page {
            width: PageWidth::Narrow,
            content,
            center: true,
            backdrop: Backdrop::Default,
            typeface: Typeface::Default,
        })
        .build()
}

fn loader(id: &str, target: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new()
        .input(
            id,
            WorkflowInputBuilder::new()
                .r#type(InputType::ServerActionLoader {
                    target: target.into(),
                })
                .build(),
        )
        .build()
}

fn is(field: &str, value: Value) -> NodeCondition {
    Cond::response_equals(field, value)
}

fn score_ticker() -> WorkflowDisplay {
    workflow::ui::score_ticker(
        "score",
        icon("activity", IconSize::Medium).tone(Tone::Primary),
    )
}

fn countdown(until: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new()
        .countdown(until, "timeleft", "{timeleft.total_seconds}s", Some(5_000))
        .build()
}

fn player_avatar(src: &str, name: &str) -> WorkflowDisplay {
    avatar(src, name, Size::Small)
}

/// One answer once the round is revealed, shown while `opt.status` is `status`.
fn revealed_option(
    status: &str,
    state: State,
    lead: WorkflowDisplay,
    trail: Vec<WorkflowDisplay>,
    mark: Option<&str>,
) -> WorkflowDisplay {
    let row = list_row("{opt.label}")
        .state(state)
        .lead_with(vec![lead])
        .trail(trail);
    let row = match mark {
        Some(name) => row.mark(name),
        None => row,
    };
    when(is("opt.status", json!(status)), vec![row])
}

const WINNER: &str = "winner";
const LOSER: &str = "loser";
const FINAL_SCORE: &str = "trivia-score";

fn marked(name: &str) -> String {
    format!("[data-mark~=\"{name}\"]")
}

/// Plays `animation` on every display marked `mark` as it appears.
fn mark_animation(
    id: &str,
    animation: &str,
    mark: &str,
    meta: HashMap<String, Value>,
) -> WorkflowVisual {
    WorkflowVisualBuilder::new(id)
        .animation(
            marked(mark),
            animation,
            Some("on_appear".into()),
            None,
            meta,
        )
        .build()
}

fn creator_heading(title_text: &str, lead_text: &str) -> Vec<WorkflowDisplay> {
    vec![title(title_text).centered(), body(lead_text).centered()]
}

pub fn build_trivia_creator(sa: &TriviaServerActions) -> CreateWorkflowDefinition {
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

    let mut input_content = creator_heading(
        "Create Trivia",
        "Type a topic like “basic math”, and we’ll generate a quiz.",
    );
    input_content.extend([
        WorkflowDisplayBuilder::new()
            .input(
                "topic",
                WorkflowInputBuilder::new()
                    .r#type(InputType::Text { placeholder: None })
                    .required(true)
                    .build(),
            )
            .build(),
        when(
            is("gen_state", json!("error")),
            vec![notice(Tone::Danger, "{gen_error}")],
        ),
        primary_action("Generate", act("start_generate"))
            .submits()
            .full_width(),
    ]);
    let input_node = WorkflowNodeBuilder::new("Create Trivia")
        .condition(Cond::and(vec![
            Cond::not(is("gen_state", json!("running"))),
            Cond::not(is("gen_state", json!("ready"))),
        ]))
        .visual(
            WorkflowVisualBuilder::new("creator")
                .modal(vec![
                    WorkflowDisplayBuilder::new()
                        .form(
                            "start_generate",
                            vec![column(Gap::Normal, input_content)],
                            workflow::SubmitWhen::OnSubmit,
                        )
                        .build(),
                ])
                .build(),
        )
        .build();

    let mut loading_content = creator_heading("Generating…", "Cooking up questions for: “{topic}”");
    loading_content.extend([
        loading("Please wait"),
        loader("poll_loader", "poll_generate"),
    ]);
    let loading_node = WorkflowNodeBuilder::new("Generating")
        .condition(is("gen_state", json!("running")))
        .visual(
            WorkflowVisualBuilder::new("loading")
                .modal(vec![column(Gap::Normal, loading_content)])
                .build(),
        )
        .build();

    let mut ready_content = creator_heading("Trivia Ready", "Topic: “{topic}”");
    ready_content.extend([
        panel(vec![
            eyebrow("Preview"),
            subheading("{preview_question_0.text}"),
        ]),
        primary_action("Finish", act("complete").input("trivia_id", "{trivia_id}")).full_width(),
    ]);
    let ready_node = WorkflowNodeBuilder::new("Ready")
        .condition(is("gen_state", json!("ready")))
        .visual(
            WorkflowVisualBuilder::new("ready")
                .modal(vec![column(Gap::Normal, ready_content)])
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

    let revealed = || is("revealed", json!(true));
    let you = |tone: Tone| badge("YOU", tone);

    let answer_choices = items(
        "current_question.options",
        "opt",
        Layout::Stack { gap: Gap::Normal },
        vec![
            choice(
                vec![
                    icon("{opt.icon}", IconSize::Small).tone(Tone::Muted),
                    body("{opt.label}").tone(Tone::Strong),
                ],
                act("submit_answer").input("user_answer", "{opt.value}"),
            )
            .across(),
        ],
    );

    let revealed_answers = items(
        "current_question.options",
        "opt",
        Layout::Stack { gap: Gap::Normal },
        vec![
            revealed_option(
                "correct",
                State::Success,
                icon("check", IconSize::Small).tone(Tone::Success),
                vec![when(
                    is("opt.selected", json!(true)),
                    vec![you(Tone::Success)],
                )],
                Some(WINNER),
            ),
            revealed_option(
                "wrong",
                State::Danger,
                icon("x", IconSize::Small).tone(Tone::Danger),
                vec![you(Tone::Danger)],
                Some(LOSER),
            ),
            revealed_option(
                "neutral",
                State::Muted,
                icon("{opt.icon}", IconSize::Small).tone(Tone::Muted),
                vec![],
                None,
            ),
        ],
    );

    let question_card = vec![
        progress("progress_val", Tone::Primary),
        row(
            Gap::Normal,
            vec![
                eyebrow("Question {current_question_number} / {total_questions}").tone(Tone::Muted),
                row(
                    Gap::Normal,
                    vec![
                        when(Cond::not(revealed()), vec![countdown("{question_ends_at}")]),
                        score_ticker(),
                    ],
                ),
            ],
        )
        .between(),
        title("{current_question.text}").centered(),
        when(Cond::not(revealed()), vec![answer_choices]),
        when(
            revealed(),
            vec![
                revealed_answers,
                loader("loader", "advance_answer"),
                when(
                    Cond::response_exists("current_question.annotation_url"),
                    vec![fine("{current_question.annotation_url}").centered()],
                ),
                when(
                    Cond::and(vec![revealed(), is("no_answer", json!(true))]),
                    vec![notice(Tone::Warning, "No answer recorded").titled("Time’s up")],
                ),
                loading("Next round starting..."),
            ],
        ),
    ];

    let answered_you = Cond::and(vec![
        is("opt.selected", json!(true)),
        Cond::not(is("entry.no_answer", json!(true))),
    ]);
    let audit_entry = panel(vec![
        row(
            Gap::Normal,
            vec![
                subheading("{entry.question}"),
                when(
                    is("entry.is_correct", json!(true)),
                    vec![badge("+{entry.points_awarded}", Tone::Success)],
                ),
                when(
                    is("entry.is_correct", json!(false)),
                    vec![badge("0", Tone::Muted)],
                ),
            ],
        )
        .between()
        .align(Align::Start)
        .nowrap(),
        items(
            "entry.options",
            "opt",
            Layout::Stack { gap: Gap::Tight },
            vec![
                revealed_option(
                    "correct",
                    State::Success,
                    icon("check", IconSize::Small).tone(Tone::Success),
                    vec![
                        when(answered_you.clone(), vec![you(Tone::Success)]),
                        when(
                            is("entry.no_answer", json!(true)),
                            vec![badge("NO ANSWER", Tone::Danger)],
                        ),
                    ],
                    None,
                ),
                revealed_option(
                    "wrong",
                    State::Danger,
                    icon("x", IconSize::Small).tone(Tone::Danger),
                    vec![
                        when(answered_you, vec![you(Tone::Danger)]),
                        when(
                            Cond::and(vec![
                                is("entry.no_answer", json!(true)),
                                is("opt.selected", json!(false)),
                            ]),
                            vec![badge("TIME OUT", Tone::Warning)],
                        ),
                    ],
                    None,
                ),
                revealed_option(
                    "neutral",
                    State::Muted,
                    icon("circle", IconSize::Small).tone(Tone::Faint),
                    vec![],
                    None,
                ),
            ],
        ),
    ]);

    let result_card = vec![
        hero("Trivia Complete")
            .aside(vec![icon("trophy", IconSize::Large).tone(Tone::Gold)])
            .build(),
        panel(vec![
            row(
                Gap::Normal,
                vec![eyebrow("Final Score"), score_ticker().mark(FINAL_SCORE)],
            )
            .between(),
        ])
        .tone(Tone::Primary),
        items(
            "audit",
            "entry",
            Layout::Stack { gap: Gap::Normal },
            vec![audit_entry],
        ),
        WorkflowDisplayBuilder::new()
            .finished_result("trivia_share", "Share your result")
            .build(),
        primary_action("Return to Lobby", act("exit_quiz")).full_width(),
    ];

    let game_node = WorkflowNodeBuilder::new("Game Loop")
        .condition(Cond::not(is("is_finished", json!(true))))
        .visual(game_page("card", question_card))
        .visual(mark_animation("win_anim", "winner", WINNER, HashMap::new()))
        .visual(mark_animation("lose_anim", "loser", LOSER, HashMap::new()))
        .build();

    let result_node = WorkflowNodeBuilder::new("Score Screen")
        .parent_id("game_loop")
        .condition(is("is_finished", json!(true)))
        .visual(game_page("res", result_card))
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
        .add_action(submit_action)
        .add_action(record_final_score_action)
        .add_action(advance_action)
        .add_action(finish_action)
        .add_node("game_loop", game_node)
        .add_node("results", result_node)
        .build()
}

pub fn build_trivia_multiplayer(
    sa: &TriviaServerActions,
    actions: &TriviaMultiplayerActionIds,
) -> CreateWorkflowDefinition {
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

    let generate_form = panel(vec![
        eyebrow("Make your own"),
        heading("Generate a quiz with AI"),
        body("Type a topic and the lobby gets a fresh set of questions in a moment."),
        WorkflowDisplayBuilder::new()
            .form(
                "start_generate",
                vec![column(
                    Gap::Normal,
                    vec![
                        WorkflowDisplayBuilder::new()
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
                        row(
                            Gap::Normal,
                            vec![primary_action("Generate", act("start_generate")).submits()],
                        ),
                    ],
                )],
                workflow::SubmitWhen::OnSubmit,
            )
            .build(),
        when(
            is("gen_state", json!("error")),
            vec![notice(Tone::Danger, "{gen_error}")],
        ),
    ])
    .tone(Tone::Primary);

    let no_trivia_yet = || {
        Cond::or(vec![
            is("game", json!(null)),
            is("game.trivia", json!(null)),
        ])
    };

    let generating_node = WorkflowNodeBuilder::new("Generating")
        .condition(Cond::and(vec![
            is("gen_state", json!("running")),
            no_trivia_yet(),
        ]))
        .visual(game_page(
            "generating",
            vec![
                title("Generating…").centered(),
                body("Cooking up questions for “{topic}”").centered(),
                loading("Please wait"),
                loader("poll_loader", "poll_generate"),
            ],
        ))
        .build();
    let list_start_action = WorkflowActionBuilder::new("start")
        .run_server_action()
        .target(&sa.mp_load)
        .build();

    let list_loader = WorkflowVisualBuilder::new("load_list")
        .page(PageWidth::Wide, vec![loader("list_loader", "load_list")])
        .build();

    let auto_join_visual = WorkflowVisualBuilder::new("auto_join")
        .page(
            PageWidth::Wide,
            vec![when(
                Cond::or(vec![
                    is("game", json!(null)),
                    is("game.viewer_joined", json!(false)),
                ]),
                vec![loader("auto_join_pick", "games.trivia.join")],
            )],
        )
        .build();

    let list_node = list_node_with(vec![generate_form])
        .condition(Cond::and(vec![
            Cond::not(is("gen_state", json!("running"))),
            no_trivia_yet(),
        ]))
        .visual(list_loader)
        .visual(auto_join_visual);

    let has_trivia = || Cond::not(is("game.trivia", json!(null)));

    let lobby_players = items(
        "game.players",
        "player",
        Layout::Stack { gap: Gap::Tight },
        vec![
            list_row("{player.name}")
                .lead_with(vec![player_avatar("{player.avatar_url}", "{player.name}")])
                .trail(vec![
                    when(
                        is("player.answered", json!(true)),
                        vec![badge("Ready", Tone::Success)],
                    ),
                    when(is("player.answered", json!(false)), vec![fine("...")]),
                ]),
        ],
    );

    let lobby_node = WorkflowNodeBuilder::new("Lobby")
        .condition(Cond::and(vec![
            Cond::or(vec![
                is("game.stage", json!("lobby")),
                is("game", json!(null)),
            ]),
            has_trivia(),
        ]))
        .visual(game_page(
            "card",
            vec![
                hero("{game.trivia.title}")
                    .lead("{game.trivia.description}")
                    .actions(vec![
                        when(
                            Cond::not(is("game.viewer_joined", json!(true))),
                            vec![primary_action("Join Game", act("games.trivia.join"))],
                        ),
                        when(
                            Cond::and(vec![is("game.viewer_joined", json!(true)), has_trivia()]),
                            vec![primary_action("Start Trivia", act("games.trivia.start"))],
                        ),
                    ])
                    .build(),
                section("Players in Lobby")
                    .content(vec![lobby_players])
                    .build(),
                when(
                    Cond::and(vec![is("game.viewer_joined", json!(false)), has_trivia()]),
                    vec![loader("auto_join", "games.trivia.join")],
                ),
            ],
        ))
        .build();

    let question_options = items(
        "game.current_question.options",
        "opt",
        Layout::Stack { gap: Gap::Normal },
        vec![
            choice(
                vec![
                    icon("{opt.icon}", IconSize::Small).tone(Tone::Muted),
                    body("{opt.label}").tone(Tone::Strong),
                ],
                act("games.trivia.answer").input("answer_id", "{opt.value}"),
            )
            .across(),
        ],
    );

    let player_progress = items(
        "game.players",
        "player",
        Layout::Row { gap: Gap::Tight },
        vec![
            when(
                is("player.answered", json!(true)),
                vec![chip("{player.name}").with_icon("check")],
            ),
            when(
                is("player.answered", json!(false)),
                vec![chip("{player.name}").with_icon("clock")],
            ),
        ],
    );

    let question_node = WorkflowNodeBuilder::new("Question")
        .condition(is("game.stage", json!("question")))
        .visual(game_page(
            "card",
            vec![
                row(
                    Gap::Normal,
                    vec![
                        eyebrow("Question {game.question_number} / {game.total_questions}")
                            .tone(Tone::Muted),
                        countdown("{game.question_ends_at}"),
                    ],
                )
                .between(),
                title("{game.current_question.text}").centered(),
                when(
                    Cond::not(is("game.viewer_joined", json!(true))),
                    vec![
                        row(
                            Gap::Normal,
                            vec![primary_action("Join to Answer", act("games.trivia.join"))],
                        )
                        .justify(Justify::Center),
                    ],
                ),
                when(
                    Cond::and(vec![
                        is("game.viewer_joined", json!(true)),
                        Cond::not(is("game.viewer_answered", json!(true))),
                    ]),
                    vec![question_options],
                ),
                when(
                    is("game.viewer_answered", json!(true)),
                    vec![
                        notice(Tone::Success, "Waiting for other players...")
                            .titled("Answer Locked In")
                            .with_icon("check"),
                    ],
                ),
                divider(),
                fine("{game.answered_count} Players Answered").centered(),
                row(Gap::Tight, vec![player_progress]).justify(Justify::Center),
            ],
        ))
        .build();

    let reveal_entry = |correct: bool, state: State, tone: Tone, verdict: &str, mark: &str| {
        when(
            is("entry.is_correct", json!(correct)),
            vec![
                list_row("{entry.name}")
                    .meta("{entry.answer_label}")
                    .state(state)
                    .lead_with(vec![player_avatar("{entry.avatar_url}", "{entry.name}")])
                    .trail(vec![
                        badge(verdict, tone),
                        stat_text("+{entry.points_awarded}").tone(tone).numeric(),
                    ])
                    .mark(mark),
            ],
        )
    };

    let replay = || HashMap::from([("repeat".to_string(), json!("always"))]);

    let reveal_node = WorkflowNodeBuilder::new("Reveal")
        .condition(is("game.stage", json!("reveal")))
        .visual(game_page(
            "card",
            vec![
                eyebrow("Round Complete").tone(Tone::Muted).centered(),
                heading("{game.current_question.text}").centered(),
                panel(vec![
                    eyebrow("Correct Answer").tone(Tone::Success).centered(),
                    title("{game.correct_answer_label}").centered(),
                ])
                .tone(Tone::Success),
                items(
                    "game.reveal_answers",
                    "entry",
                    Layout::Stack { gap: Gap::Tight },
                    vec![
                        reveal_entry(true, State::Success, Tone::Success, "Correct", WINNER),
                        reveal_entry(false, State::Danger, Tone::Danger, "Wrong", LOSER),
                    ],
                ),
            ],
        ))
        .visual(mark_animation("win_anim", "winner", WINNER, replay()))
        .visual(mark_animation("lose_anim", "loser", LOSER, replay()))
        .build();

    let finished_node = WorkflowNodeBuilder::new("Finished")
        .condition(is("game.stage", json!("finished")))
        .visual(game_page(
            "card",
            vec![
                hero("Final Standings")
                    .lead("{game.trivia.title}")
                    .aside(vec![icon("trophy", IconSize::Large).tone(Tone::Gold)])
                    .build(),
                items(
                    "game.leaderboard",
                    "player",
                    Layout::Stack { gap: Gap::Tight },
                    vec![
                        list_row("{player.name}")
                            .lead_with(vec![
                                stat_text("#{player.rank}").tone(Tone::Muted).numeric(),
                                avatar("{player.avatar_url}", "{player.name}", Size::Medium),
                            ])
                            .trail(vec![
                                stat_text("{player.total_score}")
                                    .tone(Tone::Success)
                                    .numeric(),
                            ]),
                    ],
                ),
                secondary_action("Return to Lobby", act("games.trivia.reset")).full_width(),
            ],
        ))
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

fn quiz_card() -> WorkflowDisplay {
    let about = column(
        Gap::Normal,
        vec![
            row(Gap::Tight, vec![badge("{edge.node.topic}", Tone::Primary)]),
            heading("{edge.node.title}"),
            body("{edge.node.description}"),
            row(
                Gap::Tight,
                vec![
                    eyebrow("Start Quiz").tone(Tone::Muted),
                    icon("arrow-right", IconSize::Small).tone(Tone::Muted),
                ],
            ),
        ],
    );

    let top_players = column(
        Gap::Tight,
        vec![
            row(
                Gap::Tight,
                vec![
                    eyebrow("Top Agents").tone(Tone::Faint),
                    icon("trophy", IconSize::Small).tone(Tone::Faint),
                ],
            )
            .between(),
            items(
                "edge.node.top_players",
                "p",
                Layout::Stack { gap: Gap::Tight },
                vec![
                    list_row("{p.user.username}")
                        .lead_with(vec![player_avatar(
                            "{p.user.avatar_url}",
                            "{p.user.username}",
                        )])
                        .trail(vec![stat_text("{p.score}").tone(Tone::Primary).numeric()]),
                ],
            ),
        ],
    );

    choice(
        vec![grid(2, vec![about, top_players])],
        act("start").input("trivia_id", "{edge.node.id}"),
    )
    .across()
}

/// The trivia library with extra displays rendered between the header and
/// the list, e.g. the lobby's generate form.
pub fn list_node_with(top: Vec<WorkflowDisplay>) -> WorkflowNodeBuilder {
    let take = PaginationArgs::default_take();

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

    let mut content = vec![
        hero("Trivia Library")
            .eyebrow("Arcade Mode")
            .aside(vec![badge(
                "{data.page_info.total_count} Quizzes",
                Tone::Muted,
            )])
            .build(),
    ];
    content.extend(top);
    content.push(items(
        "data.edges",
        "edge",
        Layout::Stack { gap: Gap::Loose },
        vec![quiz_card()],
    ));
    content.push(when(
        Cond::not(Cond::response_list_not_empty("data.edges")),
        vec![empty("No quizzes found.").with_icon("ghost")],
    ));

    let pager = stepper(
        "",
        Some(
            act("load_list")
                .inputs(prev_input_map)
                .disabled_when(is("data.page_info.has_prev_page", json!(false))),
        ),
        Some(
            act("load_list")
                .inputs(next_input_map)
                .disabled_when(is("data.page_info.has_next_page", json!(false))),
        ),
    );

    WorkflowNodeBuilder::new("Trivia List")
        .visual(
            WorkflowVisualBuilder::new("buttons")
                .sticky(
                    workflow::StickyPosition::Toolbar,
                    vec![icon_action("plus", "Create trivia", act("create_trivia"))],
                )
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("list")
                .page(PageWidth::Wide, content)
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("footer")
                .sticky(workflow::StickyPosition::Bottom, vec![pager])
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

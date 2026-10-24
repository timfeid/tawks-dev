use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tracing::{error, instrument, trace, warn};

use tawks_bot::{ServerActionHandler, TawksBot};
use tawks_bot::{full_avatar_url, types::UserResource};
use tawks_data::pagination::connection_from_repository;
use workflow::{
    NodeCondition,
    schedule::{ScheduledAction, ScheduledJob},
    server_action::ServerActionResult,
    service::{ProcessWorkflowActionArgs, ScheduleJobsArgs, WorkflowResourceWithContext},
};

use crate::{
    TriviaResource,
    multiplayer::TriviaAction,
    repository::{
        ListTriviaArgsWithCommunityId, StartAttemptArgs, StartQuestionArgs, SubmitAnswerArgs,
        TriviaWithQuestions,
    },
    scoring,
    state::TriviaState,
};

#[derive(Clone, Debug)]
pub struct TriviaServerActions {
    pub reveal: String,
    pub gen_start: String,
    pub gen_poll: String,
    pub gen_run: String,
    pub advance: String,
    pub start: String,
    pub list: String,
    pub mp_load: String,
    pub mp_reset: String,
    pub mp_gen_run: String,

    // workflow-action ids (internal to the workflow definition; used by scheduler jobs)
    pub wf_action_run_generate: String,
    pub wf_action_submit_answer: String,
    pub wf_action_record_final_score: String,
    pub record_final_score: String,
    pub wf_action_db_added: String,
}

impl TriviaServerActions {
    #[instrument]
    pub fn new_defaults() -> Self {
        Self {
            reveal: String::new(),
            gen_start: String::new(),
            gen_poll: String::new(),
            gen_run: String::new(),
            advance: String::new(),
            start: String::new(),
            list: String::new(),
            mp_load: String::new(),
            mp_reset: String::new(),
            mp_gen_run: String::new(),
            record_final_score: String::new(),
            wf_action_run_generate: "run_generate".to_string(),
            wf_action_submit_answer: "submit_answer".to_string(),
            wf_action_record_final_score: "record_final_score".to_string(),
            wf_action_db_added: "db_score.added".to_string(),
        }
    }
}

/// --------------------------------------
/// Public: register all trivia server actions
/// --------------------------------------
#[instrument(skip(state))]
pub async fn register_server_actions(state: Arc<TriviaState>) -> Result<TriviaServerActions> {
    let mut ids = TriviaServerActions::new_defaults();

    ids.record_final_score = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action(
            "trivia.record_final_score",
            record_final_score(state.clone(), ids.clone()),
        )
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.list = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.list", list(state.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.mp_load = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.multiplayer.load", mp_load(state.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.mp_reset = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.multiplayer.reset", mp_reset(state.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;
    ids.mp_gen_run = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.multiplayer.gen-run", mp_gen_run(state.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.reveal = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.reveal", reveal(state.clone(), ids.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.gen_start = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.gen-start", gen_start(state.clone(), ids.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.gen_poll = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.gen-poll", gen_poll())
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.gen_run = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.gen-run", gen_run(state.clone(), ids.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.advance = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.advance", advance(state.clone(), ids.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    ids.start = state
        .tawks_client
        .as_ref()
        .expect("need tawks client")
        .register_server_action("trivia.quiz", start(state.clone(), ids.clone()))
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(ids)
}

// -------------------------
// Handlers (factory fns)
// -------------------------

#[instrument(skip(state, ids))]
fn record_final_score(state: Arc<TriviaState>, ids: TriviaServerActions) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        let ids = ids.clone();
        Box::pin(async move {
            trace!("Handler: record_final_score");
            record_final_score_impl(state, ids, ctx)
                .await
                .map_err(|e| e.into())
        })
    }))
}

#[instrument(skip(state))]
fn list(state: Arc<TriviaState>) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: list");
            list_impl(state, ctx).await.map_err(|e| e.into())
        })
    }))
}

#[instrument(skip(state))]
fn mp_load(state: Arc<TriviaState>) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: mp_load");
            mp_load_impl(state, ctx).await.map_err(|e| e.into())
        })
    }))
}

fn mp_gen_run(state: Arc<TriviaState>) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: mp_gen_run");
            mp_gen_run_impl(state, ctx).await.map_err(|e| e.into())
        })
    }))
}

fn mp_reset(state: Arc<TriviaState>) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: mp_reset");
            mp_reset_impl(state, ctx).await.map_err(|e| e.into())
        })
    }))
}

// #[instrument(skip(state, ids))]
// fn init(state: Arc<TriviaState>, ids: TriviaServerActions) -> ServerActionHandler {
//     Arc::new(Box::new(move |ctx| {
//         let state = state.clone();
//         let ids = ids.clone();
//         Box::pin(async move {
//             trace!("Handler: init");
//             quiz_init_impl(state, ids, ctx).await.map_err(|e| e.into())
//         })
//     }))
// }

#[instrument(skip(state, _ids))]
fn reveal(state: Arc<TriviaState>, _ids: TriviaServerActions) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: reveal");
            quiz_reveal_impl(state, ctx).await.map_err(|e| e.into())
        })
    }))
}

#[instrument(skip(state, ids))]
fn gen_start(state: Arc<TriviaState>, ids: TriviaServerActions) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        let ids = ids.clone();
        Box::pin(async move {
            trace!("Handler: gen_start");
            gen_start_impl(state, ids, ctx).await.map_err(|e| e.into())
        })
    }))
}

#[instrument]
fn gen_poll() -> ServerActionHandler {
    Arc::new(Box::new(move |_ctx| {
        Box::pin(async move {
            trace!("Handler: gen_poll");
            Ok(ServerActionResult::UpdateResponses(HashMap::new()))
        })
    }))
}

#[instrument(skip(state, _ids))]
fn gen_run(state: Arc<TriviaState>, _ids: TriviaServerActions) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: gen_run");
            gen_run_impl(state, ctx).await.map_err(|e| e.into())
        })
    }))
}

#[instrument(skip(state, ids))]
fn advance(state: Arc<TriviaState>, ids: TriviaServerActions) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let state = state.clone();
        let ids = ids.clone();
        Box::pin(async move {
            trace!("Handler: advance");
            quiz_advance_impl(state, ids, ctx)
                .await
                .map_err(|e| e.into())
        })
    }))
}

#[instrument(skip(state))]
fn start(state: Arc<TriviaState>, ids: TriviaServerActions) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let ids = ids.clone();
        let state = state.clone();
        Box::pin(async move {
            trace!("Handler: start");
            start_quiz_impl(&ids, state, ctx)
                .await
                .map_err(|e| e.into())
        })
    }))
}

// -------------------------
// Impl fns (pure logic)
// -------------------------

#[instrument(skip(state, ids, ctx))]
async fn record_final_score_impl(
    state: Arc<TriviaState>,
    ids: TriviaServerActions,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("record_final_score_impl called");
    let attempt_id = ctx.ctx_required_str("attempt_id")?;

    let _ = state.repo.finish_attempt(attempt_id.clone()).await?;

    Ok(ServerActionResult::UpdateContextAndResponses {
        context: HashMap::from([("results_recorded".into(), json!(true))]),
        responses: HashMap::from([("results_recorded".into(), json!(true))]),
    })
}

#[instrument(skip(state, ctx))]
async fn list_impl(
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("list_impl called");
    let pagination: ListTriviaArgsWithCommunityId =
        ctx.get_root_input_as().map_err(|e| anyhow!(e))?;
    let trivia_list = connection_from_repository(&pagination, state.repo.clone()).await?;

    Ok(ServerActionResult::UpdateResponses(HashMap::from([(
        "data".into(),
        json!(trivia_list),
    )])))
}

#[instrument(skip(state, ctx))]
async fn mp_load_impl(
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("mp_load_impl called");
    let trivia_id = ctx.input_required_str("trivia_id")?.to_string();
    let trivia = state.repo.get_trivia(trivia_id).await?;
    let trivia_resource = TriviaResource::from(&trivia);
    load_trivia_into_lobby(&state, &ctx, &trivia_resource).await?;
    Ok(ServerActionResult::UpdateResponses(HashMap::from([
        ("trivia".into(), json!(trivia_resource)),
        ("trivia_loaded".into(), json!(true)),
    ])))
}

/// Joins the caller to the lobby game and hands it the chosen set.
async fn load_trivia_into_lobby(
    state: &TriviaState,
    ctx: &WorkflowResourceWithContext,
    trivia_resource: &TriviaResource,
) -> Result<()> {
    let lobby_id = ctx
        .get_path_any("lobby_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if let (Some(engine), Some(lobby_id)) = (state.multiplayer_engine.as_ref(), lobby_id) {
        let user: UserResource = ctx
            .ctx_required_as_result("$user")
            .map_err(|e| anyhow!(e.to_string()))?;
        let _ = engine
            .handle_game_action_with_effects(
                lobby_id.clone(),
                ctx.workflow.instance_id.clone(),
                ctx.workflow.user_id.clone(),
                TriviaAction::Join {
                    name: user.display_name.clone(),
                    avatar: full_avatar_url(&user),
                    instance_id: ctx.workflow.instance_id.clone(),
                },
            )
            .await?;
        let _ = engine
            .handle_game_action_with_effects(
                lobby_id,
                ctx.workflow.instance_id.clone(),
                ctx.workflow.user_id.clone(),
                TriviaAction::SetTrivia {
                    trivia: trivia_resource.clone(),
                },
            )
            .await?;
    }
    Ok(())
}

/// Lobby variant of the generate job: builds the set from the topic, saves
/// it, and loads it into the lobby so everyone can start playing.
#[instrument(skip(state, ctx))]
async fn mp_gen_run_impl(
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let topic = ctx
        .context
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let quiz_model = match state.generator.generate(&topic).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to generate lobby quiz: {}", e);
            return Ok(ServerActionResult::UpdateResponses(HashMap::from([
                ("gen_state".into(), json!("error")),
                ("gen_error".into(), json!(e.to_string())),
            ])));
        }
    };
    let trivia = state.repo.save_generated_model(quiz_model).await?;
    let trivia_resource = TriviaResource::from(&trivia);
    load_trivia_into_lobby(&state, &ctx, &trivia_resource).await?;
    Ok(ServerActionResult::UpdateContextAndResponses {
        context: HashMap::from([
            ("trivia_id".into(), json!(trivia.trivia.id)),
            ("trivia".into(), json!(trivia_resource)),
        ]),
        responses: HashMap::from([
            ("trivia".into(), json!(trivia_resource)),
            ("trivia_loaded".into(), json!(true)),
            ("gen_state".into(), json!("ready")),
            ("gen_error".into(), json!(null)),
        ]),
    })
}

#[instrument(skip(state, ctx))]
async fn mp_reset_impl(
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    // Remove the player from the game engine so they stop receiving fanouts
    if let Some(engine) = state.multiplayer_engine.as_ref() {
        let lobby_id = ctx
            .get_path_any("lobby_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(lobby_id) = lobby_id {
            let _ = engine
                .handle_game_action_with_effects(
                    lobby_id,
                    ctx.workflow.instance_id.clone(),
                    ctx.workflow.user_id.clone(),
                    TriviaAction::Leave,
                )
                .await;
        }
    }
    Ok(ServerActionResult::UpdateResponses(HashMap::from([
        ("trivia".into(), json!(null)),
        ("trivia_loaded".into(), json!(false)),
        ("game".into(), json!(null)),
    ])))
}

#[instrument(skip(state, ctx))]
async fn quiz_reveal_impl(
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("quiz_reveal_impl called");
    let current_idx = ctx.ctx_get_u64("current_index", 0) as usize;
    let score = ctx.ctx_get_i64("score", 0);
    let mut audit = ctx.ctx_get_array("audit");

    let timed_out = ctx.ctx_get_bool("timed_out", false);

    let user_answer_opt = ctx
        .get_path_any("user_answer")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let did_answer = user_answer_opt
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let no_answer = timed_out || !did_answer;
    let user_answer = user_answer_opt.clone().unwrap_or_default();

    cancel_question_timeout(
        &state.tawks_client.as_ref().unwrap(),
        &ctx.workflow.instance_id,
        current_idx,
    )
    .await?;

    let quiz: TriviaResource = ctx.ctx_required_as_result("trivia")?;
    let questions = quiz.questions;

    let current_q_obj = questions
        .get(current_idx)
        .ok_or_else(|| anyhow!("Question idx out of bounds"))?;

    let correct_val = current_q_obj.correct_value.clone();

    let is_correct = !no_answer && user_answer == correct_val;

    let max_points = current_q_obj.points;

    let ends_at_str = ctx.context.get("question_ends_at").and_then(|v| v.as_str());

    let (time_scaled_award, time_left_ms, time_taken_ms, speed_pct) =
        scoring::compute_time_scaled_points(max_points, 15000, ends_at_str, chrono::Utc::now());

    let points_awarded = if is_correct { time_scaled_award } else { 0 };
    let new_score = score + points_awarded as i64;

    let options = current_q_obj.options.clone();

    let enriched_options = current_q_obj
        .options
        .iter()
        .map(|o| {
            let val = o.value.clone();
            let selected = !no_answer && val == user_answer;

            let status = if val == correct_val {
                "correct"
            } else if !no_answer && val == user_answer {
                "wrong"
            } else {
                "neutral"
            };

            let mut new_obj = o.clone();
            new_obj.status = Some(status.into());
            new_obj.selected = Some(selected);
            new_obj
        })
        .collect::<Vec<_>>();

    let find_label = |target_val: &str| -> String {
        for opt in options {
            if opt.value == target_val {
                return opt.label.clone();
            }
        }
        target_val.to_string()
    };

    audit.push(json!({
        "idx": current_idx,
        "ts": chrono::Utc::now().to_rfc3339(),
        "a_time_left_ms": time_left_ms,
        "a_time_taken_ms": time_taken_ms,
        "a_speed_pct": speed_pct,
        "max_points": max_points,
        "question": current_q_obj.text,
        "timed_out": timed_out,
        "did_answer": did_answer,
        "no_answer": no_answer,
        "user_answer": if no_answer { json!(null) } else { json!(find_label.clone()(&user_answer)) },
        "correct_answer": find_label(&correct_val),
        "is_correct": is_correct,
        "points_awarded": points_awarded,
        "options": enriched_options
    }));

    let attempt_id = ctx.ctx_required_str("attempt_id")?;
    let trivia_question_id = current_q_obj.id.clone();

    let trivia_answer_id = if no_answer {
        None
    } else {
        user_answer_opt
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    state
        .repo
        .submit_answer(SubmitAnswerArgs {
            attempt_id,
            trivia_question_id,
            trivia_answer_id,
            points: points_awarded,
        })
        .await?;

    let mut new_current_question = current_q_obj.clone();
    new_current_question.options = enriched_options;

    Ok(ServerActionResult::UpdateContextAndResponses {
        context: HashMap::from([
            ("score".into(), json!(new_score)),
            ("audit".into(), json!(audit)),
            ("revealed".into(), json!(true)),
            ("no_answer".into(), json!(no_answer)),
        ]),
        responses: HashMap::from([
            ("score".into(), json!(new_score)),
            ("audit".into(), json!(audit)),
            ("revealed".into(), json!(true)),
            ("no_answer".into(), json!(no_answer)),
            ("current_question".into(), json!(new_current_question)),
            ("db_delta".into(), json!(points_awarded)),
            ("db_anim_should_run".into(), json!(points_awarded > 0)),
            ("db_anim_key".into(), json!(ulid::Ulid::new().to_string())),
        ]),
    })
}

#[instrument(skip(state, ids, ctx))]
async fn gen_start_impl(
    state: Arc<TriviaState>,
    ids: TriviaServerActions,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("gen_start_impl called");
    let topic = ctx.input_required_str("topic")?;

    let generation_id = ulid::Ulid::new().to_string();

    let job = schedule_generate_job(&ids, &ctx.workflow.instance_id, &generation_id, 0);

    Ok(ServerActionResult::ScheduleJobs {
        reconcile_node: true,
        jobs: vec![job],
        context: HashMap::from([
            ("topic".into(), json!(topic)),
            ("generation_id".into(), json!(generation_id)),
        ]),
        inputs: HashMap::from([
            ("topic".into(), json!(topic)),
            ("gen_state".into(), json!("running")),
            ("gen_error".into(), json!(null)),
            ("generation_id".into(), json!(generation_id)),
        ]),
    })
}

#[instrument(skip(state, ctx))]
async fn gen_run_impl(
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("gen_run_impl called");
    let topic = ctx
        .context
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    let mut quiz_model = match state.generator.generate(&topic).await {
        Ok(q) => q,
        Err(e) => {
            error!("Failed to generate quiz: {}", e);
            return Ok(ServerActionResult::UpdateResponses(HashMap::from([
                ("gen_state".into(), json!("error")),
                ("gen_error".into(), json!(e.to_string())),
            ])));
        }
    };

    let trivia = state.repo.save_generated_model(quiz_model).await?;

    let trivia_resource = TriviaResource::from(&trivia);
    let preview_question_0 = trivia_resource
        .questions
        .get(0)
        .map(|q| json!(q))
        .unwrap_or(json!(null));

    Ok(ServerActionResult::UpdateContextAndResponses {
        context: HashMap::from([
            ("trivia_id".into(), json!(trivia.trivia.id)),
            ("trivia".into(), json!(trivia_resource)),
        ]),
        responses: HashMap::from([
            ("trivia_id".into(), json!(trivia.trivia.id)),
            ("gen_state".into(), json!("ready")),
            ("gen_error".into(), json!(null)),
            ("preview_question_0".into(), preview_question_0),
        ]),
    })
}

#[instrument(skip(state, ids, ctx))]
async fn quiz_advance_impl(
    state: Arc<TriviaState>,
    ids: TriviaServerActions,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    trace!("quiz_advance_impl called");
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    let current_idx = ctx.ctx_get_u64("current_index", 0) as usize;
    let already_recorded = ctx.ctx_get_bool("results_recorded", false);
    let attempt_id = ctx.ctx_required_str("attempt_id")?;

    let quiz: TriviaResource = ctx.ctx_required_as_result("trivia")?;
    let questions = quiz.questions.clone();

    let audit = ctx.ctx_get_array("audit");
    let score = ctx.ctx_get_i64("score", 0);

    let next_idx = current_idx + 1;
    let total_questions = questions.len();
    let progress_pct = ((next_idx as f64) / (total_questions as f64)).clamp(0.0, 1.0) * 100.0;

    if next_idx >= total_questions {
        let share = crate::share::view(
            &quiz.title,
            &audit,
            score,
            &chrono::Utc::now().date_naive().to_string(),
        );
        let inputs = HashMap::from([
            ("trivia_share".into(), share.clone()),
            ("is_finished".into(), json!(true)),
            ("progress_val".into(), json!(100)),
            ("question_ends_at".into(), json!(null)),
            ("selected".into(), json!(null)),
        ]);
        let context = HashMap::from([
            ("trivia_share".into(), share),
            ("is_finished".into(), json!(true)),
            ("progress_val".into(), json!(100)),
            ("question_ends_at".into(), json!(null)),
            ("selected".into(), json!(null)),
        ]);
        if !already_recorded {
            let payload = json!({
                "amount": score,
                "reason": "TRIVIA_SCORE",
                "reference_id": ctx.workflow.instance_id.clone(),
                "meta": {
                    "total_questions": total_questions,
                    "final_score": score,
                    "audit": audit
                },
            });

            let db_job = schedule_record_final_score(&ids, &attempt_id);
            let noti_job = schedule_notify_final_score(&ids, &attempt_id, payload);
            return Ok(ServerActionResult::ScheduleJobs {
                jobs: vec![db_job, noti_job],
                context,
                inputs,
                reconcile_node: true,
            });
        }

        return Ok(ServerActionResult::UpdateContextAndResponses {
            context,
            responses: inputs,
        });
    }

    let job = schedule_question_timeout(&ids, &ctx.workflow.instance_id, next_idx, 15000);

    let next_question = questions
        .get(next_idx)
        .cloned()
        .ok_or_else(|| anyhow!("Question idx out of bounds"))?;
    let attempt_id = ctx.ctx_required_str("attempt_id")?;
    let trivia_question_id = next_question.id.clone();

    state
        .repo
        .start_question(StartQuestionArgs {
            attempt_id: attempt_id.clone(),
            trivia_question_id,
        })
        .await?;

    let ends_at = scoring::question_ends_at_ms(15000, chrono::Utc::now());

    Ok(ServerActionResult::ScheduleJobs {
        jobs: vec![job],
        reconcile_node: true,
        context: HashMap::from([
            ("current_index".into(), json!(next_idx as u64)),
            (
                "current_question_number".into(),
                json!((next_idx + 1) as u64),
            ),
            ("current_question".into(), json!(next_question)),
            ("progress_val".into(), json!(progress_pct)),
            ("revealed".into(), json!(false)),
            ("timed_out".into(), json!(false)),
            ("no_answer".into(), json!(false)),
            ("user_answer".into(), json!(null)),
            ("question_ends_at".into(), json!(ends_at)),
        ]),
        inputs: HashMap::from([
            ("current_index".into(), json!(next_idx)),
            ("current_question_number".into(), json!(next_idx + 1)),
            ("current_question".into(), json!(next_question)),
            ("progress_val".into(), json!(progress_pct)),
            ("revealed".into(), json!(false)),
            ("total_questions".into(), json!(total_questions)),
            ("user_answer".into(), json!(null)),
            ("timed_out".into(), json!(false)),
            ("no_answer".into(), json!(false)),
            ("question_ends_at".into(), json!(ends_at)),
            ("selected".into(), json!(null)),
        ]),
    })
}

#[instrument()]
fn schedule_notify_final_score(
    ids: &TriviaServerActions,
    attempt_id: &str,
    payload: Value,
) -> ScheduledJob {
    use workflow::schedule::job::{ScheduledAction, ScheduledJob};

    ScheduledJob {
        idempotency_key: format!("trivia-notify-final-score:{}", attempt_id),
        after_ms: 0,
        guard: None,
        action: ScheduledAction::RunServerAction {
            action_id: ids.wf_action_db_added.clone(),
            payload,
        },
    }
}

#[instrument()]
fn schedule_record_final_score(ids: &TriviaServerActions, attempt_id: &str) -> ScheduledJob {
    use workflow::schedule::job::{ScheduledAction, ScheduledJob};

    ScheduledJob {
        idempotency_key: format!("trivia-final-score:{}", attempt_id),
        after_ms: 0,
        guard: None,
        action: ScheduledAction::RunServerAction {
            action_id: ids.wf_action_record_final_score.clone(),
            payload: json!(null),
        },
    }
}

#[instrument(skip(state, ctx))]
async fn start_quiz_impl(
    ids: &TriviaServerActions,
    state: Arc<TriviaState>,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    println!("ctx: {:?}", ctx);
    let trivia_id = ctx.input_required_str("trivia_id")?.to_string();

    let channel_id = json!(ctx.get_path_any("channel_id"));
    let niche_id = json!(ctx.get_path_any("niche_id"));
    let lobby_id = json!(ctx.get_path_any("lobby_id"));

    let trivia = state.repo.get_trivia(trivia_id).await?;
    let trivia = TriviaResource::from(&trivia);

    let questions = trivia.questions.clone();

    if questions.is_empty() {
        warn!("Quiz has no questions");
        return Err(anyhow!("Quiz has no questions"));
    }

    let job = schedule_question_timeout(&ids, &ctx.workflow.instance_id, 0, 15000);

    let total = questions.len();
    let attempt = state
        .repo
        .start_attempt(
            ctx.workflow.user_id,
            StartAttemptArgs {
                trivia_id: trivia.id.clone(),
            },
        )
        .await?;
    state
        .repo
        .start_question(StartQuestionArgs {
            attempt_id: attempt.id.clone(),
            trivia_question_id: questions[0].id.clone(),
        })
        .await?;

    Ok(ServerActionResult::ScheduleJobs {
        jobs: vec![job],
        reconcile_node: true,
        context: HashMap::from([
            ("trivia".into(), json!(trivia)),
            ("total_questions".into(), json!(total)),
            ("current_index".into(), json!(0u64)),
            ("current_question_number".into(), json!(1u64)),
            ("current_question".into(), json!(questions[0].clone())),
            ("revealed".into(), json!(false)),
            ("is_finished".into(), json!(false)),
            ("timed_out".into(), json!(false)),
            ("no_answer".into(), json!(false)),
            ("score".into(), json!(0i64)),
            ("lobby_id".into(), json!(lobby_id)),
            ("channel_id".into(), json!(channel_id)),
            ("niche_id".into(), json!(niche_id)),
            ("audit".into(), json!([])),
            ("trivia_share".into(), json!(null)),
            ("attempt_id".into(), json!(attempt.id)),
            ("results_recorded".into(), json!(false)),
            (
                "question_ends_at".into(),
                json!(scoring::question_ends_at_ms(15000, chrono::Utc::now())),
            ),
        ]),
        inputs: HashMap::from([
            ("total_questions".into(), json!(total)),
            ("current_index".into(), json!(0)),
            ("current_question_number".into(), json!(1)),
            ("current_question".into(), json!(questions[0].clone())),
            (
                "question_ends_at".into(),
                json!(scoring::question_ends_at_ms(15000, chrono::Utc::now())),
            ),
            ("revealed".into(), json!(false)),
            ("is_finished".into(), json!(false)),
            ("timed_out".into(), json!(false)),
            ("no_answer".into(), json!(false)),
            ("score".into(), json!(0)),
            ("audit".into(), json!([])),
            ("trivia_share".into(), json!(null)),
            ("results_recorded".into(), json!(false)),
        ]),
    })
}

// -------------------------
// Scheduling helpers
// -------------------------

#[instrument()]
fn schedule_generate_job(
    ids: &TriviaServerActions,
    instance_id: &str,
    generation_id: &str,
    after_ms: u32,
) -> ScheduledJob {
    ScheduledJob {
        idempotency_key: format!("trivia-gen:{}:{}", instance_id, generation_id),
        after_ms,
        guard: Some(NodeCondition::And(vec![
            NodeCondition::ResponseEquals {
                field: "gen_state".into(),
                value: json!("running"),
            },
            NodeCondition::ResponseEquals {
                field: "generation_id".into(),
                value: json!(generation_id),
            },
        ])),
        action: ScheduledAction::RunServerAction {
            action_id: ids.wf_action_run_generate.clone(),
            payload: json!({}),
        },
    }
}

#[instrument()]
fn schedule_question_timeout(
    ids: &TriviaServerActions,
    instance_id: &str,
    idx: usize,
    after_ms: u32,
) -> ScheduledJob {
    ScheduledJob {
        idempotency_key: timeout_key(instance_id, idx),
        after_ms,
        guard: Some(workflow::NodeCondition::And(vec![
            workflow::NodeCondition::ResponseEquals {
                field: "current_index".to_string(),
                value: json!(idx as u64),
            },
            workflow::NodeCondition::ResponseEquals {
                field: "revealed".to_string(),
                value: json!(false),
            },
            workflow::NodeCondition::ResponseEquals {
                field: "is_finished".to_string(),
                value: json!(false),
            },
        ])),
        action: workflow::schedule::job::ScheduledAction::RunServerAction {
            action_id: ids.wf_action_submit_answer.clone(),
            payload: json!({ "timed_out": true, "user_answer": null }),
        },
    }
}

#[instrument(skip(runtime))]
async fn cancel_question_timeout(
    runtime: &TawksBot,
    instance_id: &str,
    idx: usize,
) -> anyhow::Result<()> {
    trace!("cancel_question_timeout called");
    let key = timeout_key(instance_id, idx);

    let _: bool = runtime
        .sdk
        .web_client
        .mutate("workflow_cancel_job", &json!({ "idempotency_key": key }))
        .await?;

    Ok(())
}

#[instrument]
fn timeout_key(instance_id: &str, idx: usize) -> String {
    format!("trivia-timeout:{}:{}", instance_id, idx)
}

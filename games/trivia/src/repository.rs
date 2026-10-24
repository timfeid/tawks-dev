use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use rand::{rng, rngs, seq::SliceRandom};
use sqlx::postgres::PgArguments;
use std::{collections::HashMap, fmt, ops::Deref, path::Path, sync::Arc};
use tawks_bot::{TawksSdk, types::UserResource};
use tawks_data::{
    pagination::{Cursor, Model, Node, PaginationArgs, WithPagination},
    repository::{CursorDirection, Repository},
};
use tokio::fs;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Execute, Pool, Postgres, QueryBuilder, Row, query, query_as, query_scalar};
use ulid::Ulid;

use crate::{TriviaResource, generator::GeneratedTrivia};
use tracing::instrument;

#[derive(Type, Serialize, Debug, Clone)]
pub struct NicheListMembershipsInput {
    pub niche_id: String,
    pub search: Option<String>,
    pub within_role_id: Option<String>,
    pub not_within_role_id: Option<String>,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MembershipResource {
    pub id: String,
    pub user: UserResource,
}

#[derive(sqlx::FromRow, Serialize, Debug, Clone)]
pub struct TriviaRow {
    pub id: String,
    pub community_id: String,
    pub title: String,
    pub topic: String,
    pub description: String,
    pub closes_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Serialize, Debug, Clone)]
pub struct Trivia {
    pub id: String,
    pub community_id: String,
    pub title: String,
    pub topic: String,
    pub description: String,
    pub top_players: Vec<TriviaTopPlayer>,
    pub closes_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TriviaQuestion {
    pub id: String,
    pub trivia_id: String,
    pub order: i32,
    pub text: String,

    pub annotation_url: Option<String>,
    pub random_fact: Option<String>,
    pub incorrect_fluff: Option<String>,

    pub max_points: i32,
    pub duration_seconds: i32,

    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TriviaAnswer {
    pub id: String,
    pub trivia_question_id: String,
    pub text: String,
    pub is_correct: bool,
    pub created_at: time::OffsetDateTime,
    pub order: i32,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TriviaAttempt {
    pub id: String,
    pub trivia_id: String,
    pub user_id: String,
    pub started_at: time::OffsetDateTime,
    pub finished_at: Option<time::OffsetDateTime>,
    pub final_score: i32,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TriviaUserAnswer {
    pub id: String,
    pub attempt_id: String,
    pub trivia_question_id: String,
    pub trivia_answer_id: Option<String>,
    pub started_at: time::OffsetDateTime,
    pub finished_at: Option<time::OffsetDateTime>,
    pub points_awarded: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct CreateTriviaArgs {
    pub community_id: String,
    pub title: String,
    pub topic: String,
    pub description: String,
    pub closes_at: Option<time::OffsetDateTime>,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct CreateTriviaQuestionArgs {
    pub trivia_id: String,
    pub order: i32,
    pub text: String,

    pub annotation_url: Option<String>,
    pub random_fact: Option<String>,
    pub incorrect_fluff: Option<String>,

    pub max_points: i32,
    pub duration_seconds: Option<i32>,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct CreateTriviaAnswerArgs {
    pub trivia_question_id: String,
    pub text: String,
    pub is_correct: bool,
    pub id: String,
    order: i32,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct StartAttemptArgs {
    pub trivia_id: String,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct StartQuestionArgs {
    pub attempt_id: String,
    pub trivia_question_id: String,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct SubmitAnswerArgs {
    pub attempt_id: String,
    pub trivia_question_id: String,
    pub trivia_answer_id: Option<String>,
    pub points: i64,
}

#[derive(Debug, Clone)]
pub struct TriviaWithQuestions {
    pub trivia: Trivia,
    pub questions: Vec<TriviaQuestionWithAnswers>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaTopPlayer {
    pub user: UserResource,
    pub score: i32,
}

#[derive(Debug, Clone)]
pub struct TriviaQuestionWithAnswers {
    pub question: TriviaQuestion,
    pub answers: Vec<TriviaAnswer>,
    pub top_players: Vec<TriviaTopPlayer>,
}

pub struct TriviaRepository {
    tawks: Option<Arc<TawksSdk>>,
    pool: Arc<Pool<Postgres>>,
}

impl From<TriviaRow> for Trivia {
    fn from(r: TriviaRow) -> Self {
        Self {
            id: r.id,
            community_id: r.community_id,
            title: r.title,
            topic: r.topic,
            description: r.description,
            closes_at: r.closes_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            top_players: vec![],
        }
    }
}

impl TriviaRepository {
    async fn top_3_players_for_trivia(
        &self,
        trivia_id: &str,
        niche_id: &str,
    ) -> anyhow::Result<Vec<TriviaTopPlayer>> {
        // 1) Top 3 scores for this trivia
        let rows = sqlx::query(
            r#"
            SELECT user_id, final_score
            FROM trivia_attempts
            WHERE trivia_id = $1
              AND finished_at IS NOT NULL
            ORDER BY final_score DESC, finished_at ASC
            LIMIT 3
            "#,
        )
        .bind(trivia_id)
        .fetch_all(self.pool.deref())
        .await?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        // 2) Pull all niche memberships (auth/permissions handled by rspc)
        //    (You can add paging/search later if needed.)

        let memberships: Vec<MembershipResource> = self
            .tawks
            .as_ref()
            .as_ref()
            .ok_or(anyhow!("need sdk to do this"))?
            .web_client
            .query(
                "niche_list_memberships",
                &NicheListMembershipsInput {
                    niche_id: niche_id.to_string(),
                    search: None,
                    within_role_id: None,
                    not_within_role_id: None,
                },
            )
            .await?;

        let by_user: HashMap<String, MembershipResource> = memberships
            .into_iter()
            .map(|m| (m.user.id.clone(), m))
            .collect();

        // 3) Hydrate top 3 -> user resources
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let user_id: String = r.try_get("user_id")?;
            let final_score: i32 = r.try_get("final_score")?;
            if let Some(m) = by_user.get(&user_id) {
                out.push(TriviaTopPlayer {
                    user: m.user.clone(),
                    score: final_score,
                });
            }
        }

        Ok(out)
    }

    async fn top_players_for_trivia_ids(
        &self,
        trivia_ids: &[String],
        niche_id: &str,
    ) -> anyhow::Result<HashMap<String, Vec<TriviaTopPlayer>>> {
        if trivia_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // memberships once (auth already handled in rspc)
        let memberships: Vec<MembershipResource> = self
            .tawks
            .as_ref()
            .as_ref()
            .ok_or(anyhow!("need sdk to do this"))?
            .web_client
            .query(
                "niche_list_memberships",
                &NicheListMembershipsInput {
                    niche_id: niche_id.to_string(),
                    search: None,
                    within_role_id: None,
                    not_within_role_id: None,
                },
            )
            .await?;

        let by_user: HashMap<String, UserResource> = memberships
            .into_iter()
            .map(|m| (m.user.id.clone(), m.user))
            .collect();

        // top-3 rows for ALL trivias in this page
        let rows = sqlx::query(
            r#"
            WITH ranked AS (
              SELECT
                trivia_id,
                user_id,
                final_score,
                ROW_NUMBER() OVER (
                  PARTITION BY trivia_id
                  ORDER BY final_score DESC, finished_at ASC
                ) AS rn
              FROM trivia_attempts
              WHERE trivia_id = ANY($1)
                AND finished_at IS NOT NULL
            )
            SELECT trivia_id, user_id, final_score
            FROM ranked
            WHERE rn <= 3
            ORDER BY trivia_id, rn
            "#,
        )
        .bind(trivia_ids)
        .fetch_all(self.pool.deref())
        .await?;

        let mut out: HashMap<String, Vec<TriviaTopPlayer>> = HashMap::new();
        for r in rows {
            let trivia_id: String = r.try_get("trivia_id")?;
            let user_id: String = r.try_get("user_id")?;
            let final_score: i32 = r.try_get("final_score")?;
            if let Some(user) = by_user.get(&user_id) {
                out.entry(trivia_id).or_default().push(TriviaTopPlayer {
                    user: user.clone(),
                    score: final_score,
                });
            }
        }

        Ok(out)
    }

    pub async fn list_niche_users(
        &self,
        niche_id: String,
    ) -> anyhow::Result<Vec<MembershipResource>> {
        let memberships: Vec<MembershipResource> = self
            .tawks
            .as_ref()
            .as_ref()
            .ok_or(anyhow!("need sdk to do this"))?
            .web_client
            .query(
                "niche_list_memberships",
                &NicheListMembershipsInput {
                    niche_id: niche_id.clone(),
                    search: None,
                    within_role_id: None,
                    not_within_role_id: None,
                },
            )
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string().into()))?;

        Ok(memberships)
    }

    pub async fn get_trivia(&self, trivia_id: String) -> anyhow::Result<TriviaWithQuestions> {
        let mut trivia = self
            .find_trivia(trivia_id.clone())
            .await
            .map_err(|e| anyhow!(e.to_string()))?
            .ok_or_else(|| anyhow!("Trivia not found: {}", trivia_id))?;

        // ✅ compute once
        trivia.top_players = self
            .top_3_players_for_trivia(&trivia.id, &trivia.community_id)
            .await?;

        // existing load
        let questions = self
            .load_trivia_questions_with_answers(&trivia.id)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(TriviaWithQuestions { trivia, questions })
    }

    #[instrument(skip_all)]
    pub async fn save_generated_model(
        &self,
        quiz_model: GeneratedTrivia,
    ) -> anyhow::Result<TriviaWithQuestions> {
        let trivia = self
            .create_trivia(CreateTriviaArgs {
                community_id: "tawks".into(),
                topic: quiz_model.topic.clone(),
                title: quiz_model.title.clone(),
                description: quiz_model.description.clone(),
                closes_at: None,
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        for (i, question) in quiz_model.questions.iter().enumerate() {
            let question_model = self
                .create_question(CreateTriviaQuestionArgs {
                    trivia_id: trivia.id.clone(),
                    order: i as i32,
                    text: question.text.clone(),
                    annotation_url: question.annotation_url.clone(),
                    random_fact: question.random_fact.clone(),
                    incorrect_fluff: question.incorrect_fluff.clone(),
                    max_points: question.max_points.clone() as i32,
                    duration_seconds: Some(question.duration_seconds.clone()),
                })
                .await?;
            let mut options = question.options.clone();
            options.shuffle(&mut rng());
            for (i, answer) in options.iter().enumerate() {
                let _answer = self
                    .create_answer(CreateTriviaAnswerArgs {
                        trivia_question_id: question_model.id.clone(),
                        is_correct: answer.is_correct,
                        text: answer.label.clone(),
                        order: i as i32,
                        id: format!("v-{}", ulid::Ulid::new().to_string()),
                    })
                    .await?;
            }
        }

        self.get_trivia(trivia.id).await
    }

    #[instrument(skip_all)]
    pub fn new(pool: Arc<Pool<Postgres>>, tawks: Option<Arc<TawksSdk>>) -> TriviaRepository {
        TriviaRepository {
            pool,
            tawks,
        }
    }

    #[instrument(skip_all, fields(title = %args.title, community_id = %args.community_id))]
    pub async fn create_trivia(&self, args: CreateTriviaArgs) -> Result<Trivia, sqlx::Error> {
        let id = Ulid::new().to_string();

        let row = query_as::<_, TriviaRow>(
            r#"
            INSERT INTO trivia (id, topic, community_id, title, description, closes_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, now(), now())
            RETURNING
              id,
              topic,
              community_id,
              title,
              description,
              closes_at,
              created_at,
              updated_at
            "#,
        )
        .bind(id)
        .bind(args.topic)
        .bind(args.community_id)
        .bind(args.title)
        .bind(args.description)
        .bind(args.closes_at)
        .fetch_one(self.pool.deref())
        .await?;

        Ok(row.into())
    }

    #[instrument(skip_all, fields(trivia_id = %trivia_id))]
    pub async fn find_trivia(&self, trivia_id: String) -> Result<Option<Trivia>, sqlx::Error> {
        let row = query_as::<_, TriviaRow>(
            r#"
            SELECT id, topic, community_id, title, description, closes_at, created_at, updated_at
            FROM trivia
            WHERE id = $1
            "#,
        )
        .bind(trivia_id)
        .fetch_optional(self.pool.deref())
        .await?;

        Ok(row.map(Trivia::from))
    }

    #[instrument(skip_all, fields(community_id = %community_id))]
    pub async fn list_trivia_by_community(
        &self,
        community_id: String,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Trivia>, sqlx::Error> {
        let rows = query_as::<_, TriviaRow>(
            r#"
            SELECT id, topic, community_id, title, description, closes_at, created_at, updated_at
            FROM trivia
            WHERE community_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(community_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.deref())
        .await?;

        Ok(rows.into_iter().map(Trivia::from).collect())
    }

    #[instrument(skip_all, fields(trivia_id = %args.trivia_id, text = %args.text))]
    pub async fn create_question(
        &self,
        args: CreateTriviaQuestionArgs,
    ) -> Result<TriviaQuestion, sqlx::Error> {
        let id = Ulid::new().to_string();
        let duration = args.duration_seconds.unwrap_or(15);

        query_as::<_, TriviaQuestion>(
            r#"
            INSERT INTO trivia_questions (
              id, trivia_id, "order", text,
              annotation_url, random_fact, incorrect_fluff,
              max_points, duration_seconds,
              created_at, updated_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, now(), now())
            RETURNING
              id, trivia_id, "order" as "order", text,
              annotation_url, random_fact, incorrect_fluff,
              max_points, duration_seconds,
              created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(args.trivia_id)
        .bind(args.order)
        .bind(args.text)
        .bind(args.annotation_url)
        .bind(args.random_fact)
        .bind(args.incorrect_fluff)
        .bind(args.max_points)
        .bind(duration)
        .fetch_one(self.pool.deref())
        .await
    }

    #[instrument(skip_all, fields(trivia_question_id = %args.trivia_question_id, text = %args.text))]
    pub async fn create_answer(
        &self,
        args: CreateTriviaAnswerArgs,
    ) -> Result<TriviaAnswer, sqlx::Error> {
        let id = Ulid::new().to_string();

        query_as::<_, TriviaAnswer>(
            r#"
            INSERT INTO trivia_answers (id, trivia_question_id, text, is_correct, "order", created_at)
            VALUES ($1, $2, $3, $4, $5, now())
            RETURNING id, trivia_question_id, text, is_correct, "order", created_at
            "#,
        )
        .bind(id)
        .bind(args.trivia_question_id)
        .bind(args.text)
        .bind(args.is_correct)
        .bind(args.order)
        .fetch_one(self.pool.deref())
        .await
    }

    #[instrument(skip_all, fields(trivia_id = %trivia_id))]
    pub async fn load_trivia_questions_with_answers(
        &self,
        trivia_id: &str,
    ) -> Result<Vec<TriviaQuestionWithAnswers>, sqlx::Error> {
        let questions = query_as::<_, TriviaQuestion>(
            r#"
            SELECT
              id, trivia_id, "order" as "order", text,
              annotation_url, random_fact, incorrect_fluff,
              max_points, duration_seconds,
              created_at, updated_at
            FROM trivia_questions
            WHERE trivia_id = $1
            ORDER BY "order" ASC
            "#,
        )
        .bind(trivia_id)
        .fetch_all(self.pool.deref())
        .await?;

        if questions.is_empty() {
            return Ok(vec![]);
        }

        let question_ids: Vec<String> = questions.iter().map(|q| q.id.clone()).collect();

        let answers = query_as::<_, TriviaAnswer>(
            r#"
            SELECT id, trivia_question_id, text, is_correct, created_at, "order"
            FROM trivia_answers
            WHERE trivia_question_id = ANY($1)
            ORDER BY "order" ASC
            "#,
        )
        .bind(&question_ids)
        .fetch_all(self.pool.deref())
        .await?;

        use std::collections::HashMap;
        let mut by_q: HashMap<String, Vec<TriviaAnswer>> = HashMap::new();
        for a in answers {
            by_q.entry(a.trivia_question_id.clone())
                .or_default()
                .push(a);
        }

        Ok(questions
            .into_iter()
            .map(|q| TriviaQuestionWithAnswers {
                answers: by_q.remove(&q.id).unwrap_or_default(),
                question: q,
                top_players: vec![],
            })
            .collect())
    }

    #[instrument(skip_all, fields(user_id = %user_id, trivia_id = %args.trivia_id))]
    pub async fn start_attempt(
        &self,
        user_id: String,
        args: StartAttemptArgs,
    ) -> Result<TriviaAttempt, sqlx::Error> {
        let attempt_id = Ulid::new().to_string();

        query_as::<_, TriviaAttempt>(
            r#"
            INSERT INTO trivia_attempts (id, trivia_id, user_id, started_at, finished_at, final_score)
            VALUES ($1, $2, $3, now(), NULL, 0)
            ON CONFLICT (trivia_id, user_id)
            DO UPDATE SET started_at = trivia_attempts.started_at
            RETURNING id, trivia_id, user_id, started_at, finished_at, final_score
            "#,
        )
        .bind(attempt_id)
        .bind(args.trivia_id)
        .bind(user_id)
        .fetch_one(self.pool.deref())
        .await
    }

    #[instrument(skip_all, fields(attempt_id = %args.attempt_id, trivia_question_id = %args.trivia_question_id))]
    pub async fn start_question(
        &self,
        args: StartQuestionArgs,
    ) -> Result<TriviaUserAnswer, sqlx::Error> {
        let id = Ulid::new().to_string();

        query_as::<_, TriviaUserAnswer>(
            r#"
            INSERT INTO trivia_user_answers (
              id, attempt_id, trivia_question_id, trivia_answer_id,
              started_at, finished_at, points_awarded
            )
            VALUES ($1, $2, $3, NULL, now(), NULL, NULL)
            ON CONFLICT (attempt_id, trivia_question_id)
            DO UPDATE SET started_at = trivia_user_answers.started_at
            RETURNING
              id, attempt_id, trivia_question_id, trivia_answer_id,
              started_at, finished_at, points_awarded
            "#,
        )
        .bind(id)
        .bind(args.attempt_id)
        .bind(args.trivia_question_id)
        .fetch_one(self.pool.deref())
        .await
    }

    #[instrument(skip_all, fields(points = %args.points, attempt_id = %args.attempt_id, trivia_question_id = %args.trivia_question_id))]
    pub async fn submit_answer(
        &self,
        args: SubmitAnswerArgs,
    ) -> Result<TriviaUserAnswer, sqlx::Error> {
        let ua_id = Ulid::new().to_string();

        query_as::<_, TriviaUserAnswer>(
            r#"
INSERT INTO trivia_user_answers (
  id,
  attempt_id,
  trivia_question_id,
  trivia_answer_id,
  started_at,
  finished_at,
  points_awarded
)
VALUES (
  $1::varchar,
  $2::varchar,
  $3::varchar,
  $4::varchar,
  now(),
  now(),
  $5::bigint
)
ON CONFLICT (attempt_id, trivia_question_id)
DO UPDATE SET
  trivia_answer_id = EXCLUDED.trivia_answer_id,
  finished_at      = EXCLUDED.finished_at,
  points_awarded   = EXCLUDED.points_awarded,
  started_at       = COALESCE(trivia_user_answers.started_at, EXCLUDED.started_at)
RETURNING
  id,
  attempt_id,
  trivia_question_id,
  trivia_answer_id,
  started_at,
  finished_at,
  points_awarded;
        "#,
        )
        .bind(ua_id)
        .bind(args.attempt_id)
        .bind(args.trivia_question_id)
        .bind(args.trivia_answer_id)
        .bind(args.points)
        .fetch_one(self.pool.deref())
        .await
    }

    #[instrument(skip_all, fields(attempt_id = %attempt_id))]
    pub async fn finish_attempt(&self, attempt_id: String) -> Result<TriviaAttempt, sqlx::Error> {
        query_as::<_, TriviaAttempt>(
            r#"
            WITH summed AS (
              SELECT
                attempt_id,
                COALESCE(SUM(COALESCE(points_awarded, 0)), 0) AS score
              FROM trivia_user_answers
              WHERE attempt_id = $1
              GROUP BY attempt_id
            )
            UPDATE trivia_attempts ta
            SET
              finished_at = now(),
              final_score = COALESCE(summed.score, 0)
            FROM summed
            WHERE ta.id = $1
            RETURNING ta.id, ta.trivia_id, ta.user_id, ta.started_at, ta.finished_at, ta.final_score
            "#,
        )
        .bind(attempt_id)
        .fetch_one(self.pool.deref())
        .await
    }

    #[instrument(skip_all, fields(trivia_id = %trivia_id))]
    pub async fn is_trivia_open(&self, trivia_id: String) -> Result<bool, sqlx::Error> {
        let open = query_scalar::<_, bool>(
            r#"
            SELECT CASE
              WHEN closes_at IS NULL THEN true
              ELSE closes_at > now()
            END as open
            FROM trivia
            WHERE id = $1
            "#,
        )
        .bind(trivia_id)
        .fetch_one(self.pool.deref())
        .await?;

        Ok(open)
    }

    #[instrument(skip_all, fields(trivia_id = %trivia_id, user_id = %user_id))]
    pub async fn attempt_exists_for_user(
        &self,
        trivia_id: String,
        user_id: String,
    ) -> Result<bool, sqlx::Error> {
        let exists = query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM trivia_attempts
              WHERE trivia_id = $1 AND user_id = $2
            ) as exists
            "#,
        )
        .bind(trivia_id)
        .bind(user_id)
        .fetch_one(self.pool.deref())
        .await?;
        Ok(exists)
    }
}

#[derive(Type, Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct TriviaCursor {
    pub sort_method: ListTriviaSort,
    pub id: String,
}

impl Cursor for TriviaCursor {
    type CursorType = TriviaCursor;

    fn encode(cursor: &TriviaCursor) -> String {
        general_purpose::STANDARD.encode(cursor.to_string())
    }

    fn decode(encoded: &str) -> Option<TriviaCursor> {
        let decoded_bytes = general_purpose::STANDARD.decode(encoded).ok()?;
        let decoded_str = String::from_utf8(decoded_bytes).ok()?;
        serde_json::from_str(&decoded_str).ok()
    }

    fn sort_key(&self) -> String {
        format!("{:?}", self.sort_method)
    }

    fn id(&self) -> String {
        self.id.clone()
    }
}

impl fmt::Display for TriviaCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{json}"),
            Err(_) => write!(f, ""),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum ListTriviaSort {
    CreatedDesc,
    CreatedAsc,
    ClosesAtAsc,
    ClosesAtDesc,
    TitleAsc,
}

impl Default for ListTriviaSort {
    fn default() -> Self {
        ListTriviaSort::CreatedDesc
    }
}

impl Model<TriviaResource> for Trivia {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn to_node(&self) -> TriviaResource {
        TriviaResource {
            id: self.id.clone(),
            topic: self.topic.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            questions: vec![],
            top_players: self.top_players.clone(),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Default)]
pub struct ListTriviaArgsWithCommunityId {
    #[serde(rename = "niche_id")]
    pub community_id: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub first: Option<i32>,
    pub last: Option<i32>,
    pub sort: Option<ListTriviaSort>,
    pub include_closed: Option<bool>,
}

#[derive(Type, Deserialize, Serialize, Debug, Default)]
pub struct ListTriviaMeta {
    pub sort: ListTriviaSort,
}

impl Node for TriviaResource {
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl WithPagination for ListTriviaArgsWithCommunityId {
    fn pagination(&self) -> PaginationArgs {
        PaginationArgs {
            before: self.before.clone(),
            after: self.after.clone(),
            first: self.first.clone(),
            last: self.last.clone(),
        }
    }

    type Meta = ListTriviaMeta;
    type CursorType = TriviaCursor;

    fn get_meta(&self) -> Self::Meta {
        ListTriviaMeta {
            sort: self.sort.clone().unwrap_or_default(),
        }
    }

    fn to_cursor(&self, id: String, _sort_key: String) -> Self::CursorType {
        let mut cursor =
            TriviaCursor::decode(&self.after.as_ref().map_or("", |v| v)).unwrap_or_default();
        cursor.sort_method = self.sort.clone().unwrap_or_default();
        cursor.id = id;
        cursor
    }
}

impl Repository<Trivia, ListTriviaArgsWithCommunityId> for TriviaRepository {
    #[instrument(skip_all, fields(community_id = %args.community_id))]
    async fn count(&self, args: &ListTriviaArgsWithCommunityId) -> Result<i32> {
        let mut qb = QueryBuilder::<Postgres>::new("SELECT count(*) as count FROM trivia");

        qb.push(" WHERE trivia.community_id = ");
        qb.push_bind(args.community_id.clone());

        let include_closed = args.include_closed.unwrap_or(false);
        if !include_closed {
            qb.push(" AND (trivia.closes_at IS NULL OR trivia.closes_at > now())");
        }

        let count: (i64,) = qb.build_query_as().fetch_one(self.pool.deref()).await?;
        Ok(count.0 as i32)
    }

    #[instrument(skip_all, fields(community_id = %args.community_id, take = take))]
    async fn find(
        &self,
        cursor_options: Option<(CursorDirection, impl Cursor + Send)>,
        take: i32,
        args: &ListTriviaArgsWithCommunityId,
    ) -> Result<Vec<Trivia>> {
        let sort = args.sort.clone().unwrap_or_default();

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            SELECT
              id,
              community_id,
              topic,
              title,
              description,
              closes_at,
              created_at,
              updated_at
            FROM trivia
            "#,
        );

        qb.push(" WHERE trivia.community_id = ");
        qb.push_bind(args.community_id.clone());

        let include_closed = args.include_closed.unwrap_or(false);
        if !include_closed {
            qb.push(" AND (trivia.closes_at IS NULL OR trivia.closes_at > now())");
        }

        if let Some((direction, ref cursor)) = cursor_options {
            qb.push(" AND ");

            match direction {
                CursorDirection::After => qb.push("trivia.id < "),
                CursorDirection::Before => qb.push("trivia.id > "),
            };
            qb.push_bind(cursor.id());
        }

        let order_clause_forward = match sort {
            ListTriviaSort::CreatedDesc => " ORDER BY trivia.created_at DESC, trivia.id DESC ",
            ListTriviaSort::CreatedAsc => " ORDER BY trivia.created_at ASC, trivia.id ASC ",
            ListTriviaSort::ClosesAtAsc => {
                " ORDER BY trivia.closes_at ASC NULLS LAST, trivia.id ASC "
            }
            ListTriviaSort::ClosesAtDesc => {
                " ORDER BY trivia.closes_at DESC NULLS LAST, trivia.id DESC "
            }
            ListTriviaSort::TitleAsc => " ORDER BY trivia.title ASC, trivia.id ASC ",
        };

        let order_clause_reverse = match sort {
            ListTriviaSort::CreatedDesc => " ORDER BY trivia.created_at ASC, trivia.id ASC ",
            ListTriviaSort::CreatedAsc => " ORDER BY trivia.created_at DESC, trivia.id DESC ",
            ListTriviaSort::ClosesAtAsc => {
                " ORDER BY trivia.closes_at DESC NULLS LAST, trivia.id DESC "
            }
            ListTriviaSort::ClosesAtDesc => {
                " ORDER BY trivia.closes_at ASC NULLS LAST, trivia.id ASC "
            }
            ListTriviaSort::TitleAsc => " ORDER BY trivia.title DESC, trivia.id DESC ",
        };

        match cursor_options {
            Some((CursorDirection::Before, _)) => qb.push(order_clause_reverse),
            _ => qb.push(order_clause_forward),
        };

        qb.push(" LIMIT ");
        qb.push_bind(take);

        let query = qb.build_query_as::<TriviaRow>();
        let mut items: Vec<Trivia> = query
            .fetch_all(self.pool.deref())
            .await?
            .into_iter()
            .map(Trivia::from)
            .collect();
        // Pull all users in this niche/community using the already-permissioned app endpoint.

        // ✅ enrich list rows
        let trivia_ids: Vec<String> = items.iter().map(|t| t.id.clone()).collect();
        let by_trivia = self
            .top_players_for_trivia_ids(&trivia_ids, &args.community_id)
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string().into()))?;

        for t in &mut items {
            t.top_players = by_trivia.get(&t.id).cloned().unwrap_or_default();
        }

        if let Some((direction, _)) = cursor_options {
            if direction == CursorDirection::Before {
                items.reverse();
            }
        }

        Ok(items)
    }
}

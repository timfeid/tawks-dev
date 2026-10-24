//! Runs against the database in TRIVIA_DATABASE_URL and is skipped without it.

use std::sync::Arc;

use trivia::Generator;
use trivia::generator::local::LocalMockGenerator;
use trivia::repository::{StartAttemptArgs, StartQuestionArgs, SubmitAnswerArgs, TriviaRepository};

async fn pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("TRIVIA_DATABASE_URL").ok()?;
    Some(sqlx::PgPool::connect(&url).await.expect("connect"))
}

#[tokio::test]
async fn migrations_run_twice_and_a_quiz_can_be_played() {
    let Some(pool) = pool().await else { return };
    sqlx::migrate!().run(&pool).await.expect("first run");
    sqlx::migrate!().run(&pool).await.expect("second run");

    let repo = TriviaRepository::new(Arc::new(pool), None);
    let quiz = LocalMockGenerator::devils_fixture()
        .generate("Hockey")
        .await
        .unwrap();
    let saved = repo.save_generated_model(quiz).await.unwrap();
    assert!(!saved.questions.is_empty());

    let loaded = repo.get_trivia(saved.trivia.id.clone()).await.unwrap();
    assert_eq!(loaded.questions.len(), saved.questions.len());

    let attempt = repo
        .start_attempt(
            "player-1".into(),
            StartAttemptArgs {
                trivia_id: saved.trivia.id.clone(),
            },
        )
        .await
        .unwrap();
    let question = &loaded.questions[0];
    let correct = question
        .answers
        .iter()
        .find(|a| a.is_correct)
        .expect("a correct answer");
    repo.start_question(StartQuestionArgs {
        attempt_id: attempt.id.clone(),
        trivia_question_id: question.question.id.clone(),
    })
    .await
    .unwrap();
    let answer = repo
        .submit_answer(SubmitAnswerArgs {
            attempt_id: attempt.id.clone(),
            trivia_question_id: question.question.id.clone(),
            trivia_answer_id: Some(correct.id.clone()),
            points: 10,
        })
        .await
        .unwrap();
    assert_eq!(answer.points_awarded, Some(10));
}

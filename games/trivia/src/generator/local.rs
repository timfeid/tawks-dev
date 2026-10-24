use anyhow::{Result, anyhow};
use async_trait::async_trait;
// use async_trait::async_trait;
use chrono::Utc;

use crate::{
    Generator,
    generator::{GeneratedTrivia, GeneratedTriviaOptionItem, GeneratedTriviaQuestion},
};

/// Deterministic, local generator for tests/dev.
/// Useful for unit tests and offline runs.
///
/// If you want multiple fixtures, add variants or pass a `Quiz` in directly.
#[derive(Debug, Clone, Default)]
pub struct LocalMockGenerator {
    quiz: GeneratedTrivia,
}

impl LocalMockGenerator {
    /// Construct with a provided quiz fixture.
    pub fn new(quiz: GeneratedTrivia) -> Self {
        Self { quiz }
    }

    /// Construct with a built-in mock quiz (handy for quick testing).
    pub fn devils_fixture() -> Self {
        Self {
            quiz: mock_devils_quiz(),
        }
    }
}

#[async_trait]
impl Generator for LocalMockGenerator {
    async fn generate(&self, topic: &str) -> Result<GeneratedTrivia> {
        // Optionally enforce the same contract as OpenAI generator.
        if topic.trim().is_empty() {
            return Err(anyhow!("topic is empty"));
        }

        // If you want topic to always match what was requested:
        let mut quiz = self.quiz.clone();
        quiz.topic = topic.trim().to_string();

        Ok(quiz)
    }
}

/// A simple, valid 5-question quiz fixture.
/// Adjust to your `Quiz`/`Question`/`Option` types.
fn mock_devils_quiz() -> GeneratedTrivia {
    GeneratedTrivia {
        topic: "New Jersey Devils".to_string(),
        title: "test".into(),
        description: "test".into(),
        questions: vec![
            GeneratedTriviaQuestion {
                tts: None,
                text:
                    "In what year did the New Jersey Devils relocate from Colorado to New Jersey?"
                        .to_string(),
                max_points: 100,
                options: vec![
                    GeneratedTriviaOptionItem {
                        is_correct: false,
                        label: "1976".to_string(),
                    },
                    GeneratedTriviaOptionItem {
                        is_correct: false,
                        label: "1980".to_string(),
                    },
                    GeneratedTriviaOptionItem {
                        is_correct: true,
                        label: "1982".to_string(),
                    },
                    GeneratedTriviaOptionItem {
                        is_correct: false,
                        label: "1985".to_string(),
                    },
                ],
                annotation_url: None,
                random_fact: None,
                incorrect_fluff: None,
                duration_seconds: 15,
                explanation: None,
            },
            // GeneratedTriviaQuestion {
            //     tts: None,
            //     annotation_url: None,
            //     random_fact: None,
            //     incorrect_fluff: None,
            //     duration_seconds: 15,
            //     explanation: None,

            //     text: "How many Stanley Cups have the New Jersey Devils won?".to_string(),
            //     max_points: 100,
            //     options: vec![
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "1".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "2".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: true,
            //             label: "3".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "4".to_string(),
            //         },
            //     ],
            // },
            // GeneratedTriviaQuestion {
            //     tts: None,
            //     annotation_url: None,
            //     random_fact: None,
            //     incorrect_fluff: None,
            //     duration_seconds: 15,
            //     explanation: None,

            //     text: "Which Devils goaltender is most associated with the team's Stanley Cup era?"
            //         .to_string(),
            //     max_points: 150,
            //     options: vec![
            //         GeneratedTriviaOptionItem {
            //             is_correct: true,
            //             label: "Martin Brodeur".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Mike Richter".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Patrick Roy".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Roberto Luongo".to_string(),
            //         },
            //     ],
            // },
            // GeneratedTriviaQuestion {
            //     tts: None,
            //     annotation_url: None,
            //     random_fact: None,
            //     incorrect_fluff: None,
            //     duration_seconds: 15,
            //     explanation: None,

            //     text: "What arena do the New Jersey Devils currently play in?".to_string(),
            //     max_points: 100,
            //     options: vec![
            //         GeneratedTriviaOptionItem {
            //             is_correct: true,
            //             label: "Prudential Center".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Madison Square Garden".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Barclays Center".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Continental Airlines Arena".to_string(),
            //         },
            //     ],
            // },
            // GeneratedTriviaQuestion {
            //     tts: None,
            //     annotation_url: None,
            //     random_fact: None,
            //     incorrect_fluff: None,
            //     duration_seconds: 15,
            //     explanation: None,

            //     text: "Which Devils captain was nicknamed 'Mr. Devil'?".to_string(),
            //     max_points: 100,
            //     options: vec![
            //         GeneratedTriviaOptionItem {
            //             is_correct: true,
            //             label: "Ken Daneyko".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Scott Stevens".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Scott Niedermayer".to_string(),
            //         },
            //         GeneratedTriviaOptionItem {
            //             is_correct: false,
            //             label: "Scott Gomez".to_string(),
            //         },
            //     ],
            // },
        ],
    }
}

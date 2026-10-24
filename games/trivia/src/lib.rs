use serde::{Deserialize, Serialize};

pub mod generator;
pub(crate) mod model;
pub mod multiplayer;
pub mod repository;
pub(crate) mod scoring;
pub(crate) mod server_actions;
pub(crate) mod share;
pub mod state;
pub mod workflows;

pub use repository::TriviaWithQuestions;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TriviaResource {
    pub id: String,
    pub topic: String,
    pub title: String,
    pub description: String,
    pub questions: Vec<QuestionResource>,
    pub top_players: Vec<TriviaTopPlayer>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct QuestionResource {
    pub id: String,
    pub text: String,
    pub points: i64,
    pub correct_value: String,
    pub options: Vec<OptionResource>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OptionResource {
    pub value: String,
    pub label: String,
    pub icon: String,
    pub status: Option<String>,
    pub selected: Option<bool>,
}

pub use generator::Generator;
pub use model::*;

use crate::repository::{TriviaAnswer, TriviaQuestionWithAnswers, TriviaTopPlayer};

impl From<&TriviaWithQuestions> for TriviaResource {
    fn from(q: &TriviaWithQuestions) -> Self {
        Self {
            id: q.trivia.id.clone(),
            title: q.trivia.title.clone(),
            description: q.trivia.description.clone(),
            topic: q.trivia.topic.clone(),
            questions: q.questions.iter().map(QuestionResource::from).collect(),
            top_players: q.trivia.top_players.clone(),
        }
    }
}

impl From<&TriviaQuestionWithAnswers> for QuestionResource {
    fn from(q: &TriviaQuestionWithAnswers) -> Self {
        Self {
            text: q.question.text.clone(),
            id: q.question.id.clone(),
            points: q.question.max_points as i64,
            correct_value: q
                .answers
                .iter()
                .find(|option| option.is_correct)
                .map(|option| option.id.clone())
                .unwrap_or_default(),
            options: q.answers.iter().map(OptionResource::from).collect(),
        }
    }
}

impl From<&TriviaAnswer> for OptionResource {
    fn from(o: &TriviaAnswer) -> Self {
        Self {
            value: o.id.clone(),
            label: o.text.clone(),
            selected: None,
            status: None,
            icon: match o.order + 1 {
                1 => "a".to_string(),
                2 => "b".to_string(),
                3 => "c".to_string(),
                4 => "d".to_string(),
                5 => "e".to_string(),
                6 => "f".to_string(),
                _ => "".to_string(),
            },
        }
    }
}

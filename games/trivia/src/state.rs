use std::sync::Arc;

use anyhow::Result;
use sqlx::{Pool, Postgres};

use tawks_bot::TawksBot;
use tokio::sync::OnceCell;

use crate::{
    Generator,
    multiplayer::TriviaGameState,
    repository::TriviaRepository,
    server_actions::{TriviaServerActions, register_server_actions},
    workflows::{TriviaWorkflows, register_workflows},
};
use game_framework::engine::GameEngine;

#[derive(Clone)]
pub struct TriviaState {
    pub repo: Arc<TriviaRepository>,
    pub registered: bool,
    pub tawks_client: Option<TawksBot>,
    pub generator: Arc<dyn Generator + Send + Sync>,
    pub server_actions: Arc<OnceCell<TriviaServerActions>>,
    pub workflows: Arc<OnceCell<TriviaWorkflows>>,
    pub multiplayer_engine: Option<Arc<GameEngine<TriviaGameState>>>,
}

impl TriviaState {
    pub fn new(
        pool: Arc<Pool<Postgres>>,
        generator: Arc<dyn Generator + Send + Sync>,
        bot: Option<TawksBot>,
    ) -> Self {
        let repo = Arc::new(TriviaRepository::new(
            pool.clone(),
            bot.as_ref().and_then(|b| Some(b.sdk.clone())),
        ));

        Self {
            repo,
            tawks_client: bot,
            generator,
            server_actions: Arc::new(OnceCell::new()),
            workflows: Arc::new(OnceCell::new()),
            registered: false,
            multiplayer_engine: None,
        }
    }

    pub fn set_multiplayer_engine(&mut self, engine: Arc<GameEngine<TriviaGameState>>) {
        self.multiplayer_engine = Some(engine);
    }

    pub async fn register(&mut self) -> Result<()> {
        self.registered = true;
        let state = Arc::new(self.clone());

        let actions = register_server_actions(state.clone()).await?;
        let _ = state.server_actions.set(actions);

        let workflows = register_workflows(state.clone()).await?;
        let _ = state.workflows.set(workflows);

        Ok(())
    }
}

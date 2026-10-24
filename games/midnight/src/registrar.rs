use anyhow::Result;
use std::sync::{Arc, Mutex};

use tawks_bot::TawksBot;
use workflow::CreateWorkflowDefinition;

use crate::gamestate::GameState;
use crate::roles::{BoxedStateHandler, RoleRegistrar};
use game_framework::engine::GameEngine;

pub struct EngineRoleRegistrar {
    engine: Arc<GameEngine<GameState>>,
    bot: TawksBot,

    state_handlers: Mutex<Vec<(String, BoxedStateHandler)>>,
    workflow_defs: Mutex<Vec<CreateWorkflowDefinition>>,
}

impl EngineRoleRegistrar {
    pub fn new(engine: Arc<GameEngine<GameState>>, bot: TawksBot) -> Self {
        Self {
            engine,
            bot,
            state_handlers: Mutex::new(vec![]),
            workflow_defs: Mutex::new(vec![]),
        }
    }

    /// Call once after all roles have registered their hooks
    pub async fn flush(&self) -> Result<()> {
        let handlers = {
            let mut lock = self.state_handlers.lock().unwrap();
            std::mem::take(&mut *lock)
        };

        for (name, handler) in handlers {
            self.engine
                .clone()
                .register_state_handler(&name, move |game, ctx| (handler)(game, ctx))
                .await?;
        }

        let defs = {
            let mut lock = self.workflow_defs.lock().unwrap();
            std::mem::take(&mut *lock)
        };

        for def in defs {
            self.bot.register_workflow(def).await?;
        }

        Ok(())
    }
}

impl RoleRegistrar for EngineRoleRegistrar {
    fn register_state_handler_boxed(&self, action_name: &str, handler: BoxedStateHandler) {
        self.state_handlers
            .lock()
            .unwrap()
            .push((action_name.to_string(), handler));
    }

    fn register_workflow_definition(&self, def: CreateWorkflowDefinition) {
        self.workflow_defs.lock().unwrap().push(def);
    }
}

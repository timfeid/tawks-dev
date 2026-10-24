use std::sync::Arc;

use anyhow::Result;
use workflow::service::ProcessWorkflowActionArgs;
use workflow::service::{RegisterExternalServerActionArgs, RenewRegistrarLeaseArgs};

use crate::web_client::WebClient;

pub struct ServerActionService {
    pub web_client: Arc<WebClient>,
}

impl ServerActionService {
    pub fn new(web_client: Arc<WebClient>) -> Self {
        Self { web_client }
    }

    pub async fn register(&self, action_id: &str) -> Result<String> {
        self.register_with_registrar(action_id, None).await
    }

    pub async fn register_with_registrar(
        &self,
        action_id: &str,
        registrar_id: Option<&str>,
    ) -> Result<String> {
        let args = RegisterExternalServerActionArgs {
            action_id: action_id.to_string(),
            registrar_id: registrar_id.map(|value| value.to_string()),
        };
        let response = self
            .web_client
            .mutate("server_action_register", &args)
            .await?;

        Ok(response)
    }

    pub async fn renew_leases(&self, registrar_id: &str) -> Result<u32> {
        let args = RenewRegistrarLeaseArgs {
            registrar_id: registrar_id.to_string(),
        };
        self.web_client
            .mutate("server_action_renew_leases", &args)
            .await
    }

    pub async fn process(&self, workflow: ProcessWorkflowActionArgs) -> Result<()> {
        self.web_client
            .mutate("server_action_process", &workflow)
            .await
    }
}

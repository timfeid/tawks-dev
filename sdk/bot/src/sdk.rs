use std::sync::Arc;

use crate::{services::server_action::ServerActionService, web_client::WebClient};
use anyhow::Result;

pub struct TawksSdk {
    pub web_client: Arc<WebClient>,
    pub server_actions: ServerActionService,
}

impl TawksSdk {
    pub fn new(base_url: &str, auth: &str) -> Result<Self> {
        let web_client = Arc::new(WebClient::new(base_url, auth)?);
        Ok(Self {
            server_actions: ServerActionService::new(web_client.clone()),
            web_client,
        })
    }
}

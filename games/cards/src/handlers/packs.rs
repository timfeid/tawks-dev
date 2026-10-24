use anyhow::{Result, anyhow};
use chrono::Utc;
use game_framework::GameEffect;
use ulid::Ulid;

use crate::pack::{PackInstance, open_pack};
use crate::state::CardGameState;

pub trait PackHandlers {
    fn handle_list_pack_types(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_get_user_packs(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_open_pack(
        &mut self,
        player_id: &str,
        pack_instance_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_get_pack_opening_results(
        &mut self,
        player_id: &str,
        pack_instance_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_admin_grant_pack(
        &mut self,
        player_id: &str,
        target_user_id: String,
        pack_type_id: String,
    ) -> Result<Vec<GameEffect>>;
}

impl PackHandlers for CardGameState {
    fn handle_list_pack_types(&mut self, _player_id: &str) -> Result<Vec<GameEffect>> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        let pack_types = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(repo.list_pack_types())
        })?;

        self.pack_types_cache = pack_types;
        Ok(vec![])
    }

    fn handle_get_user_packs(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        let player_id = player_id.to_string();
        let packs = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(repo.get_user_packs(&player_id))
        })?;

        self.user_packs_cache.insert(player_id, packs);
        Ok(vec![])
    }

    fn handle_open_pack(
        &mut self,
        player_id: &str,
        pack_instance_id: String,
    ) -> Result<Vec<GameEffect>> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        let player_id = player_id.to_string();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(open_pack(
                &repo,
                &pack_instance_id,
                &player_id,
            ))
        })?;

        self.pack_open_result_cache.insert(player_id, Some(result));
        Ok(vec![])
    }

    fn handle_get_pack_opening_results(
        &mut self,
        player_id: &str,
        pack_instance_id: String,
    ) -> Result<Vec<GameEffect>> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        let results = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(repo.get_pack_opening_results(&pack_instance_id))
        })?;

        self.pack_opening_results_cache
            .insert(player_id.to_string(), results);
        Ok(vec![])
    }

    fn handle_admin_grant_pack(
        &mut self,
        _player_id: &str,
        target_user_id: String,
        pack_type_id: String,
    ) -> Result<Vec<GameEffect>> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        let pack_instance_id = Ulid::new().to_string();
        let pack = PackInstance {
            pack_instance_id: pack_instance_id.clone(),
            pack_type_id: pack_type_id.clone(),
            owner_id: target_user_id.clone(),
            source: "admin".to_string(),
            source_ref: None,
            opened: false,
            opened_at: None,
            created_at: Utc::now(),
        };

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                repo.create_pack_instance(&pack).await?;
                repo.create_pack_audit(
                    &pack_instance_id,
                    "granted",
                    &target_user_id,
                    Some(serde_json::json!({
                        "source": "admin",
                        "pack_type_id": pack_type_id,
                    })),
                )
                .await?;
                Ok::<_, anyhow::Error>(())
            })
        })?;

        Ok(vec![])
    }
}

//! Card management action handlers (save, list, search)

use crate::card::Card;
use crate::state::CardGameState;
use crate::types::CardGameStage;
use anyhow::Result;
use game_framework::GameEffect;
use tawks_data::pagination::PaginationArgs;
use tawks_data::repository::{CursorDirection, Repository};

pub trait CardHandlers {
    fn handle_save_card(&mut self, card: Card) -> Result<Vec<GameEffect>>;
    fn handle_list_cards(
        &mut self,
        player_id: &str,
        pagination: PaginationArgs,
    ) -> Result<Vec<GameEffect>>;
}

impl CardHandlers for CardGameState {
    fn handle_save_card(&mut self, card: Card) -> Result<Vec<GameEffect>> {
        if let Some(repo) = &self.repository {
            let repo = repo.clone();
            tokio::spawn(async move {
                if let Err(e) = repo.create_card(&card).await {
                    tracing::error!("Failed to save card: {:?}", e);
                }
            });
        }
        Ok(vec![])
    }

    fn handle_list_cards(
        &mut self,
        player_id: &str,
        pagination: PaginationArgs,
    ) -> Result<Vec<GameEffect>> {
        if self.stage == CardGameStage::DraftBuilding {
            return Ok(vec![]);
        }

        if let Some(repo) = &self.repository {
            let repo = repo.clone();
            let args = crate::repository::CardArgs {
                pagination,
                owner_id: None,
            };
            let result = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    repo.find(
                        None::<(CursorDirection, crate::repository::CardCursor)>,
                        args.pagination.first.unwrap_or(20),
                        &args,
                    )
                    .await
                })
            });
            match result {
                Ok(cards) => {
                    if let Some(player) = self.players.get_mut(player_id) {
                        player.library_search_results = cards;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to list cards: {:?}", e);
                }
            }
        }
        Ok(vec![])
    }
}

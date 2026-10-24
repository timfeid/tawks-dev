use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::repository::{CursorDirection, Repository};

#[derive(Type, Clone, Debug, Serialize, Deserialize, Default)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_prev_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
    pub total_count: i32,
}

pub trait Model<T> {
    fn id(&self) -> String;
    fn to_node(&self) -> T;
    /// Sort-key the row uses for cursor positioning. Used by paginators
    /// whose cursor is composite (e.g. (timestamp, id)) so the framework
    /// can construct an accurate cursor for THIS row, not "now".
    /// Defaults to empty for ID-only cursors that don't need it.
    fn cursor_sort_key(&self) -> String {
        String::new()
    }
}

pub trait Node {
    fn id(&self) -> String;
}

#[derive(Type, Clone, Serialize, Deserialize, Debug)]
pub struct Edge<T: Node> {
    pub cursor: String,
    pub node: T,
}

#[derive(Type, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ListResult<T: Node, M> {
    pub page_info: PageInfo,
    pub edges: Vec<Edge<T>>,
    pub meta: M,
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct PaginationArgs {
    pub before: Option<String>,
    pub after: Option<String>,
    pub first: Option<i32>,
    pub last: Option<i32>,
}

impl PaginationArgs {
    pub fn default_take() -> i32 {
        9
    }
}

pub trait Cursor:
    Serialize + for<'de> Deserialize<'de> + Display + Clone + PartialEq + Debug
{
    type CursorType: Cursor;
    fn sort_key(&self) -> String;
    fn id(&self) -> String;
    fn decode(encoded: &str) -> Option<Self::CursorType>;
    fn encode(cursor: &Self::CursorType) -> String;
}

pub trait WithPagination {
    type Meta: Default;
    type CursorType: Cursor;
    fn pagination(&self) -> PaginationArgs;
    fn get_meta(&self) -> Self::Meta;
    /// Build a cursor for a row given its id AND the row's cursor sort key
    /// (e.g. an rfc3339 timestamp). ID-only cursor impls can ignore the
    /// sort_key parameter.
    fn to_cursor(&self, id: String, sort_key: String) -> Self::CursorType;
}

pub async fn connection_from_repository<T, U, Pagination, R, M, CursorType>(
    args: &Pagination,
    repository: Arc<R>,
) -> Result<ListResult<U, M>>
where
    T: Model<U>,
    U: Node,
    CursorType: Cursor<CursorType = CursorType> + Send,
    R: Repository<T, Pagination> + Sync,
    Pagination: WithPagination<CursorType = CursorType, Meta = M>,
    M: Default,
{
    let pagination_args = args.pagination();

    let mut cursor_details: Option<(CursorDirection, CursorType)> = None;
    if let Some(ref cursor) = pagination_args.after {
        match Pagination::CursorType::decode(cursor) {
            Some(decoded) => {
                cursor_details = Some((CursorDirection::After, decoded));
            }
            None => {
                return Err(anyhow::anyhow!("failed to decode `after` cursor: {cursor}"));
            }
        }
    } else if let Some(ref cursor) = pagination_args.before {
        match Pagination::CursorType::decode(cursor) {
            Some(decoded) => {
                cursor_details = Some((CursorDirection::Before, decoded));
            }
            None => {
                return Err(anyhow::anyhow!(
                    "failed to decode `before` cursor: {cursor}"
                ));
            }
        }
    }

    let take = pagination_args
        .first
        .or(pagination_args.last)
        .unwrap_or(PaginationArgs::default_take());

    // IMPORTANT: decide direction *before* trimming
    let paging_forward = pagination_args.after.is_some() || pagination_args.before.is_none();

    // Fetch take + 1 so we can determine if there's more
    let mut entities = repository
        .find(cursor_details.clone(), take + 1, args)
        .await?;

    let more_in_this_direction = entities.len() as i32 > take;
    if more_in_this_direction {
        // Your chat repo returns ASC for forward paging (None/Before).
        // That means the extra row is the OLDEST, at index 0.
        if paging_forward {
            entities.remove(0);
        } else {
            // Backward paging typically returns DESC/whatever you decided;
            // for safety keep pop() here.
            entities.pop();
        }
    }

    let edges: Vec<Edge<U>> = entities
        .into_iter()
        .map(|node| {
            let id = node.id();
            let sort_key = node.cursor_sort_key();
            let cursor = args.to_cursor(id, sort_key);
            let node = node.to_node();
            Edge {
                cursor: Pagination::CursorType::encode(&cursor),
                node,
            }
        })
        .collect();

    let start_cursor = edges.first().map(|e| e.cursor.clone());
    let end_cursor = edges.last().map(|e| e.cursor.clone());

    let (has_next_page, has_prev_page) = if paging_forward {
        (more_in_this_direction, pagination_args.after.is_some())
    } else {
        (pagination_args.before.is_some(), more_in_this_direction)
    };

    let page_info = PageInfo {
        start_cursor,
        end_cursor,
        has_prev_page,
        has_next_page,
        total_count: repository.count(args).await? as i32,
    };

    Ok(ListResult {
        page_info,
        edges,
        meta: args.get_meta(),
    })
}

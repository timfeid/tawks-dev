use anyhow::Result;

use crate::pagination::Cursor;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CursorDirection {
    Before,
    After,
}

pub trait Repository<T, Y> {
    fn count(&self, args: &Y) -> impl std::future::Future<Output = Result<i32>> + Send;
    fn find(
        &self,
        after: Option<(CursorDirection, impl Cursor + Send)>,
        take: i32,
        args: &Y,
    ) -> impl std::future::Future<Output = Result<Vec<T>>> + Send;

    /// Whether `find` returns a `Before` page oldest first, which puts the
    /// extra row fetched past the page at the front instead of the end.
    fn before_pages_ascending(&self) -> bool {
        false
    }
}

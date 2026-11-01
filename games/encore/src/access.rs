//! Who may see and play the songs in the server's own folder. That folder is
//! the owner's private collection, so only the accounts listed in
//! `ENCORE_LIBRARY_USERS` reach it; an empty list means nobody does.

use std::collections::HashSet;

use party::auth::Claims;

#[derive(Clone, Debug, Default)]
pub struct LibraryAccess {
    users: HashSet<String>,
}

impl LibraryAccess {
    pub fn from_env() -> Self {
        Self::parse(&std::env::var("ENCORE_LIBRARY_USERS").unwrap_or_default())
    }

    /// User ids separated by commas or whitespace.
    pub fn parse(list: &str) -> Self {
        LibraryAccess {
            users: list
                .split(|c: char| c == ',' || c.is_whitespace())
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(String::from)
                .collect(),
        }
    }

    pub fn allows(&self, claims: &Claims) -> bool {
        !claims.guest && !claims.spectator && self.users.contains(&claims.user_id)
    }

    pub fn len(&self) -> usize {
        self.users.len()
    }

    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn claims(user_id: &str, guest: bool) -> Claims {
        Claims {
            match_id: "L1".into(),
            user_id: user_id.into(),
            name: user_id.into(),
            avatar_url: String::new(),
            exp: i64::MAX,
            spectator: false,
            guest,
            drives: Vec::new(),
            game: Some("encore".into()),
            owner: false,
        }
    }

    #[test]
    fn only_listed_accounts_reach_the_library() {
        assert!(!LibraryAccess::default().allows(&claims("owner", false)));
        assert!(!LibraryAccess::parse("").allows(&claims("owner", false)));
        let access = LibraryAccess::parse(" owner, friend\nother ");
        assert_eq!(access.len(), 3);
        assert!(access.allows(&claims("owner", false)));
        assert!(access.allows(&claims("friend", false)));
        assert!(!access.allows(&claims("stranger", false)));
        assert!(!access.allows(&claims("owner", true)));
        assert!(!access.allows(&claims("own", false)));
    }
}

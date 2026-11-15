//! The MCP server's bearer token, kept between starts (ADR-017 Amendment 1,
//! UX-26): made once and kept in the OS's protected store (Windows
//! Credential Manager, the macOS Keychain, the Linux kernel keyring), never
//! in a plain file. **Rotate** makes a new one. Where no protected store
//! works, the token is made at each start, as before the amendment.

use std::io;

/// Where a token is kept.
pub trait TokenStore: Send + Sync {
    /// The kept token, if there is one.
    fn load(&self) -> Option<String>;
    /// Keeps `token`, replacing the last; whether it was kept.
    fn save(&self, token: &str) -> bool;
}

/// The store's service name in the OS's credential list.
pub const SERVICE: &str = "Exiled Studio MCP server";
const ACCOUNT: &str = "bearer-token";

/// The OS's protected store.
#[derive(Debug)]
pub struct OsStore {
    entry: Option<keyring::Entry>,
}

impl OsStore {
    /// The OS store; one that cannot be opened keeps nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::named(SERVICE)
    }

    fn named(service: &str) -> Self {
        Self {
            entry: keyring::Entry::new(service, ACCOUNT).ok(),
        }
    }
}

impl Default for OsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStore for OsStore {
    fn load(&self) -> Option<String> {
        self.entry.as_ref()?.get_password().ok()
    }

    fn save(&self, token: &str) -> bool {
        self.entry
            .as_ref()
            .is_some_and(|e| e.set_password(token).is_ok())
    }
}

/// A store that keeps nothing: the token is per session.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoStore;

impl TokenStore for NoStore {
    fn load(&self) -> Option<String> {
        None
    }

    fn save(&self, _token: &str) -> bool {
        false
    }
}

/// Whether `token` is one this module makes: 64 lowercase hex digits.
#[must_use]
pub fn valid(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A new token: 32 random bytes as hex.
///
/// # Errors
/// The random source's.
pub fn new_token() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// The token to start with, and whether it is kept between starts: the
/// kept one when it is valid, else a new one, kept if the store can.
///
/// # Errors
/// The random source's.
pub fn kept_token(store: &dyn TokenStore) -> io::Result<(String, bool)> {
    if let Some(token) = store.load().filter(|t| valid(t)) {
        return Ok((token, true));
    }
    rotate(store)
}

/// A new token, kept if the store can (Rotate).
///
/// # Errors
/// The random source's.
pub fn rotate(store: &dyn TokenStore) -> io::Result<(String, bool)> {
    let token = new_token()?;
    let kept = store.save(&token);
    Ok((token, kept))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Memory(Mutex<Option<String>>);

    impl TokenStore for Memory {
        fn load(&self) -> Option<String> {
            self.0.lock().ok()?.clone()
        }
        fn save(&self, token: &str) -> bool {
            self.0
                .lock()
                .map(|mut t| *t = Some(token.to_owned()))
                .is_ok()
        }
    }

    #[test]
    fn a_kept_token_should_be_reused_and_rotate_should_replace_it() {
        let store = Memory::default();
        let (first, kept) = kept_token(&store).unwrap_or_default();
        assert!(kept && valid(&first));
        assert_eq!(
            kept_token(&store).unwrap_or_default(),
            (first.clone(), true)
        );
        let (second, kept) = rotate(&store).unwrap_or_default();
        assert!(kept && second != first);
        assert_eq!(kept_token(&store).unwrap_or_default().0, second);
    }

    #[test]
    fn a_store_that_keeps_nothing_or_holds_junk_should_give_a_fresh_token() {
        let (a, kept) = kept_token(&NoStore).unwrap_or_default();
        let (b, _) = kept_token(&NoStore).unwrap_or_default();
        assert!(!kept && valid(&a) && a != b);
        let store = Memory(Mutex::new(Some("not a token".to_owned())));
        let (c, kept) = kept_token(&store).unwrap_or_default();
        assert!(kept && valid(&c));
    }

    /// The real OS store, under a credential of its own that it removes.
    /// Ignored: it writes to the user's credential store; run by hand.
    #[test]
    #[ignore = "writes to the OS credential store"]
    fn the_os_store_should_keep_a_token_between_handles() {
        let service = "Exiled Studio MCP server (test)";
        let (token, kept) = kept_token(&OsStore::named(service)).unwrap_or_default();
        assert!(kept, "the OS store keeps the token");
        assert_eq!(OsStore::named(service).load(), Some(token));
        if let Some(entry) = OsStore::named(service).entry {
            let _ = entry.delete_credential();
        }
        assert_eq!(OsStore::named(service).load(), None);
    }

    #[test]
    fn valid_should_take_only_what_new_token_makes() {
        assert!(valid(&new_token().unwrap_or_default()));
        assert!(!valid(""));
        assert!(!valid(&"A".repeat(64)));
        assert!(!valid(&"a".repeat(63)));
    }
}

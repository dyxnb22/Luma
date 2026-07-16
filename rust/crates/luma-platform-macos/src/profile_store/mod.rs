//! Luma-owned Profile storage and the read-only Clash Verge manifest adapter.
//!
//! This adapter is deliberately conservative: it validates YAML before persistence, keeps
//! subscription URLs in Keychain, stores only opaque IDs in JSON, and only ever writes local
//! Profiles marked as Luma-owned.

mod clash;
mod fetch;
mod fs;
mod parse;
mod store;

use std::path::PathBuf;

pub(super) const MAX_PROFILE_BYTES: u64 = 512 * 1024;
pub(super) const MAX_REDIRECTS: &str = "3";
pub(super) const URL_ACCOUNT_PREFIX: &str = "proxy-profile-url:";

pub(super) fn default_clash_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join("Library/Application Support/io.github.clash-verge-rev.clash-verge-rev")
    })
}

pub use store::MacProfileStore;

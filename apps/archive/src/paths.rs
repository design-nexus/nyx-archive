//! Well-known locations. Every path honours the XDG overrides so the whole app
//! can be pointed at a scratch copy of `~/.config` for testing.

pub use nyx_core::paths::*;

use std::path::PathBuf;

/// Working copies of files opened from inside an archive.
pub fn open_dir() -> PathBuf {
    cache_dir().join("open")
}


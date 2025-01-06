use crate::env;
use std::path::PathBuf;

#[must_use]
pub fn cargo_home() -> Option<PathBuf> {
    if let Ok(cargo_home) = env::var(env::CARGO_HOME) {
        Some(PathBuf::from(cargo_home))
    } else {
        home_dir().map(|path| path.join(".cargo"))
    }
}

// smoelius: https://github.com/rust-lang/cargo/commit/014378f8c07bec557b6c6608e5baf0761a12d504
#[rustversion::before(1.86)]
pub use home::home_dir;

#[rustversion::since(1.86)]
pub fn home_dir() -> Option<PathBuf> {
    // smoelius: The `deprecated` attribute hasn't been removed yet:
    // https://github.com/rust-lang/rust/pull/132515#discussion_r1829715262
    #[expect(deprecated)]
    std::env::home_dir()
}

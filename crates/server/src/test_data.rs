//! Real data files tests read. A missing file fails the test instead of letting it pass
//! without testing anything; a test that may run without the data is `#[ignore]`d.

use std::path::{Path, PathBuf};

/// `data/world.db` of this checkout.
pub(crate) fn world_db_path() -> PathBuf {
    require(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"))
}

/// `path`, panicking with how to provide it when it does not exist.
pub(crate) fn require(path: PathBuf) -> PathBuf {
    assert!(
        path.exists(),
        "{} not found: tests need the real data files. In a worktree, link the canonical \
         data/ with `python3 /syncthing/Sync/Projects/world-of-osso/game-engine/scripts/agent/\
         link-worktree-data.py <canonical checkout> <worktree>` (or symlink data/ to the \
         canonical checkout's data/).",
        path.display()
    );
    path
}

//! Required immutable world input; canonical manifest depth remains unchanged.
use std::path::{Path, PathBuf};

pub fn world_db_path() -> PathBuf {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB")
        .or_else(|| std::env::var_os("FOLLOWER_DPS_WORLD_DB"))
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"));
    require(path)
}

pub fn require(path: PathBuf) -> PathBuf {
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

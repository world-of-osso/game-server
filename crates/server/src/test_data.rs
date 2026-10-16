//! Real data files tests read. A missing file fails the test instead of letting it pass
//! without testing anything; a test that may run without the data is `#[ignore]`d.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Immutable canonical input, parsed once per test process. Apps clone before editing.
/// Prepared/mutable database copies must use their own loaders, never this cache.
pub(crate) fn world_data() -> &'static crate::world_data::WorldData {
    static DATA: OnceLock<crate::world_data::WorldData> = OnceLock::new();
    DATA.get_or_init(|| crate::world_data::load_world_data(&world_db_path()))
}

pub(crate) fn achievement_catalog() -> crate::achievement::AchievementCatalog {
    static DATA: OnceLock<crate::achievement::AchievementCatalog> = OnceLock::new();
    DATA.get_or_init(|| {
        let conn = rusqlite::Connection::open_with_flags(
            world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("read-only achievement fixture input");
        crate::achievement::load_achievement_catalog(&conn).expect("native achievement fixture")
    })
    .clone()
}

#[test]
fn native_fixture_copies_do_not_share_mutable_catalogs_or_creatures() {
    let mut local = world_data().clone();
    let imp_name = local.templates[&416].name.clone();
    local.templates.get_mut(&416).unwrap().name = "Changed in one app".into();
    assert_eq!(world_data().templates[&416].name, imp_name);
    let mut local = achievement_catalog();
    assert!(local.trees.contains_key(&15057));
    local.trees.clear();
    assert!(achievement_catalog().trees.contains_key(&15057));
    let source = crate::spell_cast::test_harness::world_db().unwrap();
    let mut classes = source.classes.clone();
    let spell_count = source.classes.classes[0].learnable.len();
    assert!(spell_count > 0);
    classes.classes[0].learnable.clear();
    assert_eq!(source.classes.classes[0].learnable.len(), spell_count);
}

#[test]
fn prepared_class_fixture_remains_read_only_and_app_mutations_are_isolated() {
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let conn = rusqlite::Connection::open_with_flags(
        prepared.path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert!(
        conn.execute("CREATE TABLE forbidden_fixture_mutation (id INTEGER)", [])
            .is_err()
    );
    let mut local = prepared.world_data();
    let original = local.templates[&17252].name.clone();
    local.templates.clear();
    drop(prepared);
    let next = crate::spell_cast::test_harness::prepare_class_world();
    assert_eq!(next.world_data().templates[&17252].name, original);
}

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

//! Real data files tests read. A missing file fails the test instead of letting it pass
//! without testing anything; a test that may run without the data is `#[ignore]`d.

#[cfg(test)]
pub(crate) use server_map_rules::test_data::require;
pub use server_map_rules::test_data::world_db_path;
use std::path::PathBuf;
use std::sync::OnceLock;

pub(crate) mod catalog_cache;

/// Advance the real selection/admission path until its player is published.
#[cfg(test)]
pub(crate) fn wait_for_selected_player(
    app: &mut bevy::prelude::App,
    connection: bevy::prelude::Entity,
) -> bevy::prelude::Entity {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(link) = app.world().get::<crate::networking::PlayerLink>(connection) {
            let player = link.0;
            app.update();
            return player;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "selection never published its player"
        );
        app.update();
        drain_persistence(app.world().resource::<crate::persistence::GameDatabase>());
    }
}

/// Wait for a real FIFO fence before asserting durable fixture rows.
#[cfg(test)]
pub(crate) fn drain_persistence(db: &crate::persistence::GameDatabase) {
    let receipt = db.writer.enqueue("fixture durability fence", |_| Ok(()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(result) = receipt.result() {
            result.expect("fixture persistence fence failed");
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fixture persistence fence timed out"
        );
        std::thread::yield_now();
    }
}

/// Consume a committed operation's queued credits, then fence its durable writes.
#[cfg(test)]
pub(crate) fn advance_and_drain_persistence(app: &mut bevy::prelude::App) {
    app.update();
    drain_persistence(app.world().resource::<crate::persistence::GameDatabase>());
}

/// Immutable canonical input, parsed once per test process. Apps clone before editing.
/// Prepared/mutable database copies must use their own loaders, never this cache.
pub fn world_data() -> &'static crate::world_data::WorldData {
    static DATA: OnceLock<crate::world_data::WorldData> = OnceLock::new();
    DATA.get_or_init(|| crate::world_data::load_world_data(&world_db_path()))
}

#[cfg(test)]
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
fn native_selected_creatures_keep_source_stats_and_isolate_live_apps() {
    use crate::spell_cast::test_harness::load_native_creature_world;
    use bevy::prelude::App;

    let entries = [40, 209057, 209072];
    let mut first = App::new();
    let mut sibling = App::new();
    first.insert_resource(load_native_creature_world(&entries));
    sibling.insert_resource(load_native_creature_world(&entries));
    let source = world_data();
    let original = &source.templates[&40];
    {
        let mut local = first
            .world_mut()
            .resource_mut::<crate::world_data::WorldData>();
        assert_eq!(local.templates.len(), entries.len());
        assert!(local.spawns.is_empty());
        assert!(local.difficulty_templates.is_empty());
        let template = local.templates.get_mut(&40).unwrap();
        assert_eq!(template.name, original.name);
        assert_eq!(template.armor, original.armor);
        assert_eq!(template.models.len(), original.models.len());
        assert_eq!(
            template.tuning.as_ref().unwrap().stats,
            original.tuning.as_ref().unwrap().stats
        );
        template.name = "first app only".into();
        template.models.clear();
        template.tuning = None;
        local.templates.remove(&209072);
        local.expected_stats.by_level.clear();
    }
    let other = sibling.world().resource::<crate::world_data::WorldData>();
    assert_eq!(other.templates[&40].name, original.name);
    assert_eq!(other.templates[&40].models.len(), original.models.len());
    assert!(other.templates[&40].tuning.is_some());
    assert!(other.templates.contains_key(&209072));
    assert_eq!(
        other.expected_stats.by_level,
        source.expected_stats.by_level
    );
    assert_eq!(world_data().templates[&40].name, original.name);
    assert!(world_data().templates[&40].tuning.is_some());
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

/// Independent lazy terrain cache backed by the same read-only source maps as the fixture.
#[cfg(test)]
pub(crate) fn ground_map() -> crate::ground::GroundMap {
    let connection = rusqlite::Connection::open_with_flags(
        world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read-only terrain map metadata");
    let store = crate::maps::store::load_map_store(&connection).expect("source map metadata");
    let wdts = store
        .maps
        .into_iter()
        .map(|(id, map)| (id, map.wdt_file_data_id))
        .collect();
    crate::ground::GroundMap::new(&crate::ground::ground_data_dir(), wdts)
}

/// Install real source geometry for transport-free fixtures without exposing core types.
#[cfg(any(test, feature = "test-support"))]
pub fn install_native_collision(app: &mut bevy::prelude::App) -> Result<(), String> {
    let assets = std::env::var_os("GAME_SERVER_GROUND_DIR")
        .map(PathBuf::from)
        .ok_or("native collision fixture requires GAME_SERVER_GROUND_DIR")?;
    let connection = rusqlite::Connection::open_with_flags(
        world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| format!("open native geometry map metadata: {error}"))?;
    let maps = crate::maps::store::load_map_store(&connection)
        .map_err(|error| format!("load native geometry map metadata: {error}"))?;
    let wdts = maps
        .maps
        .into_iter()
        .map(|(id, map)| (id, map.wdt_file_data_id))
        .collect();
    let los = crate::line_of_sight::LineOfSight::load(&assets.join("los"))?;
    app.insert_resource(crate::ground::GroundMap::new(&assets, wdts));
    app.insert_resource(los);
    Ok(())
}

/// Full idle population across diverse layouts, as in the live world's optional components.
#[cfg(test)]
#[derive(bevy::prelude::Component)]
struct IdleLayout<const BIT: u8>;

#[cfg(test)]
pub(crate) fn populate_idle_world<B: bevy::prelude::Bundle>(
    world: &mut bevy::prelude::World,
    mut bundle: impl FnMut() -> B,
) {
    for index in 0..250_000_u32 {
        let mut unit = world.spawn(bundle());
        macro_rules! layout {
            ($($bit:literal),*) => { $(
                if index & (1 << $bit) != 0 { unit.insert(IdleLayout::<$bit>); }
            )* };
        }
        layout!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
    }
}

#[cfg(test)]
pub(crate) fn assert_idle_updates_are_bounded(app: &mut bevy::prelude::App, label: &str) {
    app.update();
    let fastest = (0..3)
        .map(|_| {
            let start = std::time::Instant::now();
            for _ in 0..5 {
                app.update();
            }
            start.elapsed()
        })
        .min()
        .unwrap();
    eprintln!("{label}: 250000 units/4096 layouts/five updates {fastest:?}");
    assert!(
        fastest < std::time::Duration::from_micros(100),
        "idle {label} still depends on full world population/layouts: {fastest:?}"
    );
}

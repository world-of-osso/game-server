//! An item's `proc.any_dps.default_value` from its native item effects.
use rusqlite::{Connection, OpenFlags};
use server_combat::item_instance::effects::equipped_item_effects;
use server_combat::spell_cast::test_harness::world_db;
use server_follower_apl::literal_rotation::equipment::metadata::Metadata;

/// The item's equipped native ItemEffect rows, as follower equipment reads them.
fn metadata(item: u32) -> Metadata {
    let path = server_combat::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tables = server_combat::item_instance::load::load_item_effect_tables(&db).unwrap();
    let effects = equipped_item_effects(item, &[], &tables).unwrap();
    Metadata::load(world_db().unwrap().spell_store(), &effects).unwrap()
}

/// Treemouth's Festering Splinter (193652, driver395175) only absorbs: SimC's
/// any_dps expression has no stat buff to read, so it is 0 despite its set default.
#[test]
fn absorb_only_item_proc_default_value_is_zero() {
    assert_eq!(metadata(193652).proc_default_value(), Ok(0.0));
}

/// Follower Journal stat procs whose SimC callbacks never set a default read
/// the unset value, 0: Ara-Kara Sacbrood, Improvised Seaforium Pacemaker,
/// Idol of the War Loa, Emberwing Feather.
#[test]
fn stat_item_proc_default_value_without_a_simc_default_is_zero() {
    for item in [219314, 232541, 250229, 250144] {
        let metadata = metadata(item);
        assert!(
            !metadata.stats.is_empty(),
            "item{item} grants no any_dps stat"
        );
        assert_eq!(metadata.proc_default_value(), Ok(0.0), "item{item}");
    }
}

/// Blazebinder's Hoof (193762, driver383926) sets its stat buff's default in
/// SimC; native does not compute it, so it stays an explicit gap.
#[test]
fn stat_item_proc_default_value_set_by_simc_is_explicit() {
    let metadata = metadata(193762);
    assert!(!metadata.stats.is_empty());
    let error = metadata.proc_default_value().unwrap_err();
    assert!(
        error.contains("NATIVE_ITEM_PROC_DEFAULT_VALUE_UNSOURCED"),
        "{error}"
    );
}

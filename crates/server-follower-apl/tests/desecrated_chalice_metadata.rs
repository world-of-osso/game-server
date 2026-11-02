//! The real Desecrated Chalice item has a cyclic DB2 spell-trigger graph.
use rusqlite::{Connection, OpenFlags};
use server_combat::item_instance::effects::equipped_item_effects;
use server_combat::spell_cast::test_harness::world_db;
use server_follower_apl::literal_rotation::equipment::metadata::Metadata;

#[test]
fn desecrated_chalice_cyclic_trigger_graph_loads() {
    let path = server_combat::test_data::world_db_path();
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tables = server_combat::item_instance::load::load_item_effect_tables(&db).unwrap();
    let effects = equipped_item_effects(251790, &[], &tables).unwrap();
    assert!(effects.iter().any(|effect| effect.spell_id == 1253118));

    // DB2 70448: 1253118 -> 1265323 -> 1265325 -> 1265323.
    let metadata = Metadata::load(world_db().unwrap().spell_store(), &effects).unwrap();
    assert!(metadata.stats.is_empty());
    assert!(metadata.buff_spells.is_empty());
    assert_eq!(metadata.proc_default_value(), Ok(0.0));
}

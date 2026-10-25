//! Explicit read-only producer boundary: use the existing native starter-build
//! validator, not a second implementation of talent points/conditions/curves.
use std::collections::BTreeMap;

use crate::spell_info::load_class_spells;

#[test]
#[ignore = "producer: set FOLLOWER_KIT_SOURCE_OUTPUT to an unused JSON path"]
fn follower_kit_export_native_source() {
    let output = std::env::var("FOLLOWER_KIT_SOURCE_OUTPUT").expect("explicit source output path");
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let profiles = super::catalog::load_catalog(&db).unwrap().profiles;
    let classes = load_class_spells(&db).unwrap();
    let mut loadouts = Vec::new();
    for profile in profiles {
        for level in 1..=90 {
            let (spells, traits) = crate::class_progression::evaluate_npc_starter(
                &classes,
                profile.class_id,
                profile.spec_id,
                level,
            )
            .unwrap();
            let known: BTreeMap<_, _> = spells
                .iter()
                .map(|spell| (spell.spell_id, spell.passive))
                .collect();
            let (config, budget) = traits.source_config();
            assert!(budget.iter().all(|(_, points)| *points >= 0));
            loadouts.push(serde_json::json!({
                "entry": profile.entry, "spec_id": profile.spec_id, "class_id": profile.class_id,
                "level": level, "known": known, "unspent": budget,
                "config": config.iter().map(|entry| serde_json::json!({
                    "node_id":entry.node_id,"entry_id":entry.entry_id,"ranks":entry.ranks,"granted":entry.granted
                })).collect::<Vec<_>>(),
            }));
        }
    }
    assert_eq!(loadouts.len(), 11 * 90);
    let trees: Vec<_> = classes
        .classes
        .iter()
        .filter(|class| loadouts.iter().any(|row| row["class_id"] == class.class_id))
        .map(|class| serde_json::json!({"class_id":class.class_id,"tree":class.traits}))
        .collect();
    let document = serde_json::json!({"world_db":crate::test_data::world_db_path(),"loadouts":loadouts,"trees":trees});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    serde_json::to_writer_pretty(&mut file, &document).unwrap();
}

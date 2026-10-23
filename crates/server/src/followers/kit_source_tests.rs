//! Explicit read-only producer boundary: use the existing native starter-build
//! validator, not a second implementation of talent points/conditions/curves.
use std::collections::BTreeMap;

use crate::spell_info::load_class_spells;
use crate::trait_config::{
    TraitContext, build_config, starter_purchases, talent_spell_entries, unspent,
};

fn read_source_profiles(
    db: &rusqlite::Connection,
    classes: &crate::spell_info::ClassSpells,
) -> Vec<super::Profile> {
    let mut world = bevy::prelude::World::new();
    world.insert_resource(super::catalog::load_catalog(db).unwrap());
    world.insert_resource(classes.clone());
    let mut profiles: Vec<_> = world
        .resource::<super::FollowerCatalog>()
        .profiles
        .iter()
        .cloned()
        .collect();
    for spec in [65, 256, 264, 270, 1468] {
        profiles.push(super::simulator::healer_profiles::configure(&world, spec).unwrap());
    }
    profiles
}

#[test]
#[ignore = "producer: set FOLLOWER_KIT_SOURCE_OUTPUT to an unused JSON path"]
fn follower_kit_export_native_source() {
    let output = std::env::var("FOLLOWER_KIT_SOURCE_OUTPUT").expect("explicit source output path");
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let classes = load_class_spells(&db).unwrap();
    let profiles = read_source_profiles(&db, &classes);
    let mut loadouts = Vec::new();
    for profile in &profiles {
        let class = classes
            .classes
            .iter()
            .find(|c| c.class_id == profile.class_id)
            .unwrap();
        let spec = class
            .specs
            .iter()
            .find(|s| s.spec_id == profile.spec_id)
            .unwrap();
        for level in 1..=90 {
            let context = TraitContext {
                spec_id: profile.spec_id,
                level,
            };
            let purchases = starter_purchases(&class.traits, context).unwrap();
            let config = build_config(&class.traits, context, &purchases).unwrap();
            let talents: Vec<_> = talent_spell_entries(&class.traits, &config).collect();
            let mut known = BTreeMap::new();
            for spell in &class.learnable {
                if [1, 2, 4].contains(&spell.acquire_method)
                    && spell.required_level <= u32::from(level)
                    && spell.race_masks == [0, 0]
                {
                    known.insert(spell.spell_id, spell.passive);
                }
            }
            for spell in crate::class_progression::default_skill_spells(class, 0, level) {
                known.insert(spell.spell_id, spell.passive);
            }
            for spell in &spec.spells {
                if spell.required_level <= u32::from(level) {
                    known.insert(spell.spell_id, spell.passive);
                }
            }
            for entry in &talents {
                known.insert(entry.spell_id, entry.passive);
            }
            let overrides = talents
                .iter()
                .filter(|entry| entry.overrides_spell_id != 0)
                .map(|entry| entry.overrides_spell_id)
                .chain(
                    spec.spells
                        .iter()
                        .filter(|s| {
                            s.overrides_spell_id != 0 && s.required_level <= u32::from(level)
                        })
                        .map(|s| s.overrides_spell_id),
                );
            for overridden in overrides {
                known.remove(&overridden);
            }
            let budget = unspent(&class.traits, context, &config);
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
    assert_eq!(loadouts.len(), profiles.len() * 90);
    let trees: Vec<_> = classes
        .classes
        .iter()
        .filter(|class| loadouts.iter().any(|row| row["class_id"] == class.class_id))
        .map(|class| serde_json::json!({"class_id":class.class_id,"tree":class.traits}))
        .collect();
    let source_profiles: Vec<_> = profiles
        .iter()
        .map(|profile| {
            let alternate = super::simulator::healer_profiles::alternate_source(profile.spec_id);
            let source_spec = match alternate {
                Some((_, source_spec)) => source_spec,
                None => profile.spec_id,
            };
            serde_json::json!({
                "entry":profile.entry,"class_id":profile.class_id,"spec_id":profile.spec_id,
                "source_spec_id":source_spec,
                "bridge":alternate.map(|_| super::simulator::healer_profiles::ALTERNATE_SPEC_BRIDGE),
            })
        })
        .collect();
    let document = serde_json::json!({
        "world_db":crate::test_data::world_db_path(),"profiles":source_profiles,
        "loadouts":loadouts,"trees":trees,
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    serde_json::to_writer_pretty(&mut file, &document).unwrap();
}

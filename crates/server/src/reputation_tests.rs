use super::*;
use crate::world_data::{CreatureTemplate, WorldData};
use shared::reputation::Standing;
use std::collections::HashMap;

fn test_world_data(template: CreatureTemplate) -> WorldData {
    let mut templates = HashMap::new();
    templates.insert(template.entry, template);
    WorldData {
        templates,
        spawns: Vec::new(),
        spawns_by_map: HashMap::new(),
        waypoints: HashMap::new(),
        ..Default::default()
    }
}

fn test_template(entry: u32, faction: u16, min_level: u8, max_level: u8) -> CreatureTemplate {
    CreatureTemplate {
        entry,
        name: "Fixture wolf".into(),
        subname: String::new(),
        creature_type: 1,
        family: 0,
        type_flags: 0,
        flags_extra: 0,
        min_level,
        max_level,
        faction,
        npc_flags: 0,
        speed_walk: 1.0,
        speed_run: 1.0,
        health_modifier: 1.0,
        unit_class: 1,
        expansion: 0,
        rank: 0,
        experience_modifier: 1.0,
        gives_experience: true,
        script_name: String::new(),
        detection_range: 20.0,
        aggro_mode: crate::creature_aggro::AggroMode::Aggressive,
        trigger: false,
        unit_flags: 0,
        models: Vec::new(),
        tuning: None,
        armor: 0.0,
        armor_modifier: 1.0,
        health_scaling_expansion: 0,
        mana_modifier: 1.0,
        regenerates_power: false,
        regenerates_health: true,
    }
}

#[test]
fn npc_kill_updates_primary_and_spillover_reputation() {
    let mut registry = default_faction_registry();
    // Explicit test policy: the authentic registry does not invent city spillover.
    let mut stormwind_policy = registry.get(72).unwrap().clone();
    stormwind_policy.spillover = vec![shared::reputation::SpilloverRule {
        faction_id: 47,
        rate: 0.25,
    }];
    registry.add(stormwind_policy);
    let world_data = test_world_data(test_template(1001, 72, 5, 5));
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    // Pinned race-class baselines: Human Stormwind/Ironforge/Orgrimmar start values.
    let [stormwind, ironforge, orgrimmar] = [72, 47, 76].map(|id| reputation.get_value(id));

    let message = apply_npc_kill_reputation(&mut reputation, 5, 1001, &world_data, &registry)
        .expect("kill should award reputation");

    assert_eq!(reputation.get_value(72), stormwind + 10);
    assert_eq!(reputation.get_value(47), ironforge + 2);
    assert_eq!(reputation.get_value(76), orgrimmar);
    assert!(message.contains("Stormwind"));
    assert!(message.contains("spillover"));
}

#[test]
fn grey_kill_grants_no_reputation() {
    let registry = default_faction_registry();
    let world_data = test_world_data(test_template(1001, 72, 5, 5));
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    let stormwind = reputation.get_value(72);

    let message = apply_npc_kill_reputation(&mut reputation, 20, 1001, &world_data, &registry);

    assert!(message.is_none());
    assert_eq!(reputation.get_value(72), stormwind);
}

#[test]
fn snapshot_lists_known_factions_with_standing_text() {
    let registry = default_faction_registry();
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    reputation.set_rep(72, 30_000);

    let snapshot = build_reputation_snapshot(&reputation, &registry);

    assert_eq!(snapshot.entries.len(), registry.iter().count());
    let stormwind = snapshot
        .entries
        .iter()
        .find(|entry| entry.faction_id == 72)
        .expect("stormwind entry");
    assert_eq!(stormwind.standing, format!("{:?}", Standing::Revered));
    assert_eq!(stormwind.value, 30_000);
}

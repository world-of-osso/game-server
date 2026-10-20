//! Completed producer profiles remain independent of mutable live app resources.
use super::*;

#[test]
fn prepared_encounter_profiles_and_live_difficulties_keep_app_edits_local() {
    let (mut normal, normal_map, _, _) = herald_intro2::prepared_ramparts(1);
    let (omor, _, omor_map) =
        encounter_fixture(17308, 543, 1, 2, None, EncounterState::NotStarted, false);
    let (heroic, heroic_map, _, _) = herald_intro2::prepared_ramparts(2);
    assert_eq!(normal_map.map_id, 543);
    assert_eq!(omor_map.map_id, 543);
    assert_eq!(heroic_map.map_id, 543);
    let original = normal.app.world().resource::<WorldData>().templates[&17536]
        .name
        .clone();
    assert!(
        normal
            .app
            .world()
            .resource::<SpellStore>()
            .get(30717)
            .is_some()
    );
    assert!(
        heroic
            .app
            .world()
            .resource::<SpellStore>()
            .get(30717)
            .is_some()
    );
    let omor_name = omor.app.world().resource::<WorldData>().templates[&17308]
        .name
        .clone();
    assert!(
        omor.app
            .world()
            .resource::<WorldData>()
            .templates
            .contains_key(&17540)
    );
    assert!(
        !omor
            .app
            .world()
            .resource::<WorldData>()
            .templates
            .contains_key(&17536)
    );
    {
        let mut local = normal.app.world_mut().resource_mut::<WorldData>();
        let nazan = local.templates.get_mut(&17536).unwrap();
        nazan.name = "only this app".into();
        nazan.models.clear();
        local.expected_stats.by_level.clear();
        local.difficulty_templates.clear();
    }
    normal.app.insert_resource(SpellStore::default());
    assert_eq!(
        heroic.app.world().resource::<WorldData>().templates[&17536].name,
        original
    );
    assert!(
        heroic
            .app
            .world()
            .resource::<WorldData>()
            .difficulty_templates
            .contains_key(&(17536, 2))
    );
    assert!(
        !heroic
            .app
            .world()
            .resource::<WorldData>()
            .expected_stats
            .by_level
            .is_empty()
    );
    assert!(
        heroic
            .app
            .world()
            .resource::<SpellStore>()
            .get(30717)
            .is_some()
    );
    assert_eq!(
        omor.app.world().resource::<WorldData>().templates[&17308].name,
        omor_name
    );
    let (reloaded, _, _, _) = herald_intro2::prepared_ramparts(1);
    let restored = reloaded.app.world().resource::<WorldData>();
    assert_eq!(restored.templates[&17536].name, original);
    assert!(!restored.templates[&17536].models.is_empty());
    assert!(restored.difficulty_templates.contains_key(&(17536, 2)));
    assert!(!restored.expected_stats.by_level.is_empty());
    assert!(
        reloaded
            .app
            .world()
            .resource::<SpellStore>()
            .get(30717)
            .is_some()
    );
}

#[test]
fn prepared_liquid_fire_overlay_keeps_live_apps_and_reloads_independent() {
    use crate::game_objects::summoned::{SummonedObjectTemplates, wild_unsupported_reason};
    use std::collections::BTreeSet;

    let (mut normal, _, _, _) = herald_intro2::prepared_ramparts(1);
    let (mut heroic, _, _, _) = herald_intro2::prepared_ramparts(2);
    herald_liquid_fire::install_liquid_fire_inputs(&mut normal);
    herald_liquid_fire::install_liquid_fire_inputs(&mut heroic);
    let original = normal.app.world().resource::<WorldData>().templates[&21252].clone();
    let models: Vec<_> = original
        .models
        .iter()
        .map(|model| model.display_id)
        .collect();
    let difficulties: BTreeSet<_> = normal
        .app
        .world()
        .resource::<WorldData>()
        .difficulty_templates
        .keys()
        .filter(|(entry, _)| *entry == 21252)
        .copied()
        .collect();
    let unrelated = heroic.app.world().resource::<WorldData>().templates[&17536]
        .name
        .clone();
    let effect = normal
        .app
        .world()
        .resource::<SpellStore>()
        .get(31706)
        .unwrap()
        .effects[0]
        .clone();
    assert_eq!(wild_unsupported_reason(normal.app.world(), &effect), None);
    assert_eq!(wild_unsupported_reason(heroic.app.world(), &effect), None);
    {
        let mut data = normal.app.world_mut().resource_mut::<WorldData>();
        let trigger = data.templates.get_mut(&21252).unwrap();
        trigger.name = "only the normal app".into();
        trigger.models.clear();
        data.difficulty_templates
            .retain(|(entry, _), _| *entry != 21252);
        data.expected_stats.by_level.clear();
    }
    normal
        .app
        .insert_resource(SummonedObjectTemplates::default());
    assert!(wild_unsupported_reason(normal.app.world(), &effect).is_some());
    assert_eq!(wild_unsupported_reason(heroic.app.world(), &effect), None);
    let sibling = heroic.app.world().resource::<WorldData>();
    assert_eq!(sibling.templates[&21252].name, original.name);
    assert_eq!(sibling.templates[&17536].name, unrelated);
    assert!(!sibling.expected_stats.by_level.is_empty());
    assert_eq!(
        sibling.templates[&21252]
            .models
            .iter()
            .map(|model| model.display_id)
            .collect::<Vec<_>>(),
        models
    );
    herald_liquid_fire::install_liquid_fire_inputs(&mut normal);
    let restored = normal.app.world().resource::<WorldData>();
    assert_eq!(restored.templates[&21252].name, original.name);
    assert_eq!(
        restored.templates[&21252]
            .models
            .iter()
            .map(|model| model.display_id)
            .collect::<Vec<_>>(),
        models
    );
    assert_eq!(
        restored
            .difficulty_templates
            .keys()
            .filter(|(entry, _)| *entry == 21252)
            .copied()
            .collect::<BTreeSet<_>>(),
        difficulties
    );
    assert!(restored.expected_stats.by_level.is_empty());
    assert_eq!(wild_unsupported_reason(normal.app.world(), &effect), None);
}

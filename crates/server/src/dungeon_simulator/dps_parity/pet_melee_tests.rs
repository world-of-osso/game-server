//! Actual native Hunter pet swings before owner-RAP inheritance changes.
use super::*;
use shared::protocol::CombatLogKind;

fn pet_melee_hits(world: &World, pet: Entity, target: Entity) -> Vec<i32> {
    world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(pet.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id.is_none()
        })
        .map(|event| event.amount)
        .collect()
}

fn collect_native_pet_swings(run: &mut Run, pet: Entity) -> Vec<i32> {
    let target = run.targets[0];
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(target));
    for _ in 0..600 {
        run.app.update();
        let hits = pet_melee_hits(run.app.world(), pet, target);
        if hits.len() >= 4 {
            return hits;
        }
    }
    panic!("native pet did not deliver four attributed melee swings");
}

#[test]
#[ignore = "native pet melee baseline; requires prepared parity inputs"]
fn dps_parity_pet_melee_baseline_records_actual_owner_and_swing_damage() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let hits = collect_native_pet_swings(&mut run, pet);
    let world = run.app.world();
    let owner = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    let entry = world
        .get::<shared::components::Npc>(pet)
        .unwrap()
        .template_id;
    let template = world
        .resource::<crate::world_data::WorldData>()
        .template_for(entry, 0)
        .unwrap();
    println!(
        "NATIVE_PET_MELEE_BASELINE {}",
        json!({
            "entry": entry, "level": world.get::<shared::components::UnitLevel>(pet).unwrap().0,
            "owner_melee_ap": owner.attack_power, "owner_ranged_ap": owner.ranged_attack_power,
            "template_attack_time_ms": template.base_attack_time_ms,
            "pet_character_stats": world.get::<crate::stats::CharacterStats>(pet).is_some(),
            "hits": hits,
        })
    );
    assert!(hits.iter().all(|damage| *damage > 0));
}

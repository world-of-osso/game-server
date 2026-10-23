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
                && !event.glancing
                && event.blocked == 0
                && event.absorbed == 0
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

fn owner_contribution_floor(world: &World, owner: Entity, pet: Entity, target: Entity) -> f32 {
    // TC StatSystem1344/1389/1423: integer .22 ownerRAP, /3.5 per
    // base attack second. Pet.cpp882 sets the source2000ms pet base clock.
    let owner = world.get::<crate::stats::CharacterStats>(owner).unwrap();
    let bonus = (owner.ranged_attack_power * 0.22).trunc() / 3.5 * 2.0;
    let pet_mods = world.get::<crate::auras::AuraStatMods>(pet);
    let done = pet_mods.map_or(1.0, |mods| {
        mods.damage_done(1) * mods.autoattack_damage_mult
    });
    let taken = world
        .get::<crate::auras::AuraStatMods>(target)
        .map_or(1.0, |mods| mods.damage_taken(1));
    let pet_scaling = crate::level_scaling::UnitScaling::of(world, pet);
    let target_scaling = crate::level_scaling::UnitScaling::of(world, target);
    let armor = target_scaling.armor_against(
        world
            .get::<crate::level_scaling::CreatureArmor>(target)
            .unwrap()
            .0,
        &pet_scaling,
    );
    crate::combat::armor_mitigated(
        (bonus * done * taken).round(),
        armor,
        &pet_scaling,
        &target_scaling,
        Some(world.resource::<crate::level_scaling::ArmorConstants>()),
        pet_mods,
    )
    .floor()
}

#[test]
#[ignore = "native pet owner-RAP melee RED; requires prepared parity inputs"]
fn dps_parity_pet_melee_actual_swings_include_source_owner_ranged_ap_contribution() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let hits = collect_native_pet_swings(&mut run, pet);
    let world = run.app.world();
    let floor = owner_contribution_floor(world, run.actor, pet, run.targets[0]);
    println!(
        "NATIVE_PET_MELEE_OWNER_RAP {}",
        json!({
            "minimum_owner_contribution": floor, "hits": hits,
            "pet_modifiers": format!("{:?}", world.get::<crate::auras::AuraStatMods>(pet)),
        })
    );
    assert!(floor > 0.0);
    // Intrinsic pet AP/weapon damage is unqualified, not invented here. Any
    // nonnegative intrinsic base must still include the sourced owner share.
    // Native misses/crit rolls remain; blocked/absorbed/glancing receipts are
    // excluded rather than changing RNG or loosening the contribution bound.
    assert!(
        hits.iter().all(|damage| *damage as f32 >= floor),
        "native pet swings{hits:?} omit source owner-RAP contribution floor{floor}"
    );
}

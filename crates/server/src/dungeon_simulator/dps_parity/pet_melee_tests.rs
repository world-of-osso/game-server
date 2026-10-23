//! Native Hunter pet owner-RAP melee receipts; no synthetic stats or intrinsic ranges.
use super::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const PET_ABILITY: u32 = 1264355;

fn pet_melee_events(
    world: &World,
    pet: Entity,
    target: Entity,
) -> impl Iterator<Item = &CombatLogEvent> {
    world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(move |event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(pet.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id.is_none()
                && !event.glancing
                && event.blocked == 0
                && event.absorbed == 0
        })
}

fn pet_melee_hits(world: &World, pet: Entity, target: Entity) -> Vec<i32> {
    pet_melee_events(world, pet, target)
        .map(|event| event.amount)
        .collect()
}

fn normalized_pet_swings(run: &mut Run, pet: Entity, required_hits: usize) -> Vec<f32> {
    collect_native_pet_swings(run, pet, required_hits);
    let world = run.app.world();
    let critical = 2.0
        * world
            .get::<crate::auras::AuraStatMods>(pet)
            .map_or(1.0, |mods| mods.critical_damage(1));
    pet_melee_events(world, pet, run.targets[0])
        .map(|event| event.amount as f32 / if event.crit { critical } else { 1.0 })
        .collect()
}

fn collect_native_pet_swings(run: &mut Run, pet: Entity, required_hits: usize) -> Vec<i32> {
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
        if hits.len() >= required_hits {
            return hits;
        }
    }
    panic!("native pet did not deliver {required_hits} attributed melee swings");
}

#[test]
#[ignore = "native pet melee baseline; requires prepared parity inputs"]
fn dps_parity_pet_melee_baseline_records_actual_owner_and_swing_damage() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let hits = collect_native_pet_swings(&mut run, pet, 4);
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
            "owner_total_ranged_ap": owner_total_ranged_ap(world, run.actor),
            "owner_weapon_mainhand_ap": owner.weapon_attack_power.main_hand,
            "owner_weapon_ranged_ap": owner.weapon_attack_power.ranged,
            "template_attack_time_ms": template.base_attack_time_ms,
            "pet_character_stats": world.get::<crate::stats::CharacterStats>(pet).is_some(),
            "hits": hits,
        })
    );
    assert!(hits.iter().all(|damage| *damage > 0));
}

fn physical_damage_multiplier(world: &World, pet: Entity, target: Entity) -> f32 {
    let done = world
        .get::<crate::auras::AuraStatMods>(pet)
        .map_or(1.0, |mods| mods.damage_done(1));
    let taken = world
        .get::<crate::auras::AuraStatMods>(target)
        .map_or(1.0, |mods| mods.damage_taken(1));
    done * taken * crate::auras::summon_damage::multiplier(world, pet)
}

fn owner_total_ranged_ap(world: &World, owner: Entity) -> f32 {
    world
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .total_attack_power(
            crate::stats::WeaponAttackType::Ranged,
            world.get::<crate::stats::StatModifiers>(owner),
        )
}

fn owner_contribution_floor(world: &World, owner: Entity, pet: Entity, target: Entity) -> f32 {
    // TC StatSystem1344/1389/1423: integer .22 ownerRAP, /3.5 per
    // base attack second. Pet.cpp882 sets the source2000ms pet base clock.
    let stats = world.get::<crate::stats::CharacterStats>(owner).unwrap();
    let total = owner_total_ranged_ap(world, owner);
    assert!(
        total > stats.ranged_attack_power,
        "actual native weapon must contribute owner total RAP"
    );
    let bonus = (total * 0.22).trunc() / 3.5 * 2.0;
    let auto = world
        .get::<crate::auras::AuraStatMods>(pet)
        .map_or(1.0, |mods| mods.autoattack_damage_mult);
    mitigated_physical_floor(world, pet, target, bonus * auto)
}

fn mitigated_physical_floor(world: &World, pet: Entity, target: Entity, base: f32) -> f32 {
    let damage = (base * physical_damage_multiplier(world, pet, target)).round();
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
        damage,
        armor,
        &pet_scaling,
        &target_scaling,
        Some(world.resource::<crate::level_scaling::ArmorConstants>()),
        world.get::<crate::auras::AuraStatMods>(pet),
    )
    .floor()
}

#[test]
#[ignore = "native pet owner-RAP melee RED; requires prepared parity inputs"]
fn dps_parity_pet_melee_actual_swings_include_source_owner_ranged_ap_contribution() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let hits = normalized_pet_swings(&mut run, pet, 4);
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
        hits.iter().all(|damage| *damage >= floor),
        "native pet swings{hits:?} omit source owner-RAP contribution floor{floor}"
    );
}

#[test]
#[ignore = "native pet live owner-RAP equipment control; requires prepared parity inputs"]
fn dps_parity_pet_melee_live_owner_equipment_removal_and_restore_changes_swings() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let before = normalized_pet_swings(&mut run, pet, 4);
    let owner = run.actor;
    let before_ap = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .ranged_attack_power;
    let equipment = run
        .app
        .world()
        .get::<crate::stats::EquippedItems>(owner)
        .unwrap()
        .clone();
    let modifiers = run
        .app
        .world()
        .get::<crate::stats::EquippedItemModifiers>(owner)
        .unwrap()
        .clone();
    run.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItems>(owner)
        .unwrap()
        .items
        .clear();
    run.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(owner)
        .unwrap()
        .items
        .clear();
    for _ in 0..4 {
        run.app.update();
    }
    let without_ap = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .ranged_attack_power;
    assert!(
        without_ap < before_ap,
        "actual reference gear must contribute RAP"
    );
    let without = normalized_pet_swings(&mut run, pet, 4);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert((equipment, modifiers));
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .ranged_attack_power,
        before_ap
    );
    let restored = normalized_pet_swings(&mut run, pet, 4);
    let lowest = |hits: &[f32]| hits.iter().copied().reduce(f32::min).unwrap();
    let highest = |hits: &[f32]| hits.iter().copied().reduce(f32::max).unwrap();
    println!(
        "NATIVE_PET_MELEE_GEAR {}",
        json!({
            "before_owner_rap": before_ap, "without_owner_rap": without_ap,
            "before_uncritical_hits": before, "without_uncritical_hits": without,
            "restored_uncritical_hits": restored,
        })
    );
    assert!(
        highest(&without) < lowest(&before),
        "removing real owner gear must lower pet swings"
    );
    assert!(
        lowest(&restored) > highest(&without),
        "restoring real owner gear must raise pet swings"
    );
}

#[test]
#[ignore = "native pet ranged-versus-melee AP control; requires prepared parity inputs"]
fn dps_parity_pet_melee_owner_only_melee_ap_buff_does_not_change_swings() {
    // Native70831 Maelstrom Power: caster-only aura166 +20%, no aura167,
    // party/pet recipient, extra attack, trigger or damage effect.
    const MELEE_ONLY_AP: u32 = 70831;
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let before = normalized_pet_swings(&mut run, pet, 1);
    let owner = run.actor;
    let initial = *run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap();
    let initial_total_ranged = owner_total_ranged_ap(run.app.world(), owner);
    crate::spell_cast::cast_triggered(run.app.world_mut(), owner, MELEE_ONLY_AP, Some(owner), 1);
    for _ in 0..4 {
        run.app.update();
    }
    let stats = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap();
    assert!(
        stats.attack_power > initial.attack_power,
        "native166 must change owner melee AP"
    );
    assert_eq!(stats.ranged_attack_power, initial.ranged_attack_power);
    assert_eq!(
        owner_total_ranged_ap(run.app.world(), owner),
        initial_total_ranged
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), pet, MELEE_ONLY_AP),
        "native buff is owner-only, not a direct pet AP modifier"
    );
    let after = normalized_pet_swings(&mut run, pet, 1);
    assert!(
        crate::auras::has_aura(run.app.world(), owner, MELEE_ONLY_AP),
        "native buff must remain active during the measured swing"
    );
    println!(
        "NATIVE_PET_MELEE_RANGED_ONLY {}",
        json!({"before": before, "after_melee_only_buff": after})
    );
    // Each independent integer combat receipt can lose less than one damage
    // unit before crit normalization. This bounds quantization, not AP variance.
    assert!(
        (after[0] - before[0]).abs() <= 1.0,
        "pet must inherit RAP, not the independently increased melee AP"
    );
}

fn pet_ability_hit(world: &World, pet: Entity, target: Entity) -> Option<CombatLogEvent> {
    world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(pet.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(PET_ABILITY)
                && !event.crit
                && event.blocked == 0
                && event.absorbed == 0
        })
        .cloned()
}

fn cast_native_pet_ability(run: &mut Run, pet: Entity) -> CombatLogEvent {
    let target = run.targets[0];
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..16 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), pet, PET_ABILITY, Some(target), 1);
        run.app.update();
        if let Some(hit) = pet_ability_hit(run.app.world(), pet, target) {
            return hit;
        }
    }
    panic!("native1264355 did not produce an unblocked/unabsorbed normal pet damage receipt");
}

#[test]
#[ignore = "native pet ability total-owner-RAP RED; requires prepared parity inputs"]
fn dps_parity_pet_melee_shared_owner_rap_source_includes_weapon_ap_in_actual_ability() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    collect_native_pet_swings(&mut run, pet, 1);
    let hit = cast_native_pet_ability(&mut run, pet);
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(PET_ABILITY)
        .unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == crate::spell_cast::effects::EFFECT_SCHOOL_DAMAGE)
        .unwrap();
    assert_eq!(
        effect.variance, 0.0,
        "native1264355 source has zero damage variance"
    );
    let total_rap = owner_total_ranged_ap(world, run.actor);
    let bare_rap = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ranged_attack_power;
    assert!(
        total_rap > bare_rap,
        "actual equipped weapon must contribute owner total RAP"
    );
    let pet_power = (total_rap * 0.22).trunc();
    let floor = mitigated_physical_floor(
        world,
        pet,
        run.targets[0],
        pet_power * effect.bonus_coefficient_from_ap,
    );
    println!(
        "NATIVE_PET_ABILITY_TOTAL_RAP {}",
        json!({
            "owner_bare_rap": bare_rap, "owner_total_rap": total_rap,
            "native_ap_coefficient": effect.bonus_coefficient_from_ap, "pet_power": pet_power,
            "damage": hit.amount, "minimum_owner_contribution": floor,
        })
    );
    assert!(
        hit.amount as f32 >= floor,
        "native1264355 damage{} omits equipped-weapon owner RAP; minimum{floor}",
        hit.amount
    );
}

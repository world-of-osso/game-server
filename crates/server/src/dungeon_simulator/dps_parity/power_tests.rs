//! Real native pools/events, not inferred resource gain from damage or elapsed time.
use super::*;
use crate::spell_cast::powers::PowerPools;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const CALL_PET: u32 = 883;
const COBRA_SHOT: u32 = 193455;
const PACK_TACTICS_ENERGIZE: u32 = 1282660;

fn prepare_hunter_and_pet() -> (Run, Entity) {
    let (profile, traits) = read_reference(253).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    cast_native_call_pet(&mut run);
    let pet = wait_for_native_pet(&mut run);
    (run, pet)
}

fn cast_native_call_pet(run: &mut Run) {
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            CALL_PET,
            None,
            false,
        ) {
            Ok(()) => return,
            Err(crate::spell_cast::CastFailure::OnGlobalCooldown) => run.app.update(),
            Err(error) => panic!("native Call Pet admission: {error:?}"),
        }
    }
    panic!("native Call Pet GCD did not recover");
}

fn wait_for_native_pet(run: &mut Run) -> Entity {
    for _ in 0..200 {
        let world = run.app.world();
        let cobra = world
            .resource::<crate::spell_info::SpellStore>()
            .get(COBRA_SHOT)
            .unwrap();
        let now = crate::spell_cast::history_now(world);
        let on_gcd = world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.on_gcd(cobra, now));
        if let Some(pet) = crate::hunter_pet::summoned_pet(world, run.actor)
            && !on_gcd
        {
            return pet;
        }
        run.app.update();
    }
    panic!("native pet/GCD did not reach readiness");
}

fn assert_focus_snapshot(snapshot: &Value, world: &World, unit: Entity) {
    let expected = world
        .get::<PowerPools>(unit)
        .unwrap()
        .get(PowerType::Focus)
        .unwrap();
    let pools = snapshot
        .as_array()
        .expect("receipt must expose raw native power pools");
    let focus = pools
        .iter()
        .find(|pool| pool["power_type"] == PowerType::Focus as i32)
        .unwrap();
    assert_eq!(focus["current"].as_f64(), Some(f64::from(expected.current)));
    assert_eq!(focus["max"].as_f64(), Some(f64::from(expected.max)));
    assert_eq!(
        focus["regen_per_sec"].as_f64(),
        Some(f64::from(expected.regen_per_sec))
    );
}

#[test]
#[ignore = "opt-in native power receipt; requires prepared parity inputs"]
fn dps_parity_power_receipt_reads_owner_pet_pools_and_live_regeneration() {
    let (mut run, pet) = prepare_hunter_and_pet();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        COBRA_SHOT,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    run.app.update();
    let world = run.app.world();
    assert!(
        world
            .get::<PowerPools>(run.actor)
            .unwrap()
            .get(PowerType::Focus)
            .unwrap()
            .regen_per_sec
            > 0.0
    );
    let owner = report::engagement(world, run.actor, &run.targets);
    let pet_receipt = report::pet(world, run.actor, &run.targets);
    assert_focus_snapshot(&owner["powers"], world, run.actor);
    assert_focus_snapshot(&pet_receipt["powers"], world, pet);
}

#[test]
#[ignore = "opt-in native power receipt; requires prepared parity inputs"]
fn dps_parity_power_receipt_attributes_native_energize_and_capped_overflow() {
    let (mut run, pet) = prepare_hunter_and_pet();
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        COBRA_SHOT,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for unit in [run.actor, run.actor, pet] {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            unit,
            PACK_TACTICS_ENERGIZE,
            Some(unit),
            false,
        )
        .unwrap();
    }
    let world = run.app.world();
    let events: Vec<_> = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Energize && event.spell_id == Some(PACK_TACTICS_ENERGIZE)
        })
        .collect();
    assert_eq!(events.len(), 3);
    let native_grant = world
        .resource::<crate::spell_info::SpellStore>()
        .get(PACK_TACTICS_ENERGIZE)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.effect == 30)
        .unwrap()
        .base_points as i32;
    let receipt = report::combat(world, &run.targets, 1.0);
    let gains = receipt["energize"]
        .as_array()
        .expect("receipt must account for actual Energize events");
    assert_eq!(gains.len(), 2, "owner/pet source identities must not merge");
    for unit in [run.actor, pet] {
        let owned: Vec<_> = events
            .iter()
            .filter(|event| event.source == Some(unit.to_bits()))
            .collect();
        let expected_count = if unit == run.actor { 2 } else { 1 };
        assert_eq!(owned.len(), expected_count);
        let amount = native_grant * expected_count as i32;
        let overflow: i32 = owned.iter().map(|event| event.overflow).sum();
        let gain = gains
            .iter()
            .find(|gain| gain["source"] == unit.to_bits())
            .unwrap();
        assert_eq!(gain["target"], unit.to_bits());
        assert_eq!(gain["spell_id"], PACK_TACTICS_ENERGIZE);
        assert_eq!(gain["amount"], amount);
        assert_eq!(gain["overflow"], overflow);
        assert_eq!(gain["effective_gain"], amount - overflow);
        assert_eq!(gain["events"], expected_count);
    }
    let pet_event = events
        .iter()
        .find(|event| event.source == Some(pet.to_bits()))
        .unwrap();
    assert_eq!(pet_event.amount, native_grant);
    assert_eq!(
        pet_event.overflow, native_grant,
        "idle native pet is full; capped grant is not realized gain"
    );
    let owner_event = events
        .iter()
        .find(|event| event.source == Some(run.actor.to_bits()))
        .unwrap();
    assert!(
        owner_event.amount > owner_event.overflow,
        "real Cobra cost must leave room for actual gain"
    );
}

#[test]
#[ignore = "opt-in native Mana receipt; requires prepared parity inputs"]
fn dps_parity_power_receipt_reads_authoritative_mana_in_stat_snapshots() {
    let (profile, traits) = read_reference(66).unwrap();
    let run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let world = run.app.world();
    let mana = world.get::<shared::components::Mana>(run.actor).unwrap();
    let native_rate = world
        .get::<PowerPools>(run.actor)
        .unwrap()
        .mana_regen_per_sec;
    let receipt = report::stats(world, run.actor);
    let pool = receipt["powers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pool| pool["power_type"] == PowerType::Mana as i32)
        .unwrap();
    assert_eq!(pool["current"].as_f64(), Some(f64::from(mana.current)));
    assert_eq!(pool["max"].as_f64(), Some(f64::from(mana.max)));
    assert_eq!(pool["regen_per_sec"].as_f64(), Some(f64::from(native_rate)));
}

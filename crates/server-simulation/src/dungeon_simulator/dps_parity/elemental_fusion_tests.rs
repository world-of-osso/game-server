//! SimC76f8118 sc_shaman.cpp3303–3311/7439–7447: earned Power of the
//! Maelstrom is spent by ordinary Lava Burst to execute background Fusion Blast.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TALENT: u32 = 191861;
const POWER: u32 = 191877;
const FUSION: u32 = 462840;
const BOLT: u32 = 188196;
const LAVA: u32 = 51505;
const BLAST: u32 = 117014;

fn overload_damage(multiplier: f32, mote: bool) -> (u64, f32) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        1219480,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    cast(&mut run, LAVA);
    let mote_aura = own_aura(&run, 260734).expect("native Lava earns MOTE");
    let percent = mote_aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let aura = mote_aura.instance_id;
    if !mote {
        crate::auras::remove_instance(
            run.app.world_mut(),
            run.actor,
            aura,
            crate::auras::RemoveReason::Consumed,
        );
    }
    let _cast_seed = crate::random::seed_scope(70448);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let mut blast = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(BLAST)
        .unwrap()
        .clone();
    blast.cast_damage_multiplier = Some(multiplier);
    blast.cast_overload_multiplier = Some(multiplier);
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &blast,
        Some(run.targets[0]),
        1
    ));
    settle(&mut run);
    let damage = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(120588)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| event.amount as u64)
        .sum();
    (damage, percent)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_fusion_overload_retains_parent_multiplier() {
    let (ordinary, _) = overload_damage(1.0, false);
    let (scaled, _) = overload_damage(0.4, false);
    assert!(
        ordinary > 0 && scaled > 0,
        "native Ascendance produces overload damage"
    );
    let ratio = scaled as f64 / ordinary as f64;
    assert!(
        (ratio - 0.4).abs() < 0.02,
        "native overload multiplier: {scaled}/{ordinary} = {ratio}"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_fusion_overload_applies_mote_only_once() {
    let (ordinary, _) = overload_damage(0.4, false);
    let (empowered, percent) = overload_damage(0.4, true);
    let ratio = empowered as f64 / ordinary as f64;
    let expected = 1.0 + f64::from(percent) / 100.0;
    assert!(
        ordinary > 0 && (ratio - expected).abs() < 0.03,
        "MOTE applies once: {empowered}/{ordinary} = {ratio}; expected {expected}"
    );
}

fn cast_events(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
        })
        .count()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_fusion_native_builders_earn_power_then_lava_spends_one_and_casts_free_blast() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(own_aura(&run, TALENT).is_some());
    assert!(own_aura(&run, FUSION).is_some());
    assert!(own_aura(&run, POWER).is_none());
    // Normal builders alone roll native191861 effect0. Background casts do not.
    for _ in 0..50 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            BOLT,
            Some(run.targets[0]),
            1,
        ));
    }
    settle(&mut run);
    assert!(
        own_aura(&run, POWER).is_none(),
        "background builders cannot earn Power"
    );
    for _ in 0..100 {
        cast(&mut run, BOLT);
        if own_aura(&run, POWER).is_some() {
            break;
        }
    }
    let power = own_aura(&run, POWER)
        .expect("ordinary native Bolts must earn Power at the native15% chance with seed70448");
    assert_eq!(power.stacks, 2, "native191877 grants two eligible uses");
    let before = cast_events(&run, BLAST);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        LAVA,
        Some(run.targets[0]),
        1,
    ));
    settle(&mut run);
    assert_eq!(
        own_aura(&run, POWER).unwrap().stacks,
        2,
        "background Lava does not consume Power"
    );
    assert_eq!(
        cast_events(&run, BLAST),
        before,
        "background Lava does not produce Fusion"
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let before_maelstrom = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::Maelstrom,
    )
    .unwrap()
    .0;
    cast(&mut run, LAVA);
    assert_eq!(
        own_aura(&run, POWER).unwrap().stacks,
        1,
        "ordinary Lava spends one Power use"
    );
    assert_eq!(
        cast_events(&run, BLAST),
        1,
        "one real Fusion Blast per consumed use"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(BLAST)
                && event.kind == CombatLogKind::CastStart),
        "Fusion Blast is instant background, not a second foreground hardcast"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(BLAST)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "real Fusion Blast must deal native damage"
    );
    let after_maelstrom = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::Maelstrom,
    )
    .unwrap()
    .0;
    assert!(
        after_maelstrom >= before_maelstrom,
        "Fusion never pays an ordinary spender cost"
    );
}

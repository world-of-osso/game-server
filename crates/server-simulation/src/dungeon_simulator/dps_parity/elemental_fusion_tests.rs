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

fn blast_overload_multiplier(fusion: bool) -> (f64, f64) {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
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
    let mountains = own_aura(&run, 381726)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64
        / 100.0;
    let mut blast = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(BLAST)
        .unwrap()
        .clone();
    if fusion {
        blast.cast_damage_multiplier = Some(0.4);
        blast.cast_overload_multiplier = Some(0.4);
        blast.cast_parent_owns_modifier_charges = true;
    }
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        120588,
        "blast_overload",
        1,
    ));
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &blast,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..80 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let factor = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 120588)
        .expect("actual Blast overload")["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "cast_effectiveness")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    (factor, mountains)
}

#[test]
#[ignore = "requires native Elemental262 profile and world data"]
fn elemental_blast_overload_applies_mountains_only_to_normal_variant() {
    let (normal, mountains) = blast_overload_multiplier(false);
    let (fusion, _) = blast_overload_multiplier(true);
    assert!(
        (normal - mountains).abs() < 0.001,
        "normal overload {normal}, native Mountains {mountains}"
    );
    assert!(
        (fusion - 0.4).abs() < 0.001,
        "Fusion overload keeps only its own multiplier: {fusion}"
    );
}

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
    assert_eq!(power.stacks, 1, "the first native proc grants one use");
    for _ in 0..100 {
        cast(&mut run, BOLT);
        if own_aura(&run, POWER).is_some_and(|aura| aura.stacks == 2) {
            break;
        }
    }
    assert_eq!(
        own_aura(&run, POWER).unwrap().stacks,
        2,
        "a later proc adds the second use"
    );
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

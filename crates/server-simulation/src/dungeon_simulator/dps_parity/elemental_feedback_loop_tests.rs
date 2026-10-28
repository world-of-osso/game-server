//! Feedback Loop's extra overload uses actual native spell impacts.
use super::elemental_mechanics_tests::{cast, fixture, settle};
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn set_feedback_chance(run: &mut Run, chance: f32) {
    let native = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1270064 && aura.caster == run.actor)
        .unwrap();
    assert_eq!(
        native
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount,
        25.0
    );
    let mut spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1270064)
        .unwrap()
        .clone();
    spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points = chance;
    crate::auras::apply_aura_with_spell(
        run.app.world_mut(),
        &AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 1270064,
            effect_mask: crate::auras::aura_effect_mask(&spell),
            source: AuraSource::Cast,
        },
        &spell,
    );
}

fn overload_hits(force_feedback: bool) -> usize {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1270064));
    if force_feedback {
        set_feedback_chance(&mut run, 100.0); // Deterministic proc boundary.
    } else {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            RemoveReason::Consumed,
            |aura| aura.spell_id == 1270064,
        );
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..6 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            191634,
            Some(run.actor),
            1,
        );
        cast(&mut run, 188196);
    }
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(45284)
                && event.kind == CombatLogKind::Damage
        })
        .count()
}

fn quake_overload_hits(force_feedback: bool) -> usize {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 3).unwrap();
    engage(&mut run);
    set_feedback_chance(&mut run, if force_feedback { 100.0 } else { 0.0 });
    let mut powers = run
        .app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap();
    let maelstrom = powers
        .get_mut(shared::components::PowerType::Maelstrom)
        .unwrap();
    maelstrom.current = maelstrom.max;
    drop(powers);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        1219480,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 462620);
    for _ in 0..160 {
        run.app.update();
    }
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(298765)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires native Elemental262 profile and world data"]
fn elemental_feedback_loop_adds_one_nonrecursive_earthquake_field() {
    let baseline = quake_overload_hits(false);
    let guaranteed = quake_overload_hits(true);
    assert!(baseline >= 3, "Ascendance earns an actual overload field");
    assert_eq!(
        guaranteed,
        baseline * 2,
        "Feedback Loop adds exactly one field per earned field"
    );
}

#[test]
#[ignore = "requires native Elemental262 profile and world data"]
fn elemental_feedback_loop_adds_one_nonrecursive_overload_per_earned_overload() {
    let baseline = overload_hits(false);
    let guaranteed = overload_hits(true);
    assert_eq!(baseline, 6, "Stormkeeper earns one real overload per cast");
    assert_eq!(
        guaranteed, 12,
        "Feedback Loop adds exactly one overload per earned overload"
    );
}

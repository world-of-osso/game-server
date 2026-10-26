use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn launch_owned_shatters() -> Run {
    let mut run = prepare_literal(64, 1).unwrap();
    // Disable only the natural passive clock to isolate tier-earned Icicles.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| matches!(aura.spell_id, 1246832 | 205473 | 1222865 | 44544),
    );
    for _ in 0..40 {
        for _ in 0..4 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: run.actor,
                    target: run.targets[0],
                    spell_id: 1221389,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                30455,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..20 {
            run.app.update();
        }
    }
    let count = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1246949)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        count, 40,
        "the native source must really pay forty owned Shatters"
    );
    run
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_paid_shatters_earn_hand_of_frost_impacts() {
    let _seed = crate::random::seed_scope(70448);
    let run = launch_owned_shatters();
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(1262769)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "actual paid Shatters must roll selected Hand of Frost's native impact"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_shattered_stacks_earn_tier_icicles_without_the_passive_clock() {
    let _seed = crate::random::seed_scope(70448);
    let run = launch_owned_shatters();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1296585),
        "actual native tier2pc must be equipped"
    );
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 205473 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks);
    assert!(
        stacks > 0,
        "native tier must roll once per shattered stack, independent of the passive clock"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_ray_ticks_pay_selected_hand_of_frost_schedule() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        205021,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hands = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1262769)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        hands >= 4,
        "selected native Ray must pay its authored Hand schedule, got{hands}"
    );
}

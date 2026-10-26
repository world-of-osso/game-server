//! SimC76f8118 sc_priest_shadow.cpp598–645/2355–2371/2609–2650.
//! Only ordinary Pain hits/ticks after a settled Slam may fund this apparition cohort.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn cast_at(run: &mut Run, spell: u32, target: Entity) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, target, None),
    )
    .unwrap();
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_torment_real_pain_hits_and_ticks_fund_equal_owned_vt_apparition_cohorts() {
    let mut run = prepare_literal(258, 3).unwrap();
    engage_fixture_opponents(&mut run);
    let actor = run.actor;
    let targets = run.targets.clone();
    assert!(crate::auras::has_aura(run.app.world(), actor, 391284));
    cast_at(&mut run, 1227280, targets[0]);
    advance(&mut run, 80); // Settle every initial weak-Madness apparition before collecting Pain.
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for &target in &targets {
        cast_at(&mut run, 589, target);
        advance(&mut run, 40);
    }
    advance(&mut run, 20);
    for &target in &targets {
        cast_at(&mut run, 34914, target); // Real refresh keeps every receiver's VT live.
        advance(&mut run, 40);
    }
    for &target in &targets {
        cast_at(&mut run, 589, target);
        advance(&mut run, 40);
    }
    advance(&mut run, 100);
    let now = crate::spell_cast::now(run.app.world());
    for &target in &targets {
        assert!(
            run.app
                .world()
                .get::<ActiveAuras>(target)
                .unwrap()
                .iter()
                .any(|aura| {
                    let owned_vt = aura.caster == actor && aura.spell_id == 34914;
                    owned_vt && aura.expires_at.is_some_and(|expiry| expiry > now)
                }),
            "all three native VTs remain live at the source cutoff"
        );
        // Stop the source using native removal, then let all already-launched missiles land.
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            RemoveReason::Cancelled,
            |aura| aura.caster == actor && aura.spell_id == 589,
        );
    }
    advance(&mut run, 80);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let pain_ticks = events
        .iter()
        .filter(|event| {
            let owned = event.source == Some(actor.to_bits()) && event.spell_id == Some(589);
            owned && event.periodic && event.amount > 0
        })
        .count();
    assert!(
        pain_ticks >= 20,
        "actual native periodic Pain source has enough trials: {pain_ticks}"
    );
    let hits = |target: Entity, spells: &[u32]| {
        events
            .iter()
            .filter(|event| {
                let recipient =
                    event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits());
                let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
                recipient && damage && event.spell_id.is_some_and(|spell| spells.contains(&spell))
            })
            .count()
    };
    let groups = hits(targets[0], &[413231, 1264176]);
    assert!(
        groups > 0,
        "Tormented Pain must earn real travelling apparition cohorts"
    );
    for target in targets {
        assert_eq!(
            hits(target, &[413231, 1264176]),
            groups,
            "one normal/void apparition per owned VT in every earned cohort"
        );
        assert_eq!(
            hits(target, &[1231479]),
            groups,
            "one source-parent Shadeburst per cohort, not one per secondary apparition"
        );
    }
}

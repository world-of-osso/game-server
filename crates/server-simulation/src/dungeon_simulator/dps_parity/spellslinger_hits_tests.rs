use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn spellslinger_native_controlled_instincts_copies_only_to_secondary_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 3).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 444483));
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        443722,
        Some(run.targets[0]),
        1,
    );
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let impacts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(443722)
        })
        .collect();
    assert_eq!(impacts.len(), 1, "the native source must land once");
    let copies: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(444487)
        })
        .collect();
    assert_eq!(
        copies.len(),
        2,
        "a real Splinter must copy damage to both secondaries"
    );
    let fraction = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(444483)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 3)
        .unwrap()
        .base_points
        / 100.0;
    for target in &run.targets[1..] {
        let copy = copies
            .iter()
            .find(|event| event.target == Some(target.to_bits()))
            .unwrap();
        assert!(
            !copy.crit,
            "residual copy must not roll a second critical hit"
        );
        let expected = impacts[0].amount as f32 * fraction;
        assert!(
            (copy.amount as f32 - expected).abs() < 2.0,
            "native selected fraction of the real landed hit: expected{expected}, got{}",
            copy.amount
        );
    }
    assert!(
        copies
            .iter()
            .all(|event| event.target != Some(run.targets[0].to_bits()))
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn spellslinger_native_infused_splinter_impacts_earn_owned_freezing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1261080));
    for _ in 0..80 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            443722,
            Some(run.targets[0]),
            1,
        );
        for _ in 0..4 {
            run.app.update();
        }
    }
    for _ in 0..40 {
        run.app.update();
    }
    let freezing = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1221389 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks);
    assert!(
        freezing > 0,
        "actual infused Splinter impacts must earn Freezing at native chance"
    );
}

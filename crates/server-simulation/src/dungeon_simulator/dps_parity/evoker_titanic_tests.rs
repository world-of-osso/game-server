//! Actual Essence Burst consumption retains Titanic Wrath on Disintegrate ticks.
use super::*;

fn titanic_sample(selected: bool) -> (i32, bool, f32) {
    let _setup_seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(1467, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 386272));
    let amount = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 386272, 0);
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Unlearned,
            |a| a.spell_id == 386272,
        );
    }
    for _ in 0..40 {
        if crate::auras::has_aura(run.app.world(), actor, 359618) {
            break;
        }
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            actor,
            362969,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 359618),
        "real Azure Strike must earn Essence Burst"
    );
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let _tick_seed = crate::random::seed_scope(386272);
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        actor,
        356995,
        crate::spell_cast::CastTargets {
            unit: primary,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    for _ in 0..120 {
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if let Some(event) = events[before..].iter().find(|e| {
            e.source == Some(actor.to_bits())
                && e.target == Some(primary.to_bits())
                && e.spell_id == Some(356995)
                && e.kind == shared::protocol::CombatLogKind::Damage
                && e.amount > 0
        }) {
            return (event.amount, event.crit, amount);
        }
    }
    panic!("real Disintegrate must deliver a periodic hit");
}

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_titanic_actual_essence_burst_snapshot_increases_disintegrate_tick() {
    let absent = titanic_sample(false);
    let selected = titanic_sample(true);
    assert_eq!(absent.1, selected.1, "paired hits retain critical outcome");
    let expected = f64::from(absent.0) * (1.0 + f64::from(selected.2) / 100.0);
    assert!(
        (f64::from(selected.0) - expected).abs() <= 2.0,
        "native Titanic Wrath scales the actual funded periodic tick: absent={absent:?}, selected={selected:?}"
    );
}

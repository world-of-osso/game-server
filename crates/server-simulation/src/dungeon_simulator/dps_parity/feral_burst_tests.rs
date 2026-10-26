//! Native selected Bursting Growth relationships at real Vines and Bite boundaries.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_selected_vines_ticks_roll_native_rampancy_before_any_expiry() {
    let _seed = crate::random::seed_scope(1031270586);
    let mut run = prepare_literal(103, 3).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1270586));
    engage(&mut run);
    for _ in 0..32 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 439531, Some(target), 1);
        for _ in 0..45 {
            run.app.update();
        }
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 439531 && aura.caster == actor)
            .unwrap();
        assert!(
            aura.expires_at.unwrap() > crate::spell_cast::now(run.app.world()),
            "the native Vines application is still live when its damage is observed"
        );
        if burst_damage(&run) > 0 {
            break;
        }
        // Remove before any native stack deadline, so expiration bursts cannot
        // masquerade as the missing actual-periodic-tick Rampancy producer.
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 439531 && aura.caster == actor,
        );
    }
    assert!(
        burst_damage(&run) > 0,
        "selected native Vines damage ticks must roll Rampancy without Bite or expiry"
    );
}

fn remove_rampancy_for_burst_control(run: &mut Run) {
    // Isolate Bite/expiry from the separate chance producer using native removal.
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1270586 && aura.caster == actor,
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1270586));
}

fn burst_damage(run: &Run) -> u64 {
    report::combat(run.app.world(), &run.targets, 1.0)["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["spell_id"] == 440122)
        .map(|action| action["damage"].as_u64().unwrap())
        .sum()
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_selected_vines_expiry_executes_native_bursting_growth() {
    let mut run = prepare_literal(103, 3).unwrap();
    remove_rampancy_for_burst_control(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 440120));
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 439531, Some(target), 1);
    let expiry = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == actor && aura.spell_id == 439531)
        .unwrap()
        .expires_at
        .unwrap();
    while crate::spell_cast::now(run.app.world()) <= expiry + 0.1 {
        run.app.update();
    }
    assert!(
        burst_damage(&run) > 0,
        "an actual selected native Vines expiry must burst"
    );
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_selected_bite_on_owned_vines_executes_native_bursting_growth() {
    let mut run = prepare_literal(103, 3).unwrap();
    remove_rampancy_for_burst_control(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let frenzy =
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 274837).unwrap();
    feral_growth_tests::cast_native_spell_after_regeneration(&mut run, frenzy);
    for _ in 0..100 {
        run.app.update();
    }
    assert_eq!(
        crate::spell_cast::powers::power_value(
            run.app.world(),
            actor,
            shared::components::PowerType::ComboPoints
        )
        .unwrap()
        .0,
        5.0
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 439531, Some(target), 1);
    let before = burst_damage(&run);
    feral_growth_tests::cast_native_spell_after_regeneration(&mut run, 22568);
    assert!(crate::auras::has_aura(run.app.world(), target, 439531));
    assert!(
        burst_damage(&run) > before,
        "a real landed Bite on its owned Vines must burst before expiry"
    );
}

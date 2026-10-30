//! Guardian MID2 4pc extends the real Incarnation from real Thrash casts.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Guardian MID2 two-piece and native world data"]
fn guardian_thrash_earns_gorestained_claws_for_mangle() {
    use shared::protocol::CombatLogKind;

    let mut run = prepare_registered_player(104, 1).unwrap();
    let (actor, target) = (run.actor, run.targets[0]);
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let set = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1296607 && aura.caster == actor)
        .unwrap()
        .instance_id;
    assert!(crate::auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        set,
        0,
        100.0
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 1301286));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false)
        .unwrap();
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1301286),
        "Mangle consumes Gorestained Claws"
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(33917)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_thrash_extends_incarnation_to_native_five_second_cap() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let expiry = |run: &Run| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 102558 && aura.caster == actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let initial = expiry(&run);
    for _ in 0..11 {
        crate::spell_cast::scripts::reduce_cooldown(run.app.world_mut(), actor, 77758, f64::MAX);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
            .unwrap();
        for _ in 0..35 {
            run.app.update();
        }
    }
    let final_expiry = expiry(&run);
    assert!(
        (final_expiry - initial - 5.0).abs() < 0.001,
        "eleven real Thrash casts extend Incarnation only 5 seconds: {initial} -> {final_expiry}"
    );
}

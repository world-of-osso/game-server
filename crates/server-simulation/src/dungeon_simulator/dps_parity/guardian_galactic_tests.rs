//! Galactic Guardian earns a background Moonfire and a charged foreground buff.
use super::*;
use crate::auras::{self, ActiveAuras};
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires Guardian Galactic Guardian profile and native world data"]
fn guardian_galactic_mangle_does_not_proc_from_its_self_effect() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 { run.app.update(); }
    let passive = run.app.world().get::<ActiveAuras>(actor).unwrap().iter()
        .find(|a| a.spell_id == 203964 && a.caster == actor).unwrap().instance_id;
    assert!(auras::set_effect_amount(run.app.world_mut(), actor, passive, 0, 100.0));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false).unwrap();
    for _ in 0..10 { run.app.update(); }
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    let direct: Vec<_> = events.iter().filter(|e| e.spell_id == Some(164812) && e.kind == CombatLogKind::Damage && !e.periodic).collect();
    assert_eq!(direct.len(), 1, "one damaging Mangle must earn exactly one Moonfire: {direct:?}");
    assert_eq!(direct[0].target, Some(target.to_bits()));
}

#[test]
#[ignore = "requires Guardian Galactic Guardian profile and native world data"]
fn guardian_galactic_proc_casts_and_empowers_moonfire() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let passive = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 203964 && a.caster == actor)
        .unwrap()
        .instance_id;
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        100.0
    ));
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.spell_id == Some(164812)
                    && e.kind == CombatLogKind::Damage
                    && !e.periodic
                    && e.target == Some(target.to_bits())
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert!(auras::has_aura(run.app.world(), actor, 213708));
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        passive,
        0,
        0.0
    ));
    assert!(
        count(&run) >= 1,
        "actual Thrash proc must apply background Moonfire damage"
    );
    for _ in 0..30 {
        run.app.update();
    }
    let before = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 8921, Some(target), false)
        .unwrap();
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    for _ in 0..3 {
        run.app.update();
    }
    assert!(
        !auras::has_aura(run.app.world(), actor, 213708),
        "foreground Moonfire consumes the buff"
    );
    assert!(
        (after - before - 80.0).abs() < 0.1,
        "native Galactic Guardian gives 8 rage, got {before} -> {after}"
    );
}

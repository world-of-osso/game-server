//! An already charged Galactic Guardian background cast earns Moon Guardian Rage.
use super::*;
use crate::auras::{self, ActiveAuras};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires Guardian Moon Guardian profile and native world data"]
fn guardian_charged_background_moonfire_earns_eight_rage() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(auras::has_aura(run.app.world(), actor, 429520));
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
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 213708, Some(actor), 1);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Energize
                && event.spell_id == Some(8921)
                && event.amount == 80
                && event.overflow == 0),
        "charged proc must grant 8 rage"
    );
    assert!(
        auras::has_aura(run.app.world(), actor, 213708),
        "the proc replaces its consumed charge"
    );
}

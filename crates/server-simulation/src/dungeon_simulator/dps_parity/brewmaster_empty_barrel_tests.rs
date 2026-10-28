//! Real selected Bring Me Another and Keg Smash interaction.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn infusion_earns_empty_barrel_and_keg_smash_consumes_it() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1265307),
        "selected Bring Me Another must earn Empty Barrel from a real brew"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .unwrap();
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1265307),
        "Keg Smash must consume Empty Barrel"
    );
    for _ in 0..20 {
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
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1265133)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual Empty Barrel damage must reach the Keg Smash target"
    );
}

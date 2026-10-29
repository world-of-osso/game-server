//! Selected Potential Energy and Harmonic Surge use real Brewmaster casts.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn keg_smash_earns_a_surge_that_tiger_palm_spends() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.5;
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1270990),
        "Keg Smash earns Potential Energy"
    );
    for _ in 0..150 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    for _ in 0..250 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1270990),
        "Tiger Palm spends the surge"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(1271011)
            && event.kind == CombatLogKind::Damage
            && event.target == Some(target.to_bits())
            && event.amount > 0),
        "the primary target takes Harmonic Surge damage"
    );
    assert!(
        events.iter().any(|event| event.spell_id == Some(1271045)
            && event.kind == CombatLogKind::Heal
            && event.target == Some(actor.to_bits())
            && event.amount > 0),
        "Harmonic Surge heals the caster"
    );
}

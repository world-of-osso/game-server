//! Bring Me Another grants Refreshing Drink on a real defensive Brew.
use super::*;
use server::test_support::healer_intake::{Hit, HitKind};
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn defensive_brew_and_incoming_hit_trigger_refreshing_drink() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.5;
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115203, Some(actor), false)
        .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1265140),
        "Fortifying Brew grants Refreshing Drink"
    );
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 100_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
    for _ in 0..120 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1265140),
        "damage consumes the ready buff"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(1265145)
            && event.kind == CombatLogKind::Heal
            && event.amount > 0),
        "damage starts a real Refreshing Drink heal"
    );
}

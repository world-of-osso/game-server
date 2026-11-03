//! Proc recipients respect combat state without affecting unrelated area payloads.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires spec251 profile, traits, and native world data"]
fn frost_wave_hits_engaged_enemies_not_an_idle_outsider() {
    let mut run = prepare_measurement(251, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let idle = spawn_level_dummy(&mut run.app, actor, 90, 1).unwrap();
    run.app
        .world_mut()
        .entity_mut(idle)
        .insert(shared::components::Position {
            x: 25.0,
            y: 0.0,
            z: 0.0,
        });
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 439851),
        "native Wave talent must be selected"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 439843, Some(primary), false)
        .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let hits = |target: Entity| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(435802)
                    && event.kind == CombatLogKind::Damage
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    assert!(hits(primary) > 0, "the real Wave must hit an engaged enemy");
    assert_eq!(hits(idle), 0, "Wave must not select an idle enemy");
}

#[test]
#[ignore = "requires spec104 profile, traits, and native world data"]
fn guardian_spirit_attacks_engaged_enemies_not_an_idle_outsider() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let idle = spawn_level_dummy(&mut run.app, actor, 90, 1).unwrap();
    run.app
        .world_mut()
        .entity_mut(idle)
        .insert(shared::components::Position {
            x: 25.0,
            y: 0.0,
            z: 0.0,
        });
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1308399, Some(actor), 1);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(primary), false)
        .unwrap();
    for _ in 0..50 {
        run.app.update();
    }
    let hits = |target: Entity| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(1308093)
                    && event.kind == CombatLogKind::Damage
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    assert!(
        hits(primary) > 0,
        "Spirit's Wrath must hit an engaged enemy"
    );
    assert_eq!(
        hits(idle),
        0,
        "Spirit's Wrath must not select an idle enemy"
    );
}

#[test]
#[ignore = "requires spec252 profile, traits, and native world data"]
fn unholy_undeath_spreads_to_engaged_enemies_not_an_idle_outsider() {
    let mut run = prepare_literal(252, 2).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let other = run.targets[1];
    let idle = spawn_level_dummy(&mut run.app, actor, 90, 2).unwrap();
    run.app
        .world_mut()
        .entity_mut(idle)
        .insert(shared::components::Position {
            x: 25.0,
            y: 0.0,
            z: 0.0,
        });
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 444033),
        "Famine must be selected"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 444633, Some(primary), 1);
    assert!(
        crate::auras::has_aura(run.app.world(), primary, 444633),
        "primary has real Undeath"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 55090, Some(primary), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), other, 444633),
        "Undeath spreads to an engaged enemy"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), idle, 444633),
        "Undeath cannot spread to an idle enemy"
    );
}

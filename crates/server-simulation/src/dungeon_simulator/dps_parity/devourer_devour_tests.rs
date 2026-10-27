//! Native Devourer Devour resource relationship.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_devour_executes_native_fury_payload() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1213809, Some(actor), 1);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta setup"
    );
    // Meta's native 201453 pacify expires on its own; do not bypass admission.
    for _ in 0..30 {
        if !crate::auras::has_aura(run.app.world(), actor, 201453) {
            break;
        }
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 201453), "Meta pacify must expire naturally");
    let spell = literal_rotation::execution::resolve_action(run.app.world(), actor, 473662)
        .expect("native Consume resolution")
        .expect("native Consume known");
    assert_eq!(
        spell, 1217610,
        "native Meta must replace Consume with Devour"
    );
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(target)),
    )
    .expect("native Devour cast");
    for _ in 0..40 {
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let has_payload = events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(1288123)
                && event.kind == CombatLogKind::Energize
                && event.amount > 0
        });
        let has_hit = events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(1217610)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        });
        if has_payload && has_hit {
            break;
        }
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(1217610)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "Devour must land before qualifying its native payload"
    );
    // SimC76f8118 sc_demon_hunter.cpp5773–5780: Devour executes native energize1288123.
    let fury = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(1288123)
                    && event.kind == CombatLogKind::Energize
                    && event.amount > 0
            }),
        "Devour must execute the native Fury payload; Fury={fury}"
    );
    assert!(fury > 0.0, "Devour must increase the native Fury pool");
}

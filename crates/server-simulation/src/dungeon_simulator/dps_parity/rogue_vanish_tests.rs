//! SimC76f8118 rogue6103–6125/7419–7444: actual Vanish lifecycle.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn rogue_vanish_native_cast_suspends_weapons_expires_to_stealth_and_earns_garrote_window() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..140 {
        run.app.update();
    }
    assert!(!crate::auras::rogue::improved_garrote_active(
        run.app.world(),
        actor,
        703
    ));
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            }),
        "real weapon swings before Vanish"
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(1856, actor, Some(target)),
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 11327),
        "successful native1856 must grant its actual11327 Vanish stealth"
    );
    assert!(crate::auras::rogue::improved_garrote_active(
        run.app.world(),
        actor,
        703
    ));
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            }),
        "Vanish cancels weapons; natural expiry must not restart them"
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 11327));
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1784),
        "natural Vanish expiry grants actual Stealth, rather than breaking concealment"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 392401),
        "natural expiry must preserve concealment without prematurely earning the post-stealth Garrote window"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 392401),
        "breaking earned stealth grants the native Improved Garrote window"
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            }),
        "actual harmful re-entry resumes native weapons"
    );
}

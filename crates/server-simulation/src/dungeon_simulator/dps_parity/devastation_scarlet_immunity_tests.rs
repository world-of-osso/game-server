//! Scarlet Adaptation during an existing heal and intact aura immunity.
use super::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Devastation profile and world DB"]
fn scarlet_periodic_heal_survives_full_twilight_trance() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let enemy = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 372469));
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 365937,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
    health.current = health.max / 2.0;
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 361509, Some(actor), 1);
    assert!(crate::auras::has_aura(run.app.world(), actor, 361509));
    assert!(crate::auras::has_aura(run.app.world(), actor, 372470));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(enemy), false)
        .unwrap();
    for _ in 0..60 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), actor, 372470) {
            break;
        }
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 372470));
    assert!(crate::auras::has_aura(run.app.world(), actor, 361509));
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: enemy,
            target: actor,
            spell_id: 210313,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(run.app.world(), actor, 210313));
    assert!(crate::auras::has_aura(run.app.world(), actor, 361509));
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[before..];
    assert!(
        events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && event.spell_id == Some(361509)
                && event.kind == CombatLogKind::Heal
        }),
        "the existing Living Flame heal must tick during Twilight Trance"
    );
    assert!(crate::auras::has_aura(run.app.world(), actor, 210313));
    assert!(!crate::auras::has_aura(run.app.world(), actor, 372470));
}

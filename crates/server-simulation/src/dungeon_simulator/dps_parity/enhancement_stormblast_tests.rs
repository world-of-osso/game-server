//! Actual Strike weapon hits must earn Stormsurge and fund the next native Stormblast.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_stormstrike_earns_stormsurge_and_next_weapon_hits_emit_stormblast() {
    // SimC76f8118 sc_shaman.cpp4702–4711/12125–12147: each actual weapon child
    // rolls Stormsurge; previously earned Stormblast is snapshotted by the next Strike.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(319930)
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 470466));
    engage(&mut run);
    let target = run.targets[0];
    for _ in 0..60 {
        enhancement_generation_tests::cast(&mut run, 17364, target);
        if crate::auras::has_aura(run.app.world(), run.actor, 470466) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 470466),
        "actual native Strike weapon hits must earn Stormblast through Stormsurge"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 201846),
        "Stormblast is earned together with the native Stormsurge proc"
    );
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    enhancement_generation_tests::cast(&mut run, 17364, target);
    let blasts: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0[before..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(390287)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    assert!(
        blasts.len() >= 2,
        "earned Stormblast must emit damage for both real equipped weapon hits: {blasts:?}"
    );
    assert!(
        blasts.iter().all(|event| !event.crit),
        "Stormblast copies the resolved weapon result and never rolls another crit"
    );
}

//! SimC76f8118 sc_priest.cpp344–350/370–413: earned Rift enables native Void Blast.
use super::*;
use crate::spell_cast::{knowledge::resolve_action, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_void_blast_real_torrent_earns_and_expires_ordinary_native_action() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    assert_eq!(
        resolve_action(run.app.world(), actor, 450983).unwrap(),
        None,
        "no Rift means no ordinary Void Blast"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(263165, primary, None),
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 459314),
        "real Torrent earned Rift"
    );
    for _ in 0..100 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_none()
        {
            break;
        }
        run.app.update();
    }
    assert_eq!(
        resolve_action(run.app.world(), actor, 450983).unwrap(),
        Some(450983),
        "earned native Rift enables an actual known-parent Void Blast, not a learned grant"
    );
    assert_eq!(
        resolve_action(run.app.world(), actor, 8092).unwrap(),
        None,
        "literal Mind Blast is unavailable during the earned replacement window"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(450983, primary, None),
    )
    .expect("ordinary native Void Blast is admitted after real Torrent");
    for _ in 0..80 {
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
                    && event.target == Some(primary.to_bits())
                    && event.spell_id == Some(450983)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "ordinary Void Blast pays real native damage"
    );
    for _ in 0..400 {
        run.app.update();
    }
    assert_eq!(
        resolve_action(run.app.world(), actor, 450983).unwrap(),
        None,
        "Rift expiry removes earned permission"
    );
    assert_eq!(
        resolve_action(run.app.world(), actor, 8092).unwrap(),
        Some(8092),
        "ordinary Mind Blast returns after Rift expiry"
    );
}

//! Native aura264: TC Spell.cpp5774–5791 and Unit.cpp2259.
use super::test_harness::*;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const TURTLE: u32 = 186_265;
const TURTLE_PACIFY: u32 = 205_769;
const STEADY_SHOT: u32 = 56_641;
const PHALANX: u32 = 287_353;

#[test]
fn native_attack_restriction264_turtle_rejects_ranged_cast_before_pacify() {
    let mut h = harness(Vec::new(), &[TURTLE, TURTLE_PACIFY, STEADY_SHOT], 3)
        .expect("native Hunter spells");
    h.cast_and_deliver(TURTLE, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        TURTLE_PACIFY
    ));
    h.advance(31);
    h.inbox.go.clear();
    h.cast_and_deliver(STEADY_SHOT, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CantDoThatRightNow);
    assert!(h.inbox.go.is_empty());
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
}

#[test]
fn native_attack_restriction264_phalanx_stops_npc_swings_until_removed() {
    let mut h = harness(Vec::new(), &[PHALANX], 1).expect("native Phalanx");
    // A real native buff with264 but no25/60 pacify or263 cast restriction.
    super::cast_triggered(h.app.world_mut(), h.npc, PHALANX, Some(h.npc), 0);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, PHALANX));
    assert!(
        !h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .is_pacified()
    );
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::combat::AutoAttack::new(h.caster));
    h.advance(5);
    let npc_swings = |h: &Harness| {
        h.inbox
            .log
            .iter()
            .filter(|event| {
                event.source == Some(h.npc.to_bits())
                    && event.spell_id.is_none()
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .count()
    };
    assert_eq!(npc_swings(&h), 0, "{:?}", h.inbox.log);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == PHALANX,
    );
    h.advance(45);
    assert!(
        npc_swings(&h) > 0,
        "normal swings resume after native aura removal"
    );
}

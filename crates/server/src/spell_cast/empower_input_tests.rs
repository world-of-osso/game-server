//! Actual player protocol input: release differs from cancel and waits for native stage1.
use super::test_harness::{Harness, harness, world_db};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::casting::CastState;
use shared::protocol::{CombatChannel, CombatLogKind, ReleaseEmpowerSpell, StopSpellCast};

const FIRE_BREATH: u32 = 357208;
const FIRE_DAMAGE: u32 = 357209;

fn player_empower() -> Harness {
    let mut h = harness(Vec::new(), &[FIRE_BREATH], 13).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ReleaseEmpowerSpell>::default(),
        MessageReceiver::<ReleaseEmpowerSpell>::default(),
    ));
    h.cast(FIRE_BREATH, Some(h.caster));
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h
}

fn release(h: &mut Harness, spell_id: u32) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<ReleaseEmpowerSpell>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(ReleaseEmpowerSpell { spell_id });
}

fn fire_damage(h: &Harness) -> bool {
    h.inbox.log.iter().any(|event| {
        event.source == Some(h.caster.to_bits())
            && event.spell_id == Some(FIRE_DAMAGE)
            && event.kind == CombatLogKind::Damage
    })
}

#[test]
#[ignore = "RED: explicit player empower release has no server consumer"]
fn native_empower_input_early_release_waits_for_stage_one_then_fires() {
    let mut h = player_empower();
    release(&mut h, FIRE_BREATH);
    h.advance(10);
    assert!(
        !fire_damage(&h),
        "early release must not fire before native first-stage threshold"
    );
    h.advance(12);
    assert!(
        fire_damage(&h),
        "matching release must fire native damage at stage1, not max hold"
    );
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == h.caster)
        .unwrap();
    assert_eq!(aura.duration_ms, 20_000);
}

#[test]
#[ignore = "control: wrong-spell release is ignored and StopSpellCast remains cancellation"]
fn native_empower_input_wrong_spell_does_not_release_and_stop_never_fires() {
    let mut h = player_empower();
    release(&mut h, 359073);
    h.advance(30);
    assert!(!fire_damage(&h));
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    h.advance(80);
    assert!(
        !fire_damage(&h),
        "cancel input must remain distinct from release input"
    );
}

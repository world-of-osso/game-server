//! Focused native channel lifecycle controls; no CastState or aura injection.
use super::test_harness::*;
use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::protocol::CombatLogKind;

fn advance_seconds(h: &mut Harness, seconds: f64) {
    h.advance((seconds / 0.05).ceil() as usize);
}

#[test]
#[ignore = "development channel-lifecycle control"]
fn channel_completion_order_native_self_channel_with_no_recipients_finishes() {
    const STOMP: u32 = 374720;
    let mut h = harness(Vec::new(), &[STOMP, 374727, 374731], 1).unwrap();
    // Empty-recipient negative: remove the unused default Player before the
    // native NPC self-channel. No positive recipient or aura is manufactured.
    h.app.world_mut().despawn(h.caster);
    let npc = h.npc;
    super::creature_cast(h.app.world_mut(), npc, STOMP, None, false)
        .expect("native self-channel admits an empty recipient set");
    let cast = h.app.world().get::<CastState>(npc).unwrap();
    assert_eq!(cast.cast_type, CastType::Channel);
    let remaining = cast.duration - cast.elapsed;
    let recipients = h
        .app
        .world_mut()
        .query::<&crate::auras::ActiveAuras>()
        .iter(h.app.world())
        .filter(|auras| auras.iter().any(|aura| aura.spell_id == STOMP))
        .count();
    assert_eq!(recipients, 0);
    advance_seconds(&mut h, f64::from(remaining) + 0.1);
    assert!(h.app.world().get::<CastState>(npc).is_none());
    assert!(h.health(npc) > 0.0);
}

#[test]
#[ignore = "development channel-lifecycle control"]
fn channel_completion_order_native_hymn_recipient_expires_on_its_own_after_completion() {
    const HYMN: u32 = 64843;
    const RECIPIENT: u32 = 64844;
    let mut h = harness(Vec::new(), &[HYMN, RECIPIENT], 5).unwrap();
    let caster = h.caster;
    h.cast_and_deliver(HYMN, Some(caster));
    assert!(
        h.inbox.failed.is_empty(),
        "native Hymn admission: {:?}",
        h.inbox.failed
    );
    let cast = h
        .app
        .world()
        .get::<CastState>(caster)
        .expect("native Hymn channel");
    assert_eq!(cast.cast_type, CastType::Channel);
    let remaining = cast.duration - cast.elapsed;
    advance_seconds(&mut h, f64::from(remaining) + 0.1);
    assert!(h.app.world().get::<CastState>(caster).is_none());
    assert!(!crate::auras::has_aura(h.app.world(), caster, HYMN));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == RECIPIENT && aura.caster == caster)
        .expect("native1250ms Hymn pulses apply the real15s recipient aura");
    let expiry = aura.expires_at.expect("native recipient finite duration");
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    assert!(
        remaining > 0.1,
        "recipient owns a lifetime beyond the completed root channel"
    );
    h.advance(((remaining - 0.1) / 0.05).floor() as usize);
    assert!(crate::auras::has_aura(h.app.world(), caster, RECIPIENT));
    h.advance(4);
    assert!(!crate::auras::has_aura(h.app.world(), caster, RECIPIENT));
}

#[test]
#[ignore = "development channel-lifecycle control"]
fn channel_completion_order_native_counterspell_interrupt_removes_the_actual_channel_recipient() {
    const DRAIN: u32 = 234153;
    const COUNTERSPELL: u32 = 2139;
    let mut h = harness(Vec::new(), &[DRAIN, COUNTERSPELL], 9).unwrap();
    let caster = h.caster;
    let npc = h.npc;
    h.cast_and_deliver(DRAIN, Some(npc));
    assert!(
        h.inbox.failed.is_empty(),
        "native Drain Life admission: {:?}",
        h.inbox.failed
    );
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().cast_type,
        CastType::Channel
    );
    assert!(crate::auras::has_aura(h.app.world(), npc, DRAIN));
    super::creature_cast(h.app.world_mut(), npc, COUNTERSPELL, Some(caster), false)
        .expect("actual native Counterspell, not an injected interrupt state");
    h.advance(3);
    assert!(h.app.world().get::<CastState>(caster).is_none());
    assert!(!crate::auras::has_aura(h.app.world(), npc, DRAIN));
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.spell_id == Some(COUNTERSPELL)
                && event.extra_spell_id == Some(DRAIN))
    );
    let health = h.health(npc);
    h.advance(30);
    assert_eq!(
        h.health(npc),
        health,
        "no channel ticks after the real interrupt"
    );
}

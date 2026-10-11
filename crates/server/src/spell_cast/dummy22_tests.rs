//! Native Mainline15407 channel: six authored ticks, damage/slow and Insanity.
use super::test_harness::*;
use lightyear::prelude::MessageSender;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;
use shared::protocol::{CombatChannel, StopSpellCast};

fn mind_flay() -> Harness {
    let mut spell = world_db().unwrap().store.get(15407).unwrap().clone();
    for effect in &mut spell.effects {
        effect.variance = 0.0;
    }
    let mut h = harness(vec![spell], &[15407], 5).unwrap();
    h.set_power(PowerType::Insanity, 0.0);
    h
}

#[test]
fn dummy22_mind_flay_native_channel_pays_six_three_insanity_ticks() {
    let mut h = mind_flay();
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 15407));
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(15407) && e.kind == CombatLogKind::Energize)
        .collect();
    assert_eq!(grants.len(), 6, "15407s4 is the authored tick count");
    assert!(grants.iter().all(|e| e.amount == 300 && e.periodic));
    assert_eq!(h.power(PowerType::Insanity), 1800.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(15407) && e.kind == CombatLogKind::Damage && e.periodic)
            .count(),
        6
    );
}

#[test]
fn dummy22_mind_flay_has_no_initial_grant_and_first_tick_is_native_750ms() {
    let mut h = mind_flay();
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(10);
    assert_eq!(h.power(PowerType::Insanity), 0.0);
    h.advance(6);
    assert_eq!(h.power(PowerType::Insanity), 300.0);
}

#[test]
fn dummy22_mind_flay_cancel_stops_both_target_damage_and_self_resource() {
    let mut h = mind_flay();
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(17);
    assert_eq!(h.power(PowerType::Insanity), 300.0);
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(3);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 15407));
    assert!(!crate::auras::has_aura(h.app.world(), target, 15407));
    h.inbox.log.clear();
    h.advance(100);
    assert_eq!(h.power(PowerType::Insanity), 300.0);
    assert!(h.inbox.log.iter().all(|e| e.spell_id != Some(15407)));
}

#[test]
fn dummy22_mind_flay_raw_insanity_cap_reports_each_tick_overflow() {
    let mut h = mind_flay();
    h.set_power(PowerType::Insanity, 9900.0);
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(100);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(15407) && e.kind == CombatLogKind::Energize)
        .map(|e| (e.amount, e.overflow))
        .collect();
    assert_eq!(
        grants,
        vec![
            (300, 200),
            (300, 300),
            (300, 300),
            (300, 300),
            (300, 300),
            (300, 300)
        ]
    );
    assert_eq!(h.power(PowerType::Insanity), 10000.0);
}

#[test]
fn dummy22_mind_flay_friendly_rejection_pays_no_resource() {
    let mut h = mind_flay();
    let ally = spawn_player(&mut h.app, 5, 3.0);
    h.cast_and_deliver(15407, Some(ally));
    h.advance(100);
    assert!(!h.inbox.failed.is_empty());
    assert!(h.inbox.go.is_empty());
    assert_eq!(h.power(PowerType::Insanity), 0.0);
}

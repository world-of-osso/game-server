//! Queue timing through the real connection, fixed ticks, spell effects and failure messages.
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Position, PowerType};
use shared::protocol::{CombatChannel, CombatLogKind, StopSpellCast};
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::spell_info::SpellCastTime;

const ONSLAUGHT: u32 = 315720;
const ONSLAUGHT_DAMAGE: u32 = 396718;
const SLAM: u32 = 1464;
const MORTAL_STRIKE: u32 = 12294;
const CHARGE: u32 = 100;
const FIREBALL: u32 = 133;

fn delayed_builder() -> Harness {
    let data = world_db().expect("owned world.db");
    let mut builder = data.store.get(ONSLAUGHT).unwrap().clone();
    // Onslaught is instant in Retail. Only its duration is controlled here to put its
    // real 300-raw-rage effect after queue admission, at the cast-completion boundary.
    builder.misc.as_mut().unwrap().cast_time = Some(SpellCastTime {
        base_ms: 1500,
        minimum_ms: 1500,
    });
    harness(vec![builder], &[ONSLAUGHT_DAMAGE, SLAM, MORTAL_STRIKE], 1).unwrap()
}

fn queue_during_builder(h: &mut Harness, spender: u32) {
    h.cast_and_deliver(ONSLAUGHT, Some(h.npc));
    h.advance(20);
    assert_eq!(h.power(PowerType::Rage), 0.0);
    h.cast_and_deliver(spender, Some(h.npc));
    h.advance(2);
    assert!(
        h.inbox.failed.is_empty(),
        "queue admission: {:?}",
        h.inbox.failed
    );
    assert_eq!(h.power(PowerType::Rage), 0.0, "builder has not completed");
}

fn successes(h: &Harness) -> Vec<u32> {
    h.inbox
        .log
        .iter()
        .filter(|e| e.kind == CombatLogKind::CastSuccess)
        .filter_map(|e| e.spell_id)
        .collect()
}

#[test]
fn spellqueue_builder_effect_precedes_real_rage_spender() {
    let mut h = delayed_builder();
    queue_during_builder(&mut h, SLAM);
    h.advance(10);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), 100.0, "300 gained, 200 spent");
    let events = &h.inbox.log;
    let gain = events
        .iter()
        .position(|e| e.kind == CombatLogKind::Energize && e.spell_id == Some(ONSLAUGHT))
        .unwrap();
    let spender = events
        .iter()
        .position(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(SLAM))
        .unwrap();
    assert!(gain < spender, "effects before queued validation");
}

#[test]
fn spellqueue_gcd_only_accepts_unaffordable_spender_then_real_charge_funds_it() {
    let mut h = harness(Vec::new(), &[ONSLAUGHT, ONSLAUGHT_DAMAGE, SLAM, CHARGE], 1).unwrap();
    h.cast_and_deliver(ONSLAUGHT, Some(h.npc));
    h.set_power(PowerType::Rage, 0.0);
    h.advance(22);
    h.cast_and_deliver(SLAM, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    let charge_target = spawn_npc(&mut h.app, 10.0);
    // A granted/off-GCD effect funds the held request; a new player intent would
    // supersede it. Use Charge's real effect path, not a manual pool assignment.
    super::cast_granted(h.app.world_mut(), h.caster, CHARGE, Some(charge_target)).unwrap();
    h.advance(10);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(successes(&h).contains(&SLAM));
    assert_eq!(h.power(PowerType::Rage), 0.0, "Charge's 200 spent by Slam");
}

#[test]
fn spellqueue_still_unaffordable_fails_only_at_start() {
    let mut h = harness(Vec::new(), &[FIREBALL, SLAM], 1).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(27);
    h.cast_and_deliver(SLAM, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "admission: {:?}", h.inbox.failed);
    h.advance(12);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].spell_id, SLAM);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotEnoughResource);
    assert_eq!(h.inbox.failed[0].detail.as_deref(), Some("Rage"));
    assert!(!successes(&h).contains(&SLAM));
}

#[test]
fn spellqueue_outside_window_preserves_immediate_cast_and_gcd_rejection() {
    for cast_time in [false, true] {
        let mut h = if cast_time {
            delayed_builder()
        } else {
            harness(Vec::new(), &[ONSLAUGHT, ONSLAUGHT_DAMAGE, SLAM], 1).unwrap()
        };
        h.cast_and_deliver(ONSLAUGHT, Some(h.npc));
        h.cast_and_deliver(SLAM, Some(h.npc));
        h.advance(2);
        assert_eq!(h.inbox.failed.len(), 1);
        assert_eq!(
            h.inbox.failed[0].reason,
            if cast_time {
                CastFailReason::SpellInProgress
            } else {
                CastFailReason::OnGlobalCooldown
            }
        );
        h.advance(40);
        assert!(!successes(&h).contains(&SLAM));
    }
}

#[test]
fn spellqueue_newer_request_replaces_pending_spell() {
    let mut h = delayed_builder();
    queue_during_builder(&mut h, SLAM);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!successes(&h).contains(&SLAM));
    assert!(successes(&h).contains(&MORTAL_STRIKE));
    assert_eq!(h.power(PowerType::Rage), 0.0);
}

#[test]
fn spellqueue_explicit_stop_cancels_current_and_pending_cast() {
    let mut h = delayed_builder();
    queue_during_builder(&mut h, SLAM);
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(15);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(successes(&h).is_empty());
    assert_eq!(h.power(PowerType::Rage), 0.0);
    assert!(h.inbox.failed.is_empty(), "pending canceled, not attempted");
}

#[test]
fn spellqueue_hostile_interrupt_retains_pending_and_revalidates_school_lock() {
    let mut h = harness(Vec::new(), &[FIREBALL], 8).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(27);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let until = h.app.world().resource::<Time>().elapsed_secs_f64() + 6.0;
    h.app
        .world_mut()
        .get_mut::<super::SpellHistory>(h.caster)
        .unwrap()
        .lock_schools(4, until);
    let npc = h.npc;
    super::interrupt_cast(
        h.app.world_mut(),
        h.caster,
        CastFailReason::Interrupted,
        Some(npc),
    );
    h.advance(12);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::SchoolLockedOut);
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn spellqueue_death_revalidates_pending_once_without_executing_it() {
    let mut h = delayed_builder();
    queue_during_builder(&mut h, SLAM);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.advance(15);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CasterDead);
    assert!(successes(&h).is_empty());
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 1000.0;
    h.set_power(PowerType::Rage, 500.0);
    h.advance(10);
    assert!(
        successes(&h).is_empty(),
        "no stale queue after resurrection"
    );
}

#[test]
fn spellqueue_exact_750ms_boundary_is_inclusive() {
    let mut h = harness(Vec::new(), &[FIREBALL, SLAM], 1).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    let now = h.app.world().resource::<Time>().elapsed_secs_f64();
    let mut cast = h.app.world_mut().get_mut::<CastState>(h.caster).unwrap();
    cast.duration = 0.75;
    cast.elapsed = 0.0;
    h.app
        .world_mut()
        .get_mut::<super::SpellHistory>(h.caster)
        .unwrap()
        .gcd_ready_at
        .insert(133, now + 0.75);
    h.set_power(PowerType::Rage, 200.0);
    let request = shared::protocol::SpellCastIntent {
        target_item_guid: None,
        destination: None,
        spell_id: Some(SLAM),
        spell: String::new(),
        target_entity: Some(h.npc.to_bits()),
        witness: None,
    };
    let result = super::request_cast(h.app.world_mut(), h.caster, &request);
    assert_eq!(result, Ok(()), "750ms is inside the queue window");
    h.advance(15);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), 0.0);
    assert!(successes(&h).contains(&SLAM));
}

#[test]
fn spellqueue_target_range_is_checked_at_start_not_admission() {
    let mut h = delayed_builder();
    h.cast_and_deliver(ONSLAUGHT, Some(h.npc));
    h.advance(22);
    let distant = spawn_npc(&mut h.app, 100.0);
    h.cast_and_deliver(SLAM, Some(distant));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.app.world_mut().get_mut::<Position>(distant).unwrap().x = 3.0;
    h.advance(10);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(successes(&h).contains(&SLAM));
}

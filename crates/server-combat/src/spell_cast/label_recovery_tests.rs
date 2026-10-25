//! TC a352b1fa SpellHistory.cpp448–451 / SpellAuraEffects.cpp6271–6284.
//! Normal native Autumn leaf casts; no Season-cycle or charge-recovery claim.
use super::test_harness::*;
use crate::auras::ActiveAuras;
use lightyear::prelude::MessageSender;
use shared::protocol::{CancelAura, CombatChannel};
use shared::spell_data::CastFailReason;

const AUTUMN: u32 = 388_010;
const SEASONS_OVERRIDE: u32 = 388_015;
const DIVINE_PROTECTION: u32 = 498;
const HAMMER_OF_JUSTICE: u32 = 853;
const HEALING_POTION: u32 = 439;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            AUTUMN,
            SEASONS_OVERRIDE,
            DIVINE_PROTECTION,
            HAMMER_OF_JUSTICE,
            HEALING_POTION,
        ],
        2,
    )
    .unwrap()
}

fn cast_autumn(h: &mut Harness, recipient: bevy::prelude::Entity) {
    h.cast_and_deliver(AUTUMN, Some(recipient));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(recipient)
            .unwrap()
            .has_spell(AUTUMN)
    );
    h.advance(35);
}

fn cooldown(h: &Harness, id: u32) -> &shared::protocol::SpellCooldownUpdate {
    h.inbox
        .cooldowns
        .iter()
        .rev()
        .find(|update| update.spell_id == id && !update.is_gcd)
        .unwrap()
}

#[test]
fn native_label_recovery143_scales_matching_spell_cooldown() {
    let mut h = fixture();
    let player = h.caster;
    cast_autumn(&mut h, player);
    h.cast_and_deliver(DIVINE_PROTECTION, Some(player));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    // Native label16, 60000ms × 100/(100 + Autumn's30), integer truncation.
    assert_eq!(cooldown(&h, DIVINE_PROTECTION).duration_ms, 46_153);
}

#[test]
fn native_label_recovery143_scales_matching_category_cooldown() {
    let mut h = fixture();
    let player = h.caster;
    cast_autumn(&mut h, player);
    h.cast_and_deliver(HAMMER_OF_JUSTICE, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    // Native label16, category32's45000ms ×100/130.
    assert_eq!(cooldown(&h, HAMMER_OF_JUSTICE).duration_ms, 34_615);
}

#[test]
fn native_label_recovery143_leaves_unlabelled_potion_cooldown_unchanged() {
    let mut h = fixture();
    let player = h.caster;
    cast_autumn(&mut h, player);
    h.cast_and_deliver(HEALING_POTION, Some(player));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(cooldown(&h, HEALING_POTION).duration_ms, 60_000);
}

fn timer(h: &Harness) -> (f64, f64) {
    let world = h.app.world();
    (
        super::history_now(world),
        world
            .get::<super::SpellHistory>(h.caster)
            .unwrap()
            .spell_ready_at[&DIVINE_PROTECTION],
    )
}

fn assert_retimed(h: &Harness, before: (f64, f64), factor: f64) {
    let (after_time, after_deadline) = timer(h);
    let at_start = before.0 + (before.1 - before.0) * factor;
    let at_end = after_time + (before.1 - after_time) * factor;
    // The actual cast/removal is within these observed delivery frames.
    // Native integer-millisecond truncation contributes at most one ms.
    assert!(
        (at_start.min(at_end) - 0.001..=at_start.max(at_end) + 0.001).contains(&after_deadline),
        "before={before:?} after={:?}, factor={factor}",
        timer(h)
    );
}

#[test]
fn native_label_recovery143_retimes_already_running_cooldown_on_application() {
    let mut h = fixture();
    h.cast_and_deliver(DIVINE_PROTECTION, Some(h.caster));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(cooldown(&h, DIVINE_PROTECTION).duration_ms, 60_000);
    let before = timer(&h);
    let player = h.caster;
    cast_autumn(&mut h, player);
    assert_retimed(&h, before, 100.0 / 130.0);
    assert!(cooldown(&h, DIVINE_PROTECTION).remaining_ms < 46_153);
}

#[test]
fn native_label_recovery143_cancel_restores_running_timer_rate() {
    let mut h = fixture();
    let player = h.caster;
    cast_autumn(&mut h, player);
    h.cast_and_deliver(DIVINE_PROTECTION, Some(player));
    h.advance(3);
    let before = timer(&h);
    h.app
        .world_mut()
        .get_mut::<MessageSender<CancelAura>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(CancelAura { spell_id: AUTUMN });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<ActiveAuras>(player)
            .unwrap()
            .has_spell(AUTUMN)
    );
    assert_retimed(&h, before, 130.0 / 100.0);
}

#[test]
fn native_label_recovery143_expiry_restores_rate_and_actual_recast_boundary() {
    let mut h = fixture();
    let player = h.caster;
    cast_autumn(&mut h, player);
    h.cast_and_deliver(DIVINE_PROTECTION, Some(player));
    h.advance(550);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(player)
            .unwrap()
            .has_spell(AUTUMN)
    );
    let before = timer(&h);
    h.advance(15);
    assert!(
        !h.app
            .world()
            .get::<ActiveAuras>(player)
            .unwrap()
            .has_spell(AUTUMN)
    );
    assert_retimed(&h, before, 130.0 / 100.0);
    let (now, ready) = timer(&h);
    let tick = h
        .app
        .world()
        .resource::<bevy::prelude::Time>()
        .delta_secs_f64();
    h.advance(((ready - now) / tick).floor() as usize - 4);
    h.cast_and_deliver(DIVINE_PROTECTION, Some(player));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(10);
    h.cast_and_deliver(DIVINE_PROTECTION, Some(player));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(cooldown(&h, DIVINE_PROTECTION).duration_ms, 60_000);
}

#[test]
fn native_label_recovery143_recipient_not_caster_owns_rate() {
    let mut h = fixture();
    let other = spawn_player(&mut h.app, 2, 2.0);
    cast_autumn(&mut h, other);
    h.cast_and_deliver(DIVINE_PROTECTION, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(cooldown(&h, DIVINE_PROTECTION).duration_ms, 60_000);
}

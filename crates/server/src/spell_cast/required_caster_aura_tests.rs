//! TC Spell.cpp5881: native caster aura-spell identity, not an amount check.
use shared::components::Health;
use shared::protocol::SpellCooldownUpdate;
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, has_aura, remove_auras};

const VOID_TORRENT: u32 = 205065;
const VOID_ERUPTION: u32 = 228260;
const VOIDFORM: u32 = 194249;
const VOID_VOLLEY: u32 = 1242173;
const VOID_BOLT: u32 = 1242189;
const SHEILUN_GIFT: u32 = 205406;
const SHEILUN_CLOUD: u32 = 214502;

fn priest() -> Harness {
    harness(
        Vec::new(),
        &[
            VOID_TORRENT,
            VOID_ERUPTION,
            VOIDFORM,
            VOID_VOLLEY,
            VOID_BOLT,
        ],
        5,
    )
    .unwrap()
}

fn monk() -> Harness {
    harness(Vec::new(), &[SHEILUN_GIFT, SHEILUN_CLOUD], 10).unwrap()
}

fn cast_voidform(h: &mut Harness) {
    h.cast_and_deliver(VOID_ERUPTION, Some(h.npc));
    h.advance(45);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, VOIDFORM));
    h.inbox = Inbox::default();
}

fn cast_sheilun_cloud(h: &mut Harness) {
    h.cast_and_deliver(SHEILUN_CLOUD, None);
    h.advance(45); // Native two-second cast creates the sixty-second aura.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, SHEILUN_CLOUD));
    h.inbox = Inbox::default();
}

fn assert_missing_caster_aura(h: &Harness) {
    assert_caster_aura_failure(h);
    assert!(h.inbox.cooldowns.is_empty(), "{:?}", h.inbox.cooldowns);
}

fn assert_caster_aura_failure(h: &Harness) {
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::InvalidTarget);
    assert_eq!(
        h.inbox.failed[0].detail.as_deref(),
        Some("SPELL_FAILED_CASTER_AURASTATE")
    );
    assert!(h.inbox.go.is_empty(), "{:?}", h.inbox.go);
    assert!(h.inbox.charges.is_empty());
}

fn remove_caster_aura(h: &mut Harness, spell_id: u32) {
    remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == spell_id,
    );
}

#[test]
fn native_required_caster_aura_refuses_void_torrent_without_voidform_before_gcd() {
    let mut h = priest();
    assert!(!has_aura(h.app.world(), h.caster, VOIDFORM));
    h.cast_and_deliver(VOID_TORRENT, Some(h.npc));
    assert_missing_caster_aura(&h);
    h.inbox = Inbox::default();
    // No GCD was started: the ordinary producer can start immediately.
    cast_voidform(&mut h);
}

#[test]
fn native_required_caster_aura_allows_void_torrent_after_real_void_eruption() {
    let mut h = priest();
    cast_voidform(&mut h);
    h.cast_and_deliver(VOID_TORRENT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == VOID_TORRENT));
    assert!(has_aura(h.app.world(), h.npc, VOID_TORRENT));
}

#[test]
fn native_required_caster_aura_does_not_borrow_the_target_voidform() {
    let mut h = priest();
    let foreign = h.npc;
    // Exact payload of the existing Void Eruption named-buff producer.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: foreign,
            target: foreign,
            spell_id: VOIDFORM,
            effect_mask: (1 << 0) | (1 << 1),
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), foreign, VOIDFORM));
    assert!(!has_aura(h.app.world(), h.caster, VOIDFORM));
    h.cast_and_deliver(VOID_TORRENT, Some(foreign));
    assert_missing_caster_aura(&h);
}

#[test]
fn native_required_caster_aura_removal_and_expiry_restore_void_torrent_refusal() {
    for expire in [false, true] {
        let mut h = priest();
        cast_voidform(&mut h);
        if expire {
            h.advance(401); // Native twenty-second Voidform duration.
        } else {
            remove_caster_aura(&mut h, VOIDFORM);
        }
        assert!(!has_aura(h.app.world(), h.caster, VOIDFORM));
        h.inbox = Inbox::default();
        h.cast_and_deliver(VOID_TORRENT, Some(h.npc));
        assert_missing_caster_aura(&h);
    }
}

#[test]
fn native_required_caster_aura_refuses_sheilun_gift_without_cloud() {
    let mut h = monk();
    assert!(!has_aura(h.app.world(), h.caster, SHEILUN_CLOUD));
    h.cast_and_deliver(SHEILUN_GIFT, Some(h.caster));
    assert_missing_caster_aura(&h);
    assert!(h.app.world().get::<super::CastState>(h.caster).is_none());
}

#[test]
fn native_required_caster_aura_allows_sheilun_gift_heal_after_native_cloud_cast() {
    let mut h = monk();
    cast_sheilun_cloud(&mut h);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 500.0;
    h.cast_and_deliver(SHEILUN_GIFT, Some(h.caster));
    h.advance(45);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SHEILUN_GIFT));
    assert!(h.health(h.caster) > 500.0);
}

#[test]
fn native_required_caster_aura_lost_during_sheilun_cast_refuses_completion() {
    let mut h = monk();
    cast_sheilun_cloud(&mut h);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 500.0;
    h.cast_and_deliver(SHEILUN_GIFT, Some(h.caster));
    assert!(h.app.world().get::<super::CastState>(h.caster).is_some());
    remove_caster_aura(&mut h, SHEILUN_CLOUD);
    h.advance(45);
    assert_caster_aura_failure(&h);
    // The valid admission already started this native GCD; refusal adds no spell cooldown.
    assert_eq!(
        h.inbox.cooldowns,
        vec![SpellCooldownUpdate {
            spell_id: SHEILUN_GIFT,
            category: 133,
            duration_ms: 1500,
            remaining_ms: 1500,
            is_gcd: true,
        }]
    );
    assert_eq!(h.health(h.caster), 500.0);
}

#[test]
fn native_required_caster_aura_applies_to_ordinary_npc_casts() {
    let mut h = priest();
    let failure = super::creature::creature_cast(
        h.app.world_mut(),
        h.npc,
        VOID_TORRENT,
        Some(h.caster),
        false,
    )
    .expect_err("ordinary NPC needs its own native caster aura");
    let (reason, detail) = failure.to_contract();
    assert_eq!(reason, CastFailReason::InvalidTarget);
    assert_eq!(detail.as_deref(), Some("SPELL_FAILED_CASTER_AURASTATE"));
}

#[test]
fn native_required_caster_aura_refuses_full_trigger_without_voidform() {
    let mut h = priest();
    assert!(!has_aura(h.app.world(), h.caster, VOIDFORM));
    assert!(!super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        VOID_TORRENT,
        Some(h.npc),
        0,
    ));
    assert!(!has_aura(h.app.world(), h.npc, VOID_TORRENT));
}

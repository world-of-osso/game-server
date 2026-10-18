//! TC a352b1fa SpellHistory.cpp448–451 / SpellAuraEffects.cpp6271–6284.
//! Normal native Autumn leaf casts; no Season-cycle or charge-recovery claim.
use super::test_harness::*;
use crate::auras::ActiveAuras;

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
    // Native label16, category24's45000ms ×100/130.
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

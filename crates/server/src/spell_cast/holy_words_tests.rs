//! Native70448 Holy Word consequence tests, not an imported healer rotation.
use super::test_harness::{Harness, harness};

const SERENITY: u32 = 2050;
const FLASH_HEAL: u32 = 2061;
const PRAYER: u32 = 33076;
const APOTHEOSIS: u32 = 200183;
const VOICE_OF_HARMONY: u32 = 390994;
const ULTIMATE_SERENITY: u32 = 1246517;
const SERENITY_CATEGORY: u32 = 1673;

fn deadline(h: &Harness) -> f64 {
    h.app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&SERENITY_CATEGORY]
        .next_ready_at()
}

fn cast(h: &mut Harness, spell: u32) {
    h.cast_and_deliver(spell, Some(h.caster));
    h.advance(65);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn holy_words_apotheosis_increases_flash_reduction_by_native_two_hundred_percent() {
    let mut h = harness(Vec::new(), &[SERENITY, FLASH_HEAL, APOTHEOSIS], 5).unwrap();
    cast(&mut h, APOTHEOSIS);
    cast(&mut h, SERENITY);
    let before = deadline(&h);
    cast(&mut h, FLASH_HEAL);
    assert_eq!(deadline(&h), before - 18.0);
}

#[test]
fn holy_words_voice_of_harmony_mending_reduces_serenity_by_four_seconds() {
    let mut h = harness(Vec::new(), &[SERENITY, PRAYER, 41635, VOICE_OF_HARMONY], 5).unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        VOICE_OF_HARMONY
    ));
    cast(&mut h, SERENITY);
    let before = deadline(&h);
    cast(&mut h, PRAYER);
    assert_eq!(deadline(&h), before - 4.0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 41635));
}

#[test]
fn holy_words_unlearned_voice_does_not_reduce_serenity_on_mending() {
    let mut h = harness(Vec::new(), &[SERENITY, PRAYER, 41635], 5).unwrap();
    cast(&mut h, SERENITY);
    let before = deadline(&h);
    cast(&mut h, PRAYER);
    assert_eq!(deadline(&h), before);
}

#[test]
fn holy_words_ultimate_serenity_redirects_prayer_of_healing_reduction() {
    let mut h = harness(
        Vec::new(),
        &[SERENITY, 596, 98367, ULTIMATE_SERENITY, 1250548],
        5,
    )
    .unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        ULTIMATE_SERENITY
    ));
    cast(&mut h, SERENITY);
    let before = deadline(&h);
    cast(&mut h, 596);
    assert_eq!(deadline(&h), before - 6.0);
}

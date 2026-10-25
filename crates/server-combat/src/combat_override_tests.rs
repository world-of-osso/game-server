//! Real native Aura361 fields select each hand and preserve application order.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, harness_with};

const DIFFERENT_HANDS: u32 = 151791;
const MAIN_ONLY: u32 = 404542;

fn apply_native(h: &mut Harness, spell: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: spell,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn read_hand(h: &Harness, hand: Hand) -> Option<u32> {
    read_melee_override(h.app.world().get::<ActiveAuras>(h.caster), hand)
}

#[test]
fn native_aura361_selects_each_hand_and_first_applicable_in_order() {
    let mut h = harness_with(Vec::new(), &[DIFFERENT_HANDS, MAIN_ONLY], 2, true).unwrap();
    assert_eq!(read_hand(&h, Hand::Main), None);
    assert_eq!(read_hand(&h, Hand::Off), None);
    apply_native(&mut h, MAIN_ONLY);
    assert_eq!(read_hand(&h, Hand::Main), Some(408385));
    assert_eq!(read_hand(&h, Hand::Off), None);
    apply_native(&mut h, DIFFERENT_HANDS);
    assert_eq!(read_hand(&h, Hand::Main), Some(408385));
    assert_eq!(read_hand(&h, Hand::Off), Some(151788));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == MAIN_ONLY,
    );
    assert_eq!(read_hand(&h, Hand::Main), Some(151789));
    assert_eq!(read_hand(&h, Hand::Off), Some(151788));
    apply_native(&mut h, MAIN_ONLY);
    assert_eq!(read_hand(&h, Hand::Main), Some(151789));
}

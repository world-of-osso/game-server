//! Henry's actual 70448 starter loadout; no synthetic proc grants or cooldown resets.
use super::kit_clearcasting_tests::{complete_real_cast, mana_cost, own_aura, wait_for_gcd};
use super::kit_tests::at_level;
use crate::auras::has_aura;
use crate::spell_cast::creature_cast;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::casting::CastState;

const HENRY: u32 = 215011;
const FLASH_HEAL: u32 = 2061;
const SERENITY: u32 = 2050;
const SURGE: u32 = 114255;
const APOTHEOSIS: u32 = 200183;

fn acquire_surge() -> (Harness, Entity) {
    let (mut h, priest) = at_level(HENRY, 14, |_| {});
    assert!(has_aura(h.app.world(), priest, 109186));
    let chance = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(109186)
        .unwrap()
        .effects[0]
        .base_points;
    assert_eq!(chance, 8.0);
    // Finite natural acquisition confidence, not a gameplay proc guarantee.
    let bound = (1e-6_f64.ln() / (-f64::from(chance) / 100.0).ln_1p()).ceil() as usize;
    for attempt in 1..=bound {
        complete_real_cast(&mut h, priest, FLASH_HEAL);
        if own_aura(&h, priest, SURGE).is_some() {
            println!("Holy native Surge acquired after {attempt} casts (bound {bound})");
            return (h, priest);
        }
        // Ordinary creature mana regeneration remains enabled between attempts.
        h.advance(300);
    }
    panic!("HOLY_SURGE_NATIVE_PRODUCER_UNIMPLEMENTED: no proc in {bound} native healing casts");
}

#[test]
fn holy_priest_native_surge_is_instant_half_cost_and_spends_one_use() {
    let (mut h, priest) = acquire_surge();
    wait_for_gcd(&mut h, priest, FLASH_HEAL);
    let before = own_aura(&h, priest, SURGE).unwrap();
    let discounted = mana_cost(&h, priest, FLASH_HEAL);
    assert!(discounted > 0.0, "70448 Holy Surge is half-cost, not free");
    creature_cast(h.app.world_mut(), priest, FLASH_HEAL, Some(priest), false).unwrap();
    assert!(
        h.app.world().get::<CastState>(priest).is_none(),
        "native Surge Flash must be instant"
    );
    let remaining = own_aura(&h, priest, SURGE).map_or(0, |aura| aura.stacks);
    assert_eq!(
        remaining,
        before.stacks - 1,
        "one successful Flash spends exactly one use before another event can reproc"
    );
    if remaining == 0 {
        assert_eq!(mana_cost(&h, priest, FLASH_HEAL), discounted * 2.0);
    }
}

#[test]
fn holy_priest_native_apotheosis_restores_spent_serenity() {
    let (mut h, priest) = at_level(HENRY, 35, |_| {});
    complete_real_cast(&mut h, priest, SERENITY);
    complete_real_cast(&mut h, priest, APOTHEOSIS);
    assert!(has_aura(h.app.world(), priest, APOTHEOSIS));
    wait_for_gcd(&mut h, priest, SERENITY);
    assert!(
        creature_cast(h.app.world_mut(), priest, SERENITY, Some(priest), false).is_ok(),
        "HOLY_APOTHEOSIS_RESET_UNIMPLEMENTED: actual Apotheosis must restore the spent Holy Word"
    );
}

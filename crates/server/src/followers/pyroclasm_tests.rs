//! MAGE_PYROCLASM_SIMC_HOT_STREAK_TRIGGER: SimC76f8118 sc_mage.cpp2326–2428.
//! Controlled actual269650/48108/269651 applications isolate ordinary Pyroblast
//! spend and damage; natural Hot Streak acquisition has separate native tests.
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

use super::kit_proc_fidelity_tests::{complete_cast, prepare};
use crate::auras::{ActiveAuras, has_aura};
use crate::spell_cast::test_harness::Harness;

const TALENT: u32 = 269650;
const PYROCLASM: u32 = 269651;
const HOT_STREAK: u32 = 48108;
const PYROBLAST: u32 = 11366;

fn apply_native_buff(h: &mut Harness, caster: Entity, spell: u32) {
    assert!(crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        caster,
        spell,
        Some(caster),
        1
    ));
    h.advance(1);
    assert!(has_aura(h.app.world(), caster, spell));
}

fn pyroclasm_stacks(h: &Harness, caster: Entity) -> u8 {
    h.app
        .world()
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == PYROCLASM))
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "DPS parity RED: Hot Streak spend never rolls native Pyroclasm"]
fn dps_parity_pyroclasm_hot_streak_spend_uses_native_talent_chance() {
    let _seed = crate::random::seed_scope(269650);
    let (mut h, caster) = prepare(209059);
    let target = h.npc;
    apply_native_buff(&mut h, caster, TALENT);
    for _ in 0..64 {
        apply_native_buff(&mut h, caster, HOT_STREAK);
        complete_cast(&mut h, caster, PYROBLAST, target);
        assert!(!has_aura(h.app.world(), caster, HOT_STREAK));
        if pyroclasm_stacks(&h, caster) > 0 {
            return;
        }
    }
    panic!(
        "64 ordinary Hot Streak expenditures never produced269651 using actual269650e0 chance15; no proc mask/guaranteed chance was injected"
    );
}

fn cast_pyroblast_with_proc(proc: bool, instant: bool) -> (i32, bool, u8) {
    let _seed = crate::random::seed_scope(269651);
    let (mut h, caster) = prepare(209059);
    let target = h.npc;
    if proc {
        apply_native_buff(&mut h, caster, PYROCLASM);
        apply_native_buff(&mut h, caster, PYROCLASM);
        assert_eq!(pyroclasm_stacks(&h, caster), 2);
    } else {
        h.advance(2);
    }
    if instant {
        apply_native_buff(&mut h, caster, HOT_STREAK);
    }
    let first = h.inbox.log.len();
    complete_cast(&mut h, caster, PYROBLAST, target);
    let remaining = pyroclasm_stacks(&h, caster);
    h.advance(100);
    let hit = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(PYROBLAST)
                && event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
        })
        .expect("ordinary native Pyroblast must land after completion");
    (hit.amount, hit.crit, remaining)
}

#[test]
#[ignore = "DPS parity RED: Pyroclasm hardcast damage and one-stack consumer absent"]
fn dps_parity_pyroclasm_hardcast_keeps_native_bonus_at_missile_impact_and_spends_one_stack() {
    let (base, base_crit, _) = cast_pyroblast_with_proc(false, false);
    let (boosted, boosted_crit, remaining) = cast_pyroblast_with_proc(true, false);
    assert_eq!(
        remaining, 1,
        "one ordinary hardcast must spend one of the two actual native stacks"
    );
    assert_eq!(
        base_crit, boosted_crit,
        "paired native seed retains the same crit outcome"
    );
    let ratio = boosted as f32 / base as f32;
    //70448 supplies+230% (3.3×), not an invented SimC coefficient. Native
    // coefficient, variance, crit and the rest of the damage pipeline stay intact.
    assert!(
        (3.2..=3.4).contains(&ratio),
        "native269651e0+230% must survive consumption/projectile launch: base{base} boosted{boosted} ratio{ratio}"
    );
}

#[test]
fn dps_parity_pyroclasm_hot_streak_instant_neither_consumes_nor_amplifies_existing_stacks() {
    let (base, base_crit, _) = cast_pyroblast_with_proc(false, true);
    let (damage, crit, remaining) = cast_pyroblast_with_proc(true, true);
    assert_eq!(
        remaining, 2,
        "Hot Streak instant Pyroblast is not a Pyroclasm spender"
    );
    assert_eq!(base_crit, crit);
    let ratio = damage as f32 / base as f32;
    assert!(
        (0.95..=1.05).contains(&ratio),
        "instant Pyroblast must not receive the hardcast-only230%: base{base} damage{damage}"
    );
}

#[test]
fn dps_parity_pyroclasm_untalented_hot_streak_spend_never_creates_proc() {
    let _seed = crate::random::seed_scope(269650);
    let (mut h, caster) = prepare(209059);
    assert!(!has_aura(h.app.world(), caster, TALENT));
    let target = h.npc;
    for _ in 0..16 {
        apply_native_buff(&mut h, caster, HOT_STREAK);
        complete_cast(&mut h, caster, PYROBLAST, target);
        assert_eq!(pyroclasm_stacks(&h, caster), 0);
    }
}

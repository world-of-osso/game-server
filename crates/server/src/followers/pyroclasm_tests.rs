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
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, spell, Some(caster), 1);
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

fn cast_pyroblast_with_proc(proc: bool, instant: bool) -> (f32, u8) {
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
    // Actual crit outcomes need not match across native aura applications. Remove
    // only the unchanged native critical multiplier, never inject a crit stat.
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(PYROBLAST)
        .unwrap();
    let multiplier = if hit.crit {
        let school = world
            .get::<crate::auras::AuraStatMods>(caster)
            .map_or(1.0, |mods| mods.critical_damage(spell.school_mask()));
        let bonus = crate::auras::spell_mods::modify(
            world,
            caster,
            spell,
            crate::auras::SpellModOp::CritDamageAndHealing,
            2.0 * school - 1.0,
        );
        1.0 + bonus
    } else {
        1.0
    };
    (hit.amount as f32 / multiplier, remaining)
}

fn assert_native_damage_ratio(base: f32, damage: f32, expected: f32) {
    let source = crate::spell_cast::test_harness::world_db().unwrap();
    let half_variance = source.store.get(PYROBLAST).unwrap().effects[0]
        .variance
        .abs()
        / 2.0;
    let variation = (1.0 + half_variance) / (1.0 - half_variance);
    // One whole health unit covers rounding of the actual native hit events.
    let bounds = (base * expected / variation - 1.0)..=(base * expected * variation + 1.0);
    assert!(
        bounds.contains(&damage),
        "native damage ratio: base{base}, damage{damage}, multiplier{expected}, authored bounds{bounds:?}"
    );
}

#[test]
fn dps_parity_pyroclasm_hardcast_keeps_native_bonus_at_missile_impact_and_spends_one_stack() {
    let (base, _) = cast_pyroblast_with_proc(false, false);
    let (boosted, remaining) = cast_pyroblast_with_proc(true, false);
    assert_eq!(
        remaining, 1,
        "one ordinary hardcast must spend one of the two actual native stacks"
    );
    //70448 supplies+230% (3.3×), not an invented SimC coefficient.
    assert_native_damage_ratio(base, boosted, 3.3);
}

#[test]
fn dps_parity_pyroclasm_hot_streak_instant_neither_consumes_nor_amplifies_existing_stacks() {
    let (base, _) = cast_pyroblast_with_proc(false, true);
    let (damage, remaining) = cast_pyroblast_with_proc(true, true);
    assert_eq!(
        remaining, 2,
        "Hot Streak instant Pyroblast is not a Pyroclasm spender"
    );
    assert_native_damage_ratio(base, damage, 1.0);
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

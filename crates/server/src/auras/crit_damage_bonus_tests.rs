//! TC a352b1fa Unit.cpp:1222-1237,7297-7324: school-masked163 multiplies
//! critical damage; weapon crit spellmods precede163, magic spellmods follow it.
//! spell_dh.cpp:1571-1593 scales388118 effect0 from owner crit each1s.
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::*;
use crate::class_progression::KnownSpells;
use crate::combat::AutoAttack;
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with_native_enemy_dodge_reduction, world_db,
};
use crate::spell_info::SpellStore;
use crate::stats::CharacterStats;

const KNOW_YOUR_ENEMY: u32 = 388_118;
const PENETRATING_SHOTS: u32 = 459_783;
const AIMED_SHOT: u32 = 19_434;
const PRECISE_KILLER: u32 = 1_272_989;
const INNER_DEMONS: u32 = 202_388;
const CJ_BALL: u32 = 314_633;
const MAGE_CRIT_BONUS: u32 = 60_146;
const CHAOS_WEAPON_HIT: u32 = 222_031;
const HAVOC_CRIT_BONUS: u32 = 201_455;
const BURNING_GAZE: u32 = 194_312;
const PHYSICAL_CRIT_NOTES: u32 = 80_554;
const FLASH_HEAL: u32 = 2061;
const ZERO_MASK_SHARD: u32 = 171_751;
const PYRETIC_INCANTATION: u32 = 194_329;
const DEMON_HUNTER: u8 = 12;

fn set_crit(h: &mut Harness, chance: f32) {
    let unit = h.caster;
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(unit)
        .unwrap()
        .crit_pct = chance;
}

fn apply_native(h: &mut Harness, spell_id: u32) {
    let source = if is_passive(
        h.app
            .world()
            .resource::<SpellStore>()
            .get(spell_id)
            .unwrap(),
    ) {
        AuraSource::KnownSpell
    } else {
        AuraSource::Cast
    };
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask: if matches!(spell_id, KNOW_YOUR_ENEMY | PENETRATING_SHOTS) {
                u32::MAX
            } else {
                1
            },
            source,
        },
    );
}

fn cast_damage(h: &mut Harness, spell_id: u32) -> &CombatLogEvent {
    h.cast_and_deliver(spell_id, Some(h.npc));
    // Native314633 casts2250ms, then its45yd/s missile travels3yd.
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .log
        .iter()
        .rev()
        .find(|event| event.spell_id == Some(spell_id) && event.kind == CombatLogKind::Damage)
        .unwrap()
}

fn cast_aimed_shot(h: &mut Harness) -> &CombatLogEvent {
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 10.0;
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 20_000.0,
        max: 20_000.0,
    };
    h.cast_and_deliver(AIMED_SHOT, Some(h.npc));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .log
        .iter()
        .rev()
        .find(|event| event.spell_id == Some(AIMED_SHOT) && event.kind == CombatLogKind::Damage)
        .expect("native Aimed Shot must land after cast and missile travel")
}

#[test]
fn class_completeness_native_penetrating_shots_increases_actual_ranged_critical_damage() {
    let mut h = harness(Vec::new(), &[PENETRATING_SHOTS, AIMED_SHOT], 3).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, PENETRATING_SHOTS);
    let damage = cast_aimed_shot(&mut h);
    assert!(damage.crit);
    // Native AP9.72 ×100AP ×2crit ×1.25, with authored variance±2.5%.
    // Unmodified crits1944±2.5% cannot enter this interval.
    assert!((2369..=2491).contains(&damage.amount), "{damage:?}");
}

#[test]
fn class_completeness_native_penetrating_shots_refreshes_live_crit_and_unlearning_removes_bonus() {
    let mut h = harness(Vec::new(), &[PENETRATING_SHOTS, AIMED_SHOT], 3).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, PENETRATING_SHOTS);
    assert!((2369..=2491).contains(&cast_aimed_shot(&mut h).amount));
    // Guaranteed crit remains; native1s periodic update must change25% to50%.
    // Above100% is a deterministic test endpoint, not a player-build claim.
    set_crit(&mut h, 200.0);
    h.advance(800);
    let refreshed = cast_aimed_shot(&mut h);
    assert!(refreshed.crit);
    assert!((2843..=2989).contains(&refreshed.amount), "{refreshed:?}");
    let unit = h.caster;
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(KnownSpells::fixture(vec![AIMED_SHOT]));
    h.advance(800);
    let unlearned = cast_aimed_shot(&mut h);
    assert!(unlearned.crit);
    assert!((1895..=1993).contains(&unlearned.amount), "{unlearned:?}");
}

#[test]
fn class_completeness_native_penetrating_shots_zero_crit_does_not_change_normal_ranged_damage() {
    let mut h = harness(Vec::new(), &[PENETRATING_SHOTS, AIMED_SHOT], 3).unwrap();
    set_crit(&mut h, 0.0);
    apply_native(&mut h, PENETRATING_SHOTS);
    let damage = cast_aimed_shot(&mut h);
    assert!(!damage.crit);
    assert!((948..=997).contains(&damage.amount), "{damage:?}");
}

#[test]
fn class_completeness_native_penetrating_shots_does_not_increase_critical_healing() {
    let mut h = harness(Vec::new(), &[PENETRATING_SHOTS, FLASH_HEAL], 3).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, PENETRATING_SHOTS);
    let unit = h.caster;
    *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
        current: 1.0,
        max: 10_000.0,
    };
    // Cross-class native heal tests only shared damage/healing isolation.
    h.cast_and_deliver(FLASH_HEAL, Some(unit));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let gained = h.health(unit) - 1.0;
    assert!((1329.0..=1399.0).contains(&gained), "healed {gained}");
}

#[test]
fn class_completeness_native_163_precise_killer_increases_actual_critical_damage() {
    let mut h = harness(Vec::new(), &[PRECISE_KILLER, INNER_DEMONS], DEMON_HUNTER).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, PRECISE_KILLER);
    let damage = cast_damage(&mut h, INNER_DEMONS);
    assert!(damage.crit);
    // Native AP coefficient1.365 ×100 AP ×2 crit ×1.10, rounded.
    assert_eq!(damage.amount, 300);
}

#[test]
fn class_completeness_native_163_mixed_school_hit_multiplies_disjoint_masks_once_each() {
    let ids = [INNER_DEMONS, BURNING_GAZE, PHYSICAL_CRIT_NOTES];
    let mut h = harness(Vec::new(), &ids, DEMON_HUNTER).unwrap();
    set_crit(&mut h, 100.0);
    // Authored3% Fire mask4 and3% Physical mask1. Designer Notes80554 is only
    // a native mask fixture, not a claim about class spellbook availability.
    apply_native(&mut h, BURNING_GAZE);
    apply_native(&mut h, PHYSICAL_CRIT_NOTES);
    let damage = cast_damage(&mut h, INNER_DEMONS);
    assert!(damage.crit);
    // School127 intersects both:1.365 ×100 AP ×2 ×1.03 ×1.03, rounded.
    assert_eq!(damage.amount, 290);
}

#[test]
fn class_completeness_native_know_your_enemy_magic_crit_orders_spellmod_after_bonus_and_recalculates()
 {
    // Native zero-variance mage test spell314633 isolates shared magic math;
    // this cross-class fixture does not claim mage talent availability.
    let ids = [KNOW_YOUR_ENEMY, CJ_BALL, MAGE_CRIT_BONUS];
    let mut h = harness(Vec::new(), &ids, 8).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, KNOW_YOUR_ENEMY);
    apply_native(&mut h, MAGE_CRIT_BONUS);
    //59 base; (2×59×1.40−59)×1.05+59 =170.51. Wrong weapon order gives169.
    assert_eq!(cast_damage(&mut h, CJ_BALL).amount, 171);
    // An explicit >100%-crit endpoint guarantees damage crits while the
    // owner's raw crit-derived amount changes; not a player-build claim.
    set_crit(&mut h, 200.0);
    h.advance(40);
    // KnowYourEnemy80%: (2×59×1.80−59)×1.05+59 =220.07.
    assert_eq!(cast_damage(&mut h, CJ_BALL).amount, 220);
}

#[test]
fn class_completeness_native_know_your_enemy_weapon_crit_orders_spellmod_before_bonus() {
    let mut weapon_hit = world_db()
        .unwrap()
        .store
        .get(CHAOS_WEAPON_HIT)
        .unwrap()
        .clone();
    // Remove only random variance to distinguish the two native orderings;
    // authored AP coefficient0.736 and every mask remain unchanged.
    weapon_hit.effects[0].variance = 0.0;
    let mut h = harness_with_native_enemy_dodge_reduction(
        vec![weapon_hit],
        &[KNOW_YOUR_ENEMY, HAVOC_CRIT_BONUS],
        DEMON_HUNTER,
    )
    .unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, KNOW_YOUR_ENEMY);
    apply_native(&mut h, HAVOC_CRIT_BONUS);
    // (73.6+73.6×1.06)×1.40 =212.2624. Wrong magic order gives214.
    assert_eq!(cast_damage(&mut h, CHAOS_WEAPON_HIT).amount, 212);
}

#[test]
fn class_completeness_native_163_damage_bonus_does_not_increase_critical_healing() {
    let mut h = harness(Vec::new(), &[KNOW_YOUR_ENEMY, FLASH_HEAL], 8).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, KNOW_YOUR_ENEMY);
    let unit = h.caster;
    *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
        current: 1.0,
        max: 10_000.0,
    };
    h.cast_and_deliver(FLASH_HEAL, Some(unit));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let gained = h.health(unit) - 1.0;
    // Native SP coefficient6.82 ×100 SP ×2 heal crit, variance±2.5%; no163.
    assert!((1329.0..=1399.0).contains(&gained), "healed {gained}");
}

#[test]
fn class_completeness_native_163_precise_killer_increases_auto_attack_crit_damage() {
    let mut h = harness(Vec::new(), &[PRECISE_KILLER], DEMON_HUNTER).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, PRECISE_KILLER);
    let unit = h.caster;
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(AutoAttack::new(h.npc));
    h.advance(300);
    let crits: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id.is_none() && event.kind == CombatLogKind::Damage && event.crit
        })
        .collect();
    assert!(!crits.is_empty(), "{:?}", h.inbox.log);
    // Existing retail AP/6 formula:100 AP, unarmed1-2 at2s rounds34-35;
    // ×2 crit ×1.10 gives74.8-77; existing swing-log truncation reports74-77,
    // versus unbuffed68-70.
    assert!(
        crits.iter().all(|event| (74..=77).contains(&event.amount)),
        "{crits:?}"
    );
}

#[test]
fn class_completeness_native_163_does_not_modify_noncritical_damage() {
    let mut h = harness(Vec::new(), &[PRECISE_KILLER, INNER_DEMONS], DEMON_HUNTER).unwrap();
    set_crit(&mut h, 0.0);
    apply_native(&mut h, PRECISE_KILLER);
    let damage = cast_damage(&mut h, INNER_DEMONS);
    assert!(!damage.crit);
    assert_eq!(damage.amount, 137);
}

#[test]
fn class_completeness_native_163_zero_school_mask_does_not_become_all_schools() {
    let mut h = harness(Vec::new(), &[ZERO_MASK_SHARD, INNER_DEMONS], DEMON_HUNTER).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, ZERO_MASK_SHARD);
    assert_eq!(cast_damage(&mut h, INNER_DEMONS).amount, 273);
}

#[test]
fn class_completeness_native_163_unlearning_removes_the_damage_multiplier() {
    let mut h = harness(Vec::new(), &[PRECISE_KILLER, INNER_DEMONS], DEMON_HUNTER).unwrap();
    set_crit(&mut h, 100.0);
    apply_native(&mut h, PRECISE_KILLER);
    assert_eq!(cast_damage(&mut h, INNER_DEMONS).amount, 300);
    let unit = h.caster;
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(KnownSpells::fixture(vec![INNER_DEMONS]));
    h.advance(1);
    assert_eq!(cast_damage(&mut h, INNER_DEMONS).amount, 273);
}

#[test]
fn class_completeness_native_163_stacked_amount_caps_at_native_five_uses_not_five_products() {
    let mut h = harness(
        Vec::new(),
        &[PYRETIC_INCANTATION, INNER_DEMONS],
        DEMON_HUNTER,
    )
    .unwrap();
    set_crit(&mut h, 100.0);
    for _ in 0..6 {
        apply_native(&mut h, PYRETIC_INCANTATION);
    }
    // Native2% ×five stacks =10%, not1.02^5 and not a sixth stack.
    assert_eq!(cast_damage(&mut h, INNER_DEMONS).amount, 300);
}

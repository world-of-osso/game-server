//! Shared, source-derived class effects against native Retail rows.
use super::test_harness::*;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::Health;

const PIERCING_CHILL: u32 = 377359;
const CHILL_STREAK: u32 = 204167;
const FROSTBOLT: u32 = 116;
const PALADIN_PASSIVE: u32 = 137026;
const CRUSADER_STRIKE: u32 = 35395;
const CHARGE: u32 = 100;

fn fixture() -> Harness {
    let data = world_db().expect("real world.db");
    let spells = [PIERCING_CHILL, CHILL_STREAK, FROSTBOLT]
        .into_iter()
        .map(|id| {
            let mut spell = data.store.get(id).expect("native spell row").clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    harness(spells, &[], 6).expect("real world.db")
}

fn apply_chill(h: &mut Harness, caster: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: h.npc,
            spell_id: PIERCING_CHILL,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn hit(h: &mut Harness, spell_id: u32) -> f32 {
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 5000.0;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(h.npc)).unwrap();
    h.advance(10); // Native Chill Streak has a 20 yd/s missile.
    5000.0 - h.health(h.npc)
}

#[test]
fn class_completeness_native_label_vulnerability_increases_owned_chill_streak() {
    let mut h = fixture();
    let base = hit(&mut h, CHILL_STREAK);
    assert!(base > 0.0);
    let caster = h.caster;
    apply_chill(&mut h, caster);
    let amplified = hit(&mut h, CHILL_STREAK);
    assert_eq!(amplified, (base * 1.12).round());
}

#[test]
fn class_completeness_native_label_vulnerability_excludes_other_spell_and_caster() {
    let mut h = fixture();
    let frostbolt = hit(&mut h, FROSTBOLT);
    let chill = hit(&mut h, CHILL_STREAK);
    let other = spawn_player(&mut h.app, 6, 0.0);
    apply_chill(&mut h, other);
    assert_eq!(hit(&mut h, CHILL_STREAK), chill);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == PIERCING_CHILL,
    );
    let caster = h.caster;
    apply_chill(&mut h, caster);
    assert_eq!(hit(&mut h, FROSTBOLT), frostbolt);
}

#[test]
fn class_completeness_native_label_vulnerability_expires() {
    let mut h = fixture();
    let base = hit(&mut h, CHILL_STREAK);
    let caster = h.caster;
    apply_chill(&mut h, caster);
    h.advance(200);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        PIERCING_CHILL
    ));
    assert_eq!(hit(&mut h, CHILL_STREAK), base);
}

fn charge_fixture(passive: bool, haste: f32) -> Harness {
    let ids = if passive {
        vec![PALADIN_PASSIVE, CRUSADER_STRIKE, CHARGE]
    } else {
        vec![CRUSADER_STRIKE, CHARGE]
    };
    let mut h = harness(Vec::new(), &ids, 2).unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = haste;
    h.advance(2);
    h
}

fn cast_charge_button(h: &mut Harness, spell_id: u32) -> shared::protocol::SpellChargesUpdate {
    h.inbox.charges.clear();
    h.cast_and_deliver(spell_id, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .charges
        .iter()
        .find(|update| update.spell_id == spell_id)
        .unwrap()
        .clone()
}

#[test]
fn class_completeness_native_charge_haste_halves_crusader_strike_recovery() {
    let mut base = charge_fixture(false, 100.0);
    let normal = cast_charge_button(&mut base, CRUSADER_STRIKE);
    let mut h = charge_fixture(true, 100.0);
    let fast = cast_charge_button(&mut h, CRUSADER_STRIKE);
    assert_eq!(fast.recharge_ms, normal.recharge_ms / 2);
    h.advance((fast.recharge_ms / 50) as usize);
    let regenerated = cast_charge_button(&mut h, CRUSADER_STRIKE);
    assert_eq!(regenerated.current, regenerated.max - 1);
}

#[test]
fn class_completeness_native_charge_haste_excludes_other_category_and_zero_haste() {
    let mut base = charge_fixture(false, 100.0);
    charge_range(&mut base);
    let normal = cast_charge_button(&mut base, CHARGE);
    let mut h = charge_fixture(true, 100.0);
    charge_range(&mut h);
    assert_eq!(
        cast_charge_button(&mut h, CHARGE).recharge_ms,
        normal.recharge_ms
    );
    let mut zero = charge_fixture(true, 0.0);
    let mut absent = charge_fixture(false, 0.0);
    assert_eq!(
        cast_charge_button(&mut zero, CRUSADER_STRIKE).recharge_ms,
        cast_charge_button(&mut absent, CRUSADER_STRIKE).recharge_ms
    );
}

#[test]
fn class_completeness_native_charge_haste_retimes_inflight_recovery() {
    let mut h = charge_fixture(true, 0.0);
    let normal = cast_charge_button(&mut h, CRUSADER_STRIKE);
    h.advance(40);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = 100.0;
    let faster = cast_charge_button(&mut h, CRUSADER_STRIKE);
    assert_eq!(faster.recharge_ms, normal.recharge_ms / 2);
    // Two seconds plus bounded loopback delivery have elapsed since the first use.
    let expected = normal.remaining_ms.saturating_sub(2000) / 2;
    assert!(
        faster.remaining_ms.abs_diff(expected) <= 150,
        "{:?} vs {expected}",
        faster
    );
}

fn cooldown_fixture(passive: bool, haste: f32) -> Harness {
    let mut ids = vec![12294, 23881, 57755, 117313, 184362, 115804];
    if passive {
        ids.push(137047); // Native Warrior: cooldown haste mask, not every spell.
    }
    let mut h = harness(Vec::new(), &ids, 1).unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = haste;
    h.advance(2);
    h.set_power(shared::components::PowerType::Rage, 1000.0);
    h
}

fn cast_cooldown_button(h: &mut Harness, spell_id: u32) -> u32 {
    if spell_id == 57755 {
        charge_range(h); // Heroic Throw's native minimum range is 8 yards.
    }
    h.inbox.cooldowns.clear();
    h.cast_and_deliver(spell_id, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .cooldowns
        .iter()
        .find(|update| update.spell_id == spell_id && !update.is_gcd)
        .unwrap()
        .duration_ms
}

#[test]
fn class_completeness_native_cooldown_haste_scales_spell_and_category_timers() {
    for id in [12294, 23881] {
        let mut base = cooldown_fixture(false, 100.0);
        let normal = cast_cooldown_button(&mut base, id);
        let mut h = cooldown_fixture(true, 100.0);
        assert_eq!(cast_cooldown_button(&mut h, id), normal / 2);
    }
}

fn native80_fixture(ids: &[u32], class: u8, spec: u32) -> Harness {
    let mut h = harness(Vec::new(), ids, class).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.world_mut().entity_mut(h.caster).insert((
        shared::components::Level(80),
        shared::components::ActiveSpec(spec),
    ));
    h.advance(3);
    h
}

#[test]
fn class_completeness_native80_astral_generator_and_spender() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[190984, 78674, 137009, 137013, 197911], 11, 102);
    h.set_power(PowerType::LunarPower, 0.0);
    h.cast_and_deliver(190984, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::LunarPower), 60.0); // Native Balance modifier:6 Astral.
    h.set_power(PowerType::LunarPower, 400.0);
    h.cast_and_deliver(78674, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::LunarPower), 0.0); // Native Starsurge cost40 Astral.
}

#[test]
fn class_completeness_native80_word_of_glory_spends_three_holy_power() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[85673, 115675, 137026, 137028], 2, 66);
    h.set_power(PowerType::HolyPower, 3.0);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.cast_and_deliver(85673, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::HolyPower), 0.0);
    assert!(h.health(h.caster) > 100.0);
}

#[test]
fn class_completeness_native80_rune_runic_generator_and_spender() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 49998, 137005], 6, 251);
    h.set_power(PowerType::RunicPower, 0.0);
    h.cast_and_deliver(49184, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Runes), 5.0);
    assert_eq!(h.power(PowerType::RunicPower), 100.0); // Native negative-cost100 raw.
    h.advance(40);
    h.set_power(PowerType::RunicPower, 450.0);
    h.cast_and_deliver(49998, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::RunicPower), 0.0);
}

#[test]
fn class_completeness_native80_rune_generation_caps_at_native_max() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 137005], 6, 251);
    h.set_power(PowerType::RunicPower, 950.0);
    h.cast_and_deliver(49184, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::RunicPower), 1000.0);
    assert_eq!(h.power(PowerType::Runes), 5.0);
}

#[test]
fn class_completeness_native80_rune_rejection_never_grants_runic_power() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 137005], 6, 251);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(super::runes::Runes {
            cooldowns: vec![10.0; 6],
            order: vec![0, 1, 2, 3, 4, 5],
        });
    h.set_power(PowerType::Runes, 0.0);
    h.set_power(PowerType::RunicPower, 0.0);
    h.cast_and_deliver(49184, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.power(PowerType::RunicPower), 0.0);
    assert_eq!(h.power(PowerType::Runes), 0.0);
    assert!(!h.inbox.go.iter().any(|event| event.spell_id == 49184));
}

#[test]
fn class_completeness_native80_disintegrate_spends_three_essence() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[356995, 356809], 13, 1467);
    h.set_power(PowerType::Essence, 3.0);
    h.cast_and_deliver(356995, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == 356995));
    // Native0.2/s fractional regeneration resumes after the three-point spend.
    assert!((0.0..=0.1).contains(&h.power(PowerType::Essence)));
}

#[test]
fn class_completeness_native_cooldown_haste_truncates_native_milliseconds() {
    let mut h = cooldown_fixture(true, 40.0);
    assert_eq!(cast_cooldown_button(&mut h, 12294), 4285); // Native6000 / 1.4, truncated.
}

#[test]
fn class_completeness_native_charge_haste_syncs_without_another_cast() {
    let mut h = charge_fixture(true, 0.0);
    cast_charge_button(&mut h, CRUSADER_STRIKE);
    h.advance(40);
    h.inbox.charges.clear();
    h.inbox.go.clear();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = 100.0;
    h.advance(5);
    let changed = h
        .inbox
        .charges
        .last()
        .expect("rate update without a new cast");
    assert_eq!(changed.recharge_ms, 3000);
    assert!(changed.remaining_ms <= 2000);
    assert!(h.inbox.go.is_empty());
    h.advance(45);
    let ready = h.inbox.charges.last().expect("charge replenishment update");
    assert_eq!(ready.current, ready.max);
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_cooldown_haste_respects_mask_and_zero_haste() {
    let mut base = cooldown_fixture(false, 100.0);
    let mut h = cooldown_fixture(true, 100.0);
    assert_eq!(
        cast_cooldown_button(&mut h, 57755),
        cast_cooldown_button(&mut base, 57755)
    );
    let mut zero = cooldown_fixture(true, 0.0);
    let mut absent = cooldown_fixture(false, 0.0);
    assert_eq!(
        cast_cooldown_button(&mut zero, 12294),
        cast_cooldown_button(&mut absent, 12294)
    );
}

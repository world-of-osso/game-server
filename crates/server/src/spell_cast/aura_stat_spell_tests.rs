//! Generic aura consumers at the real spell math boundary, native70448 spells.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
use crate::spell_cast::test_harness::{Harness, harness_with, world_db};

fn fixture(ids: &[u32]) -> Harness {
    let mut h = harness_with(Vec::new(), ids, 8, false).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(2);
    h
}

fn apply(h: &mut Harness, spell_id: u32, index: u32) {
    let caster = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    h.advance(2);
}

#[test]
#[ignore = "aura-stat-mods RED: native spell crit57"]
fn aura_stat_native_57_changes_magic_crit_but_not_weapon_crit() {
    let mut h = fixture(&[450831]); //2.5 spell-only percentage points.
    let magic = world_db().unwrap().store.get(314633).unwrap();
    let weapon = world_db().unwrap().store.get(78).unwrap();
    let before_magic = crit_chance(h.app.world(), h.caster, h.npc, magic);
    let before_weapon = crit_chance(h.app.world(), h.caster, h.npc, weapon);
    apply(&mut h, 450831, 0);
    assert_eq!(
        crit_chance(h.app.world(), h.caster, h.npc, magic),
        before_magic + 2.5
    );
    assert_eq!(
        crit_chance(h.app.world(), h.caster, h.npc, weapon),
        before_weapon
    );
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(2);
    assert_eq!(
        crit_chance(h.app.world(), h.caster, h.npc, magic),
        before_magic
    );
}

#[test]
#[ignore = "aura-stat-mods RED: native healing354"]
fn aura_stat_native_354_scales_real_heal_by_missing_health_and_class_mask() {
    let mut h = fixture(&[383191]); //Tranquility mask128,20% at completely missing health.
    apply(&mut h, 383191, 1);
    let spell = world_db().unwrap().store.get(157982).unwrap();
    let unrelated = world_db().unwrap().store.get(2061).unwrap();
    let caster = h.caster;
    let target = h.npc;
    let ctx = |spell| EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    //The fixture input1000 isolates multipliers; native spell and aura masks unchanged.
    for (current, expected) in [(5000.0, 1100.0), (8000.0, 1040.0)] {
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current,
            max: 10000.0,
        };
        apply_heal(h.app.world_mut(), &ctx(spell), target, 1000.0, false, false);
        assert_eq!(h.health(target), current + expected);
    }
    *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
        current: 5000.0,
        max: 10000.0,
    };
    apply_heal(
        h.app.world_mut(),
        &ctx(unrelated),
        target,
        1000.0,
        false,
        false,
    );
    assert_eq!(h.health(target), 6000.0);
}

#[test]
#[ignore = "aura-stat-mods RED: native resistance269"]
fn aura_stat_native_269_negative_ignore_increases_physical_armor_not_damage_percent() {
    let mut h = fixture(&[1160]); //Native -20%, school127: armor×1.20, not damage×0.80.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CreatureArmor(3000.0));
    let before = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0);
    apply(&mut h, 1160, 0);
    let after = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0);
    assert!(
        after < before,
        "native-20 ignore increases armor: {after} vs {before}"
    );
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(2);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CreatureArmor(3600.0));
    assert_eq!(after, armor_reduced(h.app.world(), h.caster, h.npc, 1000.0));
}

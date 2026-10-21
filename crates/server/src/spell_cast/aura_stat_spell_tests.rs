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

fn penetration_fixture() -> Harness {
    let mut h = fixture(&[450990, 177112]);
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_expected_stats(&conn).armor_constants());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::UnitLevel(80));
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((shared::components::UnitLevel(10), CreatureArmor(2000.0)));
    h
}

#[test]
#[ignore = "RED: native280 player armor penetration with victim-level cap"]
fn census_tail_280_native_warriors_instinct_changes_physical_mitigation_then_removes() {
    let mut h = penetration_fixture();
    let spell = world_db().unwrap().store.get(12294).unwrap();
    for (level, effective_armor) in [(10, 1870.0), (60, 1760.0)] {
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert((shared::components::UnitLevel(level), CreatureArmor(2000.0)));
        let before = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell);
        apply(&mut h, 450990, 0);
        let after = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell);
        assert!(
            after > before,
            "native12% must reduce armor: {after} vs {before}"
        );
        let caster = h.caster;
        remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == 450990
        });
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert(CreatureArmor(effective_armor));
        assert_eq!(
            after,
            armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell)
        );
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert(CreatureArmor(2000.0));
        assert_eq!(
            before,
            armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell)
        );
    }
}

#[test]
#[ignore = "RED: native280 sums then clamps at100 without ignoring capped armor"]
fn census_tail_280_native_hundred_percent_is_capped_and_does_not_stack_past_cap() {
    let mut h = penetration_fixture();
    let spell = world_db().unwrap().store.get(12294).unwrap();
    apply(&mut h, 177112, 0);
    let capped = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell);
    apply(&mut h, 450990, 0);
    assert_eq!(
        capped,
        armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell)
    );
    assert!(
        capped < 1000.0,
        "100% penetrates the native cap, not all armor"
    );
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CreatureArmor(2000.0 - 3250.0 / 3.0));
    assert_eq!(
        capped,
        armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell)
    );
}

#[test]
#[ignore = "RED control:280 is Player-only, even on controlled NPCs"]
fn census_tail_280_native_npc_does_not_gain_player_armor_penetration() {
    let mut h = penetration_fixture();
    let npc = h.npc;
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(shared::components::UnitFlags(
            crate::faction::UNIT_FLAG_PLAYER_CONTROLLED,
        ));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CreatureArmor(2000.0));
    let spell = world_db().unwrap().store.get(12294).unwrap();
    let before = armor_reduced(h.app.world(), npc, h.caster, 1000.0, spell);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: npc,
            spell_id: 450990,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), npc, 450990));
    assert_eq!(
        before,
        armor_reduced(h.app.world(), npc, h.caster, 1000.0, spell)
    );
}

#[test]
fn aura_stat_native_57_changes_magic_crit_but_not_weapon_crit() {
    let mut h = fixture(&[450831]); //2.5 spell-only percentage points.
    let magic = world_db().unwrap().store.get(314633).unwrap();
    let weapon = world_db().unwrap().store.get(12294).unwrap();
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
fn aura_stat_native_269_negative_ignore_increases_physical_armor_not_damage_percent() {
    let mut h = fixture(&[1160]); //Native -20%, school127: armor×1.20, not damage×0.80.
    let spell = world_db().unwrap().store.get(12294).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_expected_stats(&conn).armor_constants());
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CreatureArmor(300.0));
    let before = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell);
    apply(&mut h, 1160, 0);
    let after = armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell);
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
        .insert(CreatureArmor(360.0));
    assert_eq!(
        after,
        armor_reduced(h.app.world(), h.caster, h.npc, 1000.0, spell)
    );
}

#[test]
fn aura_stat_native_269_scales_existing_damage_reduction_for_matching_school() {
    let mut h = fixture(&[1160, 33206]);
    let caster = h.caster;
    let target = h.npc;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: 33206,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    let spell = world_db().unwrap().store.get(314633).unwrap();
    let before = crate::auras::damage_taken_multiplier(h.app.world(), target, caster, spell);
    assert!((before - 0.6).abs() < 0.0001);
    apply(&mut h, 1160, 0);
    let after = crate::auras::damage_taken_multiplier(h.app.world(), target, caster, spell);
    assert!(
        (after - 0.52).abs() < 0.0001,
        "-20% ignore strengthens40% reduction to48%, got{after}"
    );
}

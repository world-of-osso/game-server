//! Fifth batch: real Mainline casts, bounded scripted paths.
use super::test_harness::*;
use bevy::prelude::Entity;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn damage(h: &Harness, id: u32, target: Entity) -> i32 {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(id)
                && e.target == Some(target.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .map(|e| e.amount)
        .sum()
}

fn bite(rank: bool) -> f32 {
    let mut ids = vec![162243];
    if rank {
        ids.push(258876);
    }
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 12).expect("real catalog");
    let caster = h.caster;
    if rank {
        apply(&mut h, caster, caster, 258876, u32::MAX);
    }
    h.set_power(PowerType::Fury, 0.0);
    h.cast_and_deliver(162243, Some(h.npc));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(damage(&h, 162243, h.npc) > 0);
    h.power(PowerType::Fury)
}

#[test]
fn dummy5_demons_bite_insatiable_hunger_adds_catalog_fury_range() {
    assert!((30.0..=35.0).contains(&bite(true)));
}
#[test]
fn dummy5_demons_bite_without_rank_keeps_catalog_fury() {
    assert_eq!(bite(false), 25.0);
}

fn chaos_strike(chance: u32) {
    let mut proc = world_db().unwrap().store.get(197125).unwrap().clone();
    // Explicit chance endpoints on the real catalog, not a frequency assertion.
    proc.aura_options.as_mut().unwrap().proc_chance = chance;
    let mut h = harness_with_native_enemy_dodge_reduction(
        vec![proc],
        &[162794, 222031, 199547, 193840],
        12,
    )
    .expect("real catalog");
    h.set_power(PowerType::Fury, 50.0);
    h.cast_and_deliver(162794, Some(h.npc));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.power(PowerType::Fury),
        if chance == 100 { 30.0 } else { 10.0 }
    );
    assert!(damage(&h, 222031, h.npc) > 0);
    assert!(damage(&h, 199547, h.npc) > 0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(193840))
            .count(),
        usize::from(chance == 100)
    );
}
#[test]
fn dummy5_chaos_strike_refunds_once_at_catalog_chance_endpoint() {
    chaos_strike(100);
}
#[test]
fn dummy5_chaos_strike_zero_chance_does_not_refund() {
    chaos_strike(0);
}

#[test]
fn dummy5_disintegrate_applies_three_second_slow_and_ticks() {
    let mut h = harness(Vec::new(), &[356995, 370898], 13).expect("real catalog");
    h.set_power(PowerType::Essence, 5.0);
    h.cast_and_deliver(356995, Some(h.npc));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 370898));
    h.advance(20);
    assert!(damage(&h, 356995, h.npc) > 0);
    h.advance(65);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 370898));
}

fn chaos_bolt(crit: f32) -> i32 {
    let mut h = harness(Vec::new(), &[116858], 9).expect("real catalog");
    h.set_power(PowerType::SoulShards, 50.0);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = crit;
    h.cast_and_deliver(116858, Some(h.npc));
    h.advance(75);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let event = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(116858) && e.kind == CombatLogKind::Damage)
        .unwrap();
    assert!(event.crit, "Chaos Bolt must crit at {crit}%");
    event.amount
}
#[test]
fn dummy5_chaos_bolt_always_crits_with_zero_crit_stats() {
    assert!(varied(858.09803).contains(&(chaos_bolt(0.0) as f32)));
}
#[test]
fn dummy5_chaos_bolt_damage_scales_with_critical_strike_chance() {
    assert!(varied(1115.5274).contains(&(chaos_bolt(30.0) as f32)));
}

#[test]
fn dummy5_rapture_counts_only_owned_damaging_periodic_effects() {
    let mut h = harness(
        Vec::new(),
        &[324536, 324540, 146739, 30108, 233490, 703, 119611],
        9,
    )
    .expect("real catalog");
    let caster = h.caster;
    let primary = h.npc;
    let two = spawn_npc(&mut h.app, 5.0);
    let foreign = spawn_npc(&mut h.app, 6.0);
    let unmarked = spawn_npc(&mut h.app, 7.0);
    let ally = spawn_player(&mut h.app, 9, 8.0);
    apply(&mut h, caster, primary, 146739, 1);
    apply(&mut h, caster, two, 146739, 1);
    apply(&mut h, caster, two, 233490, 1);
    apply(&mut h, foreign, foreign, 703, 1);
    apply(&mut h, caster, unmarked, 119611, 1); // periodic heal is not a damaging effect
    apply(&mut h, caster, ally, 146739, 1);
    h.set_power(PowerType::SoulShards, 50.0);
    h.cast_and_deliver(324536, None);
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(45.9196).contains(&(damage(&h, 324540, primary) as f32)));
    // Two owned DoTs plus30108s3's native10% vulnerability on this target.
    assert!(varied(91.8392 * 1.1).contains(&(damage(&h, 324540, two) as f32)));
    for unit in [foreign, unmarked, ally] {
        assert_eq!(damage(&h, 324540, unit), 0);
    }
}

#[test]
fn dummy5_shuriken_storm_awards_combo_points_per_enemy_hit() {
    let mut h = harness(Vec::new(), &[197835], 4).expect("real catalog");
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 30.0);
    let ally = spawn_player(&mut h.app, 4, 2.0);
    h.set_power(PowerType::ComboPoints, 0.0);
    h.cast_and_deliver(197835, None);
    h.advance(10);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
    assert!(damage(&h, 197835, h.npc) > 0);
    assert!(damage(&h, 197835, near) > 0);
    for unit in [far, ally] {
        assert_eq!(damage(&h, 197835, unit), 0);
    }
}

fn bone_stacks(h: &Harness) -> u8 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .and_then(|a| a.iter().find(|a| a.spell_id == 195181))
        .map_or(0, |a| a.stacks)
}
#[test]
fn dummy5_deaths_caress_generates_two_bone_shield_stacks() {
    // Guaranteed-impact fixture: Blood Plague's child uses native melee dodge.
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[195292, 55078, 195181], 6)
        .expect("real catalog");
    h.cast_and_deliver(195292, Some(h.npc));
    h.advance(10);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(bone_stacks(&h), 2);
    assert!(damage(&h, 195292, h.npc) > 0);
    assert!(
        crate::auras::has_aura(h.app.world(), h.npc, 55078),
        "Blood Plague missing after Death's Caress: {:?}",
        h.inbox.log
    );
}
#[test]
fn dummy5_deaths_caress_adds_to_existing_bone_shield_up_to_catalog_cap() {
    let mut h = harness(Vec::new(), &[195292, 55078, 195181], 6).expect("real catalog");
    let caster = h.caster;
    for _ in 0..9 {
        apply(&mut h, caster, caster, 195181, u32::MAX);
    }
    assert_eq!(bone_stacks(&h), 9);
    h.cast_and_deliver(195292, Some(h.npc));
    h.advance(10);
    assert_eq!(bone_stacks(&h), 10);
}

#[test]
fn dummy5_epidemic_eruptions_require_owned_virulent_plague() {
    let mut h =
        harness(Vec::new(), &[207317, 212739, 191587, 1240996, 77575], 6).expect("real catalog");
    let caster = h.caster;
    let primary = h.npc;
    let near = spawn_npc(&mut h.app, 5.0);
    let foreign = spawn_npc(&mut h.app, 25.0);
    let far = spawn_npc(&mut h.app, 60.0);
    let ally = spawn_player(&mut h.app, 6, 4.0);
    apply(&mut h, caster, primary, 191587, 1);
    apply(&mut h, caster, primary, 1240996, 1);
    apply(&mut h, foreign, foreign, 191587, 1);
    apply(&mut h, caster, far, 191587, 1);
    let expiry = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(primary)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 191587)
        .unwrap()
        .expires_at
        .unwrap();
    h.set_power(PowerType::RunicPower, 1000.0);
    h.cast_and_deliver(207317, None);
    h.advance(10);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(96.3301).contains(&(damage(&h, 212739, primary) as f32)));
    assert!(varied(38.5326).contains(&(damage(&h, 212739, near) as f32)));
    for unit in [foreign, far, ally] {
        assert_eq!(damage(&h, 212739, unit), 0);
    }
    let after = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(primary)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 191587)
        .unwrap()
        .expires_at
        .unwrap();
    assert!((after - expiry - 1.0).abs() < 0.001);
}

fn lava_burst(flame_shock: bool, rank: bool) -> Harness {
    let mut ids = vec![51505, 285452, 188389];
    if rank {
        ids.push(343725);
    }
    let mut h = harness(Vec::new(), &ids, 7).expect("real catalog");
    let caster = h.caster;
    let target = h.npc;
    if flame_shock {
        apply(&mut h, caster, target, 188389, u32::MAX);
    }
    if rank {
        apply(&mut h, caster, caster, 343725, u32::MAX);
    }
    h.set_power(PowerType::Maelstrom, 0.0);
    h.cast_and_deliver(51505, Some(target));
    h.advance(55);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h
}
#[test]
fn dummy5_lava_burst_crits_on_flame_shock_and_generates_eight_maelstrom() {
    let h = lava_burst(true, true);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(285452) && e.crit)
    );
    assert_eq!(h.power(PowerType::Maelstrom), 8.0);
}
#[test]
fn dummy5_lava_burst_without_flame_shock_or_rank_keeps_normal_hit() {
    let h = lava_burst(false, false);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(285452) && !e.crit)
    );
    assert_eq!(h.power(PowerType::Maelstrom), 0.0);
}

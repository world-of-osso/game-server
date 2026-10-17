//! Third batch: real Retail spell casts and explicit catalog chance endpoints.
use super::test_harness::*;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

#[test]
fn dummy3_moonfire_bridges_direct_damage_and_periodic_damage_without_base_slow() {
    let mut h = harness(Vec::new(), &[8921, 164812], 11).expect("real world.db");
    h.cast_and_deliver(8921, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(21.2).contains(&(5000.0 - h.health(h.npc))));
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 164812));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap();
    assert!(
        !aura
            .iter()
            .flat_map(|a| &a.effects)
            .any(|e| e.aura_type == 33)
    );
    let before = h.health(h.npc);
    h.advance(42);
    assert_eq!(before - h.health(h.npc), 18.0);
}

#[test]
fn dummy3_scorch_below_thirty_percent_guarantees_crit() {
    let mut h = harness(Vec::new(), &[2948], 8).expect("real world.db");
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1499.0;
    h.cast_and_deliver(2948, Some(h.npc));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(2948))
        .expect("Scorch hit");
    assert!(hit.crit);
    assert!(varied(200.0).contains(&(hit.amount as f32)));
}

#[test]
fn dummy3_scorch_at_thirty_percent_keeps_normal_crit_chance() {
    let mut h = harness(Vec::new(), &[2948], 8).expect("real world.db");
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1500.0;
    h.cast_and_deliver(2948, Some(h.npc));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(2948))
        .expect("Scorch hit");
    assert!(!hit.crit);
    assert!(varied(100.0).contains(&(hit.amount as f32)));
}

#[test]
fn dummy3_serenity_flash_heal_reduces_running_recharge_by_six_seconds() {
    let mut h = harness(Vec::new(), &[2050, 2061], 5).expect("real world.db");
    h.cast_and_deliver(2050, Some(h.caster));
    h.advance(35);
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1673]
        .next_ready_at();
    h.inbox.charges.clear();
    h.cast_and_deliver(2061, Some(h.caster));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1673]
        .next_ready_at();
    assert_eq!(after, before - 6.0);
    assert!(h.inbox.charges.iter().any(|e| e.spell_id == 2050));
}

#[test]
fn dummy3_serenity_flash_heal_does_not_start_ready_cooldown() {
    let mut h = harness(Vec::new(), &[2050, 2061], 5).expect("real world.db");
    h.cast_and_deliver(2061, Some(h.caster));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!h.inbox.cooldowns.iter().any(|e| e.spell_id == 2050));
}

#[test]
fn dummy3_prayer_heals_primary_and_four_injured_allies_within_catalog_radius() {
    let mut h = harness(Vec::new(), &[596, 98367], 5).expect("real world.db");
    let primary = spawn_player(&mut h.app, 5, 3.0);
    let allies: Vec<_> = (4..=9)
        .map(|x| spawn_player(&mut h.app, 5, x as f32))
        .collect();
    let healthy = spawn_player(&mut h.app, 5, 3.5);
    let far = spawn_player(&mut h.app, 5, 44.0);
    for &unit in allies.iter().chain([&primary, &far]) {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 100.0,
            max: 5000.0,
        };
    }
    h.cast_and_deliver(596, Some(primary));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(578.74198).contains(&(h.health(primary) - 100.0)));
    assert_eq!(
        allies
            .iter()
            .filter(|&&unit| h.health(unit) > 100.0)
            .count(),
        4
    );
    assert_eq!(h.health(healthy), 1000.0);
    assert_eq!(h.health(far), 100.0);
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn dummy3_prayer_radius_is_forty_yards_from_primary_not_caster() {
    let mut h = harness(Vec::new(), &[596, 98367], 5).expect("real world.db");
    let primary = spawn_player(&mut h.app, 5, 3.0);
    let inside = spawn_player(&mut h.app, 5, 42.0);
    let outside = spawn_player(&mut h.app, 5, 44.0);
    for unit in [primary, inside, outside] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 100.0,
            max: 5000.0,
        };
    }
    h.cast_and_deliver(596, Some(primary));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(578.74198).contains(&(h.health(inside) - 100.0)));
    assert_eq!(h.health(outside), 100.0);
}

#[test]
fn dummy3_thunder_clap_slows_each_hit_enemy_not_allies_or_far_enemies() {
    let mut h = harness(Vec::new(), &[6343, 435203], 1).expect("real world.db");
    let near = spawn_npc(&mut h.app, 5.0);
    let far = spawn_npc(&mut h.app, 20.0);
    let ally = spawn_player(&mut h.app, 1, 4.0);
    h.cast_and_deliver(6343, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    for unit in [h.npc, near] {
        assert!(h.health(unit) < 5000.0);
        assert!(crate::auras::has_aura(h.app.world(), unit, 435203));
    }
    for unit in [ally, far] {
        assert!(!crate::auras::has_aura(h.app.world(), unit, 435203));
    }
    h.advance(220);
    assert!(!crate::auras::has_aura(h.app.world(), near, 435203));
}

#[test]
fn dummy3_garrote_from_stealth_with_rank_silences_and_reduces_damage_for_five_seconds() {
    let mut h = harness(Vec::new(), &[703, 1784, 196861, 1330, 256148], 4).expect("real world.db");
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.cast_and_deliver(1784, Some(h.caster));
    h.advance(35);
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(703, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 1330));
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 256148));
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    h.advance(102);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 1330));
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 256148));
}

#[test]
fn dummy3_garrote_cannot_apply_silence_damage_reduction_to_silence_immune_target() {
    let mut h =
        harness(Vec::new(), &[703, 1784, 196861, 1330, 256148, 104773], 4).expect("real world.db");
    // Unending Resolve's real effect3 is mechanic9 (silence) immunity.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: 104773,
            effect_mask: 1 << 3,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.cast_and_deliver(1784, Some(h.caster));
    h.advance(35);
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(703, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 1330));
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 256148));
}

#[test]
fn dummy3_garrote_rank_without_stealth_has_no_silence() {
    let mut h = harness(Vec::new(), &[703, 196861, 1330, 256148], 4).expect("real world.db");
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(703, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 1330));
}

#[test]
fn dummy3_garrote_from_stealth_without_rank_has_no_silence() {
    let mut h = harness(Vec::new(), &[703, 1784, 1330, 256148], 4).expect("real world.db");
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.cast_and_deliver(1784, Some(h.caster));
    h.advance(35);
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(703, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 1330));
}

fn shred_damage(rank: bool, bleeding: bool) -> f32 {
    let mut ids = vec![768, 5221, 703];
    if rank {
        ids.push(231063);
    }
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 11).expect("real world.db");
    h.cast_and_deliver(768, Some(h.caster));
    h.advance(35);
    if bleeding {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.npc,
                spell_id: 703,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(5221, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    5000.0 - h.health(h.npc)
}

#[test]
fn dummy3_shred_rank_increases_damage_against_bleeding_target() {
    assert!(varied(260.48 * 1.25).contains(&shred_damage(true, true)));
}

#[test]
fn dummy3_shred_rank_requires_bleeding_and_bleed_bonus_requires_rank() {
    assert!(varied(260.48).contains(&shred_damage(true, false)));
    assert!(varied(260.48).contains(&shred_damage(false, true)));
}

fn devastate_reset(chance: f32, rank: bool) -> (f64, f64, bool) {
    let mut extra = Vec::new();
    if rank {
        // Real spell, explicitly varied chance endpoints; not fabricated IDs.
        let mut passive = world_db().unwrap().store.get(231834).unwrap().clone();
        passive.effects[0].base_points = chance;
        extra.push(passive);
    }
    let mut h = harness_with_native_enemy_dodge_reduction(extra, &[20243, 23922], 1)
        .expect("real world.db");
    h.cast_and_deliver(23922, Some(h.npc));
    h.advance(35);
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .spell_ready_at[&23922];
    h.inbox.cooldowns.clear();
    h.cast_and_deliver(20243, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .spell_ready_at[&23922];
    let update = h
        .inbox
        .cooldowns
        .iter()
        .any(|e| e.spell_id == 23922 && e.remaining_ms == 0);
    (before, after, update)
}

#[test]
fn dummy3_devastate_rank_chance_resets_shield_slam_and_notifies_owner() {
    let (before, after, update) = devastate_reset(100.0, true);
    assert!(after < before);
    assert!(update);
}

#[test]
fn dummy3_devastate_zero_chance_or_absent_rank_preserves_shield_slam_timer() {
    for (chance, rank) in [(0.0, true), (100.0, false)] {
        let (before, after, update) = devastate_reset(chance, rank);
        assert_eq!(after, before);
        assert!(!update);
    }
}

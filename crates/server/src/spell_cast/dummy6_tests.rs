//! Sixth batch: bounded scripts, authentic local Mainline catalog casts.
use super::test_harness::*;
use bevy::prelude::Entity;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

fn apply(h: &mut Harness, owner: Entity, target: Entity, id: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: owner,
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
fn landed(h: &Harness) {
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

fn sunfire(rank: bool) {
    let mut ids = vec![93402, 164815];
    if rank {
        ids.push(231050);
    }
    let mut h = harness(Vec::new(), &ids, 11).unwrap();
    let caster = h.caster;
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 40.0);
    let ally = spawn_player(&mut h.app, 11, 4.0);
    if rank {
        apply(&mut h, caster, caster, 231050, u32::MAX);
    }
    h.cast_and_deliver(93402, Some(h.npc));
    h.advance(12);
    landed(&h);
    assert!(damage(&h, 164815, h.npc) > 0);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 164815));
    assert_eq!(crate::auras::has_aura(h.app.world(), near, 164815), rank);
    for u in [far, ally] {
        assert!(!crate::auras::has_aura(h.app.world(), u, 164815));
    }
    assert_eq!(damage(&h, 164815, near), 0); // Splash is DoT, not additional direct hits.
    h.advance(45);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(164815) && e.periodic)
    );
    assert!(
        !h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 164815)
            .unwrap()
            .effects
            .iter()
            .any(|e| e.aura_type == 271)
    );
}
#[test]
fn dummy6_sunfire_base_damage_and_dot_exclude_optional_area() {
    sunfire(false);
}
#[test]
fn dummy6_sunfire_rank_spreads_only_dot_to_nearby_enemies() {
    sunfire(true);
}

#[test]
fn dummy6_templars_verdict_resolves_named_damage_child() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[85256, 224266], 2).unwrap();
    h.set_power(PowerType::HolyPower, 5.0);
    h.cast_and_deliver(85256, Some(h.npc));
    h.advance(8);
    landed(&h);
    assert!(varied(176.062).contains(&(damage(&h, 224266, h.npc) as f32)));
    assert_eq!(h.power(PowerType::HolyPower), 2.0);
}
#[test]
fn dummy6_ice_lance_resolves_named_damage_child() {
    let mut h = harness(Vec::new(), &[30455, 228598], 8).unwrap();
    h.cast_and_deliver(30455, Some(h.npc));
    h.advance(20);
    landed(&h);
    assert!(varied(183.75).contains(&(damage(&h, 228598, h.npc) as f32)));
}
#[test]
fn dummy6_starfire_primary_full_secondary_seventy_percent() {
    let mut h = harness(Vec::new(), &[194153], 11).unwrap();
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 40.0);
    let ally = spawn_player(&mut h.app, 11, 4.0);
    h.cast_and_deliver(194153, Some(h.npc));
    h.advance(65);
    landed(&h);
    assert!(varied(121.80001).contains(&(damage(&h, 194153, h.npc) as f32)));
    assert!(varied(85.26001).contains(&(damage(&h, 194153, near) as f32)));
    for u in [far, ally] {
        assert_eq!(damage(&h, 194153, u), 0);
    }
    assert_eq!(h.power(PowerType::LunarPower), 80.0);
}
#[test]
fn dummy6_keg_smash_primary_bonus_keeps_area_damage_and_slow() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[121253], 10).unwrap();
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 40.0);
    let ally = spawn_player(&mut h.app, 10, 4.0);
    h.cast_and_deliver(121253, Some(h.npc));
    h.advance(10);
    landed(&h);
    assert!(varied(285.0).contains(&(damage(&h, 121253, h.npc) as f32)));
    assert!(varied(190.0).contains(&(damage(&h, 121253, near) as f32)));
    for u in [h.npc, near] {
        assert!(crate::auras::has_aura(h.app.world(), u, 121253));
    }
    for u in [far, ally] {
        assert_eq!(damage(&h, 121253, u), 0);
        assert!(!crate::auras::has_aura(h.app.world(), u, 121253));
    }
}

fn bomb_ready(h: &Harness) -> f64 {
    let cat = world_db()
        .unwrap()
        .store
        .get(259495)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_category;
    h.app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&cat]
        .next_ready_at()
}
fn butchery(rank: bool, targets: usize) {
    let mut ids = vec![212436, 259495, 265163, 265157, 269747];
    if rank {
        ids.push(294029);
    }
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 3).unwrap();
    let c = h.caster;
    if rank {
        apply(&mut h, c, c, 294029, u32::MAX);
    }
    // Actual bomb starts the running recharge. Its unimplemented damage children
    // are outside this fixture's recharge contract.
    h.cast_and_deliver(259495, Some(h.npc));
    h.advance(35);
    landed(&h);
    for n in 1..targets {
        spawn_npc(&mut h.app, 3.0 + n as f32 * 0.1);
    }
    let far = spawn_npc(&mut h.app, 40.0);
    let ally = spawn_player(&mut h.app, 3, 4.0);
    let before = bomb_ready(&h);
    h.cast_and_deliver(212436, None);
    h.advance(5);
    landed(&h);
    let reduction = if rank {
        targets.min(5) as f64 * 3.0
    } else {
        0.0
    };
    assert!((bomb_ready(&h) - (before - reduction)).abs() < 1e-9);
    assert!(damage(&h, 212436, h.npc) > 0);
    for u in [far, ally] {
        assert_eq!(damage(&h, 212436, u), 0);
    }
    assert_eq!(
        h.inbox
            .charges
            .iter()
            .filter(|e| e.spell_id == 259495)
            .count()
            > 1,
        rank
    );
}
#[test]
fn dummy6_butchery_reduces_bomb_recharge_per_target_with_rank() {
    butchery(true, 2);
}
#[test]
fn dummy6_butchery_caps_recharge_reduction_at_five_hits() {
    butchery(true, 7);
}
#[test]
fn dummy6_butchery_without_rank_keeps_bomb_recharge() {
    butchery(false, 2);
}

#[test]
fn dummy6_wild_growth_caps_injured_allies_not_healthy_units() {
    let mut h = harness(Vec::new(), &[48438], 11).unwrap();
    let healthy = spawn_player(&mut h.app, 11, 1.0);
    let injured: Vec<_> = (0..7)
        .map(|n| spawn_player(&mut h.app, 11, 2.0 + n as f32))
        .collect();
    for &u in &injured {
        h.app.world_mut().get_mut::<Health>(u).unwrap().current = 100.0;
    }
    let far = spawn_player(&mut h.app, 11, 40.0);
    h.app.world_mut().get_mut::<Health>(far).unwrap().current = 100.0;
    h.cast_and_deliver(48438, Some(injured[0]));
    // Catalog cast time precedes its first1000ms periodic tick.
    h.advance(60);
    landed(&h);
    let marked = injured
        .iter()
        .filter(|&&u| crate::auras::has_aura(h.app.world(), u, 48438))
        .count();
    assert_eq!(marked, 5);
    for u in [healthy, far, h.npc] {
        assert!(!crate::auras::has_aura(h.app.world(), u, 48438));
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(48438) && e.periodic && e.kind == CombatLogKind::Heal)
    );
}
#[test]
fn dummy6_shadow_word_death_backlash_when_target_survives() {
    let mut h = harness(Vec::new(), &[32379], 5).unwrap();
    h.cast_and_deliver(32379, Some(h.npc));
    h.advance(12);
    landed(&h);
    assert!(damage(&h, 32379, h.npc) > 0);
    assert_eq!(h.health(h.caster), 950.0);
    assert_eq!(damage(&h, 32379, h.caster), 50);
}
#[test]
fn dummy6_shadow_word_death_no_backlash_on_kill() {
    let mut h = harness(Vec::new(), &[32379], 5).unwrap();
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1.0;
    h.cast_and_deliver(32379, Some(h.npc));
    h.advance(12);
    landed(&h);
    assert_eq!(h.health(h.npc), 0.0);
    assert_eq!(h.health(h.caster), 1000.0);
    assert_eq!(damage(&h, 32379, h.caster), 0);
}
fn expiry(h: &Harness, u: Entity, id: u32) -> f64 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(u)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == id)
        .unwrap()
        .expires_at
        .unwrap()
}
fn starfall(rank: bool) {
    let mut ids = vec![191034, 50286, 164812, 164815];
    if rank {
        ids.push(327541);
    }
    let mut h = harness(Vec::new(), &ids, 11).unwrap();
    let c = h.caster;
    let other = spawn_npc(&mut h.app, 4.0);
    let foreign = spawn_npc(&mut h.app, 5.0);
    if rank {
        apply(&mut h, c, c, 327541, u32::MAX);
    }
    let npc = h.npc;
    apply(&mut h, c, npc, 164812, 2);
    apply(&mut h, c, other, 164815, 2);
    apply(&mut h, foreign, foreign, 164812, 2);
    let m = expiry(&h, h.npc, 164812);
    let s = expiry(&h, other, 164815);
    let f = expiry(&h, foreign, 164812);
    h.set_power(PowerType::LunarPower, 1000.0);
    h.cast_and_deliver(191034, None);
    h.advance(5);
    landed(&h);
    let extra = if rank { 3.0 } else { 0.0 };
    assert_eq!(expiry(&h, h.npc, 164812), m + extra);
    assert_eq!(expiry(&h, other, 164815), s + extra);
    assert_eq!(expiry(&h, foreign, 164812), f);
}
#[test]
fn dummy6_starfall_extends_owned_dots_with_aetherial_kindling() {
    starfall(true);
}
#[test]
fn dummy6_starfall_without_rank_does_not_extend_dots() {
    starfall(false);
}

#[test]
fn dummy6_butchery_does_not_start_ready_bomb_recharge() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[212436, 294029, 259495], 3)
        .unwrap();
    let c = h.caster;
    apply(&mut h, c, c, 294029, u32::MAX);
    h.cast_and_deliver(212436, None);
    h.advance(5);
    landed(&h);
    assert!(damage(&h, 212436, h.npc) > 0);
    assert!(!h.inbox.charges.iter().any(|e| e.spell_id == 259495));
}
#[test]
fn dummy6_butchery_counts_fully_absorbed_hits_for_recharge() {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[212436, 294029, 259495, 265163, 265157, 269747, 17],
        3,
    )
    .unwrap();
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, caster, 294029, u32::MAX);
    h.cast_and_deliver(259495, Some(npc));
    h.advance(35);
    landed(&h);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .spell_power = 200.0;
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .healing_power = 200.0;
    apply(&mut h, caster, npc, 17, 1);
    let before = bomb_ready(&h);
    h.cast_and_deliver(212436, None);
    h.advance(5);
    landed(&h);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(212436) && e.kind == CombatLogKind::Damage)
        .unwrap();
    assert_eq!(hit.amount, 0);
    assert!(hit.absorbed > 0);
    assert!((bomb_ready(&h) - (before - 3.0)).abs() < 1e-9);
}
#[test]
fn dummy6_starfall_caps_extension_and_preserves_existing_tick_time() {
    let mut h = harness(Vec::new(), &[191034, 50286, 327541, 164812], 11).unwrap();
    let c = h.caster;
    let npc = h.npc;
    apply(&mut h, c, c, 327541, u32::MAX);
    apply(&mut h, c, npc, 164812, 2);
    // A prior extension leaves 27 seconds; next cast may add only up to28.
    crate::auras::extend_owned_duration(h.app.world_mut(), c, npc, &[164812], 10000);
    h.advance(20);
    let before = expiry(&h, npc, 164812);
    h.set_power(PowerType::LunarPower, 1000.0);
    h.cast_and_deliver(191034, None);
    h.advance(5);
    landed(&h);
    let after = expiry(&h, npc, 164812);
    assert!(
        after > before && after < before + 2.0,
        "before={before} after={after}"
    );
    // Original 2sec tick arrives; a refresh at1sec would postpone it to3sec.
    h.advance(20);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(164812) && e.periodic && e.amount > 0)
    );
}

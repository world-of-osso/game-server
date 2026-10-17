//! Seventh batch: real Mainline rows, bounded cast behavior.
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
fn logged(h: &Harness, id: u32, target: Entity, kind: CombatLogKind) -> i32 {
    h.inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(id) && e.target == Some(target.to_bits()) && e.kind == kind)
        .map(|e| e.amount)
        .sum()
}
fn landed(h: &Harness) {
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn dummy7_colossus_smash_marks_each_hit_enemy_and_only_amplifies_owners_damage() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[167105, 208086, 108853], 1)
        .unwrap();
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 40.0);
    let foreign = spawn_player(&mut h.app, 1, 0.0);
    h.cast_and_deliver(167105, Some(h.npc));
    h.advance(5);
    landed(&h);
    for u in [h.npc, near] {
        assert!(crate::auras::has_aura(h.app.world(), u, 208086));
    }
    assert!(!crate::auras::has_aura(h.app.world(), far, 208086));
    assert!(!crate::auras::has_aura(h.app.world(), foreign, 208086));
    assert!(varied(306.263).contains(&(logged(&h, 167105, h.npc, CombatLogKind::Damage) as f32)));
    let npc = h.npc;
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), caster, 108853, Some(npc), 1);
    h.advance(10);
    assert!(varied(256.425).contains(&(logged(&h, 108853, npc, CombatLogKind::Damage) as f32)));
    let before = h.health(npc);
    super::cast_triggered(h.app.world_mut(), foreign, 108853, Some(npc), 1);
    h.advance(10);
    // The unconnected foreign actor has no owner inbox; observe the actual loss.
    assert!(varied(197.25).contains(&(before - h.health(npc))));
    h.advance(250);
    assert!(!crate::auras::has_aura(h.app.world(), npc, 208086));
}

#[test]
fn dummy7_survival_instincts_halves_incoming_damage_and_cancellation_removes_child() {
    for _ in 0..100 {
        survival_instincts_damage_case();
    }
}
fn survival_instincts_damage_case() {
    let mut h = harness(Vec::new(), &[61336, 50322, 5487, 108853], 11).unwrap();
    let caster = h.caster;
    h.cast_and_deliver(5487, Some(caster));
    h.advance(35);
    h.cast_and_deliver(61336, Some(caster));
    h.advance(3);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), caster, 50322));
    let attacker = spawn_player(&mut h.app, 1, 4.0);
    h.app
        .world_mut()
        .entity_mut(attacker)
        .insert(shared::components::UnitFactionTemplate(25));
    let before = h.health(caster);
    super::cast_triggered(h.app.world_mut(), attacker, 108853, Some(caster), 1);
    h.advance(10);
    // Mainline Bear5487 effect12 already reduces Fire damage by3%.
    // Survival halves that existing97% multiplier, not an unmodified hit.
    assert!(
        varied(95.66625).contains(&(before - h.health(caster))),
        "before={before}, after={}",
        h.health(caster)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 61336,
    );
    assert!(!crate::auras::has_aura(h.app.world(), caster, 50322));
    let before = h.health(caster);
    super::cast_triggered(h.app.world_mut(), attacker, 108853, Some(caster), 1);
    h.advance(10);
    assert!(
        varied(191.3325).contains(&(before - h.health(caster))),
        "Bear's remaining3% reduction: before={before}, after={}",
        h.health(caster)
    );
}
#[test]
fn dummy7_survival_instincts_child_expires_with_parent() {
    let mut h = harness(Vec::new(), &[61336, 50322, 5487], 11).unwrap();
    let c = h.caster;
    h.cast_and_deliver(5487, Some(c));
    h.advance(35);
    h.cast_and_deliver(61336, Some(c));
    h.advance(3);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), c, 50322));
    h.advance(130);
    assert!(!crate::auras::has_aura(h.app.world(), c, 61336));
    assert!(!crate::auras::has_aura(h.app.world(), c, 50322));
}

fn heartbreaker(rank: bool) {
    let mut ids = vec![206930, 210738];
    if rank {
        ids.push(221536);
    }
    // Every chain target must hit; avoidance has separate native tests.
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 6).unwrap();
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 40.0);
    let ally = spawn_player(&mut h.app, 6, 4.0);
    h.set_power(PowerType::RunicPower, 0.0);
    h.cast_and_deliver(206930, Some(h.npc));
    h.advance(4);
    landed(&h);
    assert!(logged(&h, 206930, h.npc, CombatLogKind::Damage) > 0);
    assert!(logged(&h, 206930, near, CombatLogKind::Damage) > 0);
    for u in [far, ally] {
        assert_eq!(logged(&h, 206930, u, CombatLogKind::Damage), 0);
    }
    assert_eq!(
        logged(&h, 210738, h.caster, CombatLogKind::Energize),
        if rank { 40 } else { 0 }
    );
}
#[test]
fn dummy7_heartbreaker_energizes_per_native_chain_target() {
    heartbreaker(true);
}
#[test]
fn dummy7_heart_strike_without_heartbreaker_has_no_bonus() {
    heartbreaker(false);
}

#[test]
fn dummy7_radiance_heals_primary_and_four_injured_allies_inside_catalog_radius() {
    let mut h = harness(Vec::new(), &[194509, 194384], 5).unwrap();
    let primary = spawn_player(&mut h.app, 5, 3.0); // Healthy primary still selected.
    let healthy = spawn_player(&mut h.app, 5, 4.0);
    let injured: Vec<Entity> = (0..5)
        .map(|i| {
            let u = spawn_player(&mut h.app, 5, 5.0 + i as f32);
            h.app.world_mut().get_mut::<Health>(u).unwrap().current = 100.0;
            u
        })
        .collect();
    let far = spawn_player(&mut h.app, 5, 60.0);
    h.app.world_mut().get_mut::<Health>(far).unwrap().current = 100.0;
    h.cast_and_deliver(194509, Some(primary));
    h.advance(65);
    landed(&h);
    assert!(logged(&h, 194509, primary, CombatLogKind::Heal) > 0);
    for &u in &injured[..4] {
        assert!(h.health(u) > 100.0);
    }
    assert_eq!(h.health(injured[4]), 100.0);
    assert_eq!(logged(&h, 194509, healthy, CombatLogKind::Heal), 0);
    assert_eq!(h.health(far), 100.0);
    assert_eq!(logged(&h, 194509, h.npc, CombatLogKind::Heal), 0);
}
#[test]
fn dummy7_radiance_prefers_injured_allies_without_owners_atonement() {
    let mut h = harness(Vec::new(), &[194509, 194384], 5).unwrap();
    let c = h.caster;
    let primary = spawn_player(&mut h.app, 5, 3.0);
    let marked = spawn_player(&mut h.app, 5, 4.0);
    h.app.world_mut().get_mut::<Health>(marked).unwrap().current = 100.0;
    apply(&mut h, c, marked, 194384, 1);
    let unmarked: Vec<Entity> = (0..4)
        .map(|i| {
            let u = spawn_player(&mut h.app, 5, 5.0 + i as f32);
            h.app.world_mut().get_mut::<Health>(u).unwrap().current = 100.0;
            u
        })
        .collect();
    // A foreign mark must not demote this ally.
    apply(&mut h, primary, unmarked[0], 194384, 1);
    h.cast_and_deliver(194509, Some(primary));
    h.advance(60);
    landed(&h);
    for u in unmarked {
        assert!(h.health(u) > 100.0);
    }
    assert_eq!(h.health(marked), 100.0);
}

fn raging_blow(chance: f32, rank: bool) {
    let mut root = world_db().unwrap().store.get(85288).unwrap().clone();
    // Controlled probability endpoints on the real root, no fabricated spell.
    root.effects
        .iter_mut()
        .find(|e| e.index == 0)
        .unwrap()
        .base_points = chance;
    let mut ids = vec![85288, 96103, 85384, 280270];
    if rank {
        ids.push(383854);
    }
    let mut h = harness_with_native_enemy_dodge_reduction(vec![root], &ids, 1).unwrap();
    h.cast_and_deliver(85288, Some(h.npc));
    h.advance(4);
    landed(&h);
    let cat = world_db()
        .unwrap()
        .store
        .get(85288)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_category;
    let used = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&cat]
        .used();
    let reset = rank && chance == 100.0;
    assert_eq!(used, if reset { 0 } else { 1 });
    // This real one-charge proc marker can be consumed by the root's damage
    // event before delivery. Assert application, not continued presence.
    assert_eq!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(280270) && e.kind == CombatLogKind::AuraApplied),
        reset
    );
    if reset {
        assert!(
            h.inbox
                .charges
                .iter()
                .any(|e| e.spell_id == 85288 && e.current == e.max)
        );
    }
}
#[test]
fn dummy7_raging_blow_rank_success_restores_one_charge_and_notifies_owner() {
    raging_blow(100.0, true);
}
#[test]
fn dummy7_raging_blow_restores_only_latest_charge_preserving_oldest_recharge() {
    let ids = [85288, 96103, 85384, 280270, 383854];
    let mut root = world_db().unwrap().store.get(85288).unwrap().clone();
    root.effects
        .iter_mut()
        .find(|e| e.index == 0)
        .unwrap()
        .base_points = 0.0;
    let mut h = harness_with_native_enemy_dodge_reduction(vec![root.clone()], &ids, 1).unwrap();
    h.cast_and_deliver(85288, Some(h.npc));
    h.advance(4);
    landed(&h);
    let cat = root.categories.as_ref().unwrap().charge_category;
    let oldest = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&cat]
        .next_ready_at();
    h.advance(35);
    root.effects
        .iter_mut()
        .find(|e| e.index == 0)
        .unwrap()
        .base_points = 100.0;
    let spells = ids
        .into_iter()
        .map(|id| {
            if id == 85288 {
                root.clone()
            } else {
                world_db().unwrap().store.get(id).unwrap().clone()
            }
        })
        .collect();
    h.app
        .world_mut()
        .insert_resource(crate::spell_info::SpellStore::fixture(spells));
    h.cast_and_deliver(85288, Some(h.npc));
    h.advance(4);
    landed(&h);
    let charges = &h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&cat];
    assert_eq!(charges.used(), 1);
    assert_eq!(charges.next_ready_at(), oldest);
}
#[test]
fn dummy7_raging_blow_rank_zero_chance_keeps_recharge() {
    raging_blow(0.0, true);
}
#[test]
fn dummy7_raging_blow_without_rank_does_not_reset() {
    raging_blow(100.0, false);
}

fn whirlwind(rank: bool, enemies: usize) {
    let mut ids = vec![190411, 199667, 44949, 199852, 199851, 280715, 85739];
    if rank {
        ids.push(12950);
    }
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 1).unwrap();
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 40.0;
    for i in 0..enemies {
        spawn_npc(&mut h.app, 2.0 + i as f32 * 0.2);
    }
    let ally = spawn_player(&mut h.app, 1, 3.0);
    h.set_power(PowerType::Rage, 0.0);
    h.cast_and_deliver(190411, None);
    h.advance(4);
    landed(&h);
    let raw = (3 + if rank { enemies.min(5) } else { 0 }) as f32 * 10.0;
    assert_eq!(h.power(PowerType::Rage), raw);
    assert_eq!(
        logged(&h, 280715, h.caster, CombatLogKind::Energize),
        raw as i32
    );
    assert_eq!(
        crate::auras::has_aura(h.app.world(), h.caster, 85739),
        rank && enemies > 0
    );
    if rank && enemies > 0 {
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 85739)
            .unwrap();
        assert_eq!(aura.stacks, 4); // Mainline85739 CumulativeAura=4.
    }
    assert_eq!(logged(&h, 199667, ally, CombatLogKind::Damage), 0);
}
#[test]
fn dummy7_whirlwind_rank_generates_per_enemy_rage_up_to_five() {
    whirlwind(true, 7);
}
#[test]
fn dummy7_whirlwind_rank_counts_two_enemies_without_using_maximum() {
    whirlwind(true, 2);
}
#[test]
fn dummy7_whirlwind_without_rank_generates_only_base_rage() {
    whirlwind(false, 2);
}
#[test]
fn dummy7_whirlwind_empty_area_keeps_base_rage_without_cleave_buff() {
    whirlwind(true, 0);
}

fn panacea(rank: bool) {
    let mut ids = vec![360995, 387763];
    if rank {
        ids.push(387761);
    }
    let mut h = harness(Vec::new(), &ids, 13).unwrap();
    let c = h.caster;
    h.app.world_mut().get_mut::<Health>(c).unwrap().current = 100.0;
    let ally = spawn_player(&mut h.app, 13, 4.0);
    h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 100.0;
    h.cast_and_deliver(360995, Some(ally));
    h.advance(5);
    landed(&h);
    let heal = logged(&h, 387763, c, CombatLogKind::Heal);
    if rank {
        assert!(varied(250.0).contains(&(heal as f32)));
        assert_eq!(h.health(c), 100.0 + heal as f32);
    } else {
        assert_eq!(heal, 0);
        assert_eq!(h.health(c), 100.0);
    }
    assert_eq!(logged(&h, 387763, ally, CombatLogKind::Heal), 0);
}
#[test]
fn dummy7_verdant_embrace_panacea_heals_caster_not_selected_ally() {
    panacea(true);
}
#[test]
fn dummy7_verdant_embrace_without_panacea_does_not_self_heal() {
    panacea(false);
}

fn rake(stealth: bool, rank: Option<u32>) {
    let mut ids = vec![1822, 155722, 163505, 768, 5215];
    ids.extend(rank);
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 11).unwrap();
    let c = h.caster;
    h.cast_and_deliver(768, Some(c));
    h.advance(35);
    if stealth {
        apply(&mut h, c, c, 5215, 1);
    }
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(1822, Some(h.npc));
    h.advance(3);
    landed(&h);
    let bonus = stealth && rank.is_some();
    let amount = if bonus { 46.56 } else { 29.1 };
    assert!(varied(amount).contains(&(logged(&h, 1822, h.npc, CombatLogKind::Damage) as f32)));
    assert_eq!(crate::auras::has_aura(h.app.world(), h.npc, 163505), bonus);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 155722));
}
#[test]
fn dummy7_rake_stealth_and_rank_grant_initial_bonus_and_stun() {
    rake(true, Some(231052));
}
#[test]
fn dummy7_rake_pouncing_strikes_also_gates_stealth_bonus() {
    rake(true, Some(405834));
}
#[test]
fn dummy7_rake_stealth_without_rank_keeps_base_initial_damage() {
    rake(true, None);
}
#[test]
fn dummy7_rake_rank_without_stealth_keeps_base_initial_damage() {
    rake(false, Some(231052));
}

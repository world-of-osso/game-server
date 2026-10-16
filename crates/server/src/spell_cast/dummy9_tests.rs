//! Batch9: real Mainline spell rows; no world.db mutation.
use super::test_harness::*;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{Health, Position, PowerType};
use shared::protocol::{CombatChannel, CombatLogKind, SpellCastIntent};

fn landed(h: &Harness) {
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}
fn logged(h: &Harness, spell: u32, target: Entity, kind: CombatLogKind) -> i32 {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(spell) && e.target == Some(target.to_bits()) && e.kind == kind
        })
        .map(|e| e.amount)
        .sum()
}
fn ground(h: &mut Harness, id: u32, point: [f32; 3]) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(id),
            spell: String::new(),
            target_entity: None,
            destination: Some(point),
            target_item_guid: None,
            witness: None,
        });
    h.advance(3);
}
fn injured(h: &mut Harness, x: f32) -> Entity {
    let e = spawn_player(&mut h.app, 13, x);
    h.app.world_mut().get_mut::<Health>(e).unwrap().current = 50.0;
    e
}
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
#[test]
fn dummy9_healing_rain_ticks_at_ground_caps_six_and_stops_at_expiry() {
    let mut h = harness(Vec::new(), &[73920, 73921, 1298679], 7).unwrap();
    let allies: Vec<_> = (0..7).map(|i| injured(&mut h, 20.0 + i as f32)).collect();
    let healthy = spawn_player(&mut h.app, 7, 19.0);
    let far = injured(&mut h, 32.0);
    ground(&mut h, 73920, [20.0, 0.0, 0.0]);
    h.advance(42); // Complete native 2000ms cast, but not first 2000ms pulse.
    landed(&h);
    assert!(allies.iter().all(|&u| h.health(u) == 50.0));
    h.advance(42);
    assert_eq!(allies.iter().filter(|&&u| h.health(u) > 50.0).count(), 6);
    assert_eq!(h.health(far), 50.0);
    assert_eq!(logged(&h, 73921, healthy, CombatLogKind::Heal), 0);
    assert_eq!(logged(&h, 73921, h.npc, CombatLogKind::Heal), 0);
    h.advance(360);
    let before = h.inbox.log.len();
    for &u in &allies {
        h.app.world_mut().get_mut::<Health>(u).unwrap().current = 50.0;
    }
    h.advance(45);
    assert_eq!(h.inbox.log.len(), before);
}
#[test]
fn dummy9_healing_rain_recast_replaces_only_owners_previous_ground_field() {
    let mut h = harness(Vec::new(), &[73920, 73921, 1298679], 7).unwrap();
    let old = injured(&mut h, 0.0);
    let new = injured(&mut h, 25.0);
    ground(&mut h, 73920, [0.0, 0.0, 0.0]);
    h.advance(300); // Native cooldown is12000ms, starting at cast completion.
    landed(&h);
    h.app.world_mut().get_mut::<Health>(old).unwrap().current = 50.0;
    ground(&mut h, 73920, [25.0, 0.0, 0.0]);
    h.advance(42);
    landed(&h);
    h.app.world_mut().get_mut::<Health>(old).unwrap().current = 50.0;
    h.advance(42);
    assert_eq!(h.health(old), 50.0);
    assert!(h.health(new) > 50.0);
}
#[test]
fn dummy9_earthquake_pulses_six_times_at_requested_ground_and_excludes_allies() {
    let mut h = harness(Vec::new(), &[61882, 77478], 7).unwrap();
    h.set_power(PowerType::Maelstrom, 100.0);
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 20.0,
        y: 0.0,
        z: 0.0,
    });
    let far = spawn_npc(&mut h.app, 28.1);
    let ally = spawn_player(&mut h.app, 7, 20.0);
    ground(&mut h, 61882, [20.0, 0.0, 0.0]);
    h.advance(8);
    landed(&h);
    assert_eq!(logged(&h, 77478, h.npc, CombatLogKind::Damage), 0);
    h.advance(125);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(77478)
                && e.target == Some(h.npc.to_bits())
                && e.kind == CombatLogKind::Damage)
            .count(),
        6
    );
    assert_eq!(logged(&h, 77478, far, CombatLogKind::Damage), 0);
    assert_eq!(logged(&h, 77478, ally, CombatLogKind::Damage), 0);
    let before = logged(&h, 77478, h.npc, CombatLogKind::Damage);
    h.advance(40);
    assert_eq!(logged(&h, 77478, h.npc, CombatLogKind::Damage), before);
}
#[test]
fn dummy9_emerald_blossom_delays_and_heals_three_injured_at_fixed_ally_location() {
    let mut h = harness(Vec::new(), &[355913, 355916], 13).unwrap();
    let primary = injured(&mut h, 20.0);
    let healthy = spawn_player(&mut h.app, 13, 20.0);
    let others: Vec<_> = (0..4).map(|i| injured(&mut h, 21.0 + i as f32)).collect();
    let far = injured(&mut h, 31.0);
    h.cast_and_deliver(355913, Some(primary));
    h.advance(10);
    landed(&h);
    assert_eq!(h.health(primary), 50.0);
    h.app.world_mut().entity_mut(primary).insert(Position {
        x: 40.0,
        y: 0.0,
        z: 0.0,
    });
    h.advance(30);
    assert_eq!(others.iter().filter(|&&u| h.health(u) > 50.0).count(), 3);
    assert_eq!(h.health(primary), 50.0);
    assert_eq!(h.health(far), 50.0);
    assert_eq!(logged(&h, 355916, healthy, CombatLogKind::Heal), 0);
    let before = logged(&h, 355916, others[0], CombatLogKind::Heal);
    h.advance(40);
    assert_eq!(logged(&h, 355916, others[0], CombatLogKind::Heal), before);
}
#[test]
fn dummy9_remorseless_winter_hit_applies_slow_and_native_expiry_removes_it() {
    // These children have no radius in local DB2. Exercise native coincident-unit
    // selection, not an invented eight-yard radius or moving projectile.
    let mut h = harness(Vec::new(), &[196770, 196771, 211793], 6).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let far = spawn_npc(&mut h.app, 20.0);
    h.cast_and_deliver(196770, None);
    h.advance(25);
    landed(&h);
    assert!(logged(&h, 196771, h.npc, CombatLogKind::Damage) > 0);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 211793));
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.npc)
            .unwrap()
            .movement_speed,
        0.8
    );
    assert!(!crate::auras::has_aura(h.app.world(), far, 211793));
    super::cancel_cast(h.app.world_mut(), h.caster);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 196770,
    );
    h.advance(65);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 211793));
}
#[test]
fn dummy9_scourge_strike_applies_base_virulent_plague_without_refreshing_existing_owned_mark() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[55090, 191587], 6).unwrap();
    h.cast_and_deliver(55090, Some(h.npc));
    h.advance(5);
    landed(&h);
    let a = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 191587 && a.caster == h.caster)
        .unwrap();
    let expiry = a.expires_at;
    h.advance(35);
    h.cast_and_deliver(55090, Some(h.npc));
    h.advance(4);
    landed(&h);
    let a = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 191587 && a.caster == h.caster)
        .unwrap();
    assert_eq!(a.expires_at, expiry);
    assert_eq!(a.effects.len(), 1);
    h.advance(60);
    assert!(logged(&h, 191587, h.npc, CombatLogKind::Damage) > 0);
}
#[test]
fn dummy9_fracture_requires_both_meta_and_rank_for_fifteen_additional_fury() {
    for (meta, rank, expected) in [
        (false, false, 25.0),
        (true, false, 25.0),
        (false, true, 25.0),
        (true, true, 40.0),
    ] {
        let mut ids = vec![263642, 225919, 225921, 187827];
        if rank {
            ids.push(321068);
        }
        let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 12).unwrap();
        let caster = h.caster;
        if rank {
            apply(&mut h, caster, caster, 321068, u32::MAX);
        }
        if meta {
            apply(&mut h, caster, caster, 187827, u32::MAX);
        }
        h.set_power(PowerType::Fury, 0.0);
        h.cast_and_deliver(263642, Some(h.npc));
        h.advance(5);
        landed(&h);
        assert_eq!(
            h.power(PowerType::Fury),
            expected,
            "meta={meta} rank={rank}"
        );
    }
}
#[test]
fn dummy9_void_step_teleports_to_requested_location_preserving_map() {
    let mut h = harness(Vec::new(), &[1234796], 12).unwrap();
    let before = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    ground(&mut h, 1234796, [10.0, 0.0, 3.0]);
    h.advance(3);
    landed(&h);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        Position {
            x: 10.0,
            y: 0.0,
            z: 3.0
        }
    );
    assert_eq!(
        *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap(),
        before
    );
}
#[test]
fn dummy9_named_ground_spells_reject_missing_or_out_of_range_ground_before_cooldown() {
    for id in [73920, 61882, 1234796] {
        for point in [None, Some([1000.0, 0.0, 0.0]), Some([f32::NAN, 0.0, 0.0])] {
            let mut h = harness(Vec::new(), &[id], if id == 1234796 { 12 } else { 7 }).unwrap();
            if id == 61882 {
                h.set_power(PowerType::Maelstrom, 100.0);
            }
            if let Some(p) = point {
                ground(&mut h, id, p);
            } else {
                h.cast_and_deliver(id, None);
            }
            h.advance(3);
            assert_eq!(h.inbox.failed.len(), 1, "spell={id} point={point:?}");
            assert!(h.inbox.go.is_empty());
        }
    }
}
#[test]
fn dummy9_radiance_applies_nine_second_atonement_to_selected_allies_preserves_foreign_mark() {
    let mut h = harness(Vec::new(), &[194509, 194384], 5).unwrap();
    let primary = injured(&mut h, 10.0);
    let foreign = spawn_player(&mut h.app, 5, 12.0);
    apply(&mut h, foreign, primary, 194384, u32::MAX);
    let allies: Vec<_> = (0..5).map(|i| injured(&mut h, 14.0 + i as f32)).collect();
    h.cast_and_deliver(194509, Some(primary));
    h.advance(45);
    landed(&h);
    let owned: Vec<_> = std::iter::once(primary)
        .chain(allies.iter().copied())
        .filter(|&u| {
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(u)
                .is_some_and(|a| {
                    a.iter()
                        .any(|a| a.spell_id == 194384 && a.caster == h.caster)
                })
        })
        .collect();
    assert_eq!(owned.len(), 5);
    for &u in &owned {
        let a = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(u)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 194384 && a.caster == h.caster)
            .unwrap();
        assert_eq!(a.duration_ms, 9000);
    }
    h.advance(185);
    for &u in &owned {
        assert!(
            !h.app
                .world()
                .get::<crate::auras::ActiveAuras>(u)
                .unwrap()
                .iter()
                .any(|a| a.spell_id == 194384 && a.caster == h.caster)
        );
    }
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(primary)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 194384 && a.caster == foreign)
    );
}
#[test]
fn dummy9_tranquility_shield_absorbs_sixty_percent_max_health_and_ends_with_channel() {
    let mut h = harness(Vec::new(), &[740, 157982, 133], 11).unwrap();
    h.cast_and_deliver(740, None);
    h.advance(3);
    landed(&h);
    let fire = world_db().unwrap().store.get(133).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: &fire,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, h.caster, 900.0, false, false);
    assert_eq!(h.health(h.caster), 700.0);
    // Depleting the shield must not remove Tranquility's healing/channel aura.
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 740));
    h.advance(22);
    assert!(h.health(h.caster) > 700.0);
    let healed = h.health(h.caster);
    super::cancel_cast(h.app.world_mut(), h.caster);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 740));
    super::effects::deal_damage(h.app.world_mut(), &ctx, h.caster, 200.0, false, false);
    assert_eq!(h.health(h.caster), healed - 200.0);
}

#[test]
fn dummy9_earthquake_overlapping_casts_keep_independent_pulse_clocks() {
    let mut h = harness(Vec::new(), &[61882, 77478], 7).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert((
        Position {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        },
        Health {
            current: 2000.0,
            max: 2000.0,
        },
    ));
    h.set_power(PowerType::Maelstrom, 100.0);
    ground(&mut h, 61882, [20.0, 0.0, 0.0]);
    h.advance(40);
    h.set_power(PowerType::Maelstrom, 100.0);
    ground(&mut h, 61882, [20.0, 0.0, 0.0]);
    h.advance(140);
    landed(&h);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(77478)
                && e.target == Some(h.npc.to_bits())
                && e.kind == CombatLogKind::Damage)
            .count(),
        12
    );
}
#[test]
fn dummy9_healing_rain_created_in_empty_area_heals_ally_walking_in_later() {
    let mut h = harness(Vec::new(), &[73920, 73921, 1298679], 7).unwrap();
    ground(&mut h, 73920, [20.0, 0.0, 0.0]);
    h.advance(42);
    landed(&h);
    let ally = injured(&mut h, 20.0);
    h.advance(42);
    assert!(h.health(ally) > 50.0);
    h.app.world_mut().entity_mut(ally).insert(Position {
        x: 40.0,
        y: 0.0,
        z: 0.0,
    });
    h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 50.0;
    h.advance(45);
    assert_eq!(h.health(ally), 50.0);
}
#[test]
fn dummy9_tranquility_cancellation_removes_unspent_shield() {
    let mut h = harness(Vec::new(), &[740, 157982, 133], 11).unwrap();
    h.cast_and_deliver(740, None);
    h.advance(3);
    landed(&h);
    let fire = world_db().unwrap().store.get(133).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: &fire,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, h.caster, 200.0, false, false);
    assert_eq!(h.health(h.caster), 1000.0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 740));
    super::cancel_cast(h.app.world_mut(), h.caster);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 740));
    super::effects::deal_damage(h.app.world_mut(), &ctx, h.caster, 200.0, false, false);
    assert_eq!(h.health(h.caster), 800.0);
}

//! Batch8: authenticated casts against real, read-only Mainline catalog rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, Npc, Position, PowerType, ThreatTable, UnitSummonedBy};
use shared::protocol::CombatLogKind;

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
fn at(h: &mut Harness, x: f32, z: f32) -> Entity {
    let e = spawn_npc(&mut h.app, x);
    h.app
        .world_mut()
        .entity_mut(e)
        .insert(Position { x, y: 0.0, z });
    e
}
#[test]
fn dummy8_reap_hits_enemies_along_destination_line_not_single_target_or_radial_area() {
    let mut h = harness(Vec::new(), &[1226019, 1225827], 12).unwrap();
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert(Position {
        x: 10.0,
        y: 0.0,
        z: 10.0,
    });
    let beyond = at(&mut h, 18.0, 18.0);
    let behind = at(&mut h, -5.0, -5.0);
    let outside = at(&mut h, 24.0, -15.0);
    let far = at(&mut h, 24.0, 24.0);
    let ally = spawn_player(&mut h.app, 12, 5.0);
    let other_map = at(&mut h, 8.0, 8.0);
    h.app
        .world_mut()
        .entity_mut(other_map)
        .insert(crate::maps::InMap::continent(1));
    h.cast_and_deliver(1226019, Some(npc));
    h.advance(40);
    landed(&h);
    for e in [npc, beyond] {
        // Active Effect1215275: 6.0156 SP × fixture SP100, rounded.
        assert_eq!(logged(&h, 1225827, e, CombatLogKind::Damage), 602);
    }
    for e in [behind, outside, far, ally, other_map] {
        assert_eq!(logged(&h, 1225827, e, CombatLogKind::Damage), 0);
    }
}
fn engage(h: &mut Harness, npc: Entity, victim: Entity) {
    let mut table = ThreatTable::default();
    table.add_threat(victim.to_bits(), 100.0);
    h.app.world_mut().entity_mut(npc).insert((
        table,
        crate::creature_ai::CreatureState::Pursuing {
            target: victim.to_bits(),
        },
        crate::networking::CombatTarget(Some(victim)),
    ));
}
#[test]
fn dummy8_provoke_changes_victim_equalizes_threat_and_holds_until_expiry() {
    let mut h = harness(Vec::new(), &[115546, 116189], 10).unwrap();
    h.app
        .add_systems(FixedUpdate, crate::creature_aggro::update_victim_system);
    let rival = spawn_player(&mut h.app, 10, 4.0);
    let npc = h.npc;
    let caster = h.caster;
    engage(&mut h, npc, rival);
    h.cast_and_deliver(115546, Some(npc));
    // TC ThreatManager.h:86 / ThreatManager.cpp:216–230: selection is periodic.
    // EffectTaunt (SpellEffects.cpp:2772–2810) equalizes threat, not an instant victim refresh.
    h.advance(24);
    landed(&h);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(npc)
            .unwrap()
            .0,
        Some(caster)
    );
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(npc)
            .unwrap()
            .threat_for(caster.to_bits()),
        100.0
    );
    crate::creature_aggro::add_threat(h.app.world_mut(), npc, caster, 10.0);
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(npc)
            .unwrap()
            .threat_for(caster.to_bits()),
        190.0
    );
    crate::creature_aggro::add_threat(h.app.world_mut(), npc, rival, 1000.0);
    h.advance(5);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(npc)
            .unwrap()
            .0,
        Some(caster)
    );
    h.advance(130); // Catalog116189 duration is6000ms.
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(npc)
            .unwrap()
            .0,
        Some(rival)
    );
    crate::creature_aggro::add_threat(h.app.world_mut(), npc, caster, 10.0);
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(npc)
            .unwrap()
            .threat_for(caster.to_bits()),
        200.0
    );
}
fn statue(h: &mut Harness, owner: Entity) -> Entity {
    let e = spawn_player(&mut h.app, 10, 5.0);
    h.app
        .world_mut()
        .entity_mut(e)
        .remove::<shared::components::Player>()
        .insert((
            Npc {
                template_id: 61146,
                name: "Black Ox Statue".into(),
            },
            UnitSummonedBy(owner.to_bits()),
        ));
    e
}
#[test]
fn dummy8_provoke_owned_statue_taunts_nearby_enemies_not_allies_or_far_units() {
    let mut h = harness(Vec::new(), &[115546, 116189, 118635], 10).unwrap();
    let caster = h.caster;
    let statue = statue(&mut h, caster);
    let near = spawn_npc(&mut h.app, 12.0);
    let far = spawn_npc(&mut h.app, 20.1);
    for e in [h.npc, near, far] {
        h.app.world_mut().entity_mut(e).insert((
            ThreatTable::default(),
            crate::creature_ai::CreatureState::Idle,
        ));
    }
    h.cast_and_deliver(115546, Some(statue));
    h.advance(3);
    landed(&h);
    for e in [h.npc, near] {
        assert_eq!(
            h.app
                .world()
                .get::<crate::networking::CombatTarget>(e)
                .unwrap()
                .0,
            Some(caster)
        );
        assert!(crate::auras::has_aura(h.app.world(), e, 118635));
    }
    for e in [far, statue, caster] {
        assert!(!crate::auras::has_aura(h.app.world(), e, 118635));
    }
}
#[test]
fn dummy8_provoke_rejects_foreign_statue_and_ordinary_ally_before_cooldown() {
    for statue_target in [false, true] {
        let mut h = harness(Vec::new(), &[115546, 116189, 118635], 10).unwrap();
        let ally = spawn_player(&mut h.app, 10, 5.0);
        let target = if statue_target {
            statue(&mut h, ally)
        } else {
            ally
        };
        h.cast_and_deliver(115546, Some(target));
        h.advance(2);
        assert_eq!(h.inbox.failed.len(), 1);
        assert!(h.inbox.go.is_empty());
        assert!(
            h.app
                .world()
                .get::<super::SpellHistory>(h.caster)
                .is_none_or(|history| !history.spell_ready_at.contains_key(&115546))
        );
    }
}
#[test]
fn dummy8_demonbolt_generates_two_whole_shards_once_on_landed_damage() {
    let mut h = harness(Vec::new(), &[264178, 460553, 280127], 9).unwrap();
    h.set_power(PowerType::SoulShards, 0.0);
    h.cast_and_deliver(264178, Some(h.npc));
    h.advance(100);
    landed(&h);
    assert_eq!(h.power(PowerType::SoulShards), 20.0);
    assert!(logged(&h, 264178, h.npc, CombatLogKind::Damage) > 0);
}
#[test]
fn dummy8_crash_lightning_uses_destination_cone_and_grants_authored_enhancement() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[187874, 187878, 195592], 7)
        .unwrap();
    let forward = at(&mut h, 5.0, 2.0);
    let side = at(&mut h, 0.0, 6.0);
    let back = at(&mut h, -4.0, 0.0);
    let far = at(&mut h, 9.0, 0.0);
    h.cast_and_deliver(187874, Some(h.npc));
    h.advance(4);
    landed(&h);
    for e in [h.npc, forward] {
        assert!(logged(&h, 187874, e, CombatLogKind::Damage) > 0);
    }
    for e in [side, back, far] {
        assert_eq!(logged(&h, 187874, e, CombatLogKind::Damage), 0);
    }
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 187878));
    h.advance(260);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 187878));
}
#[test]
fn dummy8_breath_of_fire_burns_only_owned_keg_smash_targets_in_front() {
    let mut h = harness(Vec::new(), &[115181, 123725, 121253], 10).unwrap();
    let marked = h.npc;
    let unmarked = at(&mut h, 4.0, 1.0);
    let behind = at(&mut h, -4.0, 0.0);
    let foreign = at(&mut h, 5.0, 1.0);
    let caster = h.caster;
    let other = spawn_player(&mut h.app, 10, 0.0);
    for (owner, target) in [(caster, marked), (caster, behind), (other, foreign)] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: owner,
                target,
                spell_id: 121253,
                effect_mask: 1 << 4,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    h.cast_and_deliver(115181, Some(marked));
    h.advance(4);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), marked, 123725));
    for e in [unmarked, behind, foreign] {
        assert!(!crate::auras::has_aura(h.app.world(), e, 123725));
    }
    assert_eq!(logged(&h, 115181, behind, CombatLogKind::Damage), 0);
    h.advance(45);
    assert!(logged(&h, 123725, marked, CombatLogKind::Damage) > 0);
}
#[test]
fn dummy8_fan_of_knives_awards_one_combo_point_not_one_per_enemy() {
    let mut h = harness(Vec::new(), &[51723], 4).unwrap();
    spawn_npc(&mut h.app, 4.0);
    spawn_npc(&mut h.app, 5.0);
    h.set_power(PowerType::ComboPoints, 0.0);
    h.cast_and_deliver(51723, None);
    h.advance(3);
    landed(&h);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
}
#[test]
fn dummy8_fan_of_knives_without_an_enemy_hit_awards_no_combo_point() {
    let mut h = harness(Vec::new(), &[51723], 4).unwrap();
    let npc = h.npc;
    h.app.world_mut().despawn(npc);
    h.set_power(PowerType::ComboPoints, 0.0);
    h.cast_and_deliver(51723, None);
    h.advance(3);
    landed(&h);
    assert_eq!(h.power(PowerType::ComboPoints), 0.0);
}
#[test]
fn dummy8_sanctify_caps_five_injured_allies_at_requested_location() {
    let mut h = harness(Vec::new(), &[34861], 5).unwrap();
    let healthy = spawn_player(&mut h.app, 5, 20.0);
    let units: Vec<_> = (0..6)
        .map(|i| {
            let e = spawn_player(&mut h.app, 5, 21.0 + i as f32);
            h.app.world_mut().get_mut::<Health>(e).unwrap().current = 100.0;
            e
        })
        .collect();
    let far = spawn_player(&mut h.app, 5, 40.0);
    h.app.world_mut().get_mut::<Health>(far).unwrap().current = 100.0;
    use lightyear::prelude::MessageSender;
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::SpellCastIntent {
            spell_id: Some(34861),
            spell: String::new(),
            target_entity: None,
            target_item_guid: None,
            destination: Some([20.0, 0.0, 0.0]),
            witness: None,
        });
    h.advance(7);
    landed(&h);
    for &e in &units[..5] {
        assert!(h.health(e) > 100.0);
    }
    assert_eq!(h.health(units[5]), 100.0);
    assert_eq!(h.health(far), 100.0);
    assert_eq!(logged(&h, 34861, healthy, CombatLogKind::Heal), 0);
}
#[test]
fn dummy8_incinerate_generates_authored_base_fragments_once_not_per_area_target() {
    let mut h = harness(Vec::new(), &[29722, 244670], 9).unwrap();
    spawn_npc(&mut h.app, 4.0);
    h.set_power(PowerType::SoulShards, 0.0);
    h.cast_and_deliver(29722, Some(h.npc));
    h.advance(100);
    landed(&h);
    assert_eq!(h.power(PowerType::SoulShards), 2.0);
}
#[test]
fn dummy8_unstable_affliction_applies_named_periodic_child_and_ticks() {
    let mut h = harness(Vec::new(), &[30108, 233490], 9).unwrap();
    h.set_power(PowerType::SoulShards, 50.0);
    let ally = spawn_player(&mut h.app, 9, 4.0);
    h.cast_and_deliver(30108, Some(h.npc));
    h.advance(65);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 233490));
    assert!(!crate::auras::has_aura(h.app.world(), ally, 233490));
    assert!(logged(&h, 233490, h.npc, CombatLogKind::Damage) > 0);
    h.advance(400);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 233490));
}
#[test]
fn dummy8_arcane_barrage_consumes_all_charges_without_inventing_damage_or_targets() {
    let mut h = harness(Vec::new(), &[44425, 36032], 8).unwrap();
    h.set_power(PowerType::ArcaneCharges, 3.0);
    h.cast_and_deliver(44425, Some(h.npc));
    h.advance(40);
    landed(&h);
    assert_eq!(h.power(PowerType::ArcaneCharges), 0.0);
    assert!(logged(&h, 44425, h.npc, CombatLogKind::Damage) > 0);
}

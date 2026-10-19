//! Native admitted casts are collected independently of the tank's selected victim.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use super::utility_tests::{prepare_enemy_spec, start_enemy_fireball};
use super::{Follower, ai, tank_planner};
use crate::maps::InMap;
use crate::networking::CombatTarget;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Position, ThreatTable};

fn caster_fixture() -> (Harness, Entity, Entity) {
    let (mut h, tank, _) = spawn_profile_fixture_with_plugins(209057, |_| {});
    start_enemy_fireball(&mut h, tank);
    let main = h.npc;
    h.app
        .world_mut()
        .entity_mut(main)
        .insert(CombatTarget(Some(tank)));
    let loose = spawn_npc(&mut h.app, 8.0);
    prepare_enemy_spec(&mut h, loose, 63);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(loose)
        .insert(CombatTarget(Some(owner)));
    crate::spell_cast::creature_cast(h.app.world_mut(), loose, 133, Some(owner), false).unwrap();
    assert!(h.app.world().get::<CastState>(loose).unwrap().interruptible);
    (h, tank, loose)
}

fn candidates(h: &mut Harness, tank: Entity) -> Vec<Entity> {
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let party = ai::party_units(h.app.world_mut(), follower.owner);
    tank_planner::read_engaged_caster_candidates(h.app.world_mut(), tank, &follower, &party)
}

#[test]
fn tank_planner_collects_loose_engaged_caster_without_forcing_target_or_moving() {
    let (mut h, tank, loose) = caster_fixture();
    let main = h.npc;
    let owner = h.caster;
    let before_target = h.app.world().get::<CombatTarget>(owner).unwrap().0;
    let before_position = *h.app.world().get::<Position>(tank).unwrap();
    // Reselection belongs to10482: threat top is deliberately NOT current victim.
    let mut threat = ThreatTable::default();
    threat.add_damage_threat(tank.to_bits(), 1000.0, 1.0);
    h.app.world_mut().entity_mut(loose).insert(threat);
    let result = candidates(&mut h, tank);
    assert_eq!(
        result,
        vec![loose, main],
        "collect loose current victim before tank-held caster"
    );
    assert_eq!(
        h.app.world().get::<CombatTarget>(loose).unwrap().0,
        Some(owner)
    );
    assert_eq!(
        h.app.world().get::<CombatTarget>(owner).unwrap().0,
        before_target
    );
    assert_eq!(
        *h.app.world().get::<Position>(tank).unwrap(),
        before_position
    );
    assert!(
        h.app.world().get::<CastState>(loose).is_some(),
        "planner never casts interrupts"
    );
}

#[test]
fn tank_planner_excludes_finished_uninterruptible_dead_and_other_copy_casters() {
    let (mut h, tank, loose) = caster_fixture();
    let main = h.npc;
    assert!(candidates(&mut h, tank).contains(&loose));
    h.app
        .world_mut()
        .get_mut::<CastState>(loose)
        .unwrap()
        .interruptible = false;
    assert_eq!(candidates(&mut h, tank), vec![main]);
    h.app
        .world_mut()
        .get_mut::<CastState>(loose)
        .unwrap()
        .interruptible = true;
    h.app.world_mut().entity_mut(loose).insert(InMap {
        map_id: 0,
        instance_id: 77,
    });
    assert_eq!(candidates(&mut h, tank), vec![main]);
    h.app
        .world_mut()
        .entity_mut(loose)
        .insert(InMap::continent(0));
    h.app.world_mut().get_mut::<Health>(loose).unwrap().current = 0.0;
    assert_eq!(candidates(&mut h, tank), vec![main]);
    h.app.world_mut().get_mut::<Health>(loose).unwrap().current = 5000.0;
    h.app.world_mut().entity_mut(loose).remove::<CastState>();
    assert_eq!(candidates(&mut h, tank), vec![main]);
}

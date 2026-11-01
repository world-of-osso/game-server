//! Real Ara-Kara ledge between Anub'zekt and Ki'katal: a tank taller than its human owner
//! regroups past idle drones that see it, but not the owner, on the owner's trail.
use super::ara_kara_ixin_tests::{ara_kara_copy, at};
use crate::networking::CombatTarget;
use bevy::prelude::*;
use shared::components::{CombatStatus, Health, Level};

/// Live 2026-11-01 22:07 UTC: source GUIDs alive within 70 yd of the held tank
/// (npc-info; the only dead one nearby was Blood Overseer 30004007).
const ALIVE_AT_LEDGE: &[u32] = &[
    30004014, 30004017, 30004021, 30004031, 30004036, 30004037, 30004038, 30004039, 30004040,
    30004041, 30004043, 30004044, 30004045, 30004046, 30004047, 30004048, 30004049, 30004052,
    30004112, 30004113, 30004114, 30004115, 30004116, 30004117, 30004118,
];
/// The owner's walked waypoints (candidate-anub-kikatal.csv 85-107) and where it stopped.
const OWNER_TRAIL: &[[f32; 3]] = &[
    [-313.66568, 134.12134, -947.7799],
    [-314.00153, 134.12134, -949.73596],
    [-320.6687, 128.39949, -967.07056],
    [-322.9657, 128.47968, -973.0427],
    [-325.46222, 128.68364, -979.53375],
    [-333.334, 122.29175, -1000.0003],
    [-340.8011, 117.5073, -1007.445],
];
/// Where the live Guardian tank held ("no safe owner trail route") for over five minutes.
const HELD_TANK_AT: [f32; 3] = [-329.9473, 126.09504, -991.193];
const REGROUPED_DISTANCE: f32 = 8.0;

#[test]
#[ignore = "requires actual Ara-Kara navigation, ground and line-of-sight assets"]
fn ara_kara_tank_regroups_past_ledge_drones_without_pulling() {
    let (mut h, map) = ara_kara_copy(|guid, _| ALIVE_AT_LEDGE.contains(&guid));
    let owner = h.caster;
    let world = h.app.world_mut();
    world
        .entity_mut(owner)
        .insert((map, Level(79), CombatStatus(false)));
    world.insert_resource(super::FollowerActionProvider {
        before_passives: |_, _, _| Ok(()),
        source_actions: |_, _, _| Ok(Vec::new()),
        equipped_actions: |_, _, _| Ok(Vec::new()),
    });
    // The owner walked this trail without pulling; record it as the live owner did.
    let trail: Vec<_> = OWNER_TRAIL
        .windows(2)
        .flat_map(|leg| {
            let (from, to) = (Vec3::from(leg[0]), Vec3::from(leg[1]));
            let steps = from.distance(to).ceil() as usize;
            (0..steps).map(move |step| from.lerp(to, step as f32 / steps as f32))
        })
        .chain([Vec3::from(*OWNER_TRAIL.last().unwrap())])
        .collect();
    for point in &trail {
        world.entity_mut(owner).insert(at(*point));
        super::owner_trail::record(world, owner);
    }
    let owner_at = *trail.last().unwrap();
    let tank = super::native_fixture::spawn_class(world, owner, 104).unwrap();
    world
        .entity_mut(tank)
        .insert((at(Vec3::from(HELD_TANK_AT)), map, Level(79)));
    let drones: Vec<_> = world
        .query::<(Entity, &shared::components::Npc, &crate::maps::InMap)>()
        .iter(world)
        .filter(|(_, _, copy)| **copy == map)
        .map(|(unit, _, _)| unit)
        .collect();
    h.advance(1);
    let tank_at = |h: &crate::spell_cast::test_harness::Harness| {
        crate::spell_cast::targets::position(h.app.world(), tank).unwrap()
    };
    let mut regrouped = false;
    for _ in 0..1200 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        if tank_at(&h).distance(owner_at) <= REGROUPED_DISTANCE {
            regrouped = true;
            break;
        }
    }
    assert!(
        regrouped,
        "the tank must regroup with its owner, not hold at {:?}",
        tank_at(&h)
    );
    let world = h.app.world();
    for unit in drones {
        let health = world.get::<Health>(unit).unwrap();
        assert!(
            health.current == health.max
                && world
                    .get::<CombatTarget>(unit)
                    .is_none_or(|victim| victim.0.is_none())
                && world
                    .get::<CombatStatus>(unit)
                    .is_none_or(|status| !status.0),
            "idle ledge pack member {unit:?} must stay unpulled"
        );
    }
}

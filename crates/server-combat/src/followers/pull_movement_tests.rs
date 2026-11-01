//! Actual kit AI → pursue → native fixed chase, with and without an injected plan.
use super::pull_route_tests::{route_fixture, route_fixture_with_plugins};
use super::tank_planner::AdmittedPosition;
use super::{Follower, catalog, kit};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;

#[test]
fn tank_pull_movement_regroup_cannot_replace_the_admitted_position() {
    let (mut h, tank) = route_fixture();
    let from = *h.app.world().get::<Position>(tank).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(128.5, 52.0, -33.86);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(Position { x, y, z });
    let admitted = AdmittedPosition(Vec3::new(from.x, from.y, from.z));
    h.app.world_mut().entity_mut(tank).insert(admitted);
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    for tick in 0..90 {
        h.advance(1);
        let at = h.app.world().get::<Position>(tank).unwrap();
        assert_eq!(
            Vec3::new(at.x, at.y, at.z),
            admitted.0,
            "regroup replaced the admitted point at tick{tick}"
        );
    }
}

pub(super) fn load_native_kit(h: &mut Harness, tank: Entity) -> BTreeSet<u32> {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = catalog::load_catalog(&db).unwrap();
    let attacks = catalog
        .kits
        .iter()
        .filter(|entry| {
            entry.spec_id == 66 && matches!(entry.slot, kit::Slot::Attack | kit::Slot::Aoe)
        })
        .map(|entry| entry.spell_id)
        .collect();
    h.app.insert_resource(catalog);
    let owner = h.caster;
    let profile = h.app.world().get::<Follower>(tank).unwrap().profile.clone();
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    attacks
}

pub(super) fn place_on_source_floor(h: &mut Harness, tank: Entity) -> Position {
    // Existing Riverpaw floor oracle: inside native Shield range, outside melee reach.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(128.5, 52.0, -33.86);
    let from = Position { x, y, z };
    h.app.world_mut().entity_mut(tank).insert(from);
    {
        let mut home = h
            .app
            .world_mut()
            .get_mut::<crate::creature_ai::SpawnPoint>(tank)
            .unwrap();
        home.x = from.x;
        home.y = from.y;
        home.z = from.z;
    }
    h.app.world_mut().entity_mut(h.caster).insert(from);
    from
}

fn kit_fixture() -> (Harness, Entity, Position, BTreeSet<u32>) {
    let (mut h, tank) = route_fixture();
    let attacks = load_native_kit(&mut h, tank);
    let from = place_on_source_floor(&mut h, tank);
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    let enemy = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == 20375682 && **copy == map)
        .unwrap()
        .0;
    h.attack(enemy);
    h.advance(3);
    (h, tank, from, attacks)
}

fn sample_kit_positions(h: &mut Harness, tank: Entity, attacks: &BTreeSet<u32>) -> Vec<Position> {
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    let positions = (0..90)
        .map(|_| {
            h.advance(1);
            *h.app.world().get::<Position>(tank).unwrap()
        })
        .collect();
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(tank.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id.is_some_and(|spell| attacks.contains(&spell))),
        "real sourced kit attacks must remain available from the safe point"
    );
    positions
}

fn assert_stationary(positions: &[Position], from: Position, context: &str) {
    for (tick, at) in positions.iter().enumerate() {
        assert_eq!(
            Vec3::new(at.x, at.y, at.z),
            Vec3::new(from.x, from.y, from.z),
            "{context} at tick{tick}"
        );
    }
}

#[test]
fn tank_pull_movement_kit_pursuit_cannot_replace_the_admitted_position() {
    let (mut h, tank, from, attacks) = kit_fixture();
    h.app
        .world_mut()
        .entity_mut(tank)
        .insert(AdmittedPosition(Vec3::new(from.x, from.y, from.z)));
    let positions = sample_kit_positions(&mut h, tank, &attacks);
    assert_stationary(&positions, from, "kit pursuit overwrote the admitted point");
}

#[test]
fn production_tank_pending_admission_holds_native_chase_without_losing_victim() {
    use bevy::ecs::system::RunSystemOnce;
    let (mut h, tank, from, _) = kit_fixture();
    super::pull_history::sample_all(h.app.world_mut());
    let target = h
        .app
        .world()
        .get::<crate::combat::AutoAttack>(h.caster)
        .unwrap()
        .victim;
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    assert_eq!(
        super::tank_planner::read_movement_admission(
            h.app.world_mut(),
            tank,
            target,
            &follower,
            None
        )
        .unwrap(),
        None,
        "first real run route request must await worker publication"
    );
    assert!(super::ai::pursue_at_reach(
        h.app.world_mut(),
        tank,
        target,
        super::ai::MELEE_REACH
    ));
    assert_eq!(
        h.app.world().get::<crate::creature_ai::CreatureState>(tank),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(tank)
            .unwrap()
            .0,
        Some(target)
    );
    h.app
        .world_mut()
        .run_system_once(crate::creature_ai::chase_system)
        .unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(tank).unwrap(),
        from,
        "unpublished admission must hold the actual native chase boundary"
    );
}

#[test]
fn tank_reaches_moving_owner_attacker_without_pulling_nearby_idle_pack() {
    use crate::creature_aggro::AggroDistance;
    use bevy::ecs::system::SystemState;
    use shared::components::CombatStatus;

    const TARGET_GUID: u32 = 20375682;
    const IDLE_GUID: u32 = 20375688;
    let (mut h, tank) = route_fixture_with_plugins(|app| {
        app.insert_resource(crate::test_data::world_data().clone());
        app.add_plugins(crate::creature_aggro::CreatureAggroPlugin);
    });
    load_native_kit(&mut h, tank);
    let from = place_on_source_floor(&mut h, tank);
    let owner = h.caster;
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    let sources: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, _, copy)| **copy == map)
        .map(|(entity, origin, _)| (entity, origin.guid))
        .collect();
    let target = sources
        .iter()
        .find(|(_, guid)| *guid == TARGET_GUID)
        .unwrap()
        .0;
    let idle = sources
        .iter()
        .find(|(_, guid)| *guid == IDLE_GUID)
        .unwrap()
        .0;
    for (entity, guid) in sources {
        if guid != TARGET_GUID && guid != IDLE_GUID {
            h.app.world_mut().despawn(entity);
        }
    }
    let mut data = h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .clone();
    data.spawns = data
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, TARGET_GUID | IDLE_GUID))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data);
    h.app
        .world_mut()
        .entity_mut(tank)
        .insert(super::ai::AplControlled);
    h.app
        .world_mut()
        .get_mut::<Follower>(tank)
        .unwrap()
        .profile
        .taunt_spell = 0;
    crate::creature_aggro::attack_start(h.app.world_mut(), target, owner);
    // Move the attacker by a deterministic 0.05 yd per fixed tick below. The
    // real navigation worker must publish each result after the target moves.
    h.app
        .world_mut()
        .get_mut::<crate::creature_ai::SpawnPoint>(target)
        .unwrap()
        .speed_run = 0.0;
    assert!(
        h.app
            .world()
            .get_resource::<crate::navigation::Navigation>()
            .is_some()
    );
    assert!(
        h.app
            .world()
            .get_resource::<crate::test_support::Space>()
            .is_none()
    );
    let initial_target = *h.app.world().get::<Position>(target).unwrap();
    let mut geometry = SystemState::<AggroDistance>::new(h.app.world_mut());
    let radius =
        super::pull_route::read_native_radius(h.app.world(), &mut geometry, tank, idle).unwrap();
    let point = |position: Position| Vec3::new(position.x, position.y, position.z);
    let idle_position = *h.app.world().get::<Position>(idle).unwrap();
    assert!(point(from).distance(point(idle_position)) > radius + 0.5);
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    let mut reached = false;
    let mut moved = false;
    let mut target_moved = false;
    let mut previous = point(from);
    let mut checkpoints = Vec::new();
    for tick in 0..160 {
        h.advance(1);
        let tank_position = *h.app.world().get::<Position>(tank).unwrap();
        let target_position = *h.app.world().get::<Position>(target).unwrap();
        let toward_owner = (point(from) - point(target_position)).normalize();
        let next = point(target_position) + toward_owner * 0.05;
        h.app.world_mut().entity_mut(target).insert(Position {
            x: next.x,
            y: next.y,
            z: next.z,
        });
        moved |= point(tank_position).distance(point(from)) > 1.0;
        target_moved |= point(target_position).distance(point(initial_target)) > 1.0;
        if tick % 20 == 0 {
            checkpoints.push((
                tick,
                tank_position,
                target_position,
                h.app
                    .world()
                    .get::<crate::creature_ai::CreatureState>(tank)
                    .cloned(),
                h.app
                    .world()
                    .resource::<crate::navigation::Navigation>()
                    .query_owner(tank),
                super::tank_pull_back::active(h.app.world(), tank),
            ));
        }
        let current = point(tank_position);
        assert!(
            super::pull_route::segment_distance_squared(point(idle_position), previous, current)
                >= (radius + 0.5).powi(2),
            "tank crossed the idle source's native aggro radius plus 0.5 yd"
        );
        previous = current;
        assert!(
            h.app
                .world()
                .get::<CombatStatus>(idle)
                .is_none_or(|status| !status.0),
            "the unrelated idle pack must not enter combat"
        );
        assert!(
            crate::combat_refs::active_opponents(h.app.world(), idle)
                .next()
                .is_none()
        );
        let melee_range = crate::scripted_ai::model_reach::melee_range(h.app.world(), tank, target)
            .expect("native tank and source combat reaches");
        if moved
            && target_moved
            && point(tank_position).distance(point(target_position)) <= melee_range
        {
            reached = true;
            break;
        }
    }
    assert!(
        target_moved,
        "the engaged source must actually move toward the owner"
    );
    assert!(
        moved,
        "the tank must move rather than wait for the source to arrive"
    );
    assert!(
        reached,
        "the tank must reach the moving source within 8 seconds: {checkpoints:?}"
    );
}

#[test]
fn production_tank_incomplete_run_admission_keeps_pursuit_held() {
    let (mut h, tank, from, _) = kit_fixture();
    let target = h
        .app
        .world()
        .get::<crate::combat::AutoAttack>(h.caster)
        .unwrap()
        .victim;
    // The admitted run record remains, but its copy no longer matches the owner.
    h.app
        .world_mut()
        .get_mut::<super::stockade_diagnostic::DiagnosticParty>(h.caster)
        .unwrap()
        .copy
        .instance_id += 1;
    assert!(super::ai::pursue_at_reach(
        h.app.world_mut(),
        tank,
        target,
        super::ai::MELEE_REACH
    ));
    assert_eq!(
        h.app.world().get::<crate::creature_ai::CreatureState>(tank),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(tank)
            .unwrap()
            .0,
        Some(target),
        "unavailable run admission must hold movement without losing the living victim"
    );
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    h.advance(1);
    assert_eq!(
        *h.app.world().get::<Position>(tank).unwrap(),
        from,
        "incomplete run admission must not permit native kit/chase movement"
    );
}

#[test]
fn production_tank_goal_allows_existing_fight_defense_without_a_healer() {
    let (mut h, tank, from, attacks) = kit_fixture();
    let target = h
        .app
        .world()
        .get::<crate::combat::AutoAttack>(h.caster)
        .unwrap()
        .victim;
    let guid = h
        .app
        .world()
        .get::<crate::zones::CreatureOrigin>(target)
        .unwrap()
        .guid;
    let route = super::pull_route::inspect_route(
        h.app.world_mut(),
        &super::pull_route::Request {
            tank,
            target_guid: guid,
            difficulty: 1,
            cleared: &BTreeSet::new(),
        },
    )
    .unwrap();
    let actors: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, origin, map)| {
            route.spawns.contains(&origin.guid) && Some(*map) == h.app.world().get::<InMap>(tank)
        })
        .map(|(actor, _, _)| actor)
        .collect();
    assert_eq!(actors.len(), route.spawns.len());
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    for actor in actors {
        let context = crate::spell_cast::effects::EffectCtx {
            caster: h.caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        // Native damage/threat starts these actual source engagements, not injected victims.
        crate::spell_cast::effects::deal_damage(
            h.app.world_mut(),
            &context,
            actor,
            1.0,
            false,
            false,
        );
    }
    let positions = sample_kit_positions(&mut h, tank, &attacks);
    assert!(
        positions.iter().any(|at| *at != from),
        "existing-fight defense must still move without new-pull healer readiness"
    );
}

fn route_requires_a_new_unhealed_pull(h: &mut Harness, tank: Entity, guid: u32) -> bool {
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let history = super::pull_history::snapshot(h.app.world(), follower.owner).unwrap();
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let route = super::pull_route::inspect_route(
        h.app.world_mut(),
        &super::pull_route::Request {
            tank,
            target_guid: guid,
            difficulty: 1,
            cleared: &history.cleared_spawns,
        },
    )
    .expect("unsafe-pursuit fixture requires a complete native route");
    let sources = super::pull_sources::read_present_sources(
        h.app.world_mut(),
        map,
        1,
        &history.cleared_spawns,
    )
    .unwrap();
    let party = super::ai::party_units(h.app.world_mut(), follower.owner);
    let packs: BTreeSet<_> = super::ai::engaged_enemies(h.app.world_mut(), &follower, tank, &party)
        .iter()
        .map(|(enemy, _)| {
            let guid = h
                .app
                .world()
                .get::<crate::zones::CreatureOrigin>(*enemy)
                .unwrap()
                .guid;
            sources[&guid].pack_guid
        })
        .collect();
    let extra = route
        .spawns
        .iter()
        .any(|guid| !packs.contains(&sources[guid].pack_guid));
    if extra {
        assert!(matches!(
            super::pull_admission::read_pull_budget(h.app.world_mut(), tank, &route.spawns, 1)
                .unwrap(),
            super::pull_budget::Decision::Wait(super::pull_budget::Reason::Healer)
        ));
    }
    extra
}

#[test]
fn production_tank_goal_refuses_unbudgeted_pursuit_without_injected_position() {
    let (mut h, tank, _, _) = kit_fixture();
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let target = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == 20375669 && **copy == map)
        .unwrap()
        .0;
    h.select(Some(target));
    h.attack(target);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(h.caster)
            .unwrap()
            .victim,
        target
    );
    // Native initialization/self-casts may precede the first actual kit pursuit.
    super::update_ai(h.app.world_mut());
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    let tick = h.app.world().resource::<Time<Fixed>>().timestep();
    assert_eq!(
        tick,
        std::time::Duration::from_millis(50),
        "use production20Hz native ordering"
    );
    for _ in 0..90 {
        let before = *h.app.world().get::<Position>(tank).unwrap();
        let at = h.app.world().resource::<Time<Fixed>>().elapsed();
        let extra_before = route_requires_a_new_unhealed_pull(&mut h, tank, 20375669);
        h.advance(1);
        assert_eq!(
            h.app.world().resource::<Time<Fixed>>().elapsed() - at,
            tick,
            "each observation must cover exactly one actual kit/chase tick"
        );
        let selected = h
            .app
            .world()
            .get::<crate::networking::CombatTarget>(tank)
            .is_some_and(|current| current.0 == Some(target));
        if selected && extra_before && route_requires_a_new_unhealed_pull(&mut h, tank, 20375669) {
            assert_stationary(
                &[*h.app.world().get::<Position>(tank).unwrap()],
                before,
                "production planner started an actually unbudgeted route at its movement boundary",
            );
            return;
        }
    }
    panic!(
        "real kit never reached the unsafe pursuit boundary: current={:?}, cast={:?}",
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(tank)
            .and_then(|current| current.0),
        h.app
            .world()
            .get::<shared::casting::CastState>(tank)
            .map(|cast| cast.spell_id)
    );
}

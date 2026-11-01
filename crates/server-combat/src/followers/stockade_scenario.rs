//! Test-support Stockade pull scenario: a production class tank in an admitted
//! Normal Stockade copy with its full population and baked routes. Integration
//! tests supply the follower APL step, run between the follower brain and chase.
use super::tank_planner::AdmittedPosition;
use super::{Follower, ai, pull_admission, pull_budget, pull_history, pull_route, pull_sources};
use crate::maps::InMap;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins, world_db};
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{Level, Position, UnitLevel};
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;
use std::path::PathBuf;

const PROTECTION_PALADIN: u32 = 66;
const OWNER_LEVEL: u8 = 32;
/// Riverpaw source the tank and owner engage from its floor.
const RIVERPAW_GUID: u32 = 20375682;
/// Source whose route crosses packs a healerless party has not engaged.
const UNBUDGETED_GUID: u32 = 20375669;

pub struct Scenario {
    pub h: Harness,
    pub tank: Entity,
    apl: fn(&mut World),
}

/// Spawn the full Normal Stockade population into `map` with its ground and navmesh.
pub(super) fn install_population(h: &mut Harness, map: InMap) {
    let db = open_world_db();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(34, 791060)]),
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let count = crate::zones::spawn_map_creatures(
        &mut commands,
        crate::test_data::world_data(),
        crate::zones::MapContent {
            map,
            difficulty_id: 1,
            dead_bosses: vec![],
        },
        false,
        Some(&mut ground),
    );
    assert!(count > 2, "actual full Normal Stockade population");
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    h.app.insert_resource(crate::navigation::Navigation::new(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh"),
    ));
}

fn open_world_db() -> Connection {
    Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
}

/// The owner's admitted Normal Stockade copy, as the diagnostic admits it.
fn admit_normal_copy(h: &mut Harness, db: &Connection) -> InMap {
    let store = crate::maps::store::load_map_store(db).unwrap();
    let mut instances = crate::maps::InstanceManager::default();
    let entrant = crate::maps::instances::InstanceEntrant {
        character_id: 1,
        leader: 1,
        difficulty_id: 1,
        ..default()
    };
    let now = crate::maps::instances::Now {
        elapsed: 0.0,
        unix: 1_790_510_400,
    };
    let copy = instances.create_map(&store, 34, entrant, now).unwrap().map;
    h.app.insert_resource(store).insert_resource(instances);
    copy
}

fn move_unit(h: &mut Harness, unit: Entity, at: Position) {
    h.app.world_mut().entity_mut(unit).insert(at);
    if let Some(mut home) = h
        .app
        .world_mut()
        .get_mut::<crate::creature_ai::SpawnPoint>(unit)
    {
        home.x = at.x;
        home.y = at.y;
        home.z = at.z;
    }
}

/// A production spec-66 class tank at the route origin of a full Normal copy.
/// `configure` installs the follower action provider and the spell scripts.
pub fn route(configure: impl FnOnce(&mut App), apl: fn(&mut World)) -> Scenario {
    let mut h =
        harness_with_plugins(vec![], &[], 8, false, configure).expect("native world database");
    let data = world_db().unwrap();
    h.app.insert_resource(data.store.clone());
    let source = crate::test_data::world_data();
    h.app.insert_resource(source.clone());
    h.app.insert_resource(source.armor_constants.clone());
    super::native_fixture::install_class_sources(h.app.world_mut());
    let db = open_world_db();
    h.app
        .insert_resource(crate::item_proc_data::load(&db).unwrap())
        .insert_resource(super::catalog::load_catalog(&db).unwrap())
        .insert_resource(
            crate::scripted_ai::smart_ai::load_smart_scripts(&crate::test_data::world_db_path())
                .unwrap(),
        );
    let map = admit_normal_copy(&mut h, &db);
    let (x, y, z) = crate::terrain_height::wow_to_bevy(103.0, 76.0, -34.85623);
    let origin = Position { x, y, z };
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert((
        map,
        origin,
        Level(OWNER_LEVEL),
        UnitLevel(OWNER_LEVEL),
    ));
    let profile = super::class_profiles::read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == PROTECTION_PALADIN)
        .expect("roster Protection Paladin");
    let tank = super::class_spawn::spawn_class(
        h.app.world_mut(),
        owner,
        &profile,
        0,
        0,
        super::class_spawn::GearSource::Journal,
    )
    .unwrap();
    move_unit(&mut h, tank, origin);
    install_population(&mut h, map);
    Scenario { h, tank, apl }
}

/// `route`, with the tank and owner on the Riverpaw floor (inside native
/// Avenger's Shield range, outside melee reach) and the owner attacking it.
pub fn engaged(configure: impl FnOnce(&mut App), apl: fn(&mut World)) -> (Scenario, Position) {
    let mut s = route(configure, apl);
    let from = place_on_riverpaw_floor(&mut s);
    let enemy = s.source(RIVERPAW_GUID);
    s.h.attack(enemy);
    s.h.advance(3);
    (s, from)
}

/// Tank and owner on the Riverpaw floor, with the owner's diagnostic run record.
fn place_on_riverpaw_floor(s: &mut Scenario) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(128.5, 52.0, -33.86);
    let from = Position { x, y, z };
    let (tank, owner) = (s.tank, s.h.caster);
    move_unit(&mut s.h, tank, from);
    s.h.app.world_mut().entity_mut(owner).insert(from);
    let map = *s.h.app.world().get::<InMap>(tank).unwrap();
    s.h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    from
}

impl Scenario {
    fn source(&mut self, guid: u32) -> Entity {
        let map = *self.h.app.world().get::<InMap>(self.tank).unwrap();
        self.h
            .app
            .world_mut()
            .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
            .iter(self.h.app.world())
            .find(|(_, origin, copy)| origin.guid == guid && **copy == map)
            .unwrap()
            .0
    }

    /// Follower brain, the caller's APL, native chase and follow at production 20Hz.
    fn install_follow(&mut self) {
        crate::creature_ai::install_follower_follow_with(&mut self.h.app, self.apl);
    }

    fn owner_victim(&self) -> Entity {
        self.h
            .app
            .world()
            .get::<crate::combat::AutoAttack>(self.h.caster)
            .unwrap()
            .victim
    }

    fn position(&self) -> Position {
        *self.h.app.world().get::<Position>(self.tank).unwrap()
    }

    /// Hostile class actions the tank can actually cast, overrides resolved.
    fn hostile_actions(&self) -> BTreeSet<u32> {
        let world = self.h.app.world();
        let store = world.resource::<crate::spell_info::SpellStore>();
        super::action_spells::available(world, self.tank)
            .unwrap()
            .into_iter()
            .filter(|&id| {
                store.get(id).is_some_and(|spell| {
                    crate::spell_info::positivity::negative_effects(store, spell) != 0
                })
            })
            .collect()
    }

    fn hostile_units(&mut self) -> BTreeSet<u64> {
        let map = *self.h.app.world().get::<InMap>(self.tank).unwrap();
        let units: Vec<_> = self
            .h
            .app
            .world_mut()
            .query::<(Entity, &InMap)>()
            .iter(self.h.app.world())
            .filter(|(_, copy)| **copy == map)
            .map(|(unit, _)| unit)
            .collect();
        let world = self.h.app.world();
        units
            .into_iter()
            .filter(|&unit| crate::spell_cast::targets::is_hostile(world, self.tank, unit))
            .map(Entity::to_bits)
            .collect()
    }

    /// Sample 90 ticks; the tank's real APL must land a hostile class action
    /// on an enemy, so attacks remain available from wherever it stands.
    fn sample_attacking_positions(&mut self) -> Vec<Position> {
        let attacks = self.hostile_actions();
        assert!(
            !attacks.is_empty(),
            "class tank requires hostile APL actions"
        );
        let start = self.h.inbox.log.len();
        self.install_follow();
        let positions = (0..90)
            .map(|_| {
                self.h.advance(1);
                self.position()
            })
            .collect();
        let enemies = self.hostile_units();
        let tank = self.tank.to_bits();
        let casts: Vec<_> = self.h.inbox.log[start..]
            .iter()
            .filter(|event| event.source == Some(tank) && event.kind == CombatLogKind::CastSuccess)
            .map(|event| (event.spell_id, event.target))
            .collect();
        assert!(
            casts.iter().any(|(spell, target)| {
                spell.is_some_and(|spell| attacks.contains(&spell))
                    && target.is_some_and(|target| enemies.contains(&target))
            }),
            "real APL attacks must remain available from the safe point: hostile actions {attacks:?}, casts {casts:?}"
        );
        positions
    }
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

/// Regroup toward a displaced owner must not move the tank off its admitted point.
pub fn assert_regroup_cannot_replace_the_admitted_position(mut s: Scenario) {
    let from = s.position();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(128.5, 52.0, -33.86);
    let owner = s.h.caster;
    s.h.app
        .world_mut()
        .entity_mut(owner)
        .insert(Position { x, y, z });
    let admitted = AdmittedPosition(Vec3::new(from.x, from.y, from.z));
    s.h.app.world_mut().entity_mut(s.tank).insert(admitted);
    s.install_follow();
    for tick in 0..90 {
        s.h.advance(1);
        let at = s.position();
        assert_eq!(
            Vec3::new(at.x, at.y, at.z),
            admitted.0,
            "regroup replaced the admitted point at tick{tick}"
        );
    }
}

/// APL pursuit keeps attacking from the admitted point instead of leaving it.
pub fn assert_pursuit_cannot_replace_the_admitted_position(engaged: (Scenario, Position)) {
    let (mut s, from) = engaged;
    s.h.app
        .world_mut()
        .entity_mut(s.tank)
        .insert(AdmittedPosition(Vec3::new(from.x, from.y, from.z)));
    let positions = s.sample_attacking_positions();
    assert_stationary(&positions, from, "APL pursuit overwrote the admitted point");
}

/// An unpublished route admission holds native chase without losing the victim.
pub fn assert_pending_admission_holds_native_chase(engaged: (Scenario, Position)) {
    use bevy::ecs::system::RunSystemOnce;
    let (mut s, from) = engaged;
    pull_history::sample_all(s.h.app.world_mut());
    let target = s.owner_victim();
    let follower = s.h.app.world().get::<Follower>(s.tank).unwrap().clone();
    assert_eq!(
        super::tank_planner::read_movement_admission(
            s.h.app.world_mut(),
            s.tank,
            target,
            &follower,
            None
        )
        .unwrap(),
        None,
        "first real run route request must await worker publication"
    );
    assert!(ai::pursue_at_reach(
        s.h.app.world_mut(),
        s.tank,
        target,
        ai::MELEE_REACH
    ));
    assert_eq!(
        s.h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(s.tank),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    assert_eq!(
        s.h.app
            .world()
            .get::<crate::networking::CombatTarget>(s.tank)
            .unwrap()
            .0,
        Some(target)
    );
    s.h.app
        .world_mut()
        .run_system_once(crate::creature_ai::chase_system)
        .unwrap();
    assert_eq!(
        s.position(),
        from,
        "unpublished admission must hold the actual native chase boundary"
    );
}

/// A run record whose copy no longer matches the owner keeps pursuit held.
pub fn assert_incomplete_run_admission_keeps_pursuit_held(engaged: (Scenario, Position)) {
    let (mut s, from) = engaged;
    let target = s.owner_victim();
    let owner = s.h.caster;
    s.h.app
        .world_mut()
        .get_mut::<super::stockade_diagnostic::DiagnosticParty>(owner)
        .unwrap()
        .copy
        .instance_id += 1;
    assert!(ai::pursue_at_reach(
        s.h.app.world_mut(),
        s.tank,
        target,
        ai::MELEE_REACH
    ));
    assert_eq!(
        s.h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(s.tank),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    assert_eq!(
        s.h.app
            .world()
            .get::<crate::networking::CombatTarget>(s.tank)
            .unwrap()
            .0,
        Some(target),
        "unavailable run admission must hold movement without losing the living victim"
    );
    s.install_follow();
    s.h.advance(1);
    assert_eq!(
        s.position(),
        from,
        "incomplete run admission must not permit APL/chase movement"
    );
}

/// Defending an existing fight needs no new-pull healer readiness: it moves and attacks.
pub fn assert_existing_fight_defense_without_a_healer(engaged: (Scenario, Position)) {
    let (mut s, from) = engaged;
    let target = s.owner_victim();
    let guid =
        s.h.app
            .world()
            .get::<crate::zones::CreatureOrigin>(target)
            .unwrap()
            .guid;
    let route = pull_route::inspect_route(
        s.h.app.world_mut(),
        &pull_route::Request {
            tank: s.tank,
            target_guid: guid,
            difficulty: 1,
            cleared: &BTreeSet::new(),
        },
    )
    .unwrap();
    let map = *s.h.app.world().get::<InMap>(s.tank).unwrap();
    let actors: Vec<_> =
        s.h.app
            .world_mut()
            .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
            .iter(s.h.app.world())
            .filter(|(_, origin, copy)| route.spawns.contains(&origin.guid) && **copy == map)
            .map(|(actor, _, _)| actor)
            .collect();
    assert_eq!(actors.len(), route.spawns.len());
    let spell =
        s.h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(133)
            .unwrap()
            .clone();
    for actor in actors {
        let context = crate::spell_cast::effects::EffectCtx {
            caster: s.h.caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        // Native damage/threat starts these actual source engagements, not injected victims.
        crate::spell_cast::effects::deal_damage(
            s.h.app.world_mut(),
            &context,
            actor,
            1.0,
            false,
            false,
        );
    }
    let positions = s.sample_attacking_positions();
    if positions.iter().all(|at| *at == from) {
        let world = s.h.app.world();
        let combat_target = world
            .get::<crate::networking::CombatTarget>(s.tank)
            .and_then(|current| current.0);
        let state = world
            .get::<crate::creature_ai::CreatureState>(s.tank)
            .copied();
        let cast = world
            .get::<shared::casting::CastState>(s.tank)
            .map(|cast| cast.spell_id);
        let follower = world.get::<Follower>(s.tank).unwrap().clone();
        let admission = combat_target.map(|victim| {
            super::tank_planner::read_movement_admission(
                s.h.app.world_mut(),
                s.tank,
                victim,
                &follower,
                None,
            )
        });
        panic!(
            "existing-fight defense must still move without new-pull healer readiness: \
             target={combat_target:?} owner_victim={target:?} state={state:?} cast={cast:?} \
             admission={admission:?}"
        );
    }
}

fn route_requires_a_new_unhealed_pull(h: &mut Harness, tank: Entity, guid: u32) -> bool {
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let history = pull_history::snapshot(h.app.world(), follower.owner).unwrap();
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let route = pull_route::inspect_route(
        h.app.world_mut(),
        &pull_route::Request {
            tank,
            target_guid: guid,
            difficulty: 1,
            cleared: &history.cleared_spawns,
        },
    )
    .expect("unsafe-pursuit fixture requires a complete native route");
    let sources =
        pull_sources::read_present_sources(h.app.world_mut(), map, 1, &history.cleared_spawns)
            .unwrap();
    let party = ai::party_units(h.app.world_mut(), follower.owner);
    let packs: BTreeSet<_> = ai::engaged_enemies(h.app.world_mut(), &follower, tank, &party)
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
            pull_admission::read_pull_budget(h.app.world_mut(), tank, &route.spawns, 1).unwrap(),
            pull_budget::Decision::Wait(pull_budget::Reason::Healer)
        ));
    }
    extra
}

/// The production planner holds an unbudgeted route at its first movement boundary.
pub fn assert_unbudgeted_pursuit_refused_without_injected_position(engaged: (Scenario, Position)) {
    let (mut s, _) = engaged;
    let target = s.source(UNBUDGETED_GUID);
    s.h.select(Some(target));
    s.h.attack(target);
    s.h.advance(3);
    assert_eq!(s.owner_victim(), target);
    // Native initialization/self-casts may precede the first actual pursuit.
    super::update_ai(s.h.app.world_mut());
    (s.apl)(s.h.app.world_mut());
    s.install_follow();
    let tick = s.h.app.world().resource::<Time<Fixed>>().timestep();
    assert_eq!(
        tick,
        std::time::Duration::from_millis(50),
        "use production20Hz native ordering"
    );
    for _ in 0..90 {
        let before = s.position();
        let at = s.h.app.world().resource::<Time<Fixed>>().elapsed();
        let extra_before = route_requires_a_new_unhealed_pull(&mut s.h, s.tank, UNBUDGETED_GUID);
        s.h.advance(1);
        assert_eq!(
            s.h.app.world().resource::<Time<Fixed>>().elapsed() - at,
            tick,
            "each observation must cover exactly one actual APL/chase tick"
        );
        let selected =
            s.h.app
                .world()
                .get::<crate::networking::CombatTarget>(s.tank)
                .is_some_and(|current| current.0 == Some(target));
        if selected
            && extra_before
            && route_requires_a_new_unhealed_pull(&mut s.h, s.tank, UNBUDGETED_GUID)
        {
            assert_stationary(
                &[s.position()],
                before,
                "production planner started an actually unbudgeted route at its movement boundary",
            );
            return;
        }
    }
    panic!(
        "real class tank never reached the unsafe pursuit boundary: current={:?}, cast={:?}",
        s.h.app
            .world()
            .get::<crate::networking::CombatTarget>(s.tank)
            .and_then(|current| current.0),
        s.h.app
            .world()
            .get::<shared::casting::CastState>(s.tank)
            .map(|cast| cast.spell_id)
    );
}

/// Brain pursuit alone: the class tank runs no APL step, as `AplControlled` without a driver.
fn no_casts(_: &mut World) {}

/// An engaged source moving toward the owner is reached without pulling the
/// neighbouring idle pack: the tank never crosses its native aggro radius + 0.5 yd.
pub fn assert_tank_reaches_moving_owner_attacker_without_pulling_nearby_idle_pack(
    configure: impl FnOnce(&mut App),
) {
    use crate::creature_aggro::AggroDistance;
    use bevy::ecs::system::SystemState;
    use shared::components::CombatStatus;

    const IDLE_GUID: u32 = 20375688;
    let mut s = route(
        |app| {
            configure(app);
            app.add_plugins(crate::creature_aggro::CreatureAggroPlugin);
        },
        no_casts,
    );
    let from = place_on_riverpaw_floor(&mut s);
    let (tank, owner) = (s.tank, s.h.caster);
    let map = *s.h.app.world().get::<InMap>(tank).unwrap();
    let sources: Vec<_> =
        s.h.app
            .world_mut()
            .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
            .iter(s.h.app.world())
            .filter(|(_, _, copy)| **copy == map)
            .map(|(entity, origin, _)| (entity, origin.guid))
            .collect();
    let target = s.source(RIVERPAW_GUID);
    let idle = s.source(IDLE_GUID);
    for (entity, guid) in sources {
        if guid != RIVERPAW_GUID && guid != IDLE_GUID {
            s.h.app.world_mut().despawn(entity);
        }
    }
    let mut data =
        s.h.app
            .world()
            .resource::<crate::world_data::WorldData>()
            .clone();
    data.spawns = data
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, RIVERPAW_GUID | IDLE_GUID))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    s.h.app.insert_resource(data);
    crate::creature_aggro::attack_start(s.h.app.world_mut(), target, owner);
    // Move the attacker by a deterministic 0.05 yd per fixed tick below. The
    // real navigation worker must publish each result after the target moves.
    s.h.app
        .world_mut()
        .get_mut::<crate::creature_ai::SpawnPoint>(target)
        .unwrap()
        .speed_run = 0.0;
    assert!(
        s.h.app
            .world()
            .get_resource::<crate::navigation::Navigation>()
            .is_some()
    );
    assert!(
        s.h.app
            .world()
            .get_resource::<crate::test_support::Space>()
            .is_none()
    );
    let initial_target = *s.h.app.world().get::<Position>(target).unwrap();
    let mut geometry = SystemState::<AggroDistance>::new(s.h.app.world_mut());
    let radius =
        pull_route::read_native_radius(s.h.app.world(), &mut geometry, tank, idle).unwrap();
    let point = |position: Position| Vec3::new(position.x, position.y, position.z);
    let idle_position = *s.h.app.world().get::<Position>(idle).unwrap();
    assert!(point(from).distance(point(idle_position)) > radius + 0.5);
    s.install_follow();
    let mut reached = false;
    let mut moved = false;
    let mut target_moved = false;
    let mut previous = point(from);
    let mut checkpoints = Vec::new();
    for tick in 0..160 {
        s.h.advance(1);
        let tank_position = s.position();
        let target_position = *s.h.app.world().get::<Position>(target).unwrap();
        let toward_owner = (point(from) - point(target_position)).normalize();
        let next = point(target_position) + toward_owner * 0.05;
        s.h.app.world_mut().entity_mut(target).insert(Position {
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
                s.h.app
                    .world()
                    .get::<crate::creature_ai::CreatureState>(tank)
                    .cloned(),
                s.h.app
                    .world()
                    .resource::<crate::navigation::Navigation>()
                    .query_owner(tank),
                super::tank_pull_back::active(s.h.app.world(), tank),
            ));
        }
        let current = point(tank_position);
        assert!(
            pull_route::segment_distance_squared(point(idle_position), previous, current)
                >= (radius + 0.5).powi(2),
            "tank crossed the idle source's native aggro radius plus 0.5 yd"
        );
        previous = current;
        assert!(
            s.h.app
                .world()
                .get::<CombatStatus>(idle)
                .is_none_or(|status| !status.0),
            "the unrelated idle pack must not enter combat"
        );
        assert!(
            crate::combat_refs::active_opponents(s.h.app.world(), idle)
                .next()
                .is_none()
        );
        let melee_range =
            crate::scripted_ai::model_reach::melee_range(s.h.app.world(), tank, target)
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

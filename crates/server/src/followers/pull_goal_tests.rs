//! User pacing policy at real kit/AI/chase boundaries, not injected admitted points.
use super::pull_budget::{Decision, Reason};
use super::pull_movement_tests::load_native_kit;
use super::pull_route_tests::route_fixture_with_plugins;
use super::{pull_admission, pull_history, pull_route};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

struct Fixture {
    h: Harness,
    tank: Entity,
    candidate: Entity,
    engaged: Entity,
}

fn source(h: &mut Harness, tank: Entity, guid: u32) -> Entity {
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    h.app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == guid && **copy == map)
        .unwrap()
        .0
}

pub(super) fn place_at_authored_entrance(h: &mut Harness, tank: Entity) -> Position {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    // Native WorldSafeLoc3599, area-trigger101's Stockade entrance destination.
    let (wx, wy, wz): (f32, f32, f32) = db
        .query_row(
            "SELECT LocX, LocY, LocZ FROM tdb_world_safe_locs WHERE ID=3599 AND MapID=34",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wx, wy, wz);
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let crate::ground::GroundProbe::Supported(y) = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(x, y, z))
    else {
        panic!("authored entrance must have actual native floor support")
    };
    let from = Position { x, y, z };
    let point = Vec3::new(x, y, z);
    assert!(
        !h.app
            .world()
            .resource::<crate::navigation::Navigation>()
            .query_path(map.map_id, point, point)
            .unwrap()
            .unwrap()
            .is_empty()
    );
    h.app.world_mut().entity_mut(tank).insert(from);
    h.app.world_mut().entity_mut(h.caster).insert(from);
    let mut home = h
        .app
        .world_mut()
        .get_mut::<crate::creature_ai::SpawnPoint>(tank)
        .unwrap();
    home.x = x;
    home.y = y;
    home.z = z;
    from
}

fn fixture() -> Fixture {
    let (mut h, tank) = route_fixture_with_plugins(|app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    load_native_kit(&mut h, tank);
    let from = place_at_authored_entrance(&mut h, tank);
    let healer = super::pull_forecast_tests::add_native_healer(&mut h);
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let mut members = vec![tank, healer];
    members.extend(super::pull_admission_tests::add_native_dps(&mut h, map));
    for &member in &members {
        h.app.world_mut().entity_mut(member).insert(from);
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members,
            player_role: shared::group::GroupRole::Dps,
        });
    let candidate = source(&mut h, tank, 20375682);
    let engaged = source(&mut h, tank, 20375688);
    h.advance(3);
    assert_eq!(
        h.app.world().get::<Health>(tank).unwrap().max,
        super::pull_admission_tests::NATIVE_GARRICK_MAX_HEALTH
    );
    Fixture {
        h,
        tank,
        candidate,
        engaged,
    }
}

fn damage(h: &mut Harness, target: Entity, amount: f32) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let context = crate::spell_cast::effects::EffectCtx {
        caster: h.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::deal_damage(
        h.app.world_mut(),
        &context,
        target,
        amount,
        false,
        false,
    );
}

fn sample(f: &mut Fixture, frames: usize) {
    for _ in 0..frames {
        f.h.advance(1);
        pull_history::sample_all(f.h.app.world_mut());
    }
}

fn measured_fixture() -> Fixture {
    let mut f = fixture();
    sample(&mut f, 100);
    let map = *f.h.app.world().get::<InMap>(f.tank).unwrap();
    let actors: Vec<_> =
        f.h.app
            .world_mut()
            .query::<(Entity, &InMap)>()
            .iter(f.h.app.world())
            .filter(|(actor, copy)| **copy == map && *actor != f.candidate && *actor != f.engaged)
            .map(|(actor, _)| actor)
            .collect();
    let calibration = actors
        .into_iter()
        .filter(|actor| crate::spell_cast::targets::is_hostile(f.h.app.world(), f.tank, *actor))
        .filter_map(|actor| {
            crate::combat::forecast::read_generic_creature(f.h.app.world(), actor, f.tank)
                .ok()
                .map(|estimate| (actor, estimate.remaining_health))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap();
    damage(&mut f.h, calibration.0, calibration.1 as f32 * 2.0);
    assert_eq!(f.h.health(calibration.0), 0.0);
    sample(&mut f, 120);
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    assert_eq!(history.completed_fights, 1);
    assert!(history.party_stable);
    assert!(history.group_dps.is_some_and(|damage| damage > 0.0));
    f
}

fn candidate_budget(f: &mut Fixture) -> Decision {
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    let route = pull_route::read_route(
        f.h.app.world_mut(),
        &pull_route::Request {
            tank: f.tank,
            target_guid: 20375682,
            difficulty: 1,
            cleared: &history.cleared_spawns,
        },
    )
    .unwrap();
    assert!(route.spawns.contains(&20375682));
    pull_admission::read_pull_budget(f.h.app.world_mut(), f.tank, &route.spawns, 1).unwrap()
}

fn tank_acquired(f: &Fixture) -> bool {
    let bits = f.candidate.to_bits();
    f.h.app
        .world()
        .get::<crate::networking::CombatTarget>(f.tank)
        .is_some_and(|target| target.0 == Some(f.candidate))
        || f.h.inbox.log.iter().any(|event| {
            event.source == Some(f.tank.to_bits())
                && event.target == Some(bits)
                && event.kind == CombatLogKind::CastSuccess
        })
}

fn engaged_sources(f: &mut Fixture) -> Vec<Entity> {
    let follower =
        f.h.app
            .world()
            .get::<super::Follower>(f.tank)
            .unwrap()
            .clone();
    let party = super::ai::party_units(f.h.app.world_mut(), follower.owner);
    super::ai::engaged_enemies(f.h.app.world_mut(), &follower, f.tank, &party)
        .into_iter()
        .map(|(enemy, _)| enemy)
        .collect()
}

#[test]
fn tank_owner_pacing_idle_selection_never_acquires_or_pulls() {
    let mut f = measured_fixture();
    assert!(
        engaged_sources(&mut f).is_empty(),
        "idle fixture must have no residual native engagement"
    );
    assert!(matches!(
        candidate_budget(&mut f),
        Decision::Allow {
            bootstrap: false,
            ..
        }
    ));
    f.h.select(Some(f.candidate));
    f.h.advance(3);
    let from = *f.h.app.world().get::<Position>(f.tank).unwrap();
    crate::creature_ai::install_follower_follow_fixture(&mut f.h.app);
    f.h.advance(30);
    assert!(
        !tank_acquired(&f),
        "idle owner selection must not start a pull"
    );
    assert_eq!(*f.h.app.world().get::<Position>(f.tank).unwrap(), from);
}

#[test]
fn tank_owner_pacing_active_chain_requires_a_passing_budget() {
    let mut f = measured_fixture();
    damage(&mut f.h, f.engaged, 1.0);
    pull_history::sample_all(f.h.app.world_mut());
    let engaged = engaged_sources(&mut f);
    assert!(engaged.contains(&f.engaged));
    assert!(
        !engaged.contains(&f.candidate),
        "candidate must be a genuinely new pack"
    );
    assert!(
        matches!(
            candidate_budget(&mut f),
            Decision::Allow {
                bootstrap: false,
                ..
            }
        ),
        "active candidate must be genuinely affordable before testing production selection"
    );
    let owner_health = *f.h.app.world().get::<Health>(f.h.caster).unwrap();
    f.h.app
        .world_mut()
        .get_mut::<Health>(f.h.caster)
        .unwrap()
        .current = owner_health.max * 0.79;
    pull_history::sample_all(f.h.app.world_mut());
    assert_eq!(
        candidate_budget(&mut f),
        Decision::Wait(Reason::HealthStability)
    );
    crate::creature_ai::install_follower_follow_fixture(&mut f.h.app);
    f.h.advance(3);
    assert!(
        !tank_acquired(&f),
        "active engagement must not bypass a failing budget"
    );
}

#[test]
fn tank_owner_pacing_active_affordable_sourced_pack_can_be_selected() {
    let mut f = measured_fixture();
    damage(&mut f.h, f.engaged, 1.0);
    pull_history::sample_all(f.h.app.world_mut());
    let engaged = engaged_sources(&mut f);
    assert!(engaged.contains(&f.engaged));
    assert!(
        !engaged.contains(&f.candidate),
        "candidate must be a genuinely new pack"
    );
    assert!(
        matches!(
            candidate_budget(&mut f),
            Decision::Allow {
                bootstrap: false,
                ..
            }
        ),
        "all budget checks must pass at the positive selection boundary"
    );
    // The first actual AI decision must not be replaced by incidental area aggro
    // making the candidate an already-engaged target on a later tick.
    super::update_ai(f.h.app.world_mut());
    assert!(
        tank_acquired(&f),
        "production planner cannot select an affordable sourced new pack during active engagement"
    );
}

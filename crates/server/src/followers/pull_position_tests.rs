//! Existing-fight caster collection through actual native kit/chase, not injected goals.
use super::pull_endpoint_tests::place_party_at_authored_point;
use super::pull_goal_tests::{Fixture, physical_fixture, source};
use super::{Follower, ai, pull_history, pull_route};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::Position;
use std::collections::BTreeSet;

const OVERHEAT_GUID: u32 = 20375719;
const SELECTED_TRASH_GUID: u32 = 20375738;
// Actual boss_lord_overheat source spell, never granted to the selected trash.
const OVERHEAT_FIREBALL: u32 = 12466;
const REBUKE: u32 = 96231;

fn deal_native_tank_damage(f: &mut Fixture, enemy: Entity) {
    let spell =
        f.h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(133)
            .unwrap()
            .clone();
    let context = crate::spell_cast::effects::EffectCtx {
        caster: f.tank,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::deal_damage(
        f.h.app.world_mut(),
        &context,
        enemy,
        1.0,
        false,
        false,
    );
    assert_eq!(
        f.h.app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .unwrap()
            .0,
        Some(f.tank),
        "native damage must actually engage this source on the tank"
    );
}

fn engage_caster_fixture_sources(
    f: &mut Fixture,
    whole_route: bool,
    selected_guid: u32,
) -> Vec<u32> {
    let route = pull_route::inspect_route(
        f.h.app.world_mut(),
        &pull_route::Request {
            tank: f.tank,
            target_guid: OVERHEAT_GUID,
            difficulty: 1,
            cleared: &BTreeSet::new(),
        },
    )
    .unwrap();
    let engaged_guids = if whole_route {
        route.spawns.clone()
    } else {
        BTreeSet::from([OVERHEAT_GUID, selected_guid])
    };
    for guid in engaged_guids {
        let actor = source(&mut f.h, f.tank, guid);
        deal_native_tank_damage(f, actor);
    }
    route.spawns.into_iter().collect()
}

struct CasterFixture {
    f: Fixture,
    main: Entity,
    caster: Entity,
    from: Vec3,
    caster_point: Vec3,
}

fn overheat_fixture(whole_route: bool, selected_guid: u32) -> CasterFixture {
    let mut f = physical_fixture();
    let main = source(&mut f.h, f.tank, selected_guid);
    let caster = source(&mut f.h, f.tank, OVERHEAT_GUID);
    let (_, members, _) = super::party::read_admission(f.h.app.world(), f.h.caster).unwrap();
    let mut party = members.to_vec();
    party.push(f.h.caster);
    let main_point = crate::spell_cast::targets::position(f.h.app.world(), main).unwrap();
    let from = place_party_at_authored_point(&mut f, main_point, &party).unwrap();
    // Finish actual preparation casts/GCD without replacing them or advancing NPC chase.
    for _ in 0..200 {
        super::update_ai(f.h.app.world_mut());
        f.h.advance(1);
    }
    assert!(
        f.h.app.world().get::<CastState>(f.tank).is_none(),
        "native preparation must finish"
    );
    assert_eq!(
        crate::spell_cast::targets::position(f.h.app.world(), f.tank),
        Some(from)
    );
    let footprint = engage_caster_fixture_sources(&mut f, whole_route, selected_guid);
    assert!(footprint.contains(&OVERHEAT_GUID));
    let follower = f.h.app.world().get::<Follower>(f.tank).unwrap().clone();
    let engaged = ai::engaged_enemies(f.h.app.world_mut(), &follower, f.tank, &party);
    for guid in [OVERHEAT_GUID, selected_guid] {
        let actor = source(&mut f.h, f.tank, guid);
        assert!(engaged.iter().any(|(enemy, _)| *enemy == actor));
    }
    if whole_route {
        assert!(footprint.contains(&selected_guid));
        for &guid in &footprint {
            let actor = source(&mut f.h, f.tank, guid);
            assert!(engaged.iter().any(|(enemy, _)| *enemy == actor));
        }
    }
    f.h.select(Some(main));
    // Owner selection only prioritizes an existing fight: main and caster are already engaged.
    // Observe real kit selection after any sourced mitigation/ground attack, with no chase yet.
    let mut selected = false;
    for _ in 0..90 {
        super::update_ai(f.h.app.world_mut());
        selected =
            f.h.app
                .world()
                .get::<crate::networking::CombatTarget>(f.tank)
                .is_some_and(|target| target.0 == Some(main));
        if selected {
            break;
        }
        f.h.advance(1);
    }
    assert!(
        selected,
        "actual kit must select the already reachable trash before the caster starts"
    );
    let rebuke =
        f.h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(REBUKE)
            .unwrap()
            .clone();
    assert!(
        matches!(
            crate::spell_cast::read_native_cast_start(f.h.app.world_mut(), f.tank, &rebuke, main),
            Ok(Some(target)) if target == main
        ),
        "selected trash must pass actual native melee-range/LOS/cost/GCD validation"
    );
    crate::spell_cast::creature_cast(
        f.h.app.world_mut(),
        caster,
        OVERHEAT_FIREBALL,
        Some(f.tank),
        false,
    )
    .expect("actual Overheat Fireball must pass ordinary native start validation");
    let cast =
        f.h.app
            .world()
            .get::<CastState>(caster)
            .expect("actual sourced Fireball cast bar");
    assert_eq!(cast.spell_id, OVERHEAT_FIREBALL);
    assert!(cast.interruptible && !cast.cast_while_moving && cast.duration > 0.5);
    assert!(
        matches!(
            crate::spell_cast::read_native_cast_start(f.h.app.world_mut(), f.tank, &rebuke, caster),
            Err(crate::spell_cast::CastFailure::OutOfRange)
        ),
        "caster pickup must require a real native Rebuke approach"
    );
    pull_history::sample_all(f.h.app.world_mut());
    let caster_point = crate::spell_cast::targets::position(f.h.app.world(), caster).unwrap();
    CasterFixture {
        f,
        main,
        caster,
        from,
        caster_point,
    }
}

#[test]
fn production_tank_collects_real_overheat_caster_while_selected_trash_is_reachable() {
    let CasterFixture {
        mut f,
        main,
        caster,
        from,
        caster_point,
        ..
    } = overheat_fixture(true, SELECTED_TRASH_GUID);
    let before_distance = from.distance(caster_point);
    crate::creature_ai::install_follower_follow_fixture(&mut f.h.app);
    let mut nearest = before_distance;
    let mut acquired_caster = false;
    for _ in 0..10 {
        f.h.advance(1);
        let point = f.h.app.world().get::<Position>(f.tank).unwrap();
        nearest = nearest.min(Vec3::new(point.x, point.y, point.z).distance(caster_point));
        acquired_caster |=
            f.h.app
                .world()
                .get::<crate::networking::CombatTarget>(f.tank)
                .is_some_and(|target| target.0 == Some(caster));
    }
    assert_eq!(
        f.h.app
            .world()
            .get::<crate::networking::CombatTarget>(f.h.caster)
            .unwrap()
            .0,
        Some(main),
        "tank collection must not force the owner's selected target"
    );
    assert!(
        acquired_caster,
        "actual native tank victim must agree with the caster chase"
    );
    assert!(
        nearest < before_distance,
        "production tank stayed on reachable selected trash instead of collecting real casting Overheat: distance {before_distance}, nearest {nearest}"
    );
    // A finished/interrupted cast must not leave an unowned chase overriding later kit work.
    crate::spell_cast::cancel_cast(f.h.app.world_mut(), caster);
    let at = crate::spell_cast::targets::position(f.h.app.world(), f.tank).unwrap();
    f.h.advance(1);
    let after = crate::spell_cast::targets::position(f.h.app.world(), f.tank).unwrap();
    assert!(
        after.distance(caster_point) >= at.distance(caster_point),
        "ended source cast left a stale caster approach instead of the reachable selected fight"
    );
}

struct CasterBoundary {
    exposed: bool,
    refused: bool,
    casting: bool,
    details: String,
}

fn read_caster_boundary(f: &mut Fixture) -> CasterBoundary {
    let follower = f.h.app.world().get::<Follower>(f.tank).unwrap().clone();
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    let (map, members, _) = super::party::read_admission(f.h.app.world(), f.h.caster).unwrap();
    let mut party = members.to_vec();
    party.push(f.h.caster);
    let sources = super::pull_sources::read_present_sources(
        f.h.app.world_mut(),
        map,
        1,
        &history.cleared_spawns,
    )
    .unwrap();
    let packs: BTreeSet<_> = ai::engaged_enemies(f.h.app.world_mut(), &follower, f.tank, &party)
        .into_iter()
        .map(|(enemy, _)| {
            let guid =
                f.h.app
                    .world()
                    .get::<crate::zones::CreatureOrigin>(enemy)
                    .unwrap()
                    .guid;
            sources[&guid].pack_guid
        })
        .collect();
    let route = pull_route::inspect_route(
        f.h.app.world_mut(),
        &pull_route::Request {
            tank: f.tank,
            target_guid: OVERHEAT_GUID,
            difficulty: 1,
            cleared: &history.cleared_spawns,
        },
    )
    .unwrap();
    let exposed = route
        .spawns
        .iter()
        .any(|guid| !packs.contains(&sources[guid].pack_guid));
    let admission =
        super::pull_admission::read_pull_budget(f.h.app.world_mut(), f.tank, &route.spawns, 1);
    let refused = !matches!(admission, Ok(super::pull_budget::Decision::Allow { .. }));
    let caster = source(&mut f.h, f.tank, OVERHEAT_GUID);
    let casting =
        f.h.app
            .world()
            .get::<CastState>(caster)
            .is_some_and(|cast| cast.spell_id == OVERHEAT_FIREBALL && cast.interruptible);
    CasterBoundary {
        exposed,
        refused,
        casting,
        details: format!(
            "extra={exposed} refused={refused} casting={casting} route={:?} engaged_packs={packs:?} admission={admission:?}",
            route.spawns
        ),
    }
}

#[path = "pull_cast_origins_tests.rs"]
mod authored_origins;

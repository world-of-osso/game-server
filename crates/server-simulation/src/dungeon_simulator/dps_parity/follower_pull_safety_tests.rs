//! Class APL admission on native Windrunner assistance geometry.
use super::*;
use bevy::ecs::world::CommandQueue;
use shared::casting::CastState;
use shared::components::Position;
use std::collections::{BTreeMap, HashMap};

#[test]
#[ignore = "requires Devastation profile and native Windrunner world, ground and LOS data"]
fn windrunner_devastation_apl_fire_breath_respects_pull_closure() {
    let (profile, traits) = read_reference(1467).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let actor = run.actor;
    let owner = run
        .app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 73,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, 63805113 | 63805114 | 63805162))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 3);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    run.app.insert_resource(data.clone());
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    run.app.insert_resource(
        server::test_support::LineOfSight::load(&root.join("los")).expect("native LOS"),
    );
    let mut ground =
        server::test_support::GroundMap::new(&root, HashMap::from([(2805, 6_099_383)]));
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, run.app.world());
    assert_eq!(
        crate::zones::spawn_map_creatures(
            &mut commands,
            &data,
            crate::zones::MapContent {
                map,
                difficulty_id: 1,
                dead_bosses: vec![]
            },
            false,
            Some(&mut ground)
        ),
        3
    );
    queue.apply(run.app.world_mut());
    run.app.insert_resource(ground);
    let here = Position {
        x: 5316.415,
        y: 127.7347,
        z: 3124.3594,
    };
    run.app.world_mut().entity_mut(owner).insert((map, here));
    run.app
        .world_mut()
        .entity_mut(actor)
        .insert((map, here, crate::followers::AplControlled));
    run.app.update();
    run.app.update();
    let units: HashMap<_, _> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(run.app.world())
        .map(|(entity, origin)| (origin.guid, entity))
        .collect();
    let caller = units[&63805113];
    let helper = units[&63805114];
    let outside = units[&63805162];
    let caller_position = *run.app.world().get::<Position>(caller).unwrap();
    let outside_position = *run.app.world().get::<Position>(outside).unwrap();
    let direction = Vec2::new(
        outside_position.x - caller_position.x,
        outside_position.z - caller_position.z,
    )
    .normalize();
    let cast_position = Position {
        x: caller_position.x - direction.x * 3.0,
        y: caller_position.y,
        z: caller_position.z - direction.y * 3.0,
    };
    run.app
        .world_mut()
        .entity_mut(actor)
        .insert((cast_position, crate::networking::CombatTarget(Some(caller))));
    run.app.world_mut().entity_mut(owner).insert(cast_position);
    face_actor_toward_primary(run.app.world_mut(), actor, caller);
    let release = crate::spell_cast::script_registry::empower_release_preview(
        run.app.world(),
        actor,
        357208,
        1,
    )
    .unwrap();
    let damage = release
        .effects
        .iter()
        .find(|effect| effect.effect == crate::spell_cast::effects::EFFECT_SCHOOL_DAMAGE)
        .unwrap();
    let recipients = crate::spell_cast::targets::select_targets(
        run.app.world_mut(),
        actor,
        Some(caller),
        &release,
        damage,
        0,
        false,
    )
    .unwrap();
    assert!(
        recipients.contains(&outside),
        "native Fire Breath cone must actually reach the outsider"
    );
    crate::creature_aggro::attack_start(run.app.world_mut(), caller, actor);
    let encounter = crate::simc_apl_native::Encounter::Dungeon {
        started_at: crate::spell_cast::now(run.app.world()),
    };
    let rotation = server_follower_apl::devastation_rotation::Rotation::load(
        run.app.world(),
        actor,
        encounter,
    )
    .unwrap();
    let mut apl = server_follower_apl::FollowerApl::new(Box::new(rotation));
    let action = server::simc_apl::Action {
        source_row: None,
        name: "fire_breath".into(),
        target: Some(caller.to_bits()),
        options: BTreeMap::from([("empower_to".into(), "1".into())]),
    };
    let attempt = |apl: &mut server_follower_apl::FollowerApl, run: &mut Run| {
        apl.execute_plan(
            run.app.world_mut(),
            actor,
            action.clone(),
            server_follower_apl::literal_rotation::execution::Execution::Spell(
                357208,
                actor,
                Some(caller),
            ),
        )
        .unwrap();
    };
    attempt(&mut apl, &mut run);
    assert!(
        run.app.world().get::<CastState>(actor).is_none(),
        "APL must skip the cone while assistance is pending"
    );
    for _ in 0..60 {
        run.app.update();
        if crate::combat_refs::has_active_reference_to(run.app.world(), helper, actor) {
            break;
        }
    }
    assert!(
        crate::combat_refs::has_active_reference_to(run.app.world(), helper, actor),
        "native assistance must dispatch"
    );
    attempt(&mut apl, &mut run);
    assert!(
        run.app.world().get::<CastState>(actor).is_none(),
        "APL must skip the cone while an outsider is in reach"
    );
    let mut position = run.app.world_mut().get_mut::<Position>(outside).unwrap();
    position.x += 100.0;
    drop(position);
    run.app.update();
    face_actor_toward_primary(run.app.world_mut(), actor, caller);
    let recipients = crate::spell_cast::targets::select_targets(
        run.app.world_mut(),
        actor,
        Some(caller),
        &release,
        damage,
        0,
        false,
    )
    .unwrap();
    assert!(recipients.contains(&caller));
    assert!(
        recipients
            .iter()
            .all(|unit| [caller, helper].contains(unit)),
        "all native cone recipients must be in the first wave"
    );
    attempt(&mut apl, &mut run);
    assert!(
        run.app.world().get::<CastState>(actor).is_some(),
        "APL must cast when all reachable creatures are planned"
    );
}

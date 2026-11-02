//! Native Windrunner Spire pull-back with a class party driven by the real follower APL.
use crate::spell_cast::test_harness::ObservedCombatEvents;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;
use std::collections::HashMap;

const WINDRUNNER_SPIRE: u32 = 2805;
const FOLLOWER_DIFFICULTY: u32 = 205;
const GROUNDSKEEPER: u64 = 63805115;
const HATCHLING: u64 = 63805164;
const TICK: std::time::Duration = std::time::Duration::from_millis(50);

/// Owner route to the Groundskeeper pack, ending at the authored pull point.
const OWNER_ROUTE: [[f32; 3]; 6] = [
    [5314.594, 136.86368, 3080.4695],
    [5314.983, 136.06369, 3089.843],
    [5315.2554, 135.21678, 3096.4187],
    [5315.629, 133.40715, 3105.4285],
    [5316.076, 130.74896, 3116.221],
    [5316.6274, 126.463684, 3129.5269],
];

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(TICK));
    app.insert_resource(Time::<Fixed>::from_duration(TICK));
    app.add_plugins((
        crate::combat::CombatPlugin,
        crate::spell_cast::SpellCastPlugin,
        crate::auras::AuraPlugin,
        crate::creature_ai::CreatureAiPlugin,
        crate::creature_aggro::CreatureAggroPlugin,
        crate::creature_scripts::CreatureScriptsPlugin,
        crate::scripted_ai::ScriptedAiPlugin,
        crate::followers::FollowerPlugin,
        crate::hunter_pet::HunterPetPlugin,
        crate::pet_control::PetControlPlugin,
        crate::stats::StatsPlugin,
        server_follower_apl::FollowerAplPlugin,
    ));
    server::test_support::install_player_death(&mut app);
    app.init_resource::<crate::maps::InstanceManager>();
    app.init_resource::<ObservedCombatEvents>();
    app.init_resource::<crate::spell_cast::test_harness::ReplayCombatClock>();
    app.init_resource::<crate::spell_cast::holy_metrics::HolyMetrics>();
    // One executor order per seed, as in the simulator runtime.
    for label in [
        Startup.intern(),
        FixedPreUpdate.intern(),
        FixedUpdate.intern(),
        FixedPostUpdate.intern(),
        Update.intern(),
    ] {
        app.edit_schedule(label, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
    }
    app.finish();
    app.cleanup();
    app.update();
    super::runtime::load_gameplay_data(&mut app, &crate::test_data::world_db_path());
    app
}

struct Fixture {
    app: App,
    party: crate::followers::simulator::Party,
    target: Entity,
    hatchling: Entity,
}

fn spawn_fixture() -> Fixture {
    let mut app = build_app();
    let copy =
        super::runtime::create_map_copy(&mut app, WINDRUNNER_SPIRE, FOLLOWER_DIFFICULTY).unwrap();
    let [x, y, z] = OWNER_ROUTE[0];
    // Windrunner Spire's ContentTuning (LFGDungeons2745) spans levels 80-90.
    let party = crate::followers::simulator::spawn_test_party(
        app.world_mut(),
        copy,
        Position { x, y, z },
        90,
        // Fire Mage owner: its melee swing cannot land from the pull point, so
        // selecting a victim does not engage it.
        [66, 105, 262, 253, 63],
    )
    .expect("class follower party with Journal gear at follower difficulty");
    // The owner stands in for the production human player, who selects the
    // victim and does not act otherwise. The simulator leader's own rotation is
    // outside the follower hold contract: a player's Fire Blast may spread Ignite
    // onto out-of-combat enemies (wowpedia Mastery:_Ignite "Ignite can spread onto
    // targets who aren't in combat").
    crate::followers::simulator::stop_leader_driver(app.world_mut(), party.owner);
    {
        let mut data = app
            .world_mut()
            .resource_mut::<crate::world_data::WorldData>();
        let selected: Vec<_> = data.spawns_by_map[&(WINDRUNNER_SPIRE as u16)]
            .iter()
            .copied()
            .filter(|&index| matches!(data.spawns[index].guid, GROUNDSKEEPER | HATCHLING))
            .collect();
        data.spawns_by_map.insert(WINDRUNNER_SPIRE as u16, selected);
    }
    let root = super::graph::extraction::asset_root();
    let mut ground =
        server::test_support::GroundMap::new(&root, HashMap::from([(WINDRUNNER_SPIRE, 6_099_383)]));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let spawned =
        app.world_mut()
            .resource_scope(|world, data: Mut<crate::world_data::WorldData>| {
                crate::zones::spawn_map_creatures(
                    &mut Commands::new(&mut queue, world),
                    &data,
                    crate::zones::MapContent {
                        map: copy,
                        difficulty_id: FOLLOWER_DIFFICULTY,
                        dead_bosses: Vec::new(),
                    },
                    false,
                    Some(&mut ground),
                )
            });
    assert_eq!(spawned, 2);
    queue.apply(app.world_mut());
    app.insert_resource(ground);
    server::test_support::install_navigation(&mut app, &root.join("navmesh"));
    let population = super::runtime::live_population(app.world_mut(), copy);
    Fixture {
        app,
        party,
        target: population[&GROUNDSKEEPER],
        hatchling: population[&HATCHLING],
    }
}

fn party_engaged(world: &World, party: &[Entity], enemies: &[Entity]) -> bool {
    party.iter().any(|&unit| {
        enemies.iter().any(|&enemy| {
            crate::combat_refs::has_active_reference_to(world, enemy, unit)
                || crate::combat_refs::has_active_reference_to(world, unit, enemy)
        })
    })
}

#[test]
#[ignore = "requires Windrunner native ground, navigation and class follower data"]
fn windrunner_class_party_holds_until_engaged_and_real_healer_heals() {
    let _rng_scope = crate::random::seed_scope(1);
    let Fixture {
        mut app,
        party,
        target,
        hatchling,
    } = spawn_fixture();
    let owner = party.owner;
    let healer = party.members[1];
    let held_members = &party.members[1..];
    let units: Vec<_> = party.members.iter().copied().chain([owner]).collect();
    for [x, y, z] in OWNER_ROUTE {
        app.world_mut()
            .entity_mut(owner)
            .insert(Position { x, y, z });
        for _ in 0..20 {
            app.update();
        }
    }
    {
        let mut health = app.world_mut().get_mut::<Health>(owner).unwrap();
        health.current = health.max * 0.5;
    }
    let target_spawn = *app.world().get::<Position>(target).unwrap();
    let member_starts: Vec<_> = held_members
        .iter()
        .map(|&unit| *app.world().get::<Position>(unit).unwrap())
        .collect();
    // The owner's victim selects the pull without engaging it: the melee swing
    // cannot land from the pull point.
    let mut attack = crate::combat::AutoAttack::new(target);
    attack.timer = std::time::Duration::from_secs(60);
    app.world_mut()
        .entity_mut(owner)
        .insert((attack, crate::networking::CombatTarget(Some(target))));
    let mut held_unengaged = false;
    let mut healed_while_held = false;
    let mut released = false;
    let mut followed = false;
    let mut engaged_before = false;
    for tick in 0..300 {
        let before = app.world().resource::<ObservedCombatEvents>().0.len();
        app.update();
        let world = app.world();
        let events = &world.resource::<ObservedCombatEvents>().0[before..];
        let held: Vec<_> = held_members
            .iter()
            .map(|&unit| crate::followers::holding_pull_damage(world, unit))
            .collect();
        let engaged = party_engaged(world, &units, &[target, hatchling]);
        assert!(
            held.iter().all(|h| *h) || held.iter().all(|h| !*h),
            "non-tank members share one hold: {held:?}"
        );
        // docs/specs/follower-dungeons.md TANK_PULL_BACK_IDLE_PACK: "Before enemies
        // engage the party, non-tank followers may hold damage and follow the tank;
        // healers continue healing ... Once an enemy engages the party, release the
        // damage hold even if pull-back positioning continues."
        if held[0] {
            assert!(
                !engaged_before,
                "the hold must release once an enemy engages the party"
            );
            assert!(
                !events.iter().any(|event| {
                    event.kind == CombatLogKind::Damage
                        && held_members
                            .iter()
                            .any(|unit| event.source == Some(unit.to_bits()))
                }),
                "a held follower damaged an enemy"
            );
            followed |= held_members.iter().all(|&unit| {
                world
                    .get::<crate::creature_ai::FollowOwner>(unit)
                    .is_some_and(|follow| follow.owner == party.tank)
            });
            held_unengaged |= !engaged;
            healed_while_held |= events.iter().any(|event| {
                event.kind == CombatLogKind::Heal
                    && event.source == Some(healer.to_bits())
                    && event.target == Some(owner.to_bits())
                    && event.amount > 0
            });
        } else if engaged_before {
            released = true;
        }
        assert!(
            units.iter().all(|&unit| {
                !crate::combat_refs::has_active_reference_to(world, hatchling, unit)
                    && !crate::combat_refs::has_active_reference_to(world, unit, hatchling)
            }),
            "the idle hatchling must not be pulled at tick {tick}"
        );
        engaged_before = engaged;
    }
    assert!(
        held_unengaged,
        "the party must hold while nobody is engaged"
    );
    assert!(
        healed_while_held,
        "the real APL healer must heal the injured owner during the hold"
    );
    assert!(released, "the hold must release after the enemy engages");
    // Following starts when the tank route publishes, one fixed tick after the request.
    assert!(followed, "held followers must follow the tank");
    for (&unit, start) in held_members.iter().zip(&member_starts) {
        let end = *app.world().get::<Position>(unit).unwrap();
        let moved = Vec3::new(start.x, start.y, start.z).distance(Vec3::new(end.x, end.y, end.z));
        assert!(moved > 1.0, "{unit:?} must move during the pull-back");
    }
    let target_end = *app.world().get::<Position>(target).unwrap();
    let pulled = Vec3::new(target_spawn.x, target_spawn.y, target_spawn.z).distance(Vec3::new(
        target_end.x,
        target_end.y,
        target_end.z,
    ));
    assert!(
        pulled > 10.0,
        "the tank must pull the Groundskeeper back from its pack: {pulled:.1} yards"
    );
}

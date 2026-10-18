//! Actual Herald/Nazan source pair: template mutation, not encounter choreography.
use super::*;
use shared::components::ThreatTable;

const HERALD: u32 = 17_307;
const NAZAN: u32 = 17_536;

fn source_fixture() -> (WorldData, CreatureSpawn) {
    let mut data = super::context_tuning_tests::sourced_nazan_fixture();
    let canonical = crate::test_data::world_data();
    data.addons = canonical.addons.clone();
    data.template_addons = canonical.template_addons.clone();
    data.equipment = canonical.equipment.clone();
    data.waypoints = canonical.waypoints.clone();
    let spawn = canonical
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == HERALD && spawn.map == 543)
        .expect("actual ordinary Herald placement")
        .clone();
    assert_eq!(spawn.guid, 20_202_693);
    assert_eq!(data.addon_for(&spawn).unwrap().path_id, 36_215_440);
    (data, spawn)
}

fn spawn_herald(data: &WorldData, spawn: &CreatureSpawn, difficulty: u32) -> (World, Entity) {
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let actor = spawn_creature(
        &mut Commands::new(&mut queue, &world),
        data,
        data.template_for(HERALD, difficulty).unwrap(),
        &mut CreatureSpawnContext {
            map: crate::maps::InMap {
                map_id: 543,
                instance_id: difficulty,
            },
            difficulty_id: difficulty,
            ground: None,
            temporary: false,
        },
        false,
        spawn,
        &mut rand::thread_rng(),
    )
    .expect("source Herald template has explicit tuning");
    queue.apply(&mut world);
    (world, actor)
}

#[test]
fn herald_entry_ordinary_spawn_retains_original_source_identity() {
    let (data, spawn) = source_fixture();
    let (world, actor) = spawn_herald(&data, &spawn, 1);
    assert_eq!(
        world.get::<CreatureOrigin>(actor),
        Some(&CreatureOrigin {
            guid: 20_202_693,
            original_entry: HERALD,
        })
    );
    assert_eq!(
        world.get::<UnitPose>(actor).unwrap().sheath_state,
        SheathState::Unarmed
    );
}

#[test]
fn herald_entry_same_actor_uses_target_normal_and_heroic_stats_and_original_addon() {
    let (data, spawn) = source_fixture();
    for (difficulty, tuning, health_modifier, damage_modifier) in
        [(1, 220, 9.0, 1.0), (2, 1065, 12.0, 13.0)]
    {
        let (mut world, actor) = spawn_herald(&data, &spawn, difficulty);
        let map = *world.get::<crate::maps::InMap>(actor).unwrap();
        let home = world.get::<SpawnPoint>(actor).unwrap().clone();
        let position = Position {
            x: -1410.5204,
            y: 89.05998,
            z: -1740.5852,
        };
        let target = world.spawn_empty().id();
        let mut threat = ThreatTable::default();
        threat.add_threat(target.to_bits(), 73.0);
        world.entity_mut(actor).insert((position, threat));
        world.get_mut::<Health>(actor).unwrap().current = 1.0;
        let placement = CreatureSpawn {
            template_entry: NAZAN,
            position_x: position.x,
            position_y: -position.z,
            position_z: position.y,
            ..spawn.clone()
        };
        let projection = read_entry_projection(
            &data,
            NAZAN,
            &mut CreatureSpawnContext {
                map,
                difficulty_id: difficulty,
                ground: None,
                temporary: false,
            },
            &placement,
            None,
            &mut rand::thread_rng(),
        )
        .expect("target CT0 resolves source Ramparts map context");
        projection.apply(world.entity_mut(actor), false);
        assert_eq!(world.get::<Npc>(actor).unwrap().template_id, NAZAN);
        assert_eq!(world.get::<Npc>(actor).unwrap().name, "Nazan");
        assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 18_812);
        assert_eq!(*world.get::<Position>(actor).unwrap(), position);
        assert_eq!(*world.get::<crate::maps::InMap>(actor).unwrap(), map);
        assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, 30);
        let scaling = world.get::<LevelScaling>(actor).unwrap();
        assert_eq!((scaling.content_tuning_id, scaling.delta), (tuning, 0));
        let stats = LevelStats::build(
            &data.expected_stats,
            &TuningInputs {
                content_tuning_id: tuning,
                health_scaling_expansion: 1,
                unit_class: 1,
                health_modifier,
                damage_modifier,
                armor_modifier: 1.0,
            },
        );
        let health = world.get::<Health>(actor).unwrap();
        assert_eq!(
            (health.current, health.max),
            (stats.max_health(30), stats.max_health(30))
        );
        assert_eq!(
            world.get::<CreatureArmor>(actor).unwrap().0,
            stats.armor(30)
        );
        assert_eq!(
            world.get::<UnitPose>(actor).unwrap().sheath_state,
            SheathState::Unarmed
        );
        let current_home = world.get::<SpawnPoint>(actor).unwrap();
        assert_eq!(
            (current_home.x, current_home.y, current_home.z),
            (home.x, home.y, home.z)
        );
        assert_eq!(
            world.get::<ThreatTable>(actor).unwrap().entries()[0].threat,
            73.0
        );
        assert_eq!(world.query::<&Npc>().iter(&world).count(), 1);
    }
}

#[test]
fn herald_entry_unresolved_target_context_refuses_without_mutating_actor() {
    let (data, spawn) = source_fixture();
    let (mut world, actor) = spawn_herald(&data, &spawn, 1);
    let original = world.get::<Npc>(actor).unwrap().template_id;
    let projection = read_entry_projection(
        &data,
        NAZAN,
        &mut CreatureSpawnContext {
            map: crate::maps::InMap::continent(0),
            difficulty_id: 0,
            ground: None,
            temporary: false,
        },
        &spawn,
        None,
        &mut rand::thread_rng(),
    );
    assert!(
        projection.is_none(),
        "unknown area cannot supply legacy level1"
    );
    assert_eq!(world.get::<Npc>(actor).unwrap().template_id, original);
    assert_eq!(world.query::<&Npc>().iter(&world).count(), 1);
}

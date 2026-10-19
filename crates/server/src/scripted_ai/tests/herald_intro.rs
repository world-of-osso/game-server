//! Ordinary Ramparts actors and real lethal transitions through pre-INTRO2 arrival.
use super::*;

const HERALD: u32 = 17_307;
const SENTRY: u32 = 17_517;
const START_ENCOUNTER: i32 = 1_730_700;
const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};

pub(super) fn actors_in_copy(h: &mut Harness, entry: u32, map: InMap) -> Vec<Entity> {
    h.creatures_of(entry)
        .into_iter()
        .filter(|actor| h.app.world().get::<InMap>(*actor) == Some(&map))
        .collect()
}

fn ordinary_ramparts() -> (Harness, InMap, Entity, Vec<Entity>) {
    ordinary_ramparts_at(1)
}

pub(super) fn ordinary_ramparts_at(difficulty: u32) -> (Harness, InMap, Entity, Vec<Entity>) {
    let (mut h, _, map) = encounter_fixture(
        HERALD,
        543,
        1,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    // Shared encounter fixtures omit ordinary patrol data. Recreate only this
    // source Herald after loading its real path, before any death is exercised.
    h.app.world_mut().resource_mut::<WorldData>().waypoints =
        crate::test_data::world_data().waypoints.clone();
    let incomplete_herald = actors_in_copy(&mut h, HERALD, map)[0];
    h.app.world_mut().entity_mut(incomplete_herald).despawn();
    spawn_source_actor(&mut h, map, 20_202_693);
    h.advance(0.05);
    let heralds = actors_in_copy(&mut h, HERALD, map);
    let sentries = actors_in_copy(&mut h, SENTRY, map);
    assert_eq!(heralds.len(), 1, "ordinary Herald population");
    assert_eq!(sentries.len(), 2, "ordinary pair of Sentries");
    let herald = heralds[0];
    assert_source_origins(&h, herald, &sentries);
    (h, map, herald, sentries)
}

fn assert_source_origins(h: &Harness, herald: Entity, sentries: &[Entity]) {
    let origin = h
        .app
        .world()
        .get::<crate::zones::CreatureOrigin>(herald)
        .unwrap();
    assert_eq!(origin.guid, 20_202_693);
    let mut guids: Vec<_> = sentries
        .iter()
        .map(|actor| {
            h.app
                .world()
                .get::<crate::zones::CreatureOrigin>(*actor)
                .unwrap()
                .guid
        })
        .collect();
    guids.sort_unstable();
    assert_eq!(guids, [20_202_705, 20_202_706]);
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::WaypointState>(herald)
            .is_some()
    );
}

fn spawn_source_actor(h: &mut Harness, map: InMap, guid: u32) -> Entity {
    let difficulty = crate::zones::map_difficulty_id(h.app.world(), map);
    let data = h.app.world().resource::<WorldData>();
    let spawn = data.spawns.iter().find(|spawn| spawn.guid == guid).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let actor = crate::zones::spawn_creature(
        &mut Commands::new(&mut queue, h.app.world()),
        data,
        data.template_for(spawn.template_entry, difficulty).unwrap(),
        &mut crate::zones::CreatureSpawnContext {
            map,
            difficulty_id: difficulty,
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        spawn,
        &mut rand::thread_rng(),
    )
    .expect("ordinary source actor");
    queue.apply(h.app.world_mut());
    actor
}

pub(super) fn kill_actor(h: &mut Harness, actor: Entity) {
    let damage = h.health(actor).max * 2.0;
    let map = h.app.world().get::<InMap>(actor).unwrap();
    let player = h
        .players
        .iter()
        .find_map(|(player, _, _)| {
            (h.app.world().get::<InMap>(*player) == Some(map)).then_some(*player)
        })
        .expect("living player in the actor's allocated copy");
    crate::combat::deal_direct_damage(h.app.world_mut(), player, actor, damage);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(actor)
            .is_some()
    );
}

fn is_passive(h: &Harness, actor: Entity) -> bool {
    h.app.world().get::<ReactState>(actor) == Some(&ReactState::Passive)
}

pub(super) fn spawn_second_copy(h: &mut Harness) -> InMap {
    spawn_source_copy(h, 3001, 1)
}

pub(super) fn spawn_source_copy(h: &mut Harness, character_id: u64, difficulty: u32) -> InMap {
    let map = h.app.world_mut().resource_scope(
        |world, mut instances: Mut<crate::maps::InstanceManager>| {
            instances
                .create_map(
                    world.resource::<MapStore>(),
                    543,
                    crate::maps::instances::InstanceEntrant {
                        character_id,
                        leader: character_id,
                        difficulty_id: difficulty,
                        ..Default::default()
                    },
                    crate::maps::instances::Now {
                        elapsed: world.resource::<Time<Fixed>>().elapsed_secs_f64(),
                        unix: 0,
                    },
                )
                .expect("independently allocated source copy")
                .map
        },
    );
    fill_ramparts_copy(h, map);
    map
}

fn fill_ramparts_copy(h: &mut Harness, map: InMap) {
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        h.app.world().resource::<WorldData>(),
        crate::zones::MapContent {
            map,
            difficulty_id: crate::zones::map_difficulty_id(h.app.world(), map),
            dead_bosses: Vec::new(),
        },
        false,
        Some(&mut crate::test_data::ground_map()),
    );
    queue.apply(h.app.world_mut());
    h.advance(0.05);
}

#[test]
fn encounter_coverage_herald_intro_two_sentry_deaths_move_once_to_actual_center() {
    let (mut h, map, herald, sentries) = ordinary_ramparts();
    let listener = *h.app.world().get::<Position>(sentries[0]).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(listener);
    kill_actor(&mut h, sentries[0]);
    h.advance(0.05);
    assert!(!is_passive(&h, herald), "one death cannot start the intro");
    kill_actor(&mut h, sentries[1]);
    h.advance(0.05);
    assert!(
        is_passive(&h, herald),
        "second real Sentry death starts INTRO1"
    );
    let current = *h.app.world().get::<Position>(herald).unwrap();
    let speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(herald)
        .unwrap()
        .speed_run
        * 7.0;
    let distance = Vec3::new(current.x, current.y, current.z)
        .distance(Vec3::new(CENTER.x, CENTER.y, CENTER.z));
    let travel_ticks = (distance / (speed * 0.05)).ceil() as usize;
    for _ in 0..=travel_ticks {
        if h.app.world().get::<Position>(herald) == Some(&CENTER) {
            break;
        }
        h.advance(0.05);
    }
    assert_eq!(*h.app.world().get::<Position>(herald).unwrap(), CENTER);
    assert_eq!(
        h.app.world().get::<CreatureMotion>(herald),
        Some(&CreatureMotion::Still)
    );
    // INTRO1 owns the pre-INTRO2 window; source INTRO2 is due at arrival+1s.
    h.advance(0.8);
    assert_eq!(*h.app.world().get::<Position>(herald).unwrap(), CENTER);
    // Exercise duplicate action delivery only after proving real death ingress.
    dispatch(h.app.world_mut(), herald, Hook::DoAction(START_ENCOUNTER));
    dispatch(h.app.world_mut(), herald, Hook::DoAction(START_ENCOUNTER));
    h.advance(0.1);
    assert_eq!(*h.app.world().get::<Position>(herald).unwrap(), CENTER);
    assert_eq!(
        h.app.world().get::<CreatureMotion>(herald),
        Some(&CreatureMotion::Still)
    );
    assert_eq!(h.app.world().get::<InMap>(herald), Some(&map));
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(herald)
            .unwrap()
            .template_id,
        HERALD
    );
    let intro_yells = h.inbox(0).chat.iter().filter(|message| {
        matches!(message.channel, ChatType::MonsterYell(speaker) if speaker == herald.to_bits())
    }).count();
    assert_eq!(
        intro_yells, 1,
        "duplicate actions do not replay INTRO1 speech"
    );
    assert!(
        h.creatures_of(17_537).is_empty(),
        "no premature summon before the source arrival+1s deadline"
    );
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 1),
        Some(EncounterState::NotStarted)
    );
}

#[test]
fn encounter_coverage_herald_intro_split_copies_unrelated_and_corpse_deaths_do_not_count() {
    let (mut h, first_map, first_herald, first_sentries) = ordinary_ramparts();
    let second_map = spawn_second_copy(&mut h);
    assert_ne!(first_map, second_map);
    let second_herald = actors_in_copy(&mut h, HERALD, second_map)[0];
    let second_sentries = actors_in_copy(&mut h, SENTRY, second_map);
    assert_eq!(second_sentries.len(), 2);
    kill_actor(&mut h, first_sentries[0]);
    kill_actor(&mut h, second_sentries[0]);
    let unrelated = actors_in_copy(&mut h, 17_306, first_map)[0];
    kill_actor(&mut h, unrelated);
    // Real damage ingress refuses an already-dead actor; it must not count twice.
    kill_actor(&mut h, first_sentries[0]);
    h.advance(0.1);
    assert!(!is_passive(&h, first_herald));
    assert!(!is_passive(&h, second_herald));
    kill_actor(&mut h, first_sentries[1]);
    h.advance(0.05);
    assert!(is_passive(&h, first_herald), "two deaths in the first copy");
    assert!(
        !is_passive(&h, second_herald),
        "one death in the second copy"
    );
    kill_actor(&mut h, second_sentries[1]);
    h.advance(0.05);
    assert!(
        is_passive(&h, second_herald),
        "independent second-copy start"
    );
}

#[test]
fn encounter_coverage_herald_intro_missing_herald_retains_one_death_and_resets_a_pair() {
    for deaths_before_herald in [1, 2] {
        let (mut h, map, herald, sentries) = ordinary_ramparts();
        h.app.world_mut().entity_mut(herald).despawn();
        for sentry in sentries.iter().take(deaths_before_herald) {
            kill_actor(&mut h, *sentry);
        }
        h.advance(0.05);
        let herald = spawn_source_actor(&mut h, map, 20_202_693);
        h.advance(0.05);
        assert!(
            !is_passive(&h, herald),
            "appearance alone does not start INTRO1"
        );
        if deaths_before_herald == 1 {
            kill_actor(&mut h, sentries[1]);
        } else {
            // Fixture reconstruction, not a claim of implemented encounter reset.
            for sentry in sentries {
                h.app.world_mut().entity_mut(sentry).despawn();
            }
            let first = spawn_source_actor(&mut h, map, 20_202_705);
            let second = spawn_source_actor(&mut h, map, 20_202_706);
            kill_actor(&mut h, first);
            h.advance(0.05);
            assert!(
                !is_passive(&h, herald),
                "absent-Herald pair reset the count"
            );
            kill_actor(&mut h, second);
        }
        h.advance(0.05);
        assert!(
            is_passive(&h, herald),
            "two deaths start even across absent-Herald delivery"
        );
    }
}

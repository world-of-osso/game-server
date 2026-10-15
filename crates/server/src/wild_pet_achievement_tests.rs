use super::*;
use lightyear::prelude::{MessageReceiver, MessageSender};

fn spawn_level_achievement_fixture(
    database: redb::Database,
    carried_species: u32,
) -> (App, Entity, Entity, Entity, u64) {
    let (character_id, _) =
        crate::persistence::create_character(&database, 42, "Petleveler", 1, 1).unwrap();
    let catalog = super::tests::catalog();
    let mut journal = PetJournal::default();
    journal
        .add_with_guid(carried_species, 24, PetQuality::Rare, 3, 41)
        .unwrap();
    journal.pets[0].xp = catalog.next_level_xp[&24] - 1;
    journal
        .add_with_guid(39, 25, PetQuality::Rare, 3, 42)
        .unwrap();
    crate::battle_pets::save_journal(&database, 42, &journal);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.finish();
    app.cleanup();
    app.insert_resource(GameDatabase { db: database });
    app.insert_resource(crate::battle_pets::load_test_catalog());
    app.insert_resource(catalog);
    app.init_resource::<BattleIds>();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    app.insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&crate::achievement::AchievementPlugin, &mut app);
    app.add_systems(Update, crate::battle_pets::load_account_journals);
    let position = Position {
        x: -8949.0,
        y: 82.62,
        z: 132.0,
    };
    let owner = app
        .world_mut()
        .spawn((
            crate::collection::default_collection_state(),
            shared::components::Player {
                name: "Petleveler".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            shared::components::Level(1),
            crate::achievement::PlayerAchievementState::default(),
            position,
            crate::maps::InMap::continent(0),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 42 },
            InWorld { character_id },
            PlayerLink(owner),
            lightyear::prelude::RemoteId(lightyear::prelude::PeerId::Local(1)),
            lightyear::prelude::Connected,
            lightyear::connection::host::HostClient { buffer: Vec::new() },
            MessageSender::<WildPetBattleUpdate>::default(),
            MessageReceiver::<WildPetBattleUpdate>::default(),
        ))
        .id();
    let creature = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 61080,
                name: "Rabbit".into(),
            },
            position,
            crate::maps::InMap::continent(0),
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitLevel(20),
        ))
        .id();
    app.update();
    (app, owner, connection, creature, character_id)
}

fn play_native_carry_battle(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    creature: Entity,
) {
    start_encounter(world, connection, owner, creature.to_bits()).unwrap();
    for turn in 0..100 {
        let Some(active) = world.get::<ActiveEncounter>(owner) else {
            return;
        };
        let battle = &active.encounter.battle;
        let current = &battle.teams[0].pets[battle.teams[0].active];
        let action = if turn == 0 {
            WildPetBattleAction::Swap(2)
        } else if current.continuation.is_some() {
            WildPetBattleAction::Pass
        } else {
            current
                .abilities
                .iter()
                .enumerate()
                .find(|(_, id)| {
                    **id != 0
                        && current.cooldowns.get(id).copied().unwrap_or(0) == 0
                        && validate_ability(&world.resource::<WildCatalog>().turns, **id).is_ok()
                })
                .map_or(WildPetBattleAction::Pass, |(index, _)| {
                    WildPetBattleAction::Ability(index as u8 + 1)
                })
        };
        let request = WildPetBattleActionRequest {
            battle_id: active.encounter.id,
            round: battle.round,
            action,
        };
        advance_encounter(world, owner, request).unwrap();
    }
    panic!("native carry battle did not finish within100 turns");
}

fn refuse_stale_and_forfeit(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    creature: Entity,
) {
    start_encounter(world, connection, owner, creature.to_bits()).unwrap();
    let active = world.get::<ActiveEncounter>(owner).unwrap();
    let battle_id = active.encounter.id;
    let round = active.encounter.battle.round;
    let stale = WildPetBattleActionRequest {
        battle_id,
        round: u32::MAX,
        action: WildPetBattleAction::Pass,
    };
    assert!(
        advance_encounter(world, owner, stale)
            .unwrap_err()
            .contains("stale")
    );
    let forfeit = WildPetBattleActionRequest {
        battle_id,
        round,
        action: WildPetBattleAction::Forfeit,
    };
    advance_encounter(world, owner, forfeit).unwrap();
    assert!(world.get::<ActiveEncounter>(owner).is_none());
    let pet = world
        .get::<AccountPetJournal>(owner)
        .unwrap()
        .journal
        .get(41)
        .unwrap();
    assert_eq!(
        pet.level, 24,
        "refused/forfeited battles do not earn a level"
    );
    assert_eq!(pet.xp, 1979);
}

#[test]
fn achievements_criteria_native_pet_level_species_uses_actual_xp_reward() {
    // Falcosaur1974–1977 have no imported species states; native qualified162 is unproved.
    for species in [39, 1426] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("species.redb");
        let database = redb::Database::create(&path).unwrap();
        crate::persistence::create_tables(&database);
        let (mut app, owner, connection, creature, character_id) =
            spawn_level_achievement_fixture(database, species);
        assert_eq!(
            app.world()
                .get::<crate::achievement::PlayerAchievementState>(owner)
                .unwrap()
                .criteria_progress
                .get(&25123),
            None,
            "owned level24 is not credit"
        );
        refuse_stale_and_forfeit(app.world_mut(), connection, owner, creature);
        assert!(
            app.world()
                .get::<crate::achievement::PlayerAchievementState>(owner)
                .unwrap()
                .criteria_progress
                .get(&25123)
                .is_none()
        );
        play_native_carry_battle(app.world_mut(), connection, owner, creature);
        assert_eq!(
            app.world()
                .get::<AccountPetJournal>(owner)
                .unwrap()
                .journal
                .get(41)
                .unwrap()
                .level,
            25,
            "actual native terminal battle XP must cross24 to25"
        );
        let state = app
            .world()
            .get::<crate::achievement::PlayerAchievementState>(owner)
            .unwrap();
        for (criterion, required_species) in [(25123, 1426), (34921, 1974), (34667, 1974)] {
            assert_eq!(
                state.criteria_progress.get(&criterion),
                (species == required_species).then_some(&1),
                "actual pet species and level must match type160/162 criterion{criterion}"
            );
        }
        let expected = (species == 1426).then_some(&1);
        assert_eq!(
            state.account.as_ref().unwrap().earned_ids.contains(&9070),
            species == 1426
        );
        app.update();
        assert_eq!(
            app.world()
                .get::<crate::achievement::PlayerAchievementState>(owner)
                .unwrap()
                .criteria_progress
                .get(&25123),
            expected,
            "idle does not replay level credit"
        );
        drop(app);
        let database = redb::Database::open(&path).unwrap();
        assert_eq!(
            crate::battle_pets::load_journal(&database, 42)
                .get(41)
                .unwrap()
                .level,
            25
        );
        let saved = crate::persistence::load_achievements(&database, character_id);
        for (criterion, required_species) in [(25123, 1426), (34921, 1974), (34667, 1974)] {
            assert_eq!(
                saved.criteria_progress.get(&criterion),
                (species == required_species).then_some(&1)
            );
        }
        assert_eq!(crate::battle_pets::load_journal(&database, 43).count(), 0);
    }
}

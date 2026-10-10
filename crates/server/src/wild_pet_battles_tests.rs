use super::*;
use shared::pet_battle::{PetJournal, PetQuality};

struct Fixed(u32);
impl BattleRng for Fixed {
    fn draw(&mut self) -> u32 {
        self.0
    }
}
pub(super) fn catalog() -> WildCatalog {
    let db = Connection::open_with_flags(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    WildCatalog::load(&db).unwrap()
}
fn journal() -> PetJournal {
    let mut journal = PetJournal::default();
    for id in 41..44 {
        journal
            .add_with_guid(39, 1, PetQuality::Rare, 3, id)
            .unwrap();
    }
    journal
}
#[test]
fn wild_pet_battle_production_start_round_capture_and_account_persistence_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("production.redb");
    let database = redb::Database::create(&path).unwrap();
    crate::battle_pets::save_journal(&database, 42, &journal());
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
    app.insert_resource(catalog());
    app.init_resource::<BattleIds>();
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
            InWorld { character_id: 7 },
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
            crate::maps::InMap::continent(1),
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitLevel(1),
        ))
        .id();
    app.update();
    assert!(
        start_encounter(app.world_mut(), connection, owner, creature.to_bits())
            .unwrap_err()
            .contains("map")
    );
    app.world_mut()
        .entity_mut(creature)
        .insert(crate::maps::InMap::continent(0));
    start_encounter(app.world_mut(), connection, owner, creature.to_bits()).unwrap();
    assert!(start_encounter(app.world_mut(), connection, owner, creature.to_bits()).is_err());
    let id = app
        .world()
        .get::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .id;
    assert!(
        advance_encounter(
            app.world_mut(),
            owner,
            WildPetBattleActionRequest {
                battle_id: id,
                round: 99,
                action: WildPetBattleAction::Pass
            }
        )
        .unwrap_err()
        .contains("stale")
    );
    app.world_mut()
        .get_mut::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .battle
        .teams[1]
        .pets[0]
        .health = 1;
    app.world_mut()
        .get_mut::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .trap_failures = 4;
    for _ in 0..5 {
        let Some(active) = app.world().get::<ActiveEncounter>(owner) else {
            break;
        };
        let round = active.encounter.battle.round;
        advance_encounter(
            app.world_mut(),
            owner,
            WildPetBattleActionRequest {
                battle_id: id,
                round,
                action: WildPetBattleAction::Trap,
            },
        )
        .unwrap();
    }
    app.update();
    let delivered: Vec<_> = app
        .world_mut()
        .get_mut::<MessageReceiver<WildPetBattleUpdate>>(connection)
        .unwrap()
        .receive()
        .collect();
    let terminal_snapshot = delivered
        .iter()
        .rev()
        .find_map(|update| match update {
            WildPetBattleUpdate::Start(state)
            | WildPetBattleUpdate::State(state)
            | WildPetBattleUpdate::Round { state, .. } => Some(state),
            _ => None,
        })
        .expect("client receives authoritative battle state");
    assert_eq!(
        terminal_snapshot.teams[1][0].health, 1,
        "terminal HP reaches the client before End"
    );
    assert!(matches!(
        delivered.last(),
        Some(WildPetBattleUpdate::End {
            captured_pet_id: Some(_),
            ..
        })
    ));
    assert!(app.world().get::<ActiveEncounter>(owner).is_none());
    assert!(app.world().get::<WildPetClaim>(creature).is_none());
    assert_eq!(app.world().get::<Health>(creature).unwrap().current, 0.0);
    assert!(
        app.world()
            .get::<crate::combat::DeathState>(creature)
            .is_some()
    );
    let captured = app
        .world()
        .get::<AccountPetJournal>(owner)
        .unwrap()
        .journal
        .clone();
    assert_eq!(captured.count_species(378), 1);
    assert_eq!(captured.get(41).unwrap().level, 2);
    drop(app);
    let database = redb::Database::open(&path).unwrap();
    assert_eq!(crate::battle_pets::load_journal(&database, 42), captured);
    assert_eq!(crate::battle_pets::load_journal(&database, 43).count(), 0);
}

#[test]
fn wild_pet_battle_current_wild_xp_supports_carry_pets_without_a_legacy_level_cap() {
    assert_eq!(earned_xp(1, 1), 75);
    assert_eq!(earned_xp(1, 25), 435);
    assert_eq!(earned_xp(24, 1), 0);
}

#[test]
fn wild_pet_battle_trap_requires_a_living_active_ally() {
    let catalog = catalog();
    let journal = journal();
    let mut encounter = Encounter::start(&catalog, &journal, 378, 1, 99, 100).unwrap();
    encounter.battle.teams[1].pets[0].health = 1;
    encounter.battle.teams[0].pets[0].health = 0;
    assert!(!encounter.can_trap(&journal));
}

#[test]
fn wild_pet_battle_real_rabbit_ability_round_is_replayable_and_swap_is_ordered() {
    let catalog = catalog();
    let journal = journal();
    let encounter = Encounter::start(&catalog, &journal, 378, 1, 99, 100).unwrap();
    assert_eq!(
        encounter.snapshot(&catalog, &journal).teams[1][0].species_id,
        378
    );
    let (first, events) = encounter
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Ability(1),
            &mut Fixed(500_000),
        )
        .unwrap();
    let (replay, replay_events) = encounter
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Ability(1),
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(first.battle, replay.battle);
    assert_eq!(events, replay_events);
    let hud = first.snapshot(&catalog, &journal);
    assert_eq!(hud.teams[0][0].quality, 3);
    assert_eq!(hud.teams[0][0].icon, 656_559);
    assert_eq!(hud.teams[0][0].abilities[0].icon, 133_832);
    assert_eq!(hud.teams[0][0].abilities[0].family, 9);
    assert_eq!(hud.teams[0][0].xp, journal.get(41).unwrap().xp);
    assert!(hud.teams[0][0].next_level_xp > 0);
    let damage = encounter.battle.teams[1].pets[0].health - first.battle.teams[1].pets[0].health;
    assert!(hud.feedback.iter().any(|event| event.team == 1
        && event.slot == 0
        && event.text == damage.to_string()
        && !event.healing));
    assert!(
        !events
            .iter()
            .any(|text| text.contains("PetRef") || text.contains("Order("))
    );
    assert_eq!(hud.feedback, replay.snapshot(&catalog, &journal).feedback);
    assert!(first.battle.teams[1].pets[0].health < encounter.battle.teams[1].pets[0].health);
    let (swapped, _) = first
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Swap(2),
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(swapped.battle.teams[0].active, 1);
}
#[test]
fn wild_pet_battle_trap_is_strictly_below_35_and_capture_respects_species_limit() {
    let catalog = catalog();
    let mut journal = journal();
    let mut encounter = Encounter::start(&catalog, &journal, 378, 1, 99, 100).unwrap();
    let enemy = &mut encounter.battle.teams[1].pets[0];
    enemy.health = (enemy.stats.health * 35 + 99) / 100;
    assert!(
        encounter
            .step(&catalog, &journal, WildPetBattleAction::Trap, &mut Fixed(0))
            .is_err()
    );
    encounter.battle.teams[1].pets[0].health = 1;
    let (captured, _) = encounter
        .step(&catalog, &journal, WildPetBattleAction::Trap, &mut Fixed(0))
        .unwrap();
    assert_eq!(captured.battle.outcome, Some(Outcome::Winner(0)));
    assert!(captured.captured.is_some());
    captured
        .apply_rewards(&catalog, &mut journal, Some(999))
        .unwrap();
    assert_eq!(journal.get(999).unwrap().species_id, 378);
    for id in 998..1000 {
        if journal.get(id).is_none() {
            journal
                .add_with_guid(378, 1, PetQuality::Common, 3, id)
                .unwrap();
        }
    }
    journal
        .add_with_guid(378, 1, PetQuality::Common, 3, 1000)
        .unwrap();
    assert!(
        encounter
            .step(&catalog, &journal, WildPetBattleAction::Trap, &mut Fixed(0))
            .is_err()
    );
}
#[test]
fn wild_pet_battle_forfeit_has_no_rewards_and_win_xp_survives_reopen() {
    let catalog = catalog();
    let mut journal = journal();
    let encounter = Encounter::start(&catalog, &journal, 378, 1, 99, 100).unwrap();
    let (forfeited, _) = encounter
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Forfeit,
            &mut Fixed(0),
        )
        .unwrap();
    assert_eq!(forfeited.battle.outcome, Some(Outcome::Winner(1)));
    assert!(
        forfeited
            .apply_rewards(&catalog, &mut journal, None)
            .unwrap()
            .is_empty()
    );
    let mut winner = encounter;
    winner.battle.teams[1].pets[0].health = 1;
    let (winner, _) = winner
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Ability(1),
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(winner.battle.outcome, Some(Outcome::Winner(0)));
    let rewards = winner.apply_rewards(&catalog, &mut journal, None).unwrap();
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].xp_gained, 75);
    assert_eq!(journal.get(41).unwrap().level, 2);
    assert_eq!(journal.get(41).unwrap().xp, 25);
    assert_eq!(journal.get(42).unwrap().xp, 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wild.redb");
    {
        let db = redb::Database::create(&path).unwrap();
        crate::battle_pets::save_journal(&db, 42, &journal);
    }
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(crate::battle_pets::load_journal(&db, 42), journal);
}

use super::*;
use shared::pet_battle::{PetJournal, PetQuality};

struct Fixed(u32);
impl BattleRng for Fixed {
    fn draw(&mut self) -> u32 {
        self.0
    }
}
fn catalog() -> WildCatalog {
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
    assert!(rewards[0].xp_gained > 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wild.redb");
    {
        let db = redb::Database::create(&path).unwrap();
        crate::battle_pets::save_journal(&db, 42, &journal);
    }
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(crate::battle_pets::load_journal(&db, 42), journal);
}

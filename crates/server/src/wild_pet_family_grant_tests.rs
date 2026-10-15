use super::*;
use std::collections::{BTreeMap, BTreeSet};

// Installed70391 species, level10 criterion, level25 criterion, ordered by PetTypeEnum0–9.
const FAMILIES: [(u32, u32, u32); 10] = [
    (107, 21366, 21376),
    (56, 21363, 21373),
    (47, 21365, 21375),
    (187, 21369, 21379),
    (2, 21362, 21372),
    (93, 21368, 21378),
    (117, 21364, 21374),
    (40, 21361, 21371),
    (64, 21360, 21370),
    (39, 21367, 21377),
];
const FIGHTER: u64 = 42;

fn carried_id(family: usize, level: u8) -> u64 {
    let offset = match level {
        9 => 0,
        24 => 1,
        _ => panic!("unsupported starting fixture level{level}"),
    };
    100 + u64::try_from(family).unwrap() * 2 + offset
}

fn preowned_family_journal(catalog: &WildCatalog) -> PetJournal {
    let mut journal = PetJournal::default();
    for (family, (species, _, _)) in FAMILIES.iter().enumerate() {
        for level in [9, 24] {
            journal
                .add_with_guid(
                    *species,
                    level,
                    PetQuality::Rare,
                    3,
                    carried_id(family, level),
                )
                .unwrap();
            // Preexisting owned XP is a baseline, never an earned criterion or a battle mutation.
            journal.pets.last_mut().unwrap().xp = catalog.next_level_xp[&level] - 1;
        }
    }
    journal
        .add_with_guid(39, 25, PetQuality::Rare, 3, FIGHTER)
        .unwrap();
    journal
        .set_battle_slots([Some(carried_id(0, 9)), Some(FIGHTER), None])
        .unwrap();
    journal
}

fn select_native_repair_loadout(world: &mut World, owner: Entity) {
    for (slot, ability) in [(1, 384), (3, 278)] {
        validate_ability(&world.resource::<WildCatalog>().turns, ability)
            .unwrap_or_else(|error| panic!("source ability{ability}: {error}"));
        crate::battle_pets::choose_ability(
            world,
            owner,
            shared::protocol::SetBattlePetAbility {
                pet_id: FIGHTER,
                slot,
                ability_id: ability,
            },
        )
        .unwrap();
    }
}

fn spawn_initial_wild_targets(app: &mut App, first: Entity) -> Vec<Entity> {
    let world = app.world_mut();
    let npc = world.get::<Npc>(first).unwrap().clone();
    let position = *world.get::<Position>(first).unwrap();
    let map = *world.get::<crate::maps::InMap>(first).unwrap();
    let health = *world.get::<Health>(first).unwrap();
    let level = *world.get::<UnitLevel>(first).unwrap();
    assert!(health.current > 0.0);
    let mut targets = vec![first];
    for _ in 1..FAMILIES.len() * 2 {
        targets.push(
            world
                .spawn((npc.clone(), position, map, health, level))
                .id(),
        );
    }
    targets
}

fn play_native_repair_carry_battle(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    creature: Entity,
) -> usize {
    let stored = crate::battle_pets::load_or_initialize_pet_states(world, owner).unwrap();
    let health = stored[&FIGHTER].health;
    assert!(
        health > 0,
        "fighter must survive without fixture health resets"
    );
    start_encounter(world, connection, owner, creature.to_bits()).unwrap();
    assert_eq!(
        world
            .get::<ActiveEncounter>(owner)
            .unwrap()
            .encounter
            .battle
            .teams[0]
            .pets[1]
            .health,
        health,
        "the next encounter must use persistent injuries"
    );
    let mut repairs = 0;
    for turn in 0..100 {
        let Some(active) = world.get::<ActiveEncounter>(owner) else {
            return repairs;
        };
        let battle = &active.encounter.battle;
        let current = &battle.teams[0].pets[battle.teams[0].active];
        if turn > 0 {
            assert_eq!(
                battle.teams[0].active, 1,
                "repair fighter cannot be replaced after dying"
            );
        }
        let ready = |ability| {
            current
                .cooldowns
                .get(&ability)
                .is_none_or(|rounds| *rounds == 0)
        };
        let action = if turn == 0 {
            WildPetBattleAction::Swap(2)
        } else if current.continuation.is_some() {
            WildPetBattleAction::Pass
        } else if current.health < current.base_health && ready(278) {
            repairs += 1;
            WildPetBattleAction::Ability(3)
        } else {
            WildPetBattleAction::Ability(1)
        };
        let request = WildPetBattleActionRequest {
            battle_id: active.encounter.id,
            round: battle.round,
            action,
        };
        let health = battle.teams[0].pets[1].health;
        advance_encounter(world, owner, request)
            .unwrap_or_else(|error| panic!("turn{turn}/{action:?}, health{health}: {error}"));
        if turn < 12 {
            let health_after = crate::battle_pets::load_or_initialize_pet_states(world, owner)
                .unwrap()[&FIGHTER]
                .health;
            println!("native repair turn{turn} {action:?} hp{health}->{health_after}");
        }
    }
    panic!("native repair carry battle exceeded100 turns");
}

fn assert_family_counters(
    state: &crate::achievement::PlayerAchievementState,
    reached: &[BTreeSet<u8>; 2],
) {
    for (family, (_, ten, twenty_five)) in FAMILIES.iter().enumerate() {
        let family = u8::try_from(family).unwrap();
        for (criterion, families) in [(*ten, &reached[0]), (*twenty_five, &reached[1])] {
            assert_eq!(
                state.criteria_progress.get(&criterion),
                families.contains(&family).then_some(&1),
                "only the actual family/earned threshold may credit criterion{criterion}"
            );
        }
    }
}

fn assert_family_awards(
    world: &World,
    owner: Entity,
    reached: &[BTreeSet<u8>; 2],
    dates: &mut BTreeMap<u32, i64>,
    started: i64,
) {
    let state = world
        .get::<crate::achievement::PlayerAchievementState>(owner)
        .unwrap();
    assert_family_counters(state, reached);
    for (index, award) in [6609, 6610].into_iter().enumerate() {
        let earned = state
            .account
            .as_ref()
            .is_some_and(|account| account.earned_ids.contains(&award));
        assert_eq!(
            earned,
            reached[index].len() == 10,
            "award{award} requires every real family"
        );
        if earned {
            let date = state.account.as_ref().unwrap().earned_at[&award];
            assert!(
                (started..=world.resource::<crate::maps::ServerClock>().unix()).contains(&date)
            );
            assert_eq!(
                *dates.entry(award).or_insert(date),
                date,
                "award dates must remain original"
            );
        } else {
            assert!(
                !state
                    .account
                    .as_ref()
                    .is_some_and(|account| account.earned_at.contains_key(&award))
            );
        }
    }
}

#[test]
fn achievements_criteria_native_all_pet_families_earn_both_level_grants_without_health_resets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("families.redb");
    let database = redb::Database::create(&path).unwrap();
    crate::persistence::create_tables(&database);
    let (foreign, _) =
        crate::persistence::create_character(&database, 43, "Foreign", 1, 1).unwrap();
    let catalog = super::super::tests::catalog();
    let journal = preowned_family_journal(&catalog);
    let (mut app, owner, connection, creature, character) =
        spawn_owned_pet_achievement_fixture(database, journal, catalog);
    let mut targets = spawn_initial_wild_targets(&mut app, creature).into_iter();
    let started = app.world().resource::<crate::maps::ServerClock>().unix();
    let mut reached = [BTreeSet::new(), BTreeSet::new()];
    let mut dates = BTreeMap::new();
    assert_family_awards(app.world(), owner, &reached, &mut dates, started);
    select_native_repair_loadout(app.world_mut(), owner);
    let mut repairs = 0;
    for (family, (species, _, _)) in FAMILIES.iter().enumerate() {
        for (index, level, threshold) in [(0, 9, 10), (1, 24, 25)] {
            let guid = carried_id(family, level);
            let creature = targets
                .next()
                .expect("one distinct wild pet per real battle");
            let before =
                crate::battle_pets::load_or_initialize_pet_states(app.world(), owner).unwrap();
            set_loadout(app.world_mut(), owner, [Some(guid), Some(FIGHTER), None]).unwrap();
            let selected =
                crate::battle_pets::load_or_initialize_pet_states(app.world(), owner).unwrap();
            assert_eq!(
                selected[&FIGHTER].health, before[&FIGHTER].health,
                "loadout changes cannot heal"
            );
            println!(
                "starting family{family} species{species} threshold{threshold} fighterHP{}",
                before[&FIGHTER].health
            );
            repairs +=
                play_native_repair_carry_battle(app.world_mut(), connection, owner, creature);
            assert_eq!(
                app.world().get::<Health>(creature).unwrap().current,
                0.0,
                "a defeated wild pet stays consumed; no fixture revival"
            );
            let pet = app
                .world()
                .get::<AccountPetJournal>(owner)
                .unwrap()
                .journal
                .get(guid)
                .unwrap();
            assert_eq!(pet.species_id, *species);
            assert!(
                pet.level >= threshold,
                "genuine XP must cross level{threshold}"
            );
            if threshold == 10 {
                assert!(
                    pet.level < 25,
                    "a low-level carry cannot invent the25 milestone"
                );
            }
            reached[index].insert(u8::try_from(family).unwrap());
            assert_family_awards(app.world(), owner, &reached, &mut dates, started);
            app.update();
            assert_family_awards(app.world(), owner, &reached, &mut dates, started);
            println!(
                "native family{family} species{species} threshold{threshold} fighterHP{} -> {}",
                before[&FIGHTER].health,
                crate::battle_pets::load_or_initialize_pet_states(app.world(), owner).unwrap()
                    [&FIGHTER]
                    .health
            );
        }
    }
    assert!(
        repairs > 0,
        "source Repair must actually be selected, never a health override"
    );
    assert_eq!(dates.len(), 2);
    let health = crate::battle_pets::load_or_initialize_pet_states(app.world(), owner).unwrap()
        [&FIGHTER]
        .health;
    assert!(health > 0);
    drop(app);
    let reopened = redb::Database::open(&path).unwrap();
    let saved = crate::persistence::load_achievements(&reopened, character);
    assert_family_counters(&saved, &reached);
    assert_eq!(
        crate::pet_journal_state::load_states(&reopened, 42).unwrap()[&FIGHTER].health,
        health
    );
    let (app, owner, _, _, _) = spawn_saved_pet_achievement_fixture(
        reopened,
        super::super::tests::catalog(),
        character,
        42,
    );
    assert_family_awards(app.world(), owner, &reached, &mut dates, started);
    drop(app);
    let reopened = redb::Database::open(&path).unwrap();
    let (app, owner, _, _, _) =
        spawn_saved_pet_achievement_fixture(reopened, super::super::tests::catalog(), foreign, 43);
    assert_family_awards(
        app.world(),
        owner,
        &[BTreeSet::new(), BTreeSet::new()],
        &mut BTreeMap::new(),
        started,
    );
    assert_eq!(
        app.world()
            .get::<AccountPetJournal>(owner)
            .unwrap()
            .journal
            .count(),
        0
    );
}

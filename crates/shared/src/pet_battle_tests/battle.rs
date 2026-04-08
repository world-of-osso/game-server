use super::*;

#[test]
fn battle_starts_in_select_action() {
    let battle = PetBattle::new(vec![make_pet(100, 10, 10)], vec![make_pet(100, 10, 10)]);
    assert_eq!(battle.phase, BattlePhase::SelectAction);
    assert_eq!(battle.turn, 1);
}

#[test]
fn faster_pet_acts_first() {
    let player_pet = make_pet(200, 30, 100);
    let opp_pet = make_pet(200, 10, 50);
    let opp_initial_health = opp_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(
        battle.opponent.active_pet().current_health,
        opp_initial_health - 30
    );
}

#[test]
fn slower_pet_acts_second() {
    let player_pet = make_pet(200, 10, 50);
    let opp_pet = make_pet(200, 30, 100);
    let player_initial_health = player_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(
        battle.player.active_pet().current_health,
        player_initial_health - 30
    );
}

#[test]
fn pet_faints_and_auto_swaps() {
    let pets = vec![
        make_pet(10, 5, 10),
        make_pet(500, 5, 10),
        make_pet(500, 5, 10),
    ];
    let opp_pets = vec![make_pet(500, 50, 5)];
    let mut battle = PetBattle::new(pets, opp_pets);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(battle.player.active, 1);
    assert_eq!(battle.phase, BattlePhase::SelectAction);
}

#[test]
fn all_pets_faint_ends_battle() {
    let player_pets = vec![make_pet(1, 5, 5)];
    let opp_pets = vec![make_pet(500, 50, 10)];
    let mut battle = PetBattle::new(player_pets, opp_pets);
    battle.submit_action(BattleSide::Player, TurnAction::Pass);
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(battle.phase, BattlePhase::Ended);
    assert_eq!(battle.outcome, Some(BattleOutcome::OpponentWins));
}

#[test]
fn swap_action_changes_active() {
    let pets = vec![
        make_pet(200, 10, 10),
        make_pet(200, 10, 10),
        make_pet(200, 10, 10),
    ];
    let opp_pets = vec![make_pet(200, 10, 10)];
    let mut battle = PetBattle::new(pets, opp_pets);
    battle.submit_action(BattleSide::Player, TurnAction::Swap(1));
    battle.submit_action(BattleSide::Opponent, TurnAction::Pass);
    assert_eq!(battle.player.active, 1);
}

#[test]
fn pass_does_nothing() {
    let player_pet = make_pet(200, 10, 10);
    let opp_pet = make_pet(200, 10, 10);
    let p_health = player_pet.stats.health;
    let o_health = opp_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    let events = battle.submit_action(BattleSide::Player, TurnAction::Pass);
    assert!(events.is_empty());
    let events = battle.submit_action(BattleSide::Opponent, TurnAction::Pass);
    assert!(events.is_empty());
    assert_eq!(battle.player.active_pet().current_health, p_health);
    assert_eq!(battle.opponent.active_pet().current_health, o_health);
    assert_eq!(battle.turn, 2);
}

#[test]
fn turn_counter_increments() {
    let mut battle = PetBattle::new(vec![make_pet(500, 5, 5)], vec![make_pet(500, 5, 5)]);
    for _ in 0..3 {
        battle.submit_action(BattleSide::Player, TurnAction::Pass);
        battle.submit_action(BattleSide::Opponent, TurnAction::Pass);
    }
    assert_eq!(battle.turn, 4);
}

#[test]
fn submit_after_ended_returns_empty() {
    let player_pets = vec![make_pet(1, 5, 5)];
    let opp_pets = vec![make_pet(500, 50, 10)];
    let mut battle = PetBattle::new(player_pets, opp_pets);
    battle.submit_action(BattleSide::Player, TurnAction::Pass);
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(battle.phase, BattlePhase::Ended);
    let events = battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    assert!(events.is_empty());
}

#[test]
fn speed_tie_favors_player() {
    let player_pet = make_pet(200, 30, 50);
    let opp_pet = make_pet(200, 10, 50);
    let opp_initial_health = opp_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(
        battle.opponent.active_pet().current_health,
        opp_initial_health - 30
    );
}

use super::effects2_tests::use_ability;
use super::*;

fn harvest_battle(catalog: &Catalog) -> Battle {
    let mut initial = battle(catalog, 9, 7, [2457, 123, 110]);
    for (pet, health) in initial.teams[0].pets.iter_mut().zip([100, 200, 300]) {
        pet.health = health;
    }
    initial
}

fn slot_order(active: usize) -> Vec<usize> {
    std::iter::once(active)
        .chain((0..3).filter(|slot| *slot != active))
        .collect()
}

#[test]
fn pet_battle_blood_harvest_all_active_pairs_heal_distinct_allies_by_actual_paired_damage() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let mut initial = harvest_battle(&catalog);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            for (pet, health) in initial.teams[1].pets.iter_mut().zip([4, 20, 10000]) {
                pet.health = health;
            }
            let (state, events) = use_ability(&initial, &catalog, 2457);
            let actual_damage = [4, 20, 60];
            let mut expected_health = [100, 200, 300];
            for (ally_slot, enemy_slot) in slot_order(ally_active)
                .into_iter()
                .zip(slot_order(enemy_active))
            {
                expected_health[ally_slot] += actual_damage[enemy_slot];
            }
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                expected_health
            );
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [0, 0, 9940]
            );
            assert_eq!(
                damage(&events),
                slot_order(enemy_active)
                    .into_iter()
                    .map(|slot| actual_damage[slot])
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                state.teams[0].pets[ally_active].cooldowns.get(&2457),
                Some(&2)
            );
            assert_eq!(state.teams[0].active, ally_active);
        }
    }
}

#[test]
fn pet_battle_blood_harvest_each_and_all_misses_never_reuse_previous_damage_for_healing() {
    let catalog = catalog();
    for enemy_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = harvest_battle(&catalog);
            initial.teams[1].active = enemy_active;
            initial.teams[0].pets[0]
                .states
                .insert(catalog.state_id("Last_HitDealt").unwrap(), 777);
            for slot in 0..3 {
                if missed.is_none() || missed == Some(slot) {
                    initial.teams[1].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            let (state, events) = use_ability(&initial, &catalog, 2457);
            let mut expected_health = [100, 200, 300];
            for (ally_slot, enemy_slot) in slot_order(0).into_iter().zip(slot_order(enemy_active)) {
                let dodged = missed.is_none() || missed == Some(enemy_slot);
                expected_health[ally_slot] += if dodged { 0 } else { 60 };
                assert_eq!(
                    state.teams[1].pets[enemy_slot].health,
                    if dodged { 10000 } else { 9940 }
                );
            }
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                expected_health
            );
            assert_eq!(damage(&events).len(), if missed.is_none() { 0 } else { 2 });
        }
    }
}

#[test]
fn pet_battle_blood_harvest_dead_enemies_cannot_carry_a_previous_hit_into_another_ally() {
    let catalog = catalog();
    for dead in [&[1][..], &[2][..], &[1, 2][..]] {
        let mut initial = harvest_battle(&catalog);
        initial.teams[0].pets[0]
            .states
            .insert(catalog.state_id("Last_HitDealt").unwrap(), 777);
        for &slot in dead {
            initial.teams[1].pets[slot].health = 0;
            initial.teams[1].pets[slot]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        }
        let (state, events) = use_ability(&initial, &catalog, 2457);
        for slot in 0..3 {
            let before = initial.teams[0].pets[slot].health;
            assert_eq!(
                state.teams[0].pets[slot].health,
                before + if dead.contains(&slot) { 0 } else { 60 }
            );
            assert_eq!(
                state.teams[1].pets[slot].health,
                if dead.contains(&slot) { 0 } else { 9940 }
            );
        }
        assert_eq!(damage(&events).len(), 3 - dead.len());
    }
}

#[test]
fn pet_battle_blood_harvest_keeps_dead_allies_dead_and_caps_each_living_ally() {
    let catalog = catalog();
    for dead in [1, 2] {
        let mut initial = harvest_battle(&catalog);
        for pet in &mut initial.teams[0].pets {
            pet.health = 9990;
        }
        initial.teams[0].pets[dead].health = 0;
        initial.teams[0].pets[dead]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        let (state, events) = use_ability(&initial, &catalog, 2457);
        assert_eq!(damage(&events), [60; 3]);
        for slot in 0..3 {
            assert_eq!(
                state.teams[0].pets[slot].health,
                if slot == dead { 0 } else { 10000 }
            );
        }
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_blood_harvest_healing_tracks_critical_and_type_modified_actual_damage() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 8, [2457, 123, 110]);
    for (pet, health) in initial.teams[0].pets.iter_mut().zip([100, 200, 300]) {
        pet.health = health;
    }
    let (state, events) = initial
        .step(
            &catalog,
            [Action::Ability(2457), Action::Pass],
            &mut Fixed(0),
        )
        .unwrap();
    //Undead2 versus Flying8: strong1.5, critical1.5, variance0.9 =>floor121.
    assert_eq!(damage(&events), [121; 3]);
    assert!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Damage { .. }))
            .all(|event| matches!(event, Event::Damage { critical: true, .. }))
    );
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        [221, 321, 421]
    );
}

#[test]
fn pet_battle_blood_harvest_uses_caster_healing_done_and_each_allies_healing_taken() {
    let catalog = catalog();
    let mut initial = harvest_battle(&catalog);
    let dealt = catalog.state_id("Mod_HealingDealtPercent").unwrap();
    initial.teams[0].pets[0].states.insert(dealt, 25);
    initial.teams[0].pets[1].states.insert(dealt, 200);
    initial.teams[0].pets[2].states.insert(dealt, 200);
    initial.teams[0].pets[2]
        .states
        .insert(catalog.state_id("Mod_HealingTakenPercent").unwrap(), -50);
    let (state, events) = use_ability(&initial, &catalog, 2457);
    assert_eq!(damage(&events), [60; 3]);
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        [175, 275, 337]
    );
}

#[test]
fn pet_battle_blood_harvest_existing_end_of_action_policy_heals_after_last_enemy_faints() {
    let catalog = catalog();
    let mut initial = harvest_battle(&catalog);
    initial.teams[1].pets[0].health = 1;
    for slot in 1..3 {
        initial.teams[1].pets[slot].health = 0;
        initial.teams[1].pets[slot]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    }
    let (state, events) = use_ability(&initial, &catalog, 2457);
    assert_eq!(damage(&events), [1]);
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        [101, 200, 300]
    );
    assert_eq!(state.outcome, Some(Outcome::Winner(0)));
    let healed = events
        .iter()
        .position(|event| matches!(event, Event::Heal { amount: 1, .. }))
        .unwrap();
    let finished = events
        .iter()
        .position(|event| matches!(event, Event::Finished(Outcome::Winner(0))))
        .unwrap();
    assert!(healed < finished);
    //Final paired healing precedes Winner by existing action-completion policy;
    //neither this policy nor the paired-ally source-row inference is Retail capture.
}

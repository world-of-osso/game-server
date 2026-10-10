use super::*;

fn use_ability(b: &Battle, c: &Catalog, id: u32) -> (Battle, Vec<Event>) {
    b.step(c, [Action::Ability(id), Action::Pass], &mut Fixed(500000))
        .unwrap()
}

#[test]
fn pet_battle_cauterize_uses_half_actual_last_hit_without_power_scaling() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [173, 110, 123]);
    b.teams[0].pets[0].health = 8000;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Last_HitTaken").unwrap(), 84);
    let (b, events) = use_ability(&b, &c, 173);
    // Cauterize:20 points * (1+.05*100)=120, plus84*50%=42.
    assert_eq!(b.teams[0].pets[0].health, 8162);
    assert_eq!(
        events
            .iter()
            .filter_map(|e| match e {
                Event::Heal { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![120, 42]
    );
}

#[test]
fn pet_battle_comeback_requires_lower_health_for_additional_damage() {
    let c = catalog();
    for (health, expected) in [(10000, vec![90]), (9999, vec![90]), (9900, vec![90, 60])] {
        let mut b = battle(&c, 7, 7, [253, 110, 123]);
        b.teams[0].pets[0].health = health;
        let (_, e) = use_ability(&b, &c, 253);
        // The second row compares health AFTER the90-point first strike.
        assert_eq!(damage(&e), expected);
    }
}

#[test]
fn pet_battle_predatory_strike_quarter_health_boundary() {
    let c = catalog();
    for (health, expected) in [(2500, 180), (2499, 360)] {
        let mut b = battle(&c, 3, 7, [518, 110, 123]);
        b.teams[1].pets[0].health = health;
        let (_, e) = use_ability(&b, &c, 518);
        // Boost100 means double30 points below25%, not at25%.
        assert_eq!(damage(&e), vec![expected]);
    }
}

#[test]
fn pet_battle_nocturnal_strike_blind_guarantees_hit_but_not_targetability() {
    let c = catalog();
    let mut b = battle(&c, 3, 7, [517, 110, 123]);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    assert!(damage(&use_ability(&b, &c, 517).1).is_empty());
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Mechanic_IsBlind").unwrap(), 1);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Stat_Dodge").unwrap(), 100);
    assert_eq!(damage(&use_ability(&b, &c, 517).1), vec![240]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("untargettable").unwrap(), 1);
    assert!(damage(&use_ability(&b, &c, 517).1).is_empty());
}

#[test]
fn pet_battle_wind_up_charges_fires_and_charges_again() {
    let c = catalog();
    let b = battle(&c, 9, 0, [459, 110, 123]);
    let (b, e) = use_ability(&b, &c, 459);
    assert!(damage(&e).is_empty());
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_DamageDealtPercent").unwrap()),
        10
    );
    let (b, e) = use_ability(&b, &c, 459);
    // Charged hit:45*6*1.1=297; consumes its10% charge after firing.
    assert_eq!(damage(&e), vec![297]);
    assert!(b.auras.is_empty());
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_DamageDealtPercent").unwrap()),
        0
    );
    let (_, e) = use_ability(&b, &c, 459);
    assert!(damage(&e).is_empty());
}

#[test]
fn pet_battle_rebuild_heals_only_living_other_mechanical_allies() {
    let c = catalog();
    let mut b = battle(&c, 9, 7, [533, 110, 123]);
    b.teams[0].active = 1;
    for pet in &mut b.teams[0].pets {
        pet.health = 9000;
    }
    b.teams[0].pets[2].family = 8;
    let (b, _) = use_ability(&b, &c, 533);
    //24*6 on active;12*6 on other mechanical; aquatic bench is unaffected.
    assert_eq!(
        b.teams[0].pets.iter().map(|p| p.health).collect::<Vec<_>>(),
        vec![9072, 9144, 9000]
    );
    let mut b = b;
    b.teams[0].pets[0].health = 0;
    b.teams[0].pets[1].cooldowns.clear();
    assert_eq!(use_ability(&b, &c, 533).0.teams[0].pets[0].health, 0);
}

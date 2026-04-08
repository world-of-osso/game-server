use super::*;

fn test_species() -> PetSpecies {
    PetSpecies {
        id: 1,
        family: PetFamily::Beast,
        base_health: 100.0,
        base_power: 20.0,
        base_speed: 20.0,
    }
}

fn make_pet(health: u32, power: u32, speed: u32) -> BattlePet {
    BattlePet::new(PetStats {
        health,
        power,
        speed,
    })
}

fn test_ability(family: PetFamily, base_damage: u32) -> PetAbility {
    PetAbility {
        id: 1,
        name: "Test".into(),
        base_damage,
        family,
        cooldown: 0,
    }
}

#[path = "pet_battle_tests/abilities.rs"]
mod abilities;
#[path = "pet_battle_tests/battle.rs"]
mod battle;
#[path = "pet_battle_tests/capture.rs"]
mod capture;
#[path = "pet_battle_tests/integration.rs"]
mod integration;
#[path = "pet_battle_tests/journal.rs"]
mod journal;
#[path = "pet_battle_tests/stats.rs"]
mod stats;
#[path = "pet_battle_tests/xp.rs"]
mod xp;

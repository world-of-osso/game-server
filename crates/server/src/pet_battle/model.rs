use super::{Catalog, States, Stats};
use std::collections::BTreeMap;

/// Draw uniformly in [0, 1_000_000). Implementations own seed/replay persistence.
pub trait BattleRng {
    fn draw(&mut self) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PetRef {
    pub team: usize,
    pub slot: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Ability(u32),
    Swap(usize),
    Pass,
    Forfeit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Winner(usize),
    Draw,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Order([usize; 2]),
    Used {
        pet: PetRef,
        ability: u32,
        turn: u32,
    },
    Damage {
        caster: PetRef,
        target: PetRef,
        amount: i32,
        critical: bool,
        periodic: bool,
    },
    Heal {
        target: PetRef,
        amount: i32,
    },
    Miss {
        effect: u32,
        target: PetRef,
    },
    Immune {
        effect: u32,
        target: PetRef,
    },
    Skipped(PetRef),
    Swapped {
        team: usize,
        slot: usize,
    },
    AuraApplied {
        target: PetRef,
        ability: u32,
    },
    AuraRemoved {
        target: PetRef,
        ability: u32,
    },
    Revived(PetRef),
    Fainted(PetRef),
    Finished(Outcome),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pet {
    pub stats: Stats,
    pub health: i32,
    pub family: usize,
    pub abilities: [u32; 3],
    pub states: States,
    pub cooldowns: BTreeMap<u32, u32>,
    pub continuation: Option<(u32, u32)>,
    pub resurrection_used: bool,
    pub undead_expires: Option<u32>,
    pub dragon_bonus_round: Option<u32>,
    pub dealt_damage: bool,
    pub acted_round: u32,
}
impl Pet {
    /// Encounter/journal caller supplies validated family and equipped abilities.
    pub fn new(
        catalog: &Catalog,
        stats: Stats,
        family: usize,
        abilities: [u32; 3],
        mut states: States,
    ) -> Result<Self, String> {
        if family >= 10 || stats.health <= 0 || stats.power < 0 || stats.speed < 0 {
            return Err("invalid pet stats/family".into());
        }
        for ability in abilities {
            if !catalog.abilities.contains_key(&ability) {
                return Err(format!("missing equipped ability {ability}"));
            }
        }
        let passive = [
            "Passive_Humanoid",
            "Passive_Dragon",
            "Passive_Flying",
            "Passive_Undead",
            "Passive_Critter",
            "Passive_Magic",
            "Passive_Elemental",
            "Passive_Beast",
            "Passive_Aquatic",
            "Passive_Mechanical",
        ][family];
        states.insert(catalog.state_id(passive)?, 1);
        states
            .entry(catalog.state_id("Stat_CritChance")?)
            .or_insert(5);
        Ok(Self {
            stats,
            health: stats.health,
            family,
            abilities,
            states,
            cooldowns: BTreeMap::new(),
            continuation: None,
            resurrection_used: false,
            undead_expires: None,
            dragon_bonus_round: None,
            dealt_damage: false,
            acted_round: 0,
        })
    }
    pub fn state(&self, id: u32) -> i32 {
        self.states.get(&id).copied().unwrap_or(0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub pets: [Pet; 3],
    pub active: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aura {
    pub serial: u32,
    pub ability: u32,
    pub caster: PetRef,
    pub target: PetRef,
    pub remaining: i32,
    pub turn: u32,
    pub applied_round: u32,
    pub tick_first: bool,
    /// Property63 is a team-pad trigger (Wish); other admitted auras bind to a pet.
    pub team_bound: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battle {
    pub teams: [Team; 2],
    pub auras: Vec<Aura>,
    pub round: u32,
    pub outcome: Option<Outcome>,
    pub next_aura: u32,
}
impl Battle {
    pub fn new(teams: [Team; 2]) -> Result<Self, String> {
        if teams
            .iter()
            .any(|t| t.active >= 3 || t.pets[t.active].health <= 0)
        {
            return Err("invalid active pet".into());
        }
        Ok(Self {
            teams,
            auras: Vec::new(),
            round: 0,
            outcome: None,
            next_aura: 1,
        })
    }
    pub fn active(&self, team: usize) -> PetRef {
        PetRef {
            team,
            slot: self.teams[team].active,
        }
    }
    pub fn pet(&self, pet: PetRef) -> &Pet {
        &self.teams[pet.team].pets[pet.slot]
    }
    pub fn pet_mut(&mut self, pet: PetRef) -> &mut Pet {
        &mut self.teams[pet.team].pets[pet.slot]
    }
}

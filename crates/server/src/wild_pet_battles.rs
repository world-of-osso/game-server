//! Wild PvE decisions wrap the deterministic turn library; transport owns no rules.
use crate::pet_journal_state::{OwnedBattleStates, default_abilities};
use ::server::pet_battle::{
    Action, Battle, BattleRng, Catalog, Event, Outcome, Pet, States, Team, calculate_stats,
    validate_ability,
};
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::pet_battle::{MAX_PER_SPECIES, OwnedPet, PetJournal, PetQuality};
use shared::protocol::*;
use std::collections::{BTreeMap, BTreeSet};

const TRAP_HEALTH_PERCENT: i32 = 35;
const BASIC_TRAP_ABILITY: u32 = 427;
const MAX_LEVEL: u8 = 25;

#[derive(Clone)]
struct Species {
    creature: u32,
    name: String,
    display: u32,
    family: u8,
    icon: u32,
    wild: bool,
}
#[derive(Resource)]
struct WildCatalog {
    turns: Catalog,
    species: BTreeMap<u32, Species>,
    ability_art: BTreeMap<u32, (String, u32)>,
    next_level_xp: BTreeMap<u8, u32>,
    trap_chance: i32,
    trap_increment: i32,
}
impl WildCatalog {
    fn load(db: &Connection) -> Result<Self, String> {
        let mut statement = db.prepare("SELECT s.species_id,s.creature_id,s.name,s.display_id,m.PetTypeEnum,m.SourceTypeEnum,m.IconFileDataID FROM battle_pet_species s JOIN battle_pet_species_metadata m ON m.ID=s.species_id").map_err(|e| e.to_string())?;
        let species = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    Species {
                        creature: row.get(1)?,
                        name: row.get(2)?,
                        display: row.get(3)?,
                        family: row.get(4)?,
                        icon: row.get(6)?,
                        wild: row.get::<_, i32>(5)? == 4,
                    },
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<BTreeMap<_, _>>>()
            .map_err(|e| e.to_string())?;
        let mut statement = db
            .prepare("SELECT ID,Name_lang,IconFileDataID FROM battle_pet_ability")
            .map_err(|e| e.to_string())?;
        let ability_art = statement
            .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        let mut statement = db
            .prepare("SELECT level,next_level_xp FROM battle_pet_xp WHERE level BETWEEN 1 AND 25")
            .map_err(|e| e.to_string())?;
        let next_level_xp = statement
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?;
        let (trap_chance, trap_increment) = db.query_row("SELECT e.Param_0,e.Param_1 FROM battle_pet_ability_turn t JOIN battle_pet_ability_effect e ON e.BattlePetAbilityTurnID=t.ID WHERE t.BattlePetAbilityID=? AND e.BattlePetEffectPropertiesID=25", [BASIC_TRAP_ABILITY], |r| Ok((r.get(0)?,r.get(1)?))).map_err(|e| e.to_string())?;
        Ok(Self {
            turns: Catalog::load_world_db(db)?,
            species,
            ability_art,
            next_level_xp,
            trap_chance,
            trap_increment,
        })
    }
    fn create_pet(&self, identity: &Identity, states: &OwnedBattleStates) -> Result<Pet, String> {
        let species = self
            .species
            .get(&identity.species)
            .ok_or("unknown battle pet species")?;
        let stats = calculate_stats(
            &self.turns,
            identity.species,
            identity.breed.into(),
            identity.quality as u32,
            identity.level,
        )?;
        let (abilities, health) = match identity.guid {
            Some(id) => {
                let state = states
                    .get(&id)
                    .ok_or_else(|| format!("Owned battle state missing for pet {id}"))?;
                (state.selected_abilities, state.health)
            }
            None => (
                default_abilities(&self.turns, identity.species, identity.level),
                stats.health,
            ),
        };
        if !abilities
            .iter()
            .any(|id| *id != 0 && validate_ability(&self.turns, *id).is_ok())
        {
            return Err(format!(
                "Species {} has no supported unlocked ability at level {}",
                identity.species, identity.level
            ));
        }
        let mut pet = Pet::new(
            &self.turns,
            identity.species,
            identity.level,
            stats,
            species.family.into(),
            abilities,
            States::new(),
        )?;
        pet.health = health;
        pet.states
            .insert(self.turns.state_id("Is_Dead")?, i32::from(health <= 0));
        Ok(pet)
    }
}
#[derive(Clone)]
struct Identity {
    guid: Option<u64>,
    species: u32,
    level: u8,
    breed: u8,
    quality: PetQuality,
}
impl From<&OwnedPet> for Identity {
    fn from(pet: &OwnedPet) -> Self {
        Self {
            guid: Some(pet.id),
            species: pet.species_id,
            level: pet.level,
            breed: pet.breed_id,
            quality: pet.quality,
        }
    }
}
#[derive(Clone)]
struct Encounter {
    id: u64,
    creature: u64,
    battle: Battle,
    identities: [Vec<Identity>; 2],
    participated: BTreeSet<usize>,
    trap_failures: i32,
    captured: Option<Identity>,
    forfeited: bool,
    feedback: Vec<BattleCombatFeedback>,
}
fn build_team(
    catalog: &WildCatalog,
    pets: &[Identity],
    states: &OwnedBattleStates,
) -> Result<Team, String> {
    let first = pets.first().ok_or("equip a battle pet in the journal")?;
    let first_pet = catalog.create_pet(first, states)?;
    let mut team = Team {
        pets: std::array::from_fn(|_| first_pet.clone()),
        active: 0,
    };
    for (index, pet) in team.pets.iter_mut().enumerate() {
        if let Some(identity) = pets.get(index) {
            *pet = catalog.create_pet(identity, states)?;
        } else {
            pet.health = 0;
        }
    }
    team.active = team
        .pets
        .iter()
        .position(|pet| pet.health > 0)
        .ok_or("No living battle pet in the loadout")?;
    Ok(team)
}
impl Encounter {
    fn start(
        catalog: &WildCatalog,
        journal: &PetJournal,
        states: &OwnedBattleStates,
        species: u32,
        level: u8,
        creature: u64,
        id: u64,
    ) -> Result<Self, String> {
        if !catalog.species.get(&species).is_some_and(|s| s.wild) {
            return Err("target is not a wild battle pet".into());
        }
        let allies = journal
            .battle_slots
            .iter()
            .flatten()
            .map(|id| {
                journal
                    .get(*id)
                    .map(Identity::from)
                    .ok_or("loadout pet no longer owned".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let enemies = vec![Identity {
            guid: None,
            species,
            level,
            breed: 3,
            quality: PetQuality::Common,
        }];
        let teams = [
            build_team(catalog, &allies, states)?,
            build_team(catalog, &enemies, states)?,
        ];
        Ok(Self {
            id,
            creature,
            battle: Battle::new(teams)?,
            identities: [allies, enemies],
            participated: BTreeSet::from([0]),
            trap_failures: 0,
            captured: None,
            forfeited: false,
            feedback: vec![],
        })
    }
    /// Temporary maximum-health aura capacity does not survive leaving the battle.
    fn owned_health(&self, team: usize) -> Vec<(u64, i32)> {
        self.identities[team]
            .iter()
            .zip(&self.battle.teams[team].pets)
            .filter_map(|(identity, pet)| {
                identity
                    .guid
                    .map(|id| (id, pet.health.min(pet.base_health)))
            })
            .collect()
    }

    fn can_trap(&self, journal: &PetJournal) -> bool {
        let target = &self.battle.teams[1].pets[self.battle.teams[1].active];
        let identity = &self.identities[1][self.battle.teams[1].active];
        self.captured.is_none()
            && self.battle.teams[0].pets[self.battle.teams[0].active].health > 0
            && target.health > 0
            && target.health * 100 < target.stats.health * TRAP_HEALTH_PERCENT
            && journal.count_species(identity.species) < MAX_PER_SPECIES
            && journal.count() < shared::pet_battle::MAX_JOURNAL_SIZE
    }
    fn opponent_action(&self, catalog: &WildCatalog) -> Action {
        let team = &self.battle.teams[1];
        let pet = &team.pets[team.active];
        if pet.health <= 0 {
            return team
                .pets
                .iter()
                .position(|pet| pet.health > 0)
                .map_or(Action::Pass, Action::Swap);
        }
        if pet.continuation.is_some() {
            return Action::Pass;
        }
        pet.abilities
            .iter()
            .find(|id| {
                **id != 0
                    && pet.cooldowns.get(id).copied().unwrap_or(0) == 0
                    && validate_ability(&catalog.turns, **id).is_ok()
            })
            .map_or(Action::Pass, |id| Action::Ability(*id))
    }
    fn step(
        &self,
        catalog: &WildCatalog,
        journal: &PetJournal,
        decision: WildPetBattleAction,
        rng: &mut impl BattleRng,
    ) -> Result<(Self, Vec<String>), String> {
        if self.battle.outcome.is_some() {
            return Err("battle already finished".into());
        }
        let mut next = self.clone();
        next.feedback.clear();
        if decision == WildPetBattleAction::Trap {
            if !self.can_trap(journal) {
                return Err("trap requires enemy below 35% health and journal capacity".into());
            }
            let chance =
                (catalog.trap_chance + catalog.trap_increment * self.trap_failures).min(100);
            if rng.draw() < chance as u32 * 10_000 {
                next.captured = Some(self.identities[1][self.battle.teams[1].active].clone());
                next.battle.outcome = Some(Outcome::Winner(0));
                return Ok((next, vec!["Trap captured the wild pet".into()]));
            }
            next.trap_failures += 1;
        }
        let action = self.player_action(decision)?;
        let replacement = self.battle.teams[0].pets[self.battle.teams[0].active].health <= 0;
        let opponent = if replacement {
            Action::Pass
        } else {
            self.opponent_action(catalog)
        };
        let (battle, events) = self.battle.step(&catalog.turns, [action, opponent], rng)?;
        next.battle = battle;
        next.feedback = events.iter().filter_map(combat_feedback).collect();
        next.forfeited = decision == WildPetBattleAction::Forfeit;
        next.participated.insert(next.battle.teams[0].active);
        let mut text = events
            .iter()
            .filter_map(|event| format_combat_event(event, catalog))
            .collect::<Vec<_>>();
        if decision == WildPetBattleAction::Trap {
            text.insert(0, "Trap failed; next attempt has increased chance".into());
        }
        Ok((next, text))
    }
    fn player_action(&self, action: WildPetBattleAction) -> Result<Action, String> {
        match action {
            WildPetBattleAction::Ability(button) if (1..=3).contains(&button) => {
                let ability = self.battle.teams[0].pets[self.battle.teams[0].active].abilities
                    [usize::from(button - 1)];
                if ability == 0 {
                    return Err("ability slot locked".into());
                }
                Ok(Action::Ability(ability))
            }
            WildPetBattleAction::Swap(slot)
                if slot > 0 && usize::from(slot) <= self.identities[0].len() =>
            {
                Ok(Action::Swap(usize::from(slot - 1)))
            }
            WildPetBattleAction::Pass | WildPetBattleAction::Trap => Ok(Action::Pass),
            WildPetBattleAction::Forfeit => Ok(Action::Forfeit),
            _ => Err("invalid battle action slot".into()),
        }
    }
    fn apply_rewards(
        &self,
        catalog: &WildCatalog,
        journal: &mut PetJournal,
        captured_guid: Option<u64>,
    ) -> Result<Vec<BattlePetXpReward>, String> {
        if self.battle.outcome != Some(Outcome::Winner(0)) {
            return Ok(vec![]);
        }
        let mut updated = journal.clone();
        if let Some(captured) = &self.captured {
            let guid = captured_guid.ok_or("capture requires globally allocated pet GUID")?;
            updated
                .add_with_guid(
                    captured.species,
                    captured.level,
                    captured.quality,
                    captured.breed,
                    guid,
                )
                .map_err(|e| format!("capture journal: {e:?}"))?;
        }
        let survivors = self
            .participated
            .iter()
            .filter(|index| {
                self.battle.teams[0].pets[**index].health > 0
                    && self.identities[0][**index].level < MAX_LEVEL
            })
            .count() as u32;
        if survivors == 0 {
            *journal = updated;
            return Ok(vec![]);
        }
        let mut rewards = Vec::new();
        for index in &self.participated {
            if self.battle.teams[0].pets[*index].health <= 0 {
                continue;
            }
            let guid = self.identities[0][*index]
                .guid
                .expect("owned ally identity");
            let pet = updated
                .pets
                .iter_mut()
                .find(|p| p.id == guid)
                .ok_or("reward pet no longer owned")?;
            let gained = earned_xp(pet.level, self.identities[1][0].level) / survivors;
            if pet.level >= MAX_LEVEL || gained == 0 {
                continue;
            }
            pet.xp += gained;
            while pet.level < MAX_LEVEL {
                let threshold = catalog
                    .next_level_xp
                    .get(&pet.level)
                    .ok_or("missing pet XP game-table level")?;
                if pet.xp < *threshold {
                    break;
                }
                pet.xp -= threshold;
                pet.level += 1;
            }
            if pet.level == MAX_LEVEL {
                pet.xp = 0;
            }
            rewards.push(BattlePetXpReward {
                instance_id: guid,
                xp_gained: gained,
                level: pet.level,
                xp: pet.xp,
                next_level_xp: catalog.next_level_xp[&pet.level],
            });
        }
        *journal = updated;
        Ok(rewards)
    }
    fn snapshot(&self, catalog: &WildCatalog, journal: &PetJournal) -> WildPetBattleSnapshot {
        let teams = std::array::from_fn(|team| {
            self.identities[team]
                .iter()
                .enumerate()
                .map(|(slot, identity)| {
                    let pet = &self.battle.teams[team].pets[slot];
                    let species = &catalog.species[&identity.species];
                    let turn_locked = pet.state(
                        catalog
                            .turns
                            .state_id("turnLock")
                            .expect("loaded turnLock state"),
                    ) != 0;
                    let action_locked = turn_locked || pet.continuation.is_some();
                    let abilities = std::array::from_fn(|index| {
                        let id = pet.abilities[index];
                        let (name, icon) = if id == 0 {
                            ("Locked".into(), 0)
                        } else {
                            catalog.ability_art[&id].clone()
                        };
                        let cooldown = pet.cooldowns.get(&id).copied().unwrap_or(0);
                        BattleAbilitySnapshot {
                            id,
                            name,
                            icon,
                            family: if id == 0 {
                                0
                            } else {
                                catalog.turns.abilities[&id].family as u8
                            },
                            cooldown,
                            lockout: pet.lockouts[index],
                            usable: id != 0
                                && !action_locked
                                && cooldown == 0
                                && pet.lockouts[index] == 0
                                && validate_ability(&catalog.turns, id).is_ok(),
                        }
                    });
                    let auras = self
                        .battle
                        .auras
                        .iter()
                        .filter(|a| {
                            !a.weather
                                && !a.team_bound
                                && a.target.team == team
                                && a.target.slot == slot
                        })
                        .map(|a| aura_snapshot(a, catalog))
                        .collect();
                    BattlePetSnapshot {
                        instance_id: identity.guid,
                        species_id: identity.species,
                        name: species.name.clone(),
                        display_id: species.display,
                        icon: species.icon,
                        quality: identity.quality as u8,
                        xp: identity
                            .guid
                            .and_then(|id| journal.get(id))
                            .map_or(0, |pet| pet.xp),
                        next_level_xp: catalog.next_level_xp[&identity.level],
                        family: species.family,
                        level: identity.level,
                        health: pet.health,
                        max_health: pet.stats.health,
                        power: pet.stats.power,
                        speed: pet.stats.speed,
                        abilities,
                        auras,
                    }
                })
                .collect()
        });
        WildPetBattleSnapshot {
            battle_id: self.id,
            round: self.battle.round,
            teams,
            active: self.battle.teams.each_ref().map(|t| t.active as u8),
            wild_creature: self.creature,
            can_trap: self.can_trap(journal),
            turn_time_ms: 0,
            initial_selection_required: false,
            replacement_required: self.battle.teams[0].pets[self.battle.teams[0].active].health
                <= 0,
            feedback: self.feedback.clone(),
            weather: self
                .battle
                .auras
                .iter()
                .find(|a| a.weather)
                .map(|a| aura_snapshot(a, catalog)),
            team_auras: std::array::from_fn(|team| {
                self.battle
                    .auras
                    .iter()
                    .filter(|a| !a.weather && a.team_bound && a.target.team == team)
                    .map(|a| aura_snapshot(a, catalog))
                    .collect()
            }),
        }
    }
}
fn aura_snapshot(aura: &::server::pet_battle::Aura, catalog: &WildCatalog) -> BattleAuraSnapshot {
    let (name, icon) = &catalog.ability_art[&aura.ability];
    BattleAuraSnapshot {
        ability_id: aura.ability,
        name: name.clone(),
        icon: *icon,
        rounds_remaining: aura.remaining,
        // Admitted beneficial effects attach to the caster's team; hostile effects to the enemy.
        is_buff: aura.weather || aura.caster.team == aura.target.team,
    }
}
fn earned_xp(level: u8, opponent: u8) -> u32 {
    // Retail experiment author Jere, Blizzard forum1479190 posts6/8 (Jan2023):
    // base=(own+9)*(enemy-own+5), wild multiplier1.5, no old Legion +2 cap.
    let difference = (i32::from(opponent) - i32::from(level) + 5).max(0) as u32;
    (u32::from(level) + 9) * difference * 3 / 2
}
fn combat_feedback(event: &Event) -> Option<BattleCombatFeedback> {
    let (target, text, healing) = match event {
        Event::Damage { target, amount, .. } => (target, amount.to_string(), false),
        Event::Heal { target, amount } => (target, format!("+{amount}"), true),
        Event::Miss { target, .. } => (target, "Miss".into(), false),
        Event::Immune { target, .. } => (target, "Immune".into(), false),
        _ => return None,
    };
    Some(BattleCombatFeedback {
        team: target.team as u8,
        slot: target.slot as u8,
        text,
        healing,
    })
}
fn format_combat_event(event: &Event, catalog: &WildCatalog) -> Option<String> {
    Some(match event {
        Event::Used { ability, .. } => format!("{} used", catalog.ability_art[ability].0),
        Event::Damage {
            amount, critical, ..
        } => format!(
            "{amount} damage{}",
            if *critical { " (critical)" } else { "" }
        ),
        Event::Heal { amount, .. } => format!("{amount} healing"),
        Event::Miss { .. } => "Miss".into(),
        Event::Fainted(_) => "Pet fainted".into(),
        Event::Swapped { team, slot } => format!("Team {} swapped to pet {}", team + 1, slot + 1),
        Event::AuraApplied { ability, .. } => format!("{} applied", catalog.ability_art[ability].0),
        Event::AuraRemoved { ability, .. } => format!("{} expired", catalog.ability_art[ability].0),
        Event::Immune { .. } => "Immune".into(),
        _ => return None,
    })
}

use crate::auth::{Authenticated, InWorld};
use crate::battle_pets::{
    AccountPetJournal, load_or_initialize_pet_states, persist_battle_progress,
    refresh_catalog_snapshot, save_journal, send_pet_details, send_pet_update,
};
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;
use lightyear::prelude::{MessageReceiver, MessageSender};
use rand::{Rng, SeedableRng};
use shared::components::{Health, Npc, Position, UnitLevel};

#[derive(Clone)]
struct RuntimeRng(rand::rngs::StdRng);
impl BattleRng for RuntimeRng {
    fn draw(&mut self) -> u32 {
        self.0.gen_range(0..1_000_000)
    }
}
#[derive(Component, Clone)]
struct ActiveEncounter {
    encounter: Encounter,
    rng: RuntimeRng,
    connection: Entity,
}
#[derive(Component)]
struct WildPetClaim(Entity);
#[derive(Resource, Default)]
struct BattleIds(u64);

/// Account journal choices cannot change through an alt connection during a queued/active battle.
pub(crate) fn journal_locked(world: &mut World, owner: Entity) -> bool {
    let Some(account) = world
        .get::<AccountPetJournal>(owner)
        .map(|state| state.account_id)
    else {
        return false;
    };
    let mut journals = world.query::<(Entity, &AccountPetJournal)>();
    journals.iter(world).any(|(entity, journal)| {
        journal.account_id == account
            && (world.get::<ActiveEncounter>(entity).is_some() || pvp::busy(world, entity))
    })
}

pub(crate) fn load_wild_catalog(mut commands: Commands) {
    let db = Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open wild pet world.db");
    let catalog = WildCatalog::load(&db).expect(
        "load wild tables: scripts/import-pet-battle-tables.py --wild-xp <local game table>",
    );
    info!(
        "Loaded {} wild species from world.db",
        catalog.species.values().filter(|s| s.wild).count()
    );
    commands.insert_resource(catalog);
    commands.init_resource::<BattleIds>();
}
pub(crate) fn receive_wild_battle_requests(world: &mut World) {
    pvp::remove_abandoned(world);
    pvp::receive_queue(world);
    receive_loadouts(world);
    receive_starts(world);
    receive_actions(world);
}
fn send_update(world: &mut World, connection: Entity, update: WildPetBattleUpdate) {
    if let Some(mut sender) = world.get_mut::<MessageSender<WildPetBattleUpdate>>(connection) {
        sender.send::<CollectionChannel>(update);
    }
}
fn receive_loadouts(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<SetBattlePetLoadout>,
    ), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|r| (connection, link.0, r.slots))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner, slots) in requests {
        let result = set_loadout(world, owner, slots);
        send_pet_update(world, connection, owner, result.err());
    }
}
fn set_loadout(world: &mut World, owner: Entity, slots: [Option<u64>; 3]) -> Result<(), String> {
    if journal_locked(world, owner) {
        return Err("cannot change loadout while queued or during a battle".into());
    }
    let mut state = world
        .get_mut::<AccountPetJournal>(owner)
        .ok_or("account journal unavailable")?;
    state
        .journal
        .set_battle_slots(slots)
        .map_err(|e| format!("loadout: {e:?}"))?;
    let account = state.account_id;
    let journal = state.journal.clone();
    save_journal(&world.resource::<GameDatabase>().db, account, &journal);
    Ok(())
}
fn receive_starts(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<StartWildPetBattle>,
    ), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|r| (connection, link.0, r.creature))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner, creature) in requests {
        if let Err(error) = start_encounter(world, connection, owner, creature) {
            send_update(world, connection, WildPetBattleUpdate::Rejected(error));
        }
    }
}
fn start_encounter(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    bits: u64,
) -> Result<(), String> {
    if world.get::<ActiveEncounter>(owner).is_some() || pvp::busy(world, owner) {
        return Err("already queued or in a pet battle".into());
    }
    let creature = Entity::try_from_bits(bits).ok_or("invalid wild creature")?;
    if world.get::<WildPetClaim>(creature).is_some() {
        return Err("wild pet already in battle".into());
    }
    let npc = world
        .get::<Npc>(creature)
        .ok_or("wild creature not found")?;
    let catalog = world.resource::<WildCatalog>();
    let species = catalog
        .species
        .iter()
        .find(|(_, s)| s.wild && s.creature == npc.template_id)
        .map(|(id, _)| *id)
        .ok_or("not a wild battle pet")?;
    let owner_map = world
        .get::<crate::maps::InMap>(owner)
        .ok_or("player map unavailable")?;
    if world.get::<crate::maps::InMap>(creature) != Some(owner_map) {
        return Err("wild pet is on another map copy".into());
    }
    let a = world
        .get::<Position>(owner)
        .ok_or("player position unavailable")?;
    let b = world
        .get::<Position>(creature)
        .ok_or("wild pet position unavailable")?;
    let distance_squared = (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2);
    if distance_squared > 25.0 {
        return Err("move within five yards of the wild pet".into());
    }
    if world.get::<Health>(owner).is_none_or(|h| h.current <= 0.0)
        || world
            .get::<Health>(creature)
            .is_none_or(|h| h.current <= 0.0)
    {
        return Err("player and wild pet must be alive".into());
    }
    let state = world
        .get::<AccountPetJournal>(owner)
        .ok_or("account journal unavailable")?;
    let account = state.account_id;
    let journal = state.journal.clone();
    let mut other = world.query::<(&AccountPetJournal, &ActiveEncounter)>();
    if other.iter(world).any(|(j, _)| j.account_id == account) {
        return Err("account already has an active pet battle".into());
    }
    let id = {
        let mut ids = world.resource_mut::<BattleIds>();
        ids.0 += 1;
        ids.0
    };
    let level = world
        .get::<UnitLevel>(creature)
        .map_or(1, |level| level.0.clamp(1, 25) as u8);
    let states = load_or_initialize_pet_states(world, owner)?;
    let catalog = world.resource::<WildCatalog>();
    let encounter = Encounter::start(catalog, &journal, &states, species, level, bits, id)?;
    let snapshot = encounter.snapshot(catalog, &journal);
    info!(
        "Wild pet battle {id} started: account {account}, species {species}, creature {bits}, level {level}"
    );
    world.entity_mut(owner).insert(ActiveEncounter {
        encounter,
        rng: RuntimeRng(rand::rngs::StdRng::from_entropy()),
        connection,
    });
    world.entity_mut(creature).insert(WildPetClaim(owner));
    send_update(world, connection, WildPetBattleUpdate::Start(snapshot));
    send_pet_details(world, connection, owner, None);
    Ok(())
}
fn receive_actions(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<WildPetBattleActionRequest>,
    ), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|r| (connection, link.0, r))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner, request) in requests {
        if let Err(error) = advance_encounter(world, owner, request) {
            send_update(world, connection, WildPetBattleUpdate::Rejected(error));
        }
    }
}
fn advance_encounter(
    world: &mut World,
    owner: Entity,
    request: WildPetBattleActionRequest,
) -> Result<(), String> {
    if pvp::is_in_battle(world, owner) {
        return pvp::advance(world, owner, request);
    }
    let active = world
        .get::<ActiveEncounter>(owner)
        .ok_or("no active pet battle")?
        .clone();
    if request.battle_id != active.encounter.id || request.round != active.encounter.battle.round {
        return Err("stale battle or round".into());
    }
    let state = world
        .get::<AccountPetJournal>(owner)
        .ok_or("account journal unavailable")?;
    let mut journal = state.journal.clone();
    let account = state.account_id;
    let mut rng = active.rng;
    let (encounter, text) = active.encounter.step(
        world.resource::<WildCatalog>(),
        &journal,
        request.action,
        &mut rng,
    )?;
    if encounter.battle.outcome.is_none() {
        persist_battle_progress(world, owner, &journal, &encounter.owned_health(0))?;
        let snapshot = encounter.snapshot(world.resource::<WildCatalog>(), &journal);
        world.entity_mut(owner).insert(ActiveEncounter {
            encounter,
            rng,
            connection: active.connection,
        });
        send_update(
            world,
            active.connection,
            WildPetBattleUpdate::Round {
                state: snapshot,
                combat_text: text,
            },
        );
        return Ok(());
    }
    let terminal_state = encounter.snapshot(world.resource::<WildCatalog>(), &journal);
    let guid = encounter.captured.as_ref().map(|_| {
        crate::persistence::next_id(&world.resource::<GameDatabase>().db, "battle_pet_instance")
    });
    let before = journal.clone();
    let rewards = encounter.apply_rewards(world.resource::<WildCatalog>(), &mut journal, guid)?;
    let mut health = encounter.owned_health(0);
    if let Some(guid) = guid {
        let captured = &encounter.battle.teams[1].pets[encounter.battle.teams[1].active];
        health.push((guid, captured.health.min(captured.base_health)));
    }
    persist_battle_progress(world, owner, &journal, &health)?;
    send_update(
        world,
        active.connection,
        WildPetBattleUpdate::State(terminal_state),
    );
    crate::achievement::record_pet_level_changes(world, owner, &before, &journal);
    let outcome = if encounter.forfeited {
        WildPetBattleOutcome::Forfeited
    } else {
        match encounter.battle.outcome.expect("terminal battle") {
            Outcome::Winner(0) => WildPetBattleOutcome::Won,
            Outcome::Winner(_) => WildPetBattleOutcome::Lost,
            Outcome::Draw => WildPetBattleOutcome::Draw,
        }
    };
    if outcome == WildPetBattleOutcome::Won {
        crate::achievement::record_pet_battle_win(world, owner);
    }
    if let Some(captured) = guid.and(encounter.captured.as_ref()) {
        crate::achievement::record_pet_capture(world, owner);
        crate::achievement::record_learned_pet(world, owner, captured.species);
    }
    info!(
        "Wild pet battle {} ended: account {}, outcome {:?}, captured {:?}, rewards {:?}",
        encounter.id, account, outcome, guid, rewards
    );
    world.entity_mut(owner).remove::<ActiveEncounter>();
    refresh_catalog_snapshot(world, owner);
    send_pet_update(world, active.connection, owner, None);
    release_wild_pet(world, &encounter, outcome == WildPetBattleOutcome::Won);
    send_update(
        world,
        active.connection,
        WildPetBattleUpdate::End {
            battle_id: encounter.id,
            outcome,
            rewards,
            captured_pet_id: guid,
            combat_text: text,
        },
    );
    Ok(())
}
fn release_wild_pet(world: &mut World, encounter: &Encounter, defeated: bool) {
    let Some(creature) = Entity::try_from_bits(encounter.creature) else {
        return;
    };
    if world.get_entity(creature).is_err() {
        return;
    }
    world.entity_mut(creature).remove::<WildPetClaim>();
    if defeated {
        crate::combat::despawn_creature(world, creature);
    }
}
pub(crate) fn remove_abandoned_battles(world: &mut World) {
    let mut query = world.query::<(Entity, &ActiveEncounter)>();
    let abandoned: Vec<_> = query
        .iter(world)
        .filter(|(_, active)| world.get::<InWorld>(active.connection).is_none())
        .map(|(owner, active)| (owner, active.encounter.clone()))
        .collect();
    for (owner, encounter) in abandoned {
        world.entity_mut(owner).remove::<ActiveEncounter>();
        release_wild_pet(world, &encounter, false);
    }
    let mut claims = world.query::<(Entity, &WildPetClaim)>();
    let orphaned: Vec<_> = claims
        .iter(world)
        .filter(|(_, claim)| world.get::<ActiveEncounter>(claim.0).is_none())
        .map(|(entity, _)| entity)
        .collect();
    for entity in orphaned {
        world.entity_mut(entity).remove::<WildPetClaim>();
    }
}

#[path = "pvp_pet_battles.rs"]
mod pvp;

#[cfg(test)]
#[path = "wild_pet_battles_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "wild_pet_achievement_tests.rs"]
mod achievement_tests;

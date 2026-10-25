use rusqlite::{Connection, Row};
use std::collections::{BTreeMap, BTreeSet};

pub type States = BTreeMap<u32, i32>;

#[derive(Debug, Clone)]
pub struct UnresolvedState {
    pub table: String,
    pub owner: u32,
    pub state: u32,
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct Ability {
    pub id: u32,
    /// DB2 families are zero based, unlike the UI API.
    pub family: i32,
    pub cooldown: u32,
    pub description: String,
}
#[derive(Debug, Clone)]
pub struct Turn {
    pub id: u32,
    pub ability: u32,
    pub order: u32,
    pub kind: u32,
    pub event: u32,
}
#[derive(Debug, Clone)]
pub struct Effect {
    pub id: u32,
    pub turn: u32,
    pub order: u32,
    pub property: u32,
    pub aura: u32,
    pub params: [i32; 6],
}
#[derive(Debug, Clone)]
pub struct SpeciesAbility {
    pub species: u32,
    pub ability: u32,
    pub level: u8,
    pub slot: i32,
}
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub abilities: BTreeMap<u32, Ability>,
    pub turns: BTreeMap<u32, Turn>,
    pub effects: BTreeMap<u32, Effect>,
    pub states: BTreeMap<u32, String>,
    pub unresolved_states: Vec<UnresolvedState>,
    pub species_states: BTreeMap<u32, States>,
    pub breed_states: BTreeMap<u32, States>,
    pub ability_states: BTreeMap<u32, States>,
    pub qualities: BTreeMap<u32, f64>,
    pub species_abilities: Vec<SpeciesAbility>,
    pub property_labels: BTreeMap<u32, [String; 6]>,
}

fn query_rows<T>(
    db: &Connection,
    sql: &str,
    decode: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    let mut statement = db.prepare(sql).map_err(|e| format!("{sql}: {e}"))?;
    statement
        .query_map([], decode)
        .map_err(|e| format!("{sql}: {e}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("{sql}: {e}"))
}
fn index_unique<T>(rows: Vec<(u32, T)>) -> Result<BTreeMap<u32, T>, String> {
    let mut result = BTreeMap::new();
    for (id, row) in rows {
        if result.insert(id, row).is_some() {
            return Err(format!("duplicate DB2 ID {id}"));
        }
    }
    Ok(result)
}
fn load_states(
    db: &Connection,
    table: &str,
    owner: &str,
    known: &BTreeMap<u32, String>,
    unresolved: &mut Vec<UnresolvedState>,
) -> Result<BTreeMap<u32, States>, String> {
    let rows = query_rows(
        db,
        &format!("SELECT {owner}, BattlePetStateID, Value FROM {table} ORDER BY ID"),
        |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, i32>(2)?,
            ))
        },
    )?;
    let mut result: BTreeMap<u32, States> = BTreeMap::new();
    for (owner, state, value) in rows {
        if !known.contains_key(&state) {
            unresolved.push(UnresolvedState {
                table: table.into(),
                owner,
                state,
                value,
            });
            continue;
        }
        if let Some(previous) = result.entry(owner).or_default().insert(state, value) {
            if previous != value {
                return Err(format!("{table}: conflicting owner {owner} state {state}"));
            }
        }
    }
    Ok(result)
}
impl Catalog {
    /// Strict SQLite boundary. No CSV/runtime fallback; schemas documented in pet-battle-engine.md.
    pub fn load_world_db(db: &Connection) -> Result<Self, String> {
        let abilities = index_unique(query_rows(
            db,
            "SELECT ID, PetTypeEnum, Cooldown, Description_lang FROM battle_pet_ability ORDER BY ID",
            |r| {
                let id = r.get(0)?;
                Ok((
                    id,
                    Ability {
                        id,
                        family: r.get(1)?,
                        cooldown: r.get(2)?,
                        description: r.get(3)?,
                    },
                ))
            },
        )?)?;
        let turns = index_unique(query_rows(
            db,
            "SELECT ID, BattlePetAbilityID, OrderIndex, TurnTypeEnum, EventTypeEnum FROM battle_pet_ability_turn ORDER BY ID",
            |r| {
                let id = r.get(0)?;
                Ok((
                    id,
                    Turn {
                        id,
                        ability: r.get(1)?,
                        order: r.get(2)?,
                        kind: r.get(3)?,
                        event: r.get(4)?,
                    },
                ))
            },
        )?)?;
        let effects = index_unique(query_rows(
            db,
            "SELECT ID, BattlePetAbilityTurnID, OrderIndex, BattlePetEffectPropertiesID, AuraBattlePetAbilityID, Param_0, Param_1, Param_2, Param_3, Param_4, Param_5 FROM battle_pet_ability_effect ORDER BY ID",
            |r| {
                let id = r.get(0)?;
                Ok((
                    id,
                    Effect {
                        id,
                        turn: r.get(1)?,
                        order: r.get(2)?,
                        property: r.get(3)?,
                        aura: r.get(4)?,
                        params: [
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                            r.get(9)?,
                            r.get(10)?,
                        ],
                    },
                ))
            },
        )?)?;
        let mut catalog = Self {
            abilities,
            turns,
            effects,
            ..Self::default()
        };
        catalog.load_metadata(db)?;
        catalog.validate()?;
        Ok(catalog)
    }
    fn load_metadata(&mut self, db: &Connection) -> Result<(), String> {
        self.states = index_unique(query_rows(
            db,
            "SELECT ID, LuaName FROM battle_pet_state ORDER BY ID",
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)?;
        self.property_labels = index_unique(query_rows(
            db,
            "SELECT ID, ParamLabel_0, ParamLabel_1, ParamLabel_2, ParamLabel_3, ParamLabel_4, ParamLabel_5 FROM battle_pet_effect_properties ORDER BY ID",
            |r| {
                Ok((
                    r.get(0)?,
                    [
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ],
                ))
            },
        )?)?;
        self.qualities = index_unique(query_rows(
            db,
            "SELECT QualityEnum, StateMultiplier FROM battle_pet_breed_quality ORDER BY ID",
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)?;
        self.species_states = load_states(
            db,
            "battle_pet_species_state",
            "BattlePetSpeciesID",
            &self.states,
            &mut self.unresolved_states,
        )?;
        self.breed_states = load_states(
            db,
            "battle_pet_breed_state",
            "BattlePetBreedID",
            &self.states,
            &mut self.unresolved_states,
        )?;
        self.ability_states = load_states(
            db,
            "battle_pet_ability_state",
            "BattlePetAbilityID",
            &self.states,
            &mut self.unresolved_states,
        )?;
        self.species_abilities = query_rows(
            db,
            "SELECT BattlePetSpeciesID, BattlePetAbilityID, RequiredLevel, SlotEnum FROM battle_pet_species_x_ability ORDER BY ID",
            |r| {
                Ok(SpeciesAbility {
                    species: r.get(0)?,
                    ability: r.get(1)?,
                    level: r.get(2)?,
                    slot: r.get(3)?,
                })
            },
        )?;
        Ok(())
    }
    fn validate(&self) -> Result<(), String> {
        for turn in self.turns.values() {
            if !self.abilities.contains_key(&turn.ability) {
                return Err(format!(
                    "turn {}: missing ability {}",
                    turn.id, turn.ability
                ));
            }
        }
        for effect in self.effects.values() {
            if !self.turns.contains_key(&effect.turn) {
                return Err(format!(
                    "effect {}: missing turn {}",
                    effect.id, effect.turn
                ));
            }
            if effect.aura != 0 && !self.abilities.contains_key(&effect.aura) {
                return Err(format!(
                    "effect {}: missing aura {}",
                    effect.id, effect.aura
                ));
            }
        }
        for group in [
            &self.species_states,
            &self.breed_states,
            &self.ability_states,
        ] {
            for states in group.values() {
                for state in states.keys() {
                    if !self.states.contains_key(state) {
                        return Err(format!("missing state {state}"));
                    }
                }
            }
        }
        if self.qualities.values().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("invalid quality multiplier".into());
        }
        Ok(())
    }
    pub fn state_id(&self, name: &str) -> Result<u32, String> {
        self.states
            .iter()
            .find(|(_, value)| value.as_str() == name)
            .map(|(id, _)| *id)
            .ok_or_else(|| format!("missing BattlePetState {name}"))
    }
    pub fn ability_turns(&self, ability: u32) -> Vec<&Turn> {
        let mut turns: Vec<_> = self
            .turns
            .values()
            .filter(|t| t.ability == ability)
            .collect();
        turns.sort_by_key(|t| (t.order, t.id));
        turns
    }
    pub fn turn_effects(&self, turn: u32) -> Vec<&Effect> {
        let mut effects: Vec<_> = self.effects.values().filter(|e| e.turn == turn).collect();
        effects.sort_by_key(|e| (e.order, e.id));
        effects
    }
    pub fn property_counts(&self) -> BTreeMap<u32, usize> {
        let mut counts = BTreeMap::new();
        for effect in self.effects.values() {
            *counts.entry(effect.property).or_default() += 1;
        }
        counts
    }
    pub fn available_abilities(&self, species: u32, level: u8) -> BTreeSet<u32> {
        self.species_abilities
            .iter()
            .filter(|a| a.species == species && a.level <= level && a.slot >= 0)
            .map(|a| a.ability)
            .collect()
    }
}

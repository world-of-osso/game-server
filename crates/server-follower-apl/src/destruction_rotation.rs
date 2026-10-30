//! Literal267 adapter; native Warlock power and damage remain authoritative.
use crate::literal_rotation::{
    Driver, default_pet::ConfiguredPet, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::Value;
use server::followers::parity_profile::Profile;
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub mod bindings;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub pet: Option<ConfiguredPet>,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if profile.spec_id != 267 {
            return Err("literal Destruction requires spec267".into());
        }
        let mut bindings = bindings::build();
        let pet = ConfiguredPet::read(world, actor, profile)?;
        if let Some(pet) = &pet {
            pet.bind(&mut bindings)?;
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            pet,
            encounter,
        };
        Ok(rotation)
    }

    pub fn read_prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<Prepared, String> {
        let pet_action = self
            .pet
            .as_ref()
            .map(|pet| pet.read_action(world, actor))
            .transpose()?
            .flatten();
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions().chain(pet_action),
            self.driver.snapshot.is_none(),
        )
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<bool, String> {
        let prepared = self.read_prepared(world, actor, targets)?;
        let facts = DestructionFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            primary: targets.first().map(|target| target.to_bits()),
        };
        let list = if precombat { "precombat" } else { "default" };
        let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }
}

// SimC76f8118 apl/warlock.cpp Destruction: execute every exported row unchanged.
pub fn read_program(profile: &Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(Value::as_array)
        .ok_or("267 profile lacks literal actions")?;
    let rows = actions
        .iter()
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

pub fn parse_row(row: &Value) -> Result<(&str, &str), String> {
    let row = row.as_str().ok_or("267 action row must be text")?;
    let (head, literal) = row
        .split_once('=')
        .ok_or("267 action row lacks assignment")?;
    let head = head.trim_end_matches('+');
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("267 row is not an action list")?
    };
    Ok((list, literal.trim_start_matches('/')))
}

pub struct DestructionFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    primary: Option<u64>,
}

impl Facts for DestructionFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "demonic_art" {
            // SimC76f8118 sc_warlock.cpp876–881: any actual Demonic Art buff.
            for aura in [
                "demonic_art_overlord",
                "demonic_art_mother_of_chaos",
                "demonic_art_pit_lord",
            ] {
                if self.native.value(&format!("buff.{aura}.up"))? != 0.0 {
                    return Ok(1.0);
                }
            }
            return Ok(0.0);
        }
        if let Some(alias) = name.strip_prefix("dot_refreshable_count.") {
            // SimC76f8118 sc_warlock.cpp1002–1053: count refreshable actual recipients.
            let mut action = Action {
                name: alias.into(),
                source_row: None,
                target: None,
                options: Default::default(),
            };
            let mut count = 0.0;
            for target in self.native.target_ids(&action)? {
                action.target = Some(target);
                count += self
                    .native
                    .value_for(&action, &format!("dot.{alias}.refreshable"))?;
            }
            return Ok(count);
        }
        if name.starts_with("trinket.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for DestructionFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        match action.name.as_str() {
            "infernal_bolt" => Ok(self.native.value("buff.infernal_bolt.up")? != 0.0),
            "ruination" => Ok(self.native.value("buff.ruination.up")? != 0.0),
            "immolate" => Ok(self.native.value("talent.wither")? == 0.0),
            _ => Ok(true),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "refreshable" {
            return self
                .native
                .value_for(action, &format!("dot.{}.refreshable", action.name));
        }
        if name == "self.target" || name == "target" {
            // SimC76f8118 player.cpp self.target/target compare target identities.
            let target = if name == "self.target" {
                self.primary
            } else {
                action.target.or(self.primary)
            };
            return target
                .map(|target| target as f64)
                .ok_or("267 action has no target identity".into());
        }
        if name == "demonic_art"
            || name.starts_with("dot_refreshable_count.")
            || name.starts_with("trinket.")
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        Ok(if self.step(world, actor, targets, true)? {
            crate::literal_rotation::PrecombatPoll::Waiting
        } else {
            crate::literal_rotation::PrecombatPoll::Finished
        })
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: server::simc_apl::Action,
        execution: crate::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        // SimC76f8118 pet.cpp67–68/149: the actual pet follows its owner's target.
        crate::pet_control::order_companion_pet(world, actor, targets.first().copied());
        self.step(world, actor, targets, false).map(|_| ())
    }
}

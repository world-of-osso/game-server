//! Literal265 profile rows over the shared native APL runtime.
pub mod bindings;
pub mod facts;
use crate::literal_rotation::{
    Driver, default_pet::ConfiguredPet, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use serde_json::Value;
use server::followers::parity_profile::Profile;
use server::simc_apl::Program;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub pet: Option<ConfiguredPet>,
    pub encounter: Encounter,
    pub previous_action: Option<String>,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if profile.spec_id != 265 {
            return Err("literal Affliction requires spec265".into());
        }
        let mut bindings = bindings::build();
        let pet = ConfiguredPet::read(world, actor, profile)?;
        if let Some(pet) = &pet {
            pet.bind(&mut bindings)?;
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        // Pinned player.cpp1018–1020 reaction defaults, before native acquisition.
        crate::simc_apl_native::reaction::configure(
            world,
            actor,
            crate::simc_apl_native::reaction::ReactionSettings {
                mean: 0.25,
                stddev: 0.05,
                offset: 0.1,
                nu: 0.15,
                maximum: 1.4,
            },
            &[264571, 1260269],
        )?;
        Ok(Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            pet,
            encounter,
            previous_action: None,
        })
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<bool, String> {
        if !precombat {
            self.advance_channel(world, actor, targets)?;
        }
        let pet_action = self
            .pet
            .as_ref()
            .map(|pet| pet.read_action(world, actor))
            .transpose()?
            .flatten();
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions().chain(pet_action),
            self.driver.snapshot.is_none(),
        )?;
        let facts = facts::AfflictionFacts::read(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
            self.previous_action.as_deref(),
        );
        let list = if precombat { "precombat" } else { "default" };
        let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        let name = action.name.clone();
        self.driver.execute(world, actor, action, execution)?;
        self.previous_action = Some(name);
        Ok(true)
    }

    pub fn advance_channel(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let prepared = Prepared::read_after_channel(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
        )?;
        let facts = facts::AfflictionFacts::read(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
            self.previous_action.as_deref(),
        );
        let decision =
            self.driver
                .channel_decision(world, actor, &facts, &prepared, &self.equipment)?;
        if let Some(decision) = decision {
            self.driver
                .execute_channel_decision(world, actor, decision)?;
        }
        Ok(())
    }
}

// SimC76f8118 apl/warlock.cpp60–197: preserve every exported row unchanged.
pub fn read_program(profile: &Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(Value::as_array)
        .ok_or("265 profile lacks literal actions")?;
    let rows = actions
        .iter()
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

pub fn parse_row(row: &Value) -> Result<(&str, &str), String> {
    let row = row.as_str().ok_or("265 action row must be text")?;
    let (head, literal) = row
        .split_once('=')
        .ok_or("265 action row lacks assignment")?;
    let head = head.trim_end_matches('+');
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("265 row is not an action list")?
    };
    Ok((list, literal.trim_start_matches('/')))
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
        self.step(world, actor, targets, false).map(|_| ())
    }
}

//! Literal265 profile rows over the shared native APL runtime.
mod bindings;
mod facts;
use super::inputs::Profile;
use super::literal_rotation::{
    self, Driver, default_pet::ConfiguredPet, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    pet: Option<ConfiguredPet>,
    encounter: Encounter,
    previous_action: Option<String>,
}

impl Rotation {
    fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if profile.spec_id != 265 || !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Affliction requires spec265 and an explicit fixture".into());
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
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            pet,
            encounter,
            previous_action: None,
        })
    }

    fn step(
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

    fn advance_channel(
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

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter)?;
    if let Some(pet) = ConfiguredPet::read(run.app.world(), run.actor, &run.profile)? {
        super::preparation::wait_for_configured_pet_action(run, &pet)?;
    }
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("affliction_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_AFFLICTION_76f8118"
    }
    fn report(&self) -> Value {
        json!({"driver": self.label(), "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false)?;
        Ok(())
    }
}

// SimC76f8118 apl/warlock.cpp60–197: preserve every exported row unchanged.
fn read_program(profile: &Profile) -> Result<Program, String> {
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

fn parse_row(row: &Value) -> Result<(&str, &str), String> {
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

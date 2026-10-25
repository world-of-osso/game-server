//! Thin Devastation adapter over the shared literal APL scheduler and native casts.
mod bindings;
#[cfg(test)]
mod channel_tests;
#[cfg(test)]
mod fact_tests;
mod facts;
mod source;
use super::literal_rotation::{Driver, PrecombatPoll, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::DevFacts;
use serde_json::{Value, json};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: Driver::new(source::load()?),
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn precombat_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = self.prepare(world, actor, targets)?;
        let facts = DevFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn execute_precombat(
        &mut self,
        run: &mut super::Run,
        action: server::simc_apl::Action,
        execution: super::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        super::preparation::execute_precombat_plan(run, &mut self.driver, action, execution)
    }

    fn prepare(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<Prepared, String> {
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            self.driver.snapshot.is_none(),
        )
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("fixture-only Devastation driver")
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let facts = DevFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        if self.driver.should_cancel(world, actor, &facts)? {
            crate::spell_cast::cancel_cast(world, actor);
        }
        if self.drive_channel(world, actor, targets)? {
            return Ok(());
        }
        let prepared = self.prepare(world, actor, targets)?;
        let facts = DevFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn drive_channel(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String> {
        if world.get::<shared::casting::CastState>(actor).is_none() {
            return Ok(false);
        }
        let prepared = Prepared::read_after_channel(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
        )?;
        let facts = DevFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        let Some(decision) =
            self.driver
                .channel_decision(world, actor, &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        self.driver
            .execute_channel_decision(world, actor, decision)?;
        Ok(true)
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_DEVASTATION_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    use super::literal_rotation::PrecombatPoll;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world(), run.actor, encounter)?;
    // Pinned player.cpp6138–6183: source-order setup, through the first harmful precast.
    loop {
        match rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                rotation.execute_precombat(run, action, execution)?
            }
            PrecombatPoll::Finished => break,
        }
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("devastation_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DEVASTATION_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}

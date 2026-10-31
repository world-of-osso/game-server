//! Thin Devastation adapter over the shared literal APL scheduler and native casts.
pub mod bindings;
pub mod facts;
pub mod source;
use crate::literal_rotation::{Driver, PrecombatPoll, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::DevFacts;
use serde_json::{Value, json};

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
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

    pub fn prepare(
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
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
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

    pub fn drive_channel(
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

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        self.precombat_step(world, actor, targets)
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
        self.start_combat_at(started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }
}

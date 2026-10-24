//! Fire Player Fixture adapter; shared machinery owns scheduling and native execution.
mod bindings;
mod facts;
mod source;
#[cfg(test)]
mod tests;
use super::literal_rotation::{
    self, Driver, PrecombatPoll,
    equipment::Equipment,
    execution::{Execution, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::FireFacts;
use serde_json::{Value, json};
use server::simc_apl::Action;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Fire requires an explicit Fixture encounter".into());
        }
        if world.get::<shared::components::Player>(actor).is_none() {
            return Err("literal Fire requires its native Player".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(source::load()?),
            bindings,
            equipment,
            encounter,
        };
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = self.prepare(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = FireFacts::new(
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
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("Fixture validated at load");
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
        if targets.is_empty() {
            return Err("literal Fire has no native target".into());
        }
        let prepared = self.prepare(world, actor, targets, false)?;
        let facts = FireFacts::new(
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

    fn prepare(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        snapshot: bool,
    ) -> Result<Prepared, String> {
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            snapshot,
        )
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_FIRE_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "qualification": "STAGED_NOT_QUALIFIED",
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion",
                "NATIVE_FIRE_FACT_QUALIFICATION_OPEN", "SHARED_PRECOMBAT_EQUIPMENT_QUALIFICATION_OPEN"]})
    }
}

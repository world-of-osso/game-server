//! Exact pinned Ret Program over the shared literal lifecycle and native dispatch.
use bevy::prelude::*;
use serde_json::{Value, json};

use super::{bindings, context::Context, execution_actions, read_program};
use crate::literal_rotation::{Driver, PrecombatPoll, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter};

pub struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let follower = world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|follower| follower.profile.spec_id == 70);
        let player = world.get::<shared::components::Player>(actor).is_some()
            && world
                .get::<shared::components::ActiveSpec>(actor)
                .is_some_and(|spec| spec.0 == 70);
        if !follower && !player {
            return Err("literal Ret fixture requires a native spec70 actor".into());
        }
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: Driver::new(read_program()?),
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = self.read_prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = Context::read(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            &prepared,
            self.encounter,
            targets,
        );
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn driver(&mut self) -> &mut Driver {
        &mut self.driver
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
        if targets.is_empty() {
            return Err("literal Ret fixture has no native target".into());
        }
        let prepared = self.read_prepared(world, actor, targets, false)?;
        let facts = Context::read(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            &prepared,
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

    pub fn read_prepared(
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
            execution_actions(),
            snapshot,
        )
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_RETRIBUTION_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion",
                "NATIVE_RET_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_ITEM_PROC_QUALIFICATION_OPEN"]})
    }
}

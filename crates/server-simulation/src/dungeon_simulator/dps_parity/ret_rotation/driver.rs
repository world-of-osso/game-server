//! Exact pinned Ret Program over canonical fixture lifecycle and native dispatch.
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::Action;

use super::super::{
    Run,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{Execution, Prepared},
    },
};
use super::{bindings, context::Context, execution_actions, read_program};
use crate::simc_apl_native::{Bindings, Encounter};

pub(crate) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("DUNGEON_TTD_UNAVAILABLE: literal Ret requires a Fixture encounter".into());
        }
        if !world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|follower| follower.profile.spec_id == 70)
        {
            return Err("literal Ret fixture requires its actual spec70 follower".into());
        }
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(read_program()?),
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

    pub fn execute_precombat(
        &mut self,
        run: &mut Run,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        super::super::preparation::execute_precombat_plan(run, &mut self.driver, action, execution)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("literal Ret fixture validated at load");
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

    fn read_prepared(
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

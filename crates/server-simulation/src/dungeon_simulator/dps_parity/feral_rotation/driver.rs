//! Fixture-only Feral lifecycle over canonical Program, native admission and execution.
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::Action;

use super::super::{
    inputs::Profile,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{Execution, Prepared},
    },
};
use super::{
    bindings,
    context::{Context, read_configured_potion},
    source,
};
use crate::simc_apl_native::{Bindings, Encounter};

pub(crate) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    configured_potion: String,
    encounter: Encounter,
}

fn validate_fixture(
    world: &World,
    actor: Entity,
    profile: &Profile,
    encounter: Encounter,
) -> Result<(), String> {
    if !matches!(encounter, Encounter::Fixture { .. }) {
        return Err(
            "DUNGEON_TTD_UNAVAILABLE: literal Feral requires an explicit Fixture encounter".into(),
        );
    }
    if profile.spec_id != 103
        || !world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|follower| follower.profile.spec_id == 103)
    {
        return Err("literal Feral fixture requires its actual spec103 profile/follower".into());
    }
    Ok(())
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        validate_fixture(world, actor, profile, encounter)?;
        let mut bindings = bindings::build(world, actor)?;
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(source::load()?),
            bindings,
            equipment,
            configured_potion: read_configured_potion(profile)?,
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
        self.start_combat_at(crate::spell_cast::now(world));
        let prepared = self.read_prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = Context::read(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            &prepared,
            &self.configured_potion,
            self.encounter,
            targets,
        )?;
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn execute(
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
            unreachable!("literal Feral fixture validated at load");
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
            return Err("literal Feral fixture has no native target".into());
        }
        let prepared = self.read_prepared(world, actor, targets, false)?;
        let facts = Context::read(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            &prepared,
            &self.configured_potion,
            self.encounter,
            targets,
        )?;
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
            bindings::execution_actions(&self.bindings),
            snapshot,
        )
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_FERAL_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion",
                "NATIVE_FERAL_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_ITEM_PROC_QUALIFICATION_OPEN"]})
    }
}

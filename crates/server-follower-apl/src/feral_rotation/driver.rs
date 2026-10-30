//! Feral lifecycle over canonical Program, native admission and execution.
use bevy::prelude::*;
use serde_json::{Value, json};

use super::{bindings, context::Context, source};
use crate::literal_rotation::{Driver, PrecombatPoll, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter};
use server::followers::parity_profile::Profile;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

pub fn validate_spec(world: &World, actor: Entity, profile: &Profile) -> Result<(), String> {
    if profile.spec_id != 103
        || !world
            .get::<shared::components::ActiveSpec>(actor)
            .is_some_and(|spec| spec.0 == 103)
    {
        return Err("literal Feral requires its actual spec103 actor/profile".into());
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
        validate_spec(world, actor, profile)?;
        // Exact103.reference.single.json player reaction fields; buff.cpp2697–2703
        // observes the native acquisition, not the later AoE decision query.
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
            &[135700],
        )?;
        let mut bindings = bindings::build(world, actor)?;
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(source::load()?),
            bindings,
            equipment,
            encounter,
        };
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
            self.encounter,
            targets,
        )?;
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
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
            return Err("literal Feral has no native target".into());
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
        )?;
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
            bindings::execution_actions(&self.bindings),
            snapshot,
        )
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_FERAL_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_FERAL_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_ITEM_PROC_QUALIFICATION_OPEN"]})
    }
}

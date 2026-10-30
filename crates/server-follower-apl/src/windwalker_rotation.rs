//! Literal pinned Windwalker driver for a spec269 player or class follower.
pub mod bindings;
pub mod facts;
pub mod karma;
pub mod source;
use crate::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{Execution, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::WwFacts;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts};

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let follower = world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|f| f.profile.spec_id == 269);
        let player = world.get::<shared::components::Player>(actor).is_some()
            && world
                .get::<shared::components::ActiveSpec>(actor)
                .is_some_and(|spec| spec.0 == 269);
        if !follower && !player {
            return Err("literal WW fixture requires a native spec269 actor".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(source::load()?),
            bindings,
            equipment,
            encounter,
        };
        // Rejected dungeon loads cannot capture the controller or mutate identity.
        Ok(rotation)
    }

    pub fn precombat_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String> {
        self.start_combat_at(crate::spell_cast::now(world));
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            self.driver.snapshot.is_none(),
        )?;
        let Some((action, execution)) =
            self.select(world, actor, targets, "precombat", &prepared)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
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
            return Err("literal WW fixture has no native target".into());
        }
        self.cancel_channel(world, actor, targets)?;
        let mut prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            false,
        )?;
        karma::suppress_during_interval(world, actor, &mut prepared);
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            let karma_target = match (&*action.name, &execution) {
                ("touch_of_karma", Execution::Spell(_, _, Some(target))) => Some(*target),
                _ => None,
            };
            self.driver.execute(world, actor, action, execution)?;
            if let Some(target) = karma_target {
                karma::apply_modeled_absorption(world, actor, target)?;
            }
        }
        Ok(())
    }

    pub fn select(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, Execution)>, String> {
        let xuen = facts::read_xuen(world, actor);
        let facts = WwFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
            xuen,
        );
        self.driver.select(list, &facts, prepared, &self.equipment)
    }

    pub fn cancel_channel(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let xuen = facts::read_xuen(world, actor);
        let facts = WwFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
            xuen,
        );
        if self.driver.should_cancel(world, actor, &facts)? {
            crate::spell_cast::cancel_cast(world, actor);
        }
        Ok(())
    }

    pub fn value_for(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        action: &Action,
        name: &str,
    ) -> Result<f64, String> {
        let xuen = facts::read_xuen(world, actor);
        WwFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
            xuen,
        )
        .value_for(action, name)
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_WINDWALKER_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "named_policy": "SIMC_MISSING_COOLDOWN_PLACEHOLDER",
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion",
                "NATIVE_WW_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_ITEM_PROC_QUALIFICATION_OPEN"]})
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        Ok(if self.precombat_step(world, actor, targets)? {
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

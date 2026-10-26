//! Literal pinned Windwalker fixture driver; Stockade retains its native kit.
mod bindings;
mod facts;
mod source;
use super::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{Execution, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::WwFacts;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err(
                "DUNGEON_TTD_UNAVAILABLE: literal WW requires an explicit Fixture encounter".into(),
            );
        }
        if !world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|f| f.profile.spec_id == 269)
        {
            return Err("literal WW fixture requires its native spec269 follower".into());
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
        literal_rotation::capture_fixture(world, actor, encounter)?;
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
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("literal fixture validated at load");
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
            return Err("literal WW fixture has no native target".into());
        }
        self.cancel_channel(world, actor, targets)?;
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            false,
        )?;
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn select(
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

    fn cancel_channel(
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

#[cfg(test)]
mod channel_melee_tests;
#[cfg(test)]
mod dance_breaker_tests;
#[cfg(test)]
mod flurry_tests;
#[cfg(test)]
mod glory_tests;
#[cfg(test)]
mod hero_flurry_tests;
#[cfg(test)]
mod momentum_tests;
#[cfg(test)]
mod source_children_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod teachings_tests;
#[cfg(test)]
mod unbroken_rhythm_tests;
#[cfg(test)]
mod wdp_children_tests;
#[cfg(test)]
mod wdp_duration_tests;
#[cfg(test)]
mod wdp_window_tests;
#[cfg(test)]
mod xuens_battlegear_tests;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("windwalker_rotation");

impl super::SpecDriver for Rotation {
    fn supports_actor(identity: &crate::followers::simulator::ParityActor) -> bool {
        // Rotation::load requires the existing native spec269 follower fixture.
        matches!(identity, crate::followers::simulator::ParityActor::Follower)
    }

    fn label(&self) -> &'static str {
        "LITERAL_SIMC_WINDWALKER_76f8118"
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

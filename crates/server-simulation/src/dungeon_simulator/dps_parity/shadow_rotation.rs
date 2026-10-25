//! Literal pinned Shadow APL adapter over shared native selection/execution.
mod bindings;
mod facts;
use super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let rows = include_str!("shadow_rotation/76f8118.simc")
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
            .map(|line| {
                let (head, literal) = line
                    .split_once('=')
                    .ok_or("invalid Shadow APL assignment")?;
                let list = head
                    .trim_end_matches('+')
                    .strip_prefix("actions.")
                    .ok_or("invalid Shadow APL list")?;
                Ok((list, literal.trim_start_matches('/')))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let program = Program::parse(&rows)?;
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        literal_rotation::capture_fixture(world, actor, encounter)?;
        // Exact258.reference.single.json reaction fields; player.cpp1017–1020.
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
            &[373204, 453850, 2825],
        )?;
        Ok(Self {
            driver: Driver::new(program),
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
    ) -> Result<bool, String> {
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
            self.driver.snapshot.is_none(),
        )?;
        let facts = facts::ShadowFacts::new(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
        );
        let Some((action, execution)) =
            self.driver
                .select("precombat", &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = Encounter::Fixture {
            started_at,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.advance_channel(world, actor, targets)?;
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
            false,
        )?;
        let facts = facts::ShadowFacts::new(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
        );
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
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
            bindings::actions(),
        )?;
        let decision = {
            let facts = facts::ShadowFacts::new(
                world,
                actor,
                targets,
                &self.bindings,
                &self.equipment,
                self.encounter,
            );
            self.driver
                .channel_decision(world, actor, &facts, &prepared, &self.equipment)?
        };
        if let Some(decision) = decision {
            self.driver
                .execute_channel_decision(world, actor, decision)?;
        }
        Ok(())
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_SHADOW_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE"]})
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation =
        Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    // SimC76f8118 player.cpp6138–6183: execute precombat once in source order.
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("shadow_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_SHADOW_76f8118"
    }

    fn drive(&mut self, world: &mut World, actor: Entity, targets: &[Entity]) -> Result<(), String> {
        self.drive(world, actor, targets)
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}

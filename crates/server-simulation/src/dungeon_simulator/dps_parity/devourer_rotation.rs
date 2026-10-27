//! Thin pinned Devourer adapter; native casts and the shared interpreter do the work.
mod bindings;
mod facts;
use super::literal_rotation::{
    self, Driver, PrecombatPoll, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use facts::DevourerFacts;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

pub(super) fn configure_actor_reactions(world: &mut World, actor: Entity) -> Result<(), String> {
    // Pinned 1480 reference reaction fields; player.cpp1017–1020.
    let settings = crate::simc_apl_native::reaction::ReactionSettings {
        mean: 0.25,
        stddev: 0.05,
        offset: 0.1,
        nu: 0.15,
        maximum: 1.4,
    };
    crate::simc_apl_native::reaction::configure(world, actor, settings, &[1217607, 1256302])
}

fn load_program() -> Result<Program, String> {
    let rows = include_str!("devourer_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (head, action) = line
                .split_once('=')
                .ok_or("Devourer action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Devourer action lacks list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if world
            .get::<shared::components::ActiveSpec>(actor)
            .map(|spec| spec.0)
            != Some(1480)
        {
            return Err("literal Devourer requires native spec1480".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let driver = Driver::new(load_program()?);
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("literal encounter validated at capture");
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
    }

    pub fn precombat_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String> {
        let prepared = self.prepare(world, actor, targets)?;
        let facts = DevourerFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            world,
            actor,
        };
        match self
            .driver
            .poll_precombat(&facts, &prepared, &self.equipment)?
        {
            PrecombatPoll::Waiting => {
                Err("Devourer precombat polled before native setup finished".into())
            }
            PrecombatPoll::Finished => Ok(false),
            PrecombatPoll::Selected { action, execution } => {
                self.driver.execute(world, actor, action, execution)?;
                Ok(true)
            }
        }
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

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let pickup = {
            let facts = DevourerFacts {
                native: NativeFacts::new(
                    world,
                    actor,
                    &self.bindings,
                    self.encounter,
                    targets.len(),
                )
                .with_targets(targets.first().copied(), targets),
                equipment: &self.equipment,
                world,
                actor,
            };
            self.driver
                .fragment_pickup_permitted(world, actor, &facts)?
        };
        if let Some(permitted) = pickup {
            server::soul_fragments::finish_pickup(world, actor, permitted)?;
        }
        let prepared = self.prepare(world, actor, targets)?;
        let facts = DevourerFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            world,
            actor,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_DEVOURER_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_DEVOURER_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_DEVOURER_FRAGMENT_SIDE_EFFECTS_UNQUALIFIED"]})
    }
}

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
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("devourer_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DEVOURER_76f8118"
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

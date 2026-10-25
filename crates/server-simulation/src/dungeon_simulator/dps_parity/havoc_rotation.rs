//! Thin pinned Havoc adapter; native casts and the shared interpreter do the work.
mod bindings;
mod facts;
use super::literal_rotation::{
    self, Driver, PrecombatPoll, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use facts::HavocFacts;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

fn load_program() -> Result<Program, String> {
    let rows = include_str!("havoc_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (head, action) = line
                .split_once('=')
                .ok_or("Havoc action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Havoc action lacks list")?
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
            != Some(577)
        {
            return Err("literal Havoc requires native spec577".into());
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
        let facts = HavocFacts {
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
                Err("Havoc precombat polled before native setup finished".into())
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
        let prepared = self.prepare(world, actor, targets)?;
        let facts = HavocFacts {
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
        json!({"driver": "LITERAL_SIMC_HAVOC_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_SOUL_FRAGMENT_PICKUP_UNAVAILABLE",
                "NATIVE_HAVOC_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_DEMONSURGE_DAMAGE_UNQUALIFIED"]})
    }
}

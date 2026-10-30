//! Thin pinned Havoc adapter; native casts and the shared interpreter do the work.
pub mod bindings;
pub mod facts;
use crate::literal_rotation::{
    Driver, PrecombatPoll,
    equipment::Equipment,
    execution::{Execution, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use facts::HavocFacts;
use serde_json::{Value, json};
use server::simc_apl::{Action, Program};

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
    pub pending_retreat: Option<(Action, Execution, Entity)>,
}

pub fn load_program() -> Result<Program, String> {
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
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
            pending_retreat: None,
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
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

    pub fn prepare(
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
        if let Some((action, execution, target)) = self.pending_retreat.take() {
            if crate::followers::retreat::prepare_havoc_retreat(world, actor, target)? {
                self.driver.execute(world, actor, action, execution)?;
            } else {
                self.pending_retreat = Some((action, execution, target));
            }
            return Ok(());
        }
        let pickup = {
            let facts = HavocFacts {
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
            if matches!(execution, Execution::Spell(198793, _, _)) {
                let target = *targets.first().ok_or("Retreat requires an enemy target")?;
                if !crate::followers::retreat::prepare_havoc_retreat(world, actor, target)? {
                    self.pending_retreat = Some((action, execution, target));
                    return Ok(());
                }
            }
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_HAVOC_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_HAVOC_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_DEMONSURGE_DAMAGE_UNQUALIFIED"]})
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

//! Literal pinned Survival source; decisions use the shared evaluator and native execution.
pub mod bindings;
use crate::hunter_facts::HunterFacts;
use crate::literal_rotation::{Driver, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
    pub potion: String,
}

pub fn program() -> Result<Program, String> {
    // SimC76f8118 ActionPriorityLists/default/hunter_survival.simc, unchanged.
    let rows = include_str!("survival_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (head, row) = line
                .split_once('=')
                .ok_or("Survival source assignment missing")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Survival source list missing")?
            };
            Ok((list, row.strip_prefix('/').unwrap_or(row)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &server::followers::parity_profile::Profile,
    ) -> Result<Self, String> {
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: Driver::new(program()?),
            bindings,
            equipment,
            encounter,
            potion: profile
                .consumables
                .get("potion")
                .ok_or("Survival profile potion option missing")?
                .clone(),
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
        self.start_combat_at(crate::spell_cast::now(world));
        self.step(world, actor, targets, "precombat")
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        // SimC76f8118 sc_hunter.cpp1706–1711: reuse the actual native pet AI,
        // independently of the owner's literal APL decisions.
        crate::pet_control::order_companion_pet(world, actor, targets.first().copied());
        self.step(world, actor, targets, "default").map(|_| ())
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
    ) -> Result<bool, String> {
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
            self.driver.snapshot.is_none(),
        )?;
        let native = NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
            .with_targets(targets.first().copied(), targets);
        let facts = HunterFacts {
            native,
            equipment: &self.equipment,
            potion: &self.potion,
        };
        let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_SURVIVAL_76f8118","decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE","EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
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

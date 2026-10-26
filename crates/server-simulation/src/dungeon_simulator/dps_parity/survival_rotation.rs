//! Literal pinned Survival source; decisions use the shared evaluator and native execution.
mod bindings;
use super::hunter_facts::HunterFacts;
use super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    potion: String,
}

fn program() -> Result<Program, String> {
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
        profile: &super::inputs::Profile,
    ) -> Result<Self, String> {
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        literal_rotation::capture_fixture(world, actor, encounter)?;
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
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("literal fixture validated at load")
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

    fn step(
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

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    // SimC76f8118 sc_hunter.cpp1052: summon_pet defaults to duck. Literal
    // capture suppresses follower AI, so provision its existing native stable policy here.
    super::hunter_fixture::provision_pet(run.app.world_mut(), run.actor)?;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL"));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("survival_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_SURVIVAL_76f8118"
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

//! Literal Elemental fixture adapter. Ordinary followers retain their native kit.
mod bindings;
use super::inputs::Profile;
use super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

fn read_program(profile: &Profile) -> Result<Program, String> {
    let rows = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Elemental profile has no literal actions")?;
    let rows = rows
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("non-string Elemental action")?;
            let (head, value) = row
                .split_once('=')
                .ok_or("Elemental action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("invalid Elemental action list")?
            };
            Ok((list, value.strip_prefix('/').unwrap_or(value)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &Profile,
    ) -> Result<Self, String> {
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let driver = Driver::new(read_program(profile)?);
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = Encounter::Fixture {
            started_at,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
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
            bindings::execution_actions(),
            self.driver.snapshot.is_none(),
        )?;
        let selected = self.select(world, actor, targets, "precombat", &prepared)?;
        let Some((action, execution)) = selected else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
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
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, super::literal_rotation::execution::Execution)>, String> {
        let facts = ElementalFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        self.driver.select(list, &facts, prepared, &self.equipment)
    }
}

struct ElementalFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

fn is_equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.") || name.starts_with("set_bonus.")
}

impl Facts for ElementalFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) {
            return self.equipment.fact(name, &self.native);
        }
        // SimC76f8118 sc_shaman.cpp10596–10609: count live Lightning Rod debuffs.
        if name == "lightning_rod" {
            return self.native.value("active_dot.lightning_rod");
        }
        self.native.value(name)
    }
}

impl ActionFacts for ElementalFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_shaman.cpp8516–8524: interrupt only a casting enemy.
        if action.name == "wind_shear" {
            return self
                .native
                .value_for(action, "target.debuff.casting.up")
                .map(|value| value != 0.0);
        }
        // SimC76f8118 sc_shaman.cpp8132–8137: Ancestral Swiftness replaces NS.
        if action.name == "natures_swiftness" {
            return self
                .native
                .value("talent.ancestral_swiftness")
                .map(|value| value == 0.0);
        }
        Ok(true)
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) || name == "lightning_rod" {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
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

pub(super) const REPORT_KEY: Option<&str> = None;

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_ELEMENTAL_76f8118"
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

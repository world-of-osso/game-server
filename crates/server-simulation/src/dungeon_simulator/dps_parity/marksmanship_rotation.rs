//! Literal pinned MM source; decisions use the shared evaluator and native execution.
mod bindings;
use super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    potion: String,
}

fn program() -> Result<Program, String> {
    // SimC76f8118 ActionPriorityLists/default/hunter_marksmanship.simc, unchanged.
    let rows = include_str!("marksmanship_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (head, row) = line.split_once('=').ok_or("MM source assignment missing")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("MM source list missing")?
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
                .ok_or("MM profile potion option missing")?
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
        let facts = MmFacts {
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
        json!({"driver":"LITERAL_SIMC_MARKSMANSHIP_76f8118","decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE","EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

struct MmFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    potion: &'a str,
}

fn equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.") || name.starts_with("set_bonus.")
}

impl Facts for MmFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if equipment_operand(name) {
            return self.equipment.fact(name, &self.native);
        }
        if let Some(alias) = name.strip_prefix("potion.") {
            // SimC76f8118 player.cpp12376–12409 uses the configured potion,
            // irrespective of whether the current inventory admits a use.
            let configured = self
                .potion
                .rsplit_once('_')
                .filter(|(_, suffix)| suffix.parse::<u8>().is_ok())
                .map_or(self.potion, |(base, _)| base);
            return Ok(
                if configured.eq_ignore_ascii_case(alias) || self.potion.eq_ignore_ascii_case(alias)
                {
                    1.0
                } else {
                    0.0
                },
            );
        }
        self.native.value(name)
    }
}

impl ActionFacts for MmFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_hunter.cpp4452–4455 requires the real enabler;
        // trueshot_t6883–6914 retains its own cooldown, not its bar replacement's.
        const SOURCE_READINESS: &[(&str, &str)] = &[
            ("moonlight_chakram", "buff.moonlight_chakram.up"),
            ("trueshot", "cooldown.trueshot.ready"),
        ];
        for (_, fact) in SOURCE_READINESS
            .iter()
            .filter(|(name, _)| *name == action.name)
        {
            if self.native.value(fact)? == 0.0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if equipment_operand(name) || name.starts_with("potion.") {
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
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("marksmanship_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_MARKSMANSHIP_76f8118"
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

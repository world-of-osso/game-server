//! Literal pinned Enhancement fixture driver; all numeric facts/casts remain native.
mod bindings;
use super::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{Execution, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    ti_chain_lightning: bool,
    liquid_luster: bool,
    debuff_values: std::collections::BTreeMap<&'static str, f64>,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &super::inputs::Profile,
    ) -> Result<Self, String> {
        if profile.spec_id != 263 || !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Enhancement requires an explicit spec263 fixture".into());
        }
        let mut bindings = bindings::build();
        // SimC12758–12763: the selected apex uses its own native async Crash aura.
        if world
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1262713))
        {
            bindings.buffs.insert("crash_lightning".into(), 1252415);
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let liquid_luster = profile
            .consumables
            .get("potion")
            .is_some_and(|name| name.starts_with("potion_of_liquid_luster"));
        let debuff_values = read_debuff_values(world)?;
        let rotation = Self {
            driver: Driver::new(read_program()?),
            bindings,
            equipment,
            encounter,
            ti_chain_lightning: false,
            liquid_luster,
            debuff_values,
        };
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    pub fn start_combat_at(&mut self, time: f64) {
        let Encounter::Fixture { started_at, .. } = &mut self.encounter else {
            unreachable!()
        };
        *started_at = time;
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
    ) -> Result<bool, String> {
        let Some((action, execution)) = self.select(world, actor, targets, list)? else {
            return Ok(false);
        };
        let ti_choice = match action.name.as_str() {
            "chain_lightning" => Some(true),
            "lightning_bolt" => Some(false),
            "tempest" => Some(targets.len() > 1),
            _ => None,
        };
        self.driver.execute(world, actor, action, execution)?;
        // SimC76f8118 sc_shaman.cpp6673-6676/7448-7451/10011-10019:
        // only foreground LB/CL/Tempest changes Thorim's selected invocation.
        // No next action is admitted while the accepted cast is pending.
        if let Some(chain) = ti_choice {
            self.ti_chain_lightning = chain;
        }
        Ok(true)
    }

    fn select(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
    ) -> Result<Option<(Action, Execution)>, String> {
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            self.driver.snapshot.is_none(),
        )?;
        let native = NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
            .with_targets(targets.first().copied(), targets);
        let facts = EnhancementFacts {
            native,
            equipment: &self.equipment,
            ti_chain_lightning: self.ti_chain_lightning,
            liquid_luster: self.liquid_luster,
            debuff_values: &self.debuff_values,
        };
        self.driver.select(list, &facts, &prepared, &self.equipment)
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_ENHANCEMENT_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

struct EnhancementFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    ti_chain_lightning: bool,
    liquid_luster: bool,
    debuff_values: &'a std::collections::BTreeMap<&'static str, f64>,
}

impl EnhancementFacts<'_> {
    fn debuff_value(&self, name: &str, action: Option<&Action>) -> Option<Result<f64, String>> {
        let &value = self.debuff_values.get(name)?;
        let gate = format!("{}.up", name.strip_suffix(".value")?);
        // SimC76f8118 buff.cpp464-468: check_value is zero while inactive.
        let active = match action {
            Some(action) => self.native.value_for(action, &gate),
            None => self.native.value(&gate),
        };
        Some(active.map(|active| if active != 0.0 { value } else { 0.0 }))
    }
}

impl Facts for EnhancementFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if let Some(value) = self.debuff_value(name, None) {
            return value;
        }
        match name {
            // SimC76f8118 sc_shaman.cpp10557-10569: unset defaults to LB.
            "ti_lightning_bolt" => Ok(boolean(!self.ti_chain_lightning)),
            "ti_chain_lightning" => Ok(boolean(self.ti_chain_lightning)),
            "potion.liquid_luster" => Ok(boolean(self.liquid_luster)),
            _ if equipment_operand(name) => self.equipment.fact(name, &self.native),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for EnhancementFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if let Some(value) = self.debuff_value(name, Some(action)) {
            return value;
        }
        if matches!(
            name,
            "ti_lightning_bolt" | "ti_chain_lightning" | "potion.liquid_luster"
        ) || equipment_operand(name)
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

fn read_debuff_values(
    world: &World,
) -> Result<std::collections::BTreeMap<&'static str, f64>, String> {
    // SimC76f8118 player.cpp4880-4891 and sc_shaman.cpp11478 bind percentages
    // from these native effects, not from a guessed generic first aura amount.
    let sources = [
        ("debuff.chaos_brand.value", 1490, 0),
        ("debuff.hunters_mark.value", 259556, 0),
        ("debuff.lightning_rod.value", 210689, 1),
    ];
    let store = world.resource::<crate::spell_info::SpellStore>();
    sources
        .into_iter()
        .map(|(name, spell, index)| {
            let effect = store
                .get(spell)
                .and_then(|spell| spell.effects.iter().find(|effect| effect.index == index))
                .ok_or_else(|| {
                    format!("native {name} source spell{spell} effect{index} missing")
                })?;
            Ok((name, f64::from(effect.base_points) / 100.0))
        })
        .collect()
}

fn boolean(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("set_bonus.") || name.starts_with("equipped.")
}

fn read_program() -> Result<Program, String> {
    let rows = include_str!("enhancement_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_row(line: &str) -> Result<(&str, &str), String> {
    let (head, action) = line
        .split_once('=')
        .ok_or("Enhancement source row lacks assignment")?;
    let head = head.strip_suffix('+').unwrap_or(head);
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("Enhancement source lacks list prefix")?
    };
    Ok((list, action.strip_prefix('/').unwrap_or(action)))
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, "precombat")? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("enhancement_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_ENHANCEMENT_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, "default").map(|_| ())
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}

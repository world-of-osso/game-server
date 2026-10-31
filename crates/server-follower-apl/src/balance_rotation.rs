//! Literal pinned Balance driver over native facts and dispatch.
pub mod bindings;

use crate::literal_rotation::{
    Driver, PrecombatPoll, equipment::Equipment, execution::Execution, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::followers::parity_profile::Profile;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

/// SimC76f8118 sc_druid.cpp1293/11130: the Lunar Eclipse override aura.
const LUNAR_ECLIPSE_OVERRIDE: u32 = 1233353;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

pub fn read_program(profile: &Profile) -> Result<Program, String> {
    let rows = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Balance profile has no literal actions")?
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("non-string Balance action")?;
            let (head, value) = row
                .split_once('=')
                .ok_or("Balance action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("invalid Balance action list")?
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
        if profile.spec_id != 102 {
            return Err("literal Balance requires spec102".into());
        }
        let mut bindings = bindings::build(world, actor)?;
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            encounter,
        })
    }

    fn select(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        snapshot: bool,
    ) -> Result<Option<(Action, Execution)>, String> {
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(&self.bindings),
            snapshot,
        )?;
        let facts = BalanceFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            world,
            actor,
            bindings: &self.bindings,
            equipment: &self.equipment,
        };
        self.driver.select(list, &facts, &prepared, &self.equipment)
    }
}

pub struct BalanceFacts<'a> {
    native: NativeFacts<'a>,
    world: &'a World,
    actor: Entity,
    bindings: &'a Bindings,
    equipment: &'a Equipment,
}

fn is_equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.") || name.starts_with("set_bonus.")
}

fn is_balance_operand(name: &str) -> bool {
    is_equipment_operand(name)
        || name == "druid.no_cds"
        || name.starts_with("eclipse.")
        || name.starts_with("buff.eclipse.")
        || name.starts_with("cooldown.solar_eclipse.")
        || name.starts_with("cooldown.lunar_eclipse.")
}

impl BalanceFacts<'_> {
    fn lunar_override(&self) -> bool {
        crate::auras::has_aura(self.world, self.actor, LUNAR_ECLIPSE_OVERRIDE)
    }

    /// SimC action energize_amount: the action's own Astral Power energize, in display units.
    fn energize_amount(&self, action: &Action) -> Result<f64, String> {
        let id = *self
            .bindings
            .actions
            .get(&action.name)
            .ok_or_else(|| format!("Balance {} has no native action", action.name))?;
        let spell = self
            .world
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .ok_or_else(|| format!("Balance action spell{id} absent"))?;
        let raw: f32 = spell
            .effects
            .iter()
            .filter(|effect| {
                effect.effect == 30 && effect.misc_values[0] == PowerType::LunarPower as i32
            })
            .map(|effect| effect.base_points)
            .sum();
        let display = self
            .world
            .resource::<crate::spell_cast::powers::PowerTypeTable>()
            .get(PowerType::LunarPower)
            .ok_or("native Astral Power metadata absent")?
            .display_modifier;
        Ok(f64::from(raw) / f64::from(display))
    }
}

impl Facts for BalanceFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) {
            return self.equipment.fact(name, &self.native);
        }
        // SimC76f8118 sc_druid.cpp13414–13415: the no_cds option defaults off.
        if name == "druid.no_cds" {
            return Ok(0.0);
        }
        // SimC76f8118 sc_druid.cpp13469–13475: the next Eclipse is lunar under the override.
        match name {
            "eclipse.lunar" => return Ok(f64::from(u8::from(self.lunar_override()))),
            "eclipse.solar" => return Ok(f64::from(u8::from(!self.lunar_override()))),
            _ => {}
        }
        // SimC76f8118 sc_druid.cpp13448–13458: buff.eclipse reads the lunar
        // Eclipse while it is up, otherwise the solar one.
        if let Some(property) = name.strip_prefix("buff.eclipse.") {
            let lunar_up = self.native.value("buff.eclipse_lunar.up")? != 0.0;
            let buff = if lunar_up {
                "eclipse_lunar"
            } else {
                "eclipse_solar"
            };
            return self.native.value(&format!("buff.{buff}.{property}"));
        }
        // SimC76f8118 sc_druid.cpp13460–13466: both Eclipse spells share one cooldown.
        if let Some(property) = name
            .strip_prefix("cooldown.solar_eclipse.")
            .or_else(|| name.strip_prefix("cooldown.lunar_eclipse."))
        {
            return self.native.value(&format!("cooldown.eclipse.{property}"));
        }
        self.native.value(name)
    }
}

impl ActionFacts for BalanceFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "energize_amount" {
            return self.energize_amount(action);
        }
        if is_balance_operand(name) {
            return self.value(name);
        }
        self.native.value_for(action, name)
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let snapshot = self.driver.snapshot.is_none();
        Ok(
            match self.select(world, actor, targets, "precombat", snapshot)? {
                Some((action, execution)) => PrecombatPoll::Selected { action, execution },
                None => PrecombatPoll::Finished,
            },
        )
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if let Some((action, execution)) = self.select(world, actor, targets, "default", false)? {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
}

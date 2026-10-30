//! Literal pinned Ret fixture adapter; native facts and execution stay shared.
mod context;
mod driver;
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, NativeFacts};
use bevy::prelude::{Entity, World};
pub(super) use driver::Rotation;
use server::simc_apl::ret_bindings::{ACTIONS, bindings};
use server::simc_apl::{Action, ActionFacts, Facts};

fn execution_actions()
-> impl Iterator<Item = super::literal_rotation::execution::NativeAction<'static>> {
    // Inventory is not provisioned for the native potion; no fictitious admission.
    ACTIONS
        .iter()
        .filter(|(name, _, _)| *name != "potion")
        .map(
            |&(name, spell, self_target)| super::literal_rotation::execution::NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            },
        )
}

fn read_program() -> Result<server::simc_apl::Program, String> {
    server::simc_apl::ret_program::load()
}

struct RetFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

fn with_equipment<'a>(native: NativeFacts<'a>, equipment: &'a Equipment) -> RetFacts<'a> {
    RetFacts { native, equipment }
}

fn is_equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.")
}

impl Facts for RetFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for RetFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_paladin.cpp1537–1540: Judgment is replaced during Hammer mode.
        if action.name == "judgment" {
            return self
                .native
                .value("buff.hammer_of_wrath.up")
                .map(|up| up == 0.0);
        }
        // SimC76f8118 sc_paladin.cpp1956–1962: action policy, not cast permission.
        if action.name == "rebuke" {
            return self
                .native
                .value_for(action, "target.debuff.casting.up")
                .map(|casting| casting != 0.0);
        }
        Ok(true)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod equipment_tests;
#[cfg(test)]
mod finisher_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod readiness_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wrath_gate_tests;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    use super::literal_rotation::PrecombatPoll;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    for _ in 0..super::preparation::MAX_SETUP_TICKS {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                rotation.execute_precombat(run, action, execution)?;
            }
            PrecombatPoll::Finished => {
                rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
                return Ok(Box::new(rotation));
            }
        }
    }
    Err("native Ret precombat did not finish within the setup bound".into())
}

pub(super) const REPORT_KEY: Option<&str> = Some("ret_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_RETRIBUTION_76f8118"
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

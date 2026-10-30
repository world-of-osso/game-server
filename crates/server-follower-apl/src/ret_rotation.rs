//! Literal pinned Ret APL; native facts and execution stay shared.
pub mod context;
mod driver;
use crate::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, NativeFacts};
use bevy::prelude::{Entity, World};
pub use driver::Rotation;
pub use server::simc_apl::ret_bindings::{ACTIONS, bindings};
use server::simc_apl::{Action, ActionFacts, Facts};

pub fn execution_actions()
-> impl Iterator<Item = crate::literal_rotation::execution::NativeAction<'static>> {
    // Inventory is not provisioned for the native potion; no fictitious admission.
    ACTIONS
        .iter()
        .filter(|(name, _, _)| *name != "potion")
        .map(
            |&(name, spell, self_target)| crate::literal_rotation::execution::NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            },
        )
}

pub fn read_program() -> Result<server::simc_apl::Program, String> {
    server::simc_apl::ret_program::load()
}

pub struct RetFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

pub fn with_equipment<'a>(native: NativeFacts<'a>, equipment: &'a Equipment) -> RetFacts<'a> {
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

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        self.poll_precombat(world, actor, targets)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: crate::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver().execute(world, actor, action, execution)
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

//! Preserve shared action/target context; item and set gaps remain explicit.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct DevFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

impl<'a> DevFacts<'a> {
    pub fn new(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment,
        }
    }
}

impl Facts for DevFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for DevFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value_for(action, name)
        }
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
}

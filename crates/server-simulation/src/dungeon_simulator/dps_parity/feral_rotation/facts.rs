//! Feral consumer facts; generic target/aura/resource semantics remain shared.
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};

pub(super) struct FeralFacts<'a> {
    native: NativeFacts<'a>,
}

impl<'a> FeralFacts<'a> {
    pub fn read(
        world: &'a mut World,
        actor: Entity,
        bindings: &'a Bindings,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Result<Self, String> {
        Ok(Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
        })
    }
}

impl Facts for FeralFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.native.value(name)
    }
}

impl ActionFacts for FeralFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        self.native.value_for(action, name)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
}

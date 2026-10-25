//! Ret fact delegation; shared equipment, admission and native state stay authoritative.
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

use super::super::literal_rotation::{equipment::Equipment, execution::Prepared};
use super::{Bindings, NativeFacts, RetFacts, with_equipment};
use crate::simc_apl_native::Encounter;

pub(super) struct Context<'a> {
    ret: RetFacts<'a>,
    bindings: &'a Bindings,
    prepared: &'a Prepared,
}

impl<'a> Context<'a> {
    pub fn read(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        prepared: &'a Prepared,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Self {
        let native = NativeFacts::new(world, actor, bindings, encounter, targets.len())
            .with_targets(targets.first().copied(), targets);
        Self {
            ret: with_equipment(native, equipment),
            bindings,
            prepared,
        }
    }
}

impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        self.ret.value(name)
    }
}

impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        self.ret.value_for(action, name)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.ret.target_ids(action)
    }

    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.ret.target_ready(action)
    }

    fn primary_resource(&self) -> Result<String, String> {
        // Pinned sc_paladin.cpp4402–4408: Ret's APL primary resource is
        // HolyPower, not the first UI pool. Validate its actual native binding.
        let alias = "holy_power";
        self.ret.native.value(alias)?;
        Ok(alias.into())
    }

    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        let power = self
            .bindings
            .resources
            .get(resource)
            .ok_or_else(|| format!("unbound native Ret pool {resource}"))?;
        Ok(self
            .prepared
            .execution(action, self.ret.equipment)?
            .is_some()
            || self
                .prepared
                .resource_limited(action, self.ret.equipment, *power)?)
    }
}

//! Bind equipped/configured identities and sampled admission to Feral native facts.
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

use super::super::literal_rotation::{equipment::Equipment, execution::Prepared};
use super::facts::FeralFacts;
use crate::simc_apl_native::{Bindings, Encounter};
use crate::spell_cast::powers::PowerPools;

pub(super) struct Context<'a> {
    pub feral: FeralFacts<'a>,
    pub bindings: &'a Bindings,
    pub equipment: &'a Equipment,
    pub prepared: &'a Prepared,
}

impl<'a> Context<'a> {
    pub fn read(
        world: &'a mut World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        prepared: &'a Prepared,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Result<Self, String> {
        Ok(Self {
            feral: FeralFacts::read(world, actor, bindings, encounter, targets)?,
            bindings,
            equipment,
            prepared,
        })
    }
}

impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") {
            return self.equipment.fact(name, &self.feral.native);
        }
        self.feral.value(name)
    }
}

impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") {
            return self.value(name);
        }
        self.feral.value_for(action, name)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.feral.target_ids(action)
    }

    fn primary_resource(&self) -> Result<String, String> {
        let primary = self
            .feral
            .world
            .get::<PowerPools>(self.feral.actor)
            .and_then(|pools| pools.order.first())
            .ok_or("native Feral primary pool is unavailable")?;
        self.bindings
            .resources
            .iter()
            .find_map(|(alias, power)| (power == primary).then(|| alias.clone()))
            .ok_or_else(|| format!("native Feral primary pool {primary:?} is not bound"))
    }

    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        let power = self
            .bindings
            .resources
            .get(resource)
            .ok_or_else(|| format!("unbound native Feral pool {resource}"))?;
        Ok(self.prepared.execution(action, self.equipment)?.is_some()
            || self
                .prepared
                .resource_limited(action, self.equipment, *power)?)
    }
}

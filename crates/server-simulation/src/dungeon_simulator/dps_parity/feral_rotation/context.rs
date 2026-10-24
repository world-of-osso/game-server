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
    pub configured_potion: &'a str,
}

impl<'a> Context<'a> {
    pub fn read(
        world: &'a mut World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        prepared: &'a Prepared,
        configured_potion: &'a str,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Result<Self, String> {
        Ok(Self {
            feral: FeralFacts::read(world, actor, bindings, encounter, targets)?,
            bindings,
            equipment,
            prepared,
            configured_potion,
        })
    }
}

impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") {
            return self.equipment.fact(name, &self.feral.native);
        }
        if let Some(potion) = name.strip_prefix("potion.") {
            // Pinned player.cpp12376–12411: configured identity, not inventory.
            return Ok(
                if configured_potion_matches(self.configured_potion, potion) {
                    1.0
                } else {
                    0.0
                },
            );
        }
        self.feral.value(name)
    }
}

impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("potion.") {
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

pub(super) fn read_configured_potion(
    profile: &super::super::inputs::Profile,
) -> Result<String, String> {
    profile
        .consumables
        .get("potion")
        .cloned()
        .ok_or_else(|| "Feral exact profile lacks explicit potion configuration".into())
}

fn configured_potion_matches(configured: &str, alias: &str) -> bool {
    if configured.eq_ignore_ascii_case(alias) {
        return true;
    }
    // Same single-digit quality suffix as pinned player.cpp12399–12407.
    configured.rsplit_once('_').is_some_and(|(base, quality)| {
        quality.len() == 1
            && quality.as_bytes()[0].is_ascii_digit()
            && base.eq_ignore_ascii_case(alias)
    })
}

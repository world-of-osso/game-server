//! Feral consumer facts; generic target/aura/resource semantics remain shared.
use std::collections::BTreeMap;

use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

use super::geometry::read_area_recipients;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};

pub(super) struct FeralFacts<'a> {
    native: NativeFacts<'a>,
    area_targets: BTreeMap<&'static str, f64>,
}

impl<'a> FeralFacts<'a> {
    pub fn read(
        world: &'a mut World,
        actor: Entity,
        bindings: &'a Bindings,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Result<Self, String> {
        let area_targets = ["swipe_cat", "primal_wrath"]
            .into_iter()
            .map(|alias| {
                let spell = *bindings
                    .actions
                    .get(alias)
                    .ok_or_else(|| format!("native Feral action {alias} is unbound"))?;
                let count = read_area_recipients(world, actor, targets, spell)?;
                Ok((alias, count as f64))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            area_targets,
        })
    }
}

impl Facts for FeralFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "spell_targets" {
            // Pinned action.cpp3886–3896: distance-disabled encounter count.
            return self.native.value("active_enemies");
        }
        if let Some(alias) = name.strip_prefix("spell_targets.") {
            return self
                .area_targets
                .get(alias)
                .copied()
                .ok_or_else(|| format!("unsupported native Feral area action {alias}"));
        }
        self.native.value(name)
    }
}

impl ActionFacts for FeralFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "spell_targets" || name.starts_with("spell_targets.") {
            return self.value(name);
        }
        self.native.value_for(action, name)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
}

//! Devourer expressions read native fragment state and native spell data.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::NativeFacts;
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use server::soul_fragments::{self, FragmentFilter};

pub(super) struct DevourerFacts<'a> {
    pub native: NativeFacts<'a>,
    pub equipment: &'a Equipment,
    pub world: &'a World,
    pub actor: Entity,
}

impl Facts for DevourerFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "active_dot.soul_immolation" {
            // SimC76f8118 sc_demon_hunter.cpp5930–5958: Soul Immolation is a self heal-over-time.
            return Ok(f64::from(
                crate::auras::has_aura(self.world, self.actor, 1241937) as u8,
            ));
        }
        if name == "soul_fragments" {
            return Ok(
                soul_fragments::active_count(self.world, self.actor, FragmentFilter::Any) as f64,
            );
        }
        if name == "action.reap.souls_consumed" {
            // SimC76f8118 sc_demon_hunter.cpp6010-6018,6068-6076.
            let base =
                crate::spell_cast::scripts::script_effect_value(self.world, self.actor, 1227619, 1);
            let extra = if self.native.value("buff.moment_of_craving.up")? != 0.0 {
                crate::spell_cast::scripts::script_effect_value(self.world, self.actor, 1238495, 0)
            } else {
                0.0
            };
            let active = soul_fragments::active_count(self.world, self.actor, FragmentFilter::Any);
            return Ok((active as f64).min(f64::from(base + extra)));
        }
        if name.starts_with("trinket.") {
            return self.equipment.fact(name, &self.native);
        }
        self.native.value(name)
    }
}

impl ActionFacts for DevourerFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name == "active_dot.soul_immolation"
            || name == "soul_fragments"
            || name == "action.reap.souls_consumed"
        {
            return self.value(name);
        }
        self.native.value_for(action, name)
    }
}

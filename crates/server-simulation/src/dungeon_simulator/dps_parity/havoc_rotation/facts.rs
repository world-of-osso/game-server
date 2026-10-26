//! Havoc per-action predicates read the native combat producer's actual uses.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::NativeFacts;
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct HavocFacts<'a> {
    pub native: NativeFacts<'a>,
    pub equipment: &'a Equipment,
    pub world: &'a World,
    pub actor: Entity,
}

impl HavocFacts<'_> {
    fn demonsurge_available(&self, action: &str) -> Result<f64, String> {
        let spell = match action {
            "annihilation" => 201427,
            "death_sweep" => 210152,
            "immolation_aura" => 258920,
            _ => return Err(format!("unknown Havoc Demonsurge action {action}")),
        };
        Ok(
            if crate::spell_cast::script_registry::ability_use_available(
                self.world, self.actor, spell,
            ) {
                1.0
            } else {
                0.0
            },
        )
    }
}

impl Facts for HavocFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if let Some(action) = name
            .strip_prefix("action.")
            .and_then(|name| name.strip_suffix(".demonsurge_available"))
        {
            // SimC76f8118 sc_demon_hunter.cpp2676–2687: query the same
            // per-ability flags consumed by the native damage relationship.
            return self.demonsurge_available(action);
        }
        if name.starts_with("trinket.") {
            return self.equipment.fact(name, &self.native);
        }
        self.native.value(name)
    }
}

impl ActionFacts for HavocFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.ends_with(".demonsurge_available") {
            return self.value(name);
        }
        self.native.value_for(action, name)
    }
}

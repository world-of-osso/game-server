//! Elemental aliases over the shared caster-owned native target/context API.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct EleFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

impl<'a> EleFacts<'a> {
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

    fn chain_targets(&self) -> Result<f64, String> {
        let action = Action {
            name: "chain_lightning".into(),
            target: None,
            options: Default::default(),
        };
        self.native
            .target_ids(&action)
            .map(|targets| targets.len() as f64)
    }
}

impl Facts for EleFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "lightning_rod" => self.native.value("active_dot.lightning_rod"),
            "spell_targets.chain_lightning" => self.chain_targets(),
            _ if name.starts_with("trinket.") || name.starts_with("set_bonus.") => {
                self.equipment.fact(name, &self.native)
            }
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for EleFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "lightning_rod" | "spell_targets.chain_lightning" => self.value(name),
            _ if name.starts_with("trinket.") || name.starts_with("set_bonus.") => self.value(name),
            _ => self.native.value_for(action, name),
        }
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
}

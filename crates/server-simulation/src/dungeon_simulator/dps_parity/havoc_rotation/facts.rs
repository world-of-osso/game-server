//! Havoc's source-defined virtual per-action buff predicates.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::NativeFacts;
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct DemonsurgeUses {
    cursor: usize,
    available: BTreeSet<u32>,
}

impl DemonsurgeUses {
    pub fn observe(&mut self, world: &World, actor: Entity) {
        let selected = |id| {
            world
                .get::<crate::class_progression::TraitLoadout>(actor)
                .is_some_and(|traits| {
                    traits
                        .active_talent_spells()
                        .any(|spell| spell.spell_id == id)
                })
        };
        if !selected(452402) {
            return;
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in events.iter().skip(self.cursor).filter(|event| {
            event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        }) {
            let Some(spell) = event.spell_id else {
                continue;
            };
            // sc_demon_hunter.cpp4965–4981,9168–9169: successful Meta/Demonic
            // arms independent virtual uses; no resource, aura or damage is granted here.
            if spell == 191427 || (matches!(spell, 198013 | 452497) && selected(213410)) {
                self.available.extend([201427, 210152]);
                if spell == 191427 && selected(452415) {
                    self.available.extend([258920, 452497]);
                }
            }
            let action = match spell {
                452486 => 210152,
                452487 => 201427,
                456640 => 258920,
                spell => spell,
            };
            // sc_demon_hunter.cpp12665–12671: consume only that ability's use.
            self.available.remove(&action);
        }
        self.cursor = events.len();
        // sc_demon_hunter.cpp9265–9268: leaving Meta expires the virtual uses.
        if !crate::auras::has_aura(world, actor, 162264) {
            self.available.clear();
        }
    }

    fn available(&self, action: &str) -> Result<f64, String> {
        let id = match action {
            "annihilation" => 201427,
            "death_sweep" => 210152,
            "immolation_aura" => 258920,
            _ => return Err(format!("unknown Havoc Demonsurge action {action}")),
        };
        Ok(if self.available.contains(&id) {
            1.0
        } else {
            0.0
        })
    }
}

pub(super) struct HavocFacts<'a> {
    pub native: NativeFacts<'a>,
    pub equipment: &'a Equipment,
    pub uses: &'a DemonsurgeUses,
}

impl Facts for HavocFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if let Some(action) = name
            .strip_prefix("action.")
            .and_then(|name| name.strip_suffix(".demonsurge_available"))
        {
            // sc_demon_hunter.cpp2676–2687: these are per-action virtual flags.
            return self.uses.available(action);
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

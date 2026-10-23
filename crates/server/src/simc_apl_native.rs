//! Gameplay-world bindings for the engine-free pinned SimC APL evaluator.
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Bindings {
    pub actions: BTreeMap<String, u32>,
    pub buffs: BTreeMap<String, u32>,
    pub talents: BTreeMap<String, u32>,
    pub resources: BTreeMap<String, PowerType>,
    pub global_cooldown_spell: u32,
}

#[derive(Clone, Copy)]
pub(crate) enum FightStyle {
    Patchwerk,
}

#[derive(Clone, Copy)]
pub(crate) enum Encounter {
    Fixture {
        started_at: f64,
        duration: f64,
        style: FightStyle,
    },
    Dungeon {
        started_at: f64,
    },
}

pub(crate) struct NativeFacts<'a> {
    _world: &'a World,
    _actor: Entity,
    _bindings: &'a Bindings,
    _encounter: Encounter,
    _active_enemies: usize,
}

impl<'a> NativeFacts<'a> {
    pub fn new(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        encounter: Encounter,
        active_enemies: usize,
    ) -> Self {
        Self {
            _world: world,
            _actor: actor,
            _bindings: bindings,
            _encounter: encounter,
            _active_enemies: active_enemies,
        }
    }
}

impl Facts for NativeFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        Err(format!("native APL binding not implemented: {name}"))
    }
}

impl ActionFacts for NativeFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        Err(format!(
            "native APL action binding not implemented: {}:{name}",
            action.name
        ))
    }
}

#[cfg(test)]
mod tests;

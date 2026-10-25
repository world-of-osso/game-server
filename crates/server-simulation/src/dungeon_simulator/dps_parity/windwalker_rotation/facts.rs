//! WW-only predicates; shared native facts remain authoritative and unchanged.
use super::super::literal_rotation::equipment::Equipment;
use super::bindings::COMBO_HISTORY;
use crate::simc_apl_native::{Bindings, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct WwFacts<'a> {
    pub native: NativeFacts<'a>,
    pub world: &'a World,
    pub actor: Entity,
    pub bindings: &'a Bindings,
    pub equipment: &'a Equipment,
    pub xuen_active: bool,
    targets: &'a [Entity],
}

impl<'a> WwFacts<'a> {
    pub fn new(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: crate::simc_apl_native::Encounter,
        targets: &'a [Entity],
        xuen_active: bool,
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len()),
            world,
            actor,
            bindings,
            equipment,
            xuen_active,
            targets,
        }
    }
}

impl Facts for WwFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            // SIMC_MISSING_COOLDOWN_PLACEHOLDER, approved exact-token policy.
            // player.cpp12257–12269: typo stays independent of the real WDP CD;
            // an unconfigured external cooldown has timespan::min duration.
            "cooldown.whirling_dragon.remains" => Ok(0.0),
            "cooldown.invoke_power_infusion_0.duration" => Ok(i64::MIN as f64 / 1000.0),
            "combo_strike" => Err("WW combo_strike requires proposed action context".into()),
            "prev.celestial_conduit" => Ok(boolean(
                self.world
                    .get::<crate::spell_cast::SpellHistory>(self.actor)
                    .and_then(|history| history.last_successful_cast())
                    == Some(443028),
            )),
            "pet.xuen_the_white_tiger.active" => Ok(boolean(self.xuen_active)),
            _ if name.starts_with("trinket.") => self.equipment.fact(name, &self.native),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for WwFacts<'_> {
    fn target_ids(&self, _: &Action) -> Result<Vec<u64>, String> {
        Ok(self.targets.iter().map(|target| target.to_bits()).collect())
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "combo_strike" => self.combo_strike(action),
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => self.native.value_for(action, name),
            _ if name.starts_with("target.")
                || name.starts_with("debuff.")
                || name.starts_with("dot.") =>
            {
                self.native.value_for(action, name)
            }
            _ => self.value(name),
        }
    }
}

impl WwFacts<'_> {
    fn combo_strike(&self, action: &Action) -> Result<f64, String> {
        if matches!(action.name.as_str(), "variable" | "call_action_list") {
            return Ok(0.0);
        }
        let id = *self
            .bindings
            .actions
            .get(&action.name)
            .ok_or_else(|| format!("unknown WW combo action {}", action.name))?;
        // RWK/Slicing root actions do not may_combo_strike; their damage children
        // do. Ordinary root identity still separates subsequent other actions.
        if !COMBO_HISTORY.contains(&id) || matches!(id, 467307 | 1217413) {
            return Ok(0.0);
        }
        let previous = crate::spell_cast::last_combo_action(self.world, self.actor);
        Ok(boolean(previous != Some(id)))
    }
}

pub(super) fn read_xuen(world: &mut World, actor: Entity) -> bool {
    let summons: Vec<_> = world
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == actor && summon.spell_id == 123904)
        .map(|(entity, _)| entity)
        .collect();
    summons.into_iter().any(|entity| {
        crate::spell_cast::targets::is_alive(world, entity)
            && crate::spell_cast::targets::same_map(world, actor, entity)
    })
}

fn boolean(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

//! Fire-local recipient gates and actual travel; generic/equipment facts stay shared.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::Health;

pub(super) struct FireFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    world: &'a World,
    actor: Entity,
    bindings: &'a Bindings,
    targets: &'a [Entity],
}

impl<'a> FireFacts<'a> {
    pub fn new(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len()),
            equipment,
            world,
            actor,
            bindings,
            targets,
        }
    }

    fn recipient(&self, action: &Action) -> Result<Entity, String> {
        match action.target {
            Some(bits) => {
                Entity::try_from_bits(bits).ok_or_else(|| format!("invalid Fire recipient {bits}"))
            }
            // Shared Prepared assigns its first supplied target to None actions.
            None => self
                .targets
                .first()
                .copied()
                .ok_or("Fire recipient unavailable".into()),
        }
    }

    fn health_gate(
        &self,
        action: &Action,
        talent: &str,
        effect_index: u8,
        above: bool,
    ) -> Result<f64, String> {
        if self.native.value(&format!("talent.{talent}"))? == 0.0 {
            return Ok(0.0);
        }
        let threshold = self.native_threshold(talent, effect_index)?;
        let target = self.recipient(action)?;
        let health = self
            .world
            .get::<Health>(target)
            .ok_or_else(|| format!("Fire recipient {target:?} lacks Health"))?;
        if !health.max.is_finite() || health.max <= 0.0 || !health.current.is_finite() {
            return Err(format!("Fire recipient {target:?} has invalid Health"));
        }
        // Compare in native Health precision; both inclusive boundaries retain
        // the DB2 threshold without introducing a rounded percentage clock.
        let boundary = health.max * threshold / 100.0;
        let active = if above {
            health.current >= boundary
        } else {
            health.current <= boundary
        };
        Ok(if active { 1.0 } else { 0.0 })
    }

    fn native_threshold(&self, talent: &str, effect_index: u8) -> Result<f32, String> {
        let id = self
            .bindings
            .talents
            .get(talent)
            .ok_or_else(|| format!("Fire talent {talent} is unbound"))?;
        let spell = self
            .world
            .resource::<crate::spell_info::SpellStore>()
            .get(*id)
            .ok_or_else(|| format!("native Fire talent{id} missing"))?;
        spell
            .effects
            .iter()
            .find(|effect| effect.index == effect_index)
            .map(|effect| effect.base_points)
            .ok_or_else(|| format!("native Fire talent{id} effect{effect_index} missing"))
    }
}

impl Facts for FireFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "hot_streak_spells_in_flight" => {
                crate::spell_cast::native_apl::hot_streak_spells_in_flight(self.world, self.actor)
            }
            "firestarter.active" | "scorch_execute.active" => {
                Err("Fire recipient health requires action context".into())
            }
            _ if name.starts_with("trinket.") || name.starts_with("equipped.") => {
                self.equipment.fact(name, &self.native)
            }
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for FireFacts<'_> {
    fn target_ids(&self, _: &Action) -> Result<Vec<u64>, String> {
        Ok(self.targets.iter().map(|target| target.to_bits()).collect())
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            // Native70448 supplies205026 Effect0 /2948 Effect1; no SimC numeric override.
            "firestarter.active" => self.health_gate(action, "firestarter", 0, true),
            "scorch_execute.active" => self.health_gate(action, "scorch", 1, false),
            "hot_streak_spells_in_flight" => self.value(name),
            _ if name.starts_with("trinket.") || name.starts_with("equipped.") => self.value(name),
            _ => self.native.value_for(action, name),
        }
    }
}

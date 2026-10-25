//! Shadow-only configuration; shared operands remain native world queries.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct ShadowFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    world: &'a World,
    actor: Entity,
    targets: &'a [Entity],
}
impl<'a> ShadowFacts<'a> {
    pub fn new(
        world: &'a World,
        actor: Entity,
        targets: &'a [Entity],
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment,
            world,
            actor,
            targets,
        }
    }
}
impl Facts for ShadowFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            // SimC76f8118 sc_priest.cpp options: force_devour_matter defaults false.
            "priest.force_devour_matter" => Ok(0.0),
            // No external provider or ally-heal trigger exists in this isolated fixture.
            "time_to_bloodlust" => Ok(i64::MAX as f64 / 1000.0),
            "buff.twist_of_fate_can_trigger_on_ally_heal.up" => Ok(0.0),
            "potion.liquid_luster" => Ok(0.0),
            "health.pct" => self
                .world
                .get::<shared::components::Health>(self.actor)
                .map(|health| f64::from(health.current) * 100.0 / f64::from(health.max))
                .ok_or("native actor health missing".into()),
            "action.tentacle_slam.enabled" => self.native.value("talent.tentacle_slam"),
            _ if name.starts_with("trinket.")
                || name.starts_with("equipped.")
                || name.starts_with("set_bonus.") =>
            {
                self.equipment.fact(name, &self.native)
            }
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for ShadowFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        if action.name == "variable" {
            return Ok(self.targets.iter().map(|target| target.to_bits()).collect());
        }
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "remains" => self
                .native
                .value_for(action, &format!("dot.{}.remains", action.name)),
            "in_flight" => self
                .native
                .value(&format!("action.{}.in_flight", action.name)),
            "refreshable" => self
                .native
                .value_for(action, &format!("dot.{}.refreshable", action.name)),
            _ if name.starts_with("target.")
                || name.starts_with("dot.")
                || name.starts_with("debuff.")
                || matches!(
                    name,
                    "gcd"
                        | "gcd.max"
                        | "gcd.min"
                        | "gcd.remains"
                        | "execute_time"
                        | "charges"
                        | "ticks_remain"
                        | "ticks"
                        | "full_recharge_time"
                ) =>
            {
                self.native.value_for(action, name)
            }
            _ => self.value(name),
        }
    }
}

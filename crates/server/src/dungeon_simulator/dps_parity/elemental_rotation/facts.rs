//! Elemental target facts read only the native caster's actual aura instances.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};

const FLAME_SHOCK: u32 = 188389;
const LIGHTNING_ROD: u32 = 197209;

pub(super) struct EleFacts<'a> {
    native: NativeFacts<'a>,
    world: &'a World,
    actor: Entity,
    equipment: &'a Equipment,
    targets: &'a [Entity],
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
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len()),
            world,
            actor,
            equipment,
            targets,
        }
    }

    fn default_target(&self) -> Result<Entity, String> {
        self.targets
            .first()
            .copied()
            .ok_or_else(|| "Elemental target fact requires a native enemy".into())
    }

    fn selected_target(&self, action: &Action) -> Result<Entity, String> {
        match action.target {
            None => self.default_target(),
            Some(bits) => self
                .targets
                .iter()
                .copied()
                .find(|target| target.to_bits() == bits)
                .ok_or_else(|| {
                    format!("Elemental target{bits} is not a native encounter candidate")
                }),
        }
    }

    fn owned_aura(
        &self,
        target: Entity,
        spell: u32,
    ) -> Result<Option<&crate::auras::AuraInstance>, String> {
        self.world
            .resource::<crate::spell_info::SpellStore>()
            .get(spell)
            .ok_or_else(|| format!("native Elemental target spell{spell} absent"))?;
        self.world
            .get_entity(target)
            .map_err(|_| format!("native Elemental target{target:?} absent"))?;
        let now = crate::spell_cast::now(self.world);
        Ok(self
            .world
            .get::<crate::auras::ActiveAuras>(target)
            .and_then(|auras| {
                auras.iter().find(|aura| {
                    aura.spell_id == spell
                        && aura.caster == self.actor
                        && aura.expires_at.is_none_or(|at| at > now)
                })
            }))
    }

    fn remains(&self, target: Entity, spell: u32) -> Result<f64, String> {
        let Some(aura) = self.owned_aura(target, spell)? else {
            return Ok(0.0);
        };
        let expiry = aura
            .expires_at
            .ok_or_else(|| format!("finite Elemental aura{spell} lacks native expiry"))?;
        Ok((expiry - crate::spell_cast::now(self.world)).max(0.0))
    }

    fn active_count(&self, spell: u32) -> Result<f64, String> {
        self.targets.iter().try_fold(0.0, |count, &target| {
            Ok(count + boolean(self.owned_aura(target, spell)?.is_some()))
        })
    }

    fn target_fact(&self, target: Entity, name: &str) -> Result<f64, String> {
        match name {
            "dot.flame_shock.remains" => self.remains(target, FLAME_SHOCK),
            "debuff.lightning_rod.remains" => self.remains(target, LIGHTNING_ROD),
            "dot.flame_shock.refreshable" => {
                // Use the next native cast's maximum, not a carried refresh duration.
                let maximum =
                    crate::auras::maximum_duration_ms(self.world, self.actor, target, FLAME_SHOCK)
                        .filter(|duration| *duration > 0)
                        .ok_or("native Flame Shock maximum duration unavailable")?;
                Ok(boolean(
                    self.remains(target, FLAME_SHOCK)? <= f64::from(maximum) * 0.0003,
                ))
            }
            _ => Err(format!("unknown Elemental target operand {name}")),
        }
    }
}

fn boolean(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

impl Facts for EleFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "active_dot.flame_shock" => self.active_count(FLAME_SHOCK),
            "lightning_rod" => self.active_count(LIGHTNING_ROD),
            // Native fixture candidates are the actual encounter enemy set.
            "spell_targets.chain_lightning" => self.native.value("active_enemies"),
            _ if name.starts_with("dot.") || name.starts_with("debuff.") => {
                self.target_fact(self.default_target()?, name)
            }
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
            _ if name.starts_with("dot.") || name.starts_with("debuff.") => {
                self.target_fact(self.selected_target(action)?, name)
            }
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => self.native.value_for(action, name),
            _ => self.value(name),
        }
    }

    fn target_ids(&self, _action: &Action) -> Result<Vec<u64>, String> {
        Ok(self.targets.iter().map(|target| target.to_bits()).collect())
    }
}

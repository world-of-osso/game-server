//! Gameplay-world bindings for the engine-free pinned SimC APL evaluator.
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Bindings {
    pub actions: BTreeMap<String, u32>,
    pub buffs: BTreeMap<String, u32>,
    pub talents: BTreeMap<String, u32>,
    pub resources: BTreeMap<String, PowerType>,
    /// Native spell carrying this actor's global-cooldown category and base duration.
    pub global_cooldown_spell: u32,
}

#[derive(Clone, Copy)]
pub enum FightStyle {
    Patchwerk,
}

/// Encounter start timestamps use the simulation clock, not cooldown wall time.
#[derive(Clone, Copy)]
pub enum Encounter {
    Fixture {
        started_at: f64,
        duration: f64,
        style: FightStyle,
    },
    Dungeon {
        started_at: f64,
    },
}

pub struct NativeFacts<'a> {
    world: &'a World,
    actor: Entity,
    bindings: &'a Bindings,
    encounter: Encounter,
    active_enemies: usize,
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
            world,
            actor,
            bindings,
            encounter,
            active_enemies,
        }
    }

    fn spell(&self, id: u32) -> Result<&crate::spell_info::SpellInfo, String> {
        self.world
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .ok_or_else(|| format!("native APL spell{id} definition absent"))
    }

    fn elapsed(&self) -> f64 {
        let start = match self.encounter {
            Encounter::Fixture { started_at, .. } | Encounter::Dungeon { started_at } => started_at,
        };
        (crate::spell_cast::now(self.world) - start).max(0.0)
    }

    fn remaining(&self, operand: &str) -> Result<f64, String> {
        match self.encounter {
            Encounter::Fixture { duration, .. } => Ok((duration - self.elapsed()).max(0.0)),
            Encounter::Dungeon { .. } => {
                Err(format!("DUNGEON_TTD_UNAVAILABLE: {operand} unavailable"))
            }
        }
    }

    fn fight_style(&self, style: &str) -> Result<f64, String> {
        let matches = match style {
            "patchwerk" => matches!(
                self.encounter,
                Encounter::Fixture {
                    style: FightStyle::Patchwerk,
                    ..
                }
            ),
            "dungeonroute" => matches!(self.encounter, Encounter::Dungeon { .. }),
            "castingpatchwerk" | "dungeonslice" => false,
            _ => return Err(format!("unknown native APL fight_style {style}")),
        };
        Ok(bool_value(matches))
    }

    fn buff(&self, operand: &str) -> Result<f64, String> {
        let (alias, property) = split_property(operand, "buff")?;
        let id = bound_spell(&self.bindings.buffs, alias, "buff")?;
        self.spell(id)?;
        let auras = self
            .world
            .get::<crate::auras::ActiveAuras>(self.actor)
            .ok_or("native APL actor lacks ActiveAuras")?;
        let now = crate::spell_cast::now(self.world);
        let active: Vec<_> = auras
            .iter()
            .filter(|aura| aura.spell_id == id && aura.expires_at.is_none_or(|at| at > now))
            .collect();
        match property {
            "up" => Ok(bool_value(!active.is_empty())),
            "down" => Ok(bool_value(active.is_empty())),
            "stack" => Ok(active
                .iter()
                .map(|aura| f64::from(aura.stacks))
                .fold(0.0, f64::max)),
            "remains" => Ok(buff_remaining(&active, now)),
            "at_max_stacks" => Ok(bool_value(
                active
                    .iter()
                    .any(|aura| aura.stacks >= aura.max_stacks.max(1)),
            )),
            _ => Err(format!("unknown native APL buff property {property}")),
        }
    }

    fn talent(&self, operand: &str) -> Result<f64, String> {
        let (alias, property) = operand.split_once('.').unwrap_or((operand, "enabled"));
        let id = bound_spell(&self.bindings.talents, alias, "talent")?;
        self.spell(id)?;
        let traits = self
            .world
            .get::<crate::class_progression::TraitLoadout>(self.actor)
            .ok_or("native APL selected TraitLoadout unavailable")?;
        let enabled = traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == id);
        match property {
            "enabled" => Ok(bool_value(enabled)),
            "disabled" => Ok(bool_value(!enabled)),
            "rank" => self.talent_rank(traits, id),
            _ => Err(format!("unknown native APL talent property {property}")),
        }
    }

    fn talent_rank(
        &self,
        traits: &crate::class_progression::TraitLoadout,
        spell: u32,
    ) -> Result<f64, String> {
        let classes = self.world.resource::<crate::spell_info::ClassSpells>();
        for entry in traits.source_config().0 {
            let definition = classes
                .classes
                .iter()
                .find_map(|class| class.traits.node(entry.node_id)?.entry(entry.entry_id));
            let definition = definition.ok_or_else(|| {
                format!(
                    "native APL selected trait entry{} definition absent",
                    entry.entry_id
                )
            })?;
            if definition.spell_id == spell {
                return Ok(f64::from(entry.total()));
            }
        }
        Ok(0.0)
    }

    fn cooldown(&self, operand: &str) -> Result<f64, String> {
        let (alias, property) = split_property(operand, "cooldown")?;
        let id = bound_spell(&self.bindings.actions, alias, "action")?;
        crate::spell_cast::native_apl::cooldown_value(
            self.world,
            self.actor,
            self.spell(id)?,
            property,
        )
    }

    fn action_spell(&self, alias: &str) -> Result<&crate::spell_info::SpellInfo, String> {
        let id = bound_spell(&self.bindings.actions, alias, "action")?;
        self.spell(id)
    }

    fn action_property(&self, alias: &str, property: &str) -> Result<f64, String> {
        let spell = self.action_spell(alias)?;
        match property {
            "usable_in" => crate::spell_cast::native_apl::cooldown_value(
                self.world, self.actor, spell, "remains",
            ),
            "ready" => crate::spell_cast::native_apl::action_ready(self.world, self.actor, spell),
            _ => Err(format!("unknown native APL action property {property}")),
        }
    }

    fn spell_haste(&self) -> Result<f64, String> {
        let stats = self
            .world
            .get::<crate::stats::CharacterStats>(self.actor)
            .ok_or("native APL actor lacks derived CharacterStats")?;
        Ok(1.0 / (1.0 + f64::from(stats.haste_pct) / 100.0))
    }

    fn contextual_property(&self, action: &Action, name: &str) -> Result<f64, String> {
        let spell = self.action_spell(&action.name)?;
        match name {
            "charges" | "charges_fractional" | "full_recharge_time" => {
                crate::spell_cast::native_apl::cooldown_value(self.world, self.actor, spell, name)
            }
            "execute_time" => Ok(crate::spell_cast::native_apl::execute_time(
                self.world, self.actor, spell,
            )),
            "ticks_remain" => self.ticks_remaining(spell.id),
            _ => Err(format!("unknown native APL contextual property {name}")),
        }
    }

    fn ticks_remaining(&self, spell: u32) -> Result<f64, String> {
        let target = self
            .world
            .get::<shared::casting::CastState>(self.actor)
            .and_then(|cast| Entity::try_from_bits(cast.target))
            .or_else(|| {
                self.world
                    .get::<crate::networking::CombatTarget>(self.actor)
                    .and_then(|target| target.0)
            });
        let Some(target) = target else {
            return Err("native APL ticks_remain requires an actual target".into());
        };
        crate::auras::remaining_periodic_ticks(self.world, self.actor, target, spell).map(f64::from)
    }

    fn resource(&self, operand: &str) -> Result<f64, String> {
        let (alias, property) = operand.split_once('.').unwrap_or((operand, "current"));
        let power = self
            .bindings
            .resources
            .get(alias)
            .ok_or_else(|| format!("unknown native APL operand {operand}"))?;
        let (current, maximum, display) = self.read_displayed_resource(*power)?;
        match property {
            "current" => Ok(current),
            "max" => Ok(maximum),
            "deficit" => Ok(maximum - current),
            "pct" => Ok(current / maximum * 100.0),
            "regen" => self.regen(*power).map(|rate| rate / display),
            "time_to_max" if current >= maximum => Ok(0.0),
            "time_to_max" => self
                .regen(*power)
                .map(|rate| (maximum - current) / (rate / display)),
            _ => Err(format!("unknown native APL resource property {property}")),
        }
    }

    fn read_displayed_resource(&self, power: PowerType) -> Result<(f64, f64, f64), String> {
        let (current, maximum) =
            crate::spell_cast::powers::power_value(self.world, self.actor, power)
                .ok_or_else(|| format!("native APL actor lacks resource {power:?}"))?;
        let table = self
            .world
            .resource::<crate::spell_cast::powers::PowerTypeTable>();
        let info = table
            .get(power)
            .ok_or_else(|| format!("native APL power metadata absent: {power:?}"))?;
        if info.display_modifier <= 0 {
            return Err(format!("native APL invalid display modifier for {power:?}"));
        }
        let display = f64::from(info.display_modifier);
        Ok((
            f64::from(current) / display,
            f64::from(maximum) / display,
            display,
        ))
    }

    fn regen(&self, power: PowerType) -> Result<f64, String> {
        crate::spell_cast::powers::potential_power_regen(self.world, self.actor, power)
            .map(f64::from)
            .ok_or_else(|| format!("native APL potential regen unavailable for {power:?}"))
    }

    fn gcd(&self, action: &Action, operand: &str) -> Result<f64, String> {
        let control = matches!(action.name.as_str(), "variable" | "call_action_list");
        if control && matches!(operand, "gcd" | "gcd.min" | "gcd.remains") {
            return Ok(0.0);
        }
        let spell = if operand == "gcd.max" {
            self.spell(self.bindings.global_cooldown_spell)?
        } else {
            let id = bound_spell(&self.bindings.actions, &action.name, "action")?;
            self.spell(id)?
        };
        crate::spell_cast::native_apl::gcd_value(self.world, self.actor, spell, operand)
    }
}

impl Facts for NativeFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "time" => Ok(self.elapsed()),
            "fight_remains" | "target.time_to_die" => self.remaining(name),
            "active_enemies" => Ok(self.active_enemies as f64),
            "spell_haste" => self.spell_haste(),
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" | "charges" | "charges_fractional"
            | "full_recharge_time" | "execute_time" | "ticks_remain" => {
                Err(format!("native APL {name} requires action context"))
            }
            _ => {
                if let Some(value) = name.strip_prefix("buff.") {
                    return self.buff(value);
                }
                if let Some(value) = name.strip_prefix("talent.") {
                    return self.talent(value);
                }
                if let Some(value) = name.strip_prefix("cooldown.") {
                    return self.cooldown(value);
                }
                if let Some(value) = name.strip_prefix("action.") {
                    let (alias, property) = split_property(value, "action")?;
                    return self.action_property(alias, property);
                }
                if let Some(value) = name.strip_prefix("fight_style.") {
                    return self.fight_style(value);
                }
                self.resource(name)
            }
        }
    }
}

impl ActionFacts for NativeFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => self.gcd(action, name),
            "charges" | "charges_fractional" | "full_recharge_time" | "execute_time"
            | "ticks_remain" => self.contextual_property(action, name),
            _ => self.value(name),
        }
    }
}

fn bound_spell(bindings: &BTreeMap<String, u32>, alias: &str, kind: &str) -> Result<u32, String> {
    bindings
        .get(alias)
        .copied()
        .ok_or_else(|| format!("unknown native APL {kind} alias {alias}"))
}

fn split_property<'a>(operand: &'a str, kind: &str) -> Result<(&'a str, &'a str), String> {
    operand
        .rsplit_once('.')
        .ok_or_else(|| format!("native APL {kind} requires a property: {operand}"))
}

fn buff_remaining(auras: &[&crate::auras::AuraInstance], now: f64) -> f64 {
    // buff.cpp2018–2028/expressions.hpp135–138: permanent is the signed64
    // millisecond sentinel, not an invented finite timer.
    auras
        .iter()
        .map(|aura| {
            aura.expires_at
                .map_or(i64::MIN as f64 / 1000.0, |at| (at - now).max(0.0))
        })
        .reduce(f64::max)
        .unwrap_or(0.0)
}

fn bool_value(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests;

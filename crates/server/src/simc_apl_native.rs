//! Gameplay-world bindings for the engine-free pinned SimC APL evaluator.
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;
use std::collections::BTreeMap;
use std::time::Duration;

/// Explicit fixture lifetime/role; never changes the native creature template.
#[derive(Component, Clone, Copy)]
pub(crate) struct FixtureTarget {
    pub expires_at: Duration,
    pub lifetime: Duration,
    pub is_boss: bool,
}

impl FixtureTarget {
    pub fn remaining(self, now: Duration) -> Duration {
        self.expires_at.saturating_sub(now)
    }
}

/// Actual recurring-add state, updated when native entities retire/spawn.
#[derive(Component, Clone, Copy)]
pub(crate) struct FixtureAdds {
    pub next_spawn_at: Duration,
    pub expires_at: Option<Duration>,
    pub interval: Duration,
    pub lifetime: Duration,
    pub count: usize,
}

#[derive(Default)]
pub(crate) struct Bindings {
    pub actions: BTreeMap<String, u32>,
    pub buffs: BTreeMap<String, u32>,
    pub talents: BTreeMap<String, u32>,
    pub resources: BTreeMap<String, PowerType>,
    /// Native spell carrying this actor's global-cooldown category and base duration.
    pub global_cooldown_spell: u32,
}

#[derive(Clone, Copy)]
pub(crate) enum FightStyle {
    Patchwerk,
}

/// Encounter start timestamps use the simulation clock, not cooldown wall time.
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

    fn selected_target(&self, action: Option<&Action>) -> Result<Entity, String> {
        let target = match action.and_then(|action| action.target) {
            Some(bits) => Entity::try_from_bits(bits),
            None => {
                // Fixture followers use their real player's native selection.
                let selector = self
                    .world
                    .get::<crate::followers::Follower>(self.actor)
                    .map_or(self.actor, |follower| follower.owner);
                self.world
                    .get::<crate::networking::CombatTarget>(selector)
                    .and_then(|target| target.0)
            }
        }
        .ok_or("NATIVE_APL_TARGET_UNAVAILABLE: no selected entity")?;
        self.world
            .get_entity(target)
            .map_err(|_| "NATIVE_APL_TARGET_UNAVAILABLE: expired entity".to_owned())?;
        Ok(target)
    }

    fn target_fact(&self, property: &str, action: Option<&Action>) -> Result<f64, String> {
        if property == "time_to_die" && matches!(self.encounter, Encounter::Dungeon { .. }) {
            return Err("DUNGEON_TTD_UNAVAILABLE: target.time_to_die unavailable".into());
        }
        let target = self.selected_target(action)?;
        match property {
            "health.pct" => self.target_health_pct(target),
            "time_to_die" | "is_boss" => {
                let metadata = self
                    .world
                    .get::<FixtureTarget>(target)
                    .ok_or("NATIVE_FIXTURE_TARGET_UNAVAILABLE: explicit lifetime/role missing")?;
                if property == "is_boss" {
                    return Ok(bool_value(metadata.is_boss));
                }
                Ok(metadata
                    .remaining(self.world.resource::<Time>().elapsed())
                    .as_secs_f64())
            }
            _ => Err(format!("unknown native APL target property {property}")),
        }
    }

    fn target_health_pct(&self, target: Entity) -> Result<f64, String> {
        let health = self
            .world
            .get::<shared::components::Health>(target)
            .ok_or("native target lacks Health")?;
        if health.max <= 0.0 {
            return Err("native target has invalid maximum Health".into());
        }
        Ok(f64::from(health.current) * 100.0 / f64::from(health.max))
    }

    fn target_aura(&self, operand: &str, action: Option<&Action>) -> Result<f64, String> {
        let (alias, property) = split_property(operand, "target aura")?;
        let id = bound_spell(&self.bindings.buffs, alias, "target aura")?;
        self.spell(id)?;
        let target = self.selected_target(action)?;
        let now = crate::spell_cast::now(self.world);
        let active: Vec<_> = self
            .world
            .get::<crate::auras::ActiveAuras>(target)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .filter(|aura| {
                aura.spell_id == id
                    && aura.caster == self.actor
                    && aura.expires_at.is_none_or(|at| at > now)
            })
            .collect();
        match property {
            "up" | "ticking" => Ok(bool_value(!active.is_empty())),
            "down" => Ok(bool_value(active.is_empty())),
            "remains" => Ok(buff_remaining(&active, now)),
            "stack" => Ok(active
                .iter()
                .map(|aura| f64::from(aura.stacks))
                .fold(0.0, f64::max)),
            _ => Err(format!("unknown native target-aura property {property}")),
        }
    }

    fn adds(&self, property: &str) -> Result<f64, String> {
        if matches!(self.encounter, Encounter::Dungeon { .. }) {
            return Err(format!(
                "DUNGEON_TTD_UNAVAILABLE: raid_event.adds.{property} unavailable"
            ));
        }
        fixture_add_value(
            self.world.get::<FixtureAdds>(self.actor),
            property,
            self.world.resource::<Time>().elapsed(),
        )
    }

    fn targeted_value(&self, name: &str, action: Option<&Action>) -> Option<Result<f64, String>> {
        if let Some(property) = name.strip_prefix("target.") {
            return Some(self.target_fact(property, action));
        }
        name.strip_prefix("debuff.")
            .or_else(|| name.strip_prefix("dot."))
            .map(|operand| self.target_aura(operand, action))
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
            _ => Err(format!("unknown native APL talent property {property}")),
        }
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
        let pools = self
            .world
            .get::<crate::spell_cast::powers::PowerPools>(self.actor)
            .ok_or("native APL actor lacks PowerPools")?;
        if power == PowerType::Mana {
            return Ok(f64::from(pools.mana_regen_per_sec));
        }
        pools
            .get(power)
            .map(|pool| f64::from(pool.regen_per_sec))
            .ok_or_else(|| format!("native APL actor lacks regen pool {power:?}"))
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
        if let Some(value) = self.targeted_value(name, None) {
            return value;
        }
        if let Some(property) = name.strip_prefix("raid_event.adds.") {
            return self.adds(property);
        }
        match name {
            "time" => Ok(self.elapsed()),
            "fight_remains" => self.remaining(name),
            "active_enemies" => Ok(self.active_enemies as f64),
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => {
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
        if let Some(value) = self.targeted_value(name, Some(action)) {
            return value;
        }
        match name {
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => self.gcd(action, name),
            _ => self.value(name),
        }
    }
}

fn fixture_add_value(
    adds: Option<&FixtureAdds>,
    property: &str,
    now: Duration,
) -> Result<f64, String> {
    // raid_event.cpp3068–3141: no event / no positive next time uses max timespan.
    let never = i64::MAX as f64 / 1000.0;
    Ok(match property {
        "exists" => bool_value(adds.is_some()),
        "remains" => adds
            .and_then(|adds| adds.expires_at)
            .map_or(0.0, |at| at.saturating_sub(now).as_secs_f64()),
        "in" => adds
            .map(|adds| adds.next_spawn_at.saturating_sub(now))
            .filter(|remaining| !remaining.is_zero())
            .map_or(never, |remaining| remaining.as_secs_f64()),
        "cooldown" => adds.map_or(never, |adds| adds.interval.as_secs_f64()),
        "duration" => adds.map_or(0.0, |adds| adds.lifetime.as_secs_f64()),
        "count" => adds.map_or(0.0, |adds| adds.count as f64),
        _ => return Err(format!("unknown native add-event property {property}")),
    })
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
mod target_tests;
#[cfg(test)]
mod tests;

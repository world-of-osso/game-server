//! Gameplay-world bindings for the engine-free pinned SimC APL evaluator.
pub mod reaction;
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;
use std::collections::BTreeMap;
use std::time::Duration;

/// Explicit fixture lifetime/role; never changes the native creature template.
#[derive(Component, Clone, Copy)]
pub struct FixtureTarget {
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
pub struct FixtureAdds {
    pub next_spawn_at: Duration,
    pub expires_at: Option<Duration>,
    pub interval: Duration,
    pub lifetime: Duration,
    pub count: usize,
}

/// Explicit SimC fixture movement state, independent of native target geometry.
#[derive(Component, Clone, Copy)]
pub struct FixtureMovement {
    pub distance_to_move: f64,
}

pub type ResolveDotSpell = fn(&World, Entity, u32) -> Result<crate::spell_info::SpellInfo, String>;

#[derive(Clone, Copy)]
pub enum DotRefreshPolicy {
    Pandemic,
    FinalTick,
    Always,
    Never,
}

pub struct DotRefresh {
    pub resolve: ResolveDotSpell,
    pub policy: DotRefreshPolicy,
}

pub fn native_dot_spell(
    world: &World,
    _caster: Entity,
    spell: u32,
) -> Result<crate::spell_info::SpellInfo, String> {
    world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .cloned()
        .ok_or_else(|| format!("native dot spell{spell} absent"))
}

#[derive(Default)]
pub struct Bindings {
    pub actions: BTreeMap<String, u32>,
    pub buffs: BTreeMap<String, u32>,
    /// Fresh native metadata resolver and source-qualified refresh rule, by aura alias.
    pub dot_refresh: BTreeMap<String, DotRefresh>,
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

#[derive(Clone, Copy)]
struct TargetContext<'a> {
    current: Option<Entity>,
    ordered: &'a [Entity],
}

pub struct NativeFacts<'a> {
    world: &'a World,
    actor: Entity,
    bindings: &'a Bindings,
    encounter: Encounter,
    active_enemies: usize,
    targets: Option<TargetContext<'a>>,
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
            targets: None,
        }
    }

    /// Explicit fixture/controller target context; native entities, never a count oracle.
    pub fn with_targets(mut self, current: Option<Entity>, ordered: &'a [Entity]) -> Self {
        self.targets = Some(TargetContext { current, ordered });
        self.active_enemies = ordered.len();
        self
    }

    fn ordered_targets(&self) -> Result<&'a [Entity], String> {
        self.targets.map(|context| context.ordered).ok_or_else(|| {
            "NATIVE_APL_TARGETS_UNAVAILABLE: explicit native universe missing".into()
        })
    }

    fn read_current_target(&self) -> Option<Entity> {
        if let Some(context) = self.targets {
            return context.current;
        }
        // Unconfigured controllers still read their actual native selection.
        let selector = self
            .world
            .get::<crate::followers::Follower>(self.actor)
            .map_or(self.actor, |follower| follower.owner);
        self.world
            .get::<crate::networking::CombatTarget>(selector)
            .and_then(|target| target.0)
    }

    fn candidate_ready(
        &self,
        spell: Option<&crate::spell_info::SpellInfo>,
        target: Entity,
    ) -> Result<bool, String> {
        if !crate::spell_cast::targets::is_alive(self.world, target)
            || !crate::spell_cast::targets::same_map(self.world, self.actor, target)
        {
            return Ok(false);
        }
        match spell {
            Some(spell) => crate::spell_cast::native_apl::target_eligible(
                self.world, self.actor, spell, target,
            ),
            None => Ok(true),
        }
    }

    pub fn cooldown_for_spell(
        &self,
        spell: &crate::spell_info::SpellInfo,
        property: &str,
    ) -> Result<f64, String> {
        crate::spell_cast::cooldown_query::cooldown_value(self.world, self.actor, spell, property)
    }

    pub fn equipment_proc_cooldown(
        &self,
        spell: u32,
        slot: shared::protocol::EquipmentSlot,
    ) -> f64 {
        crate::auras::equipment_proc_cooldown_remaining(self.world, self.actor, slot, spell)
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

    fn expected_combat_length(&self) -> Result<f64, String> {
        match self.encounter {
            Encounter::Fixture { duration, .. } if duration.is_finite() && duration > 0.0 => {
                Ok(duration)
            }
            Encounter::Fixture { duration, .. } => Err(format!(
                "EXPECTED_COMBAT_LENGTH_UNAVAILABLE: fixture duration must be positive and finite, got {duration}"
            )),
            Encounter::Dungeon { .. } => Err(
                "EXPECTED_COMBAT_LENGTH_UNAVAILABLE: dungeon has no configured fight duration"
                    .into(),
            ),
        }
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
            None => self.read_current_target(),
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
            // SimC76f8118 hunter_marksmanship.simc18 scores target.health;
            // the native candidate supplies its actual current pool.
            "health" => self
                .world
                .get::<shared::components::Health>(target)
                .map(|health| f64::from(health.current))
                .ok_or("native target lacks Health".into()),
            "health.pct" => self.target_health_pct(target),
            "debuff.casting.up" => Ok(bool_value(
                self.world
                    .get::<shared::casting::CastState>(target)
                    .is_some(),
            )),
            "debuff.casting.react" => self.casting_reaction(target),
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

    fn casting_reaction(&self, target: Entity) -> Result<f64, String> {
        if self
            .world
            .get::<shared::casting::CastState>(target)
            .is_some()
        {
            return Err("NATIVE_CASTING_REACTION_UNAVAILABLE: active cast has no qualified reaction snapshot".into());
        }
        Ok(0.0)
    }

    fn read_owned_auras(&self, target: Entity, spell: u32) -> Vec<&crate::auras::AuraInstance> {
        let now = crate::spell_cast::now(self.world);
        self.world
            .get::<crate::auras::ActiveAuras>(target)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .filter(|aura| {
                aura.spell_id == spell
                    && aura.caster == self.actor
                    && aura.expires_at.is_none_or(|at| at > now)
            })
            .collect()
    }

    fn active_dot(&self, alias: &str) -> Result<f64, String> {
        let spell = bound_spell(&self.bindings.buffs, alias, "dot")?;
        self.spell(spell)?;
        Ok(self
            .ordered_targets()?
            .iter()
            .filter(|&&target| {
                crate::spell_cast::targets::is_alive(self.world, target)
                    && !self.read_owned_auras(target, spell).is_empty()
            })
            .count() as f64)
    }

    fn dot_value(&self, operand: &str, action: Option<&Action>) -> Result<f64, String> {
        let (alias, property) = split_property(operand, "dot")?;
        if property != "refreshable" {
            return self.target_aura(operand, action);
        }
        self.dot_refreshable(alias, self.selected_target(action)?)
    }

    fn dot_refreshable(&self, alias: &str, target: Entity) -> Result<f64, String> {
        let spell = bound_spell(&self.bindings.buffs, alias, "dot")?;
        self.spell(spell)?;
        let metadata = self.bindings.dot_refresh.get(alias).ok_or_else(|| {
            format!("NATIVE_DOT_REFRESH_UNAVAILABLE: {alias} has no native resolver/policy")
        })?;
        let active = self.read_owned_auras(target, spell);
        if active.is_empty() {
            return Ok(1.0);
        }
        match metadata.policy {
            DotRefreshPolicy::Always => Ok(1.0),
            DotRefreshPolicy::Never => Ok(0.0),
            DotRefreshPolicy::FinalTick => {
                crate::auras::remaining_periodic_ticks(self.world, self.actor, target, spell)
                    .map(|ticks| bool_value(ticks <= 1))
            }
            DotRefreshPolicy::Pandemic => self.pandemic_refreshable(
                metadata.resolve,
                spell,
                target,
                buff_remaining(&active, crate::spell_cast::now(self.world)),
            ),
        }
    }

    fn pandemic_refreshable(
        &self,
        resolve: ResolveDotSpell,
        spell: u32,
        target: Entity,
        remains: f64,
    ) -> Result<f64, String> {
        const PANDEMIC_FRACTION: f64 = 0.30; // action.cpp4630–4635.
        let resolved = resolve(self.world, self.actor, spell)?;
        if resolved.id != spell {
            return Err("native dot resolver returned a different spell identity".into());
        }
        let duration = crate::auras::fresh_duration_ms(self.world, self.actor, target, &resolved);
        if duration == 0 {
            return Err("native dot fresh duration unavailable/permanent".into());
        }
        Ok(bool_value(
            remains <= f64::from(duration) / 1000.0 * PANDEMIC_FRACTION,
        ))
    }

    fn target_aura(&self, operand: &str, action: Option<&Action>) -> Result<f64, String> {
        let (alias, property) = split_property(operand, "target aura")?;
        let id = bound_spell(&self.bindings.buffs, alias, "target aura")?;
        self.spell(id)?;
        let target = self.selected_target(action)?;
        let now = crate::spell_cast::now(self.world);
        let active = self.read_owned_auras(target, id);
        match property {
            "up" | "ticking" => Ok(bool_value(!active.is_empty())),
            "down" => Ok(bool_value(active.is_empty())),
            "remains" => Ok(buff_remaining(&active, now)),
            "stack" => Ok(active
                .iter()
                .map(|aura| f64::from(aura.stacks))
                .fold(0.0, f64::max)),
            // SimC76f8118 buff.cpp485–508/2004–2015: target procs use the
            // same observed stack deadlines as self buffs, on their actual recipient.
            "react" => reaction::reacted_stacks(self.world, target, id, &active),
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

    fn movement_distance(&self) -> Result<f64, String> {
        if matches!(self.encounter, Encounter::Dungeon { .. }) {
            return Err("DUNGEON_MOVEMENT_DISTANCE_UNAVAILABLE: movement.distance".into());
        }
        self.world
            .get::<FixtureMovement>(self.actor)
            .map(|movement| movement.distance_to_move)
            .ok_or_else(|| {
                "NATIVE_MOVEMENT_DISTANCE_UNAVAILABLE: configure fixture distance_to_move".into()
            })
    }

    fn movement_value(&self, property: &str) -> Result<f64, String> {
        let distance = self.movement_distance()?;
        let run = self
            .world
            .get::<crate::creature_ai::SpawnPoint>(self.actor)
            .map_or(1.0, |spawn| spawn.speed_run);
        let modifier = self
            .world
            .get::<crate::auras::AuraStatMods>(self.actor)
            .map_or(1.0, |mods| mods.movement_speed);
        let speed = f64::from(crate::creature_ai::BASE_RUN_SPEED * run * modifier);
        match property {
            "speed" => Ok(speed),
            "remains" if distance > 0.0 => Ok(distance / speed),
            // All admitted fixtures configure no movement event/buff.
            "remains" => Ok(0.0),
            _ => Err(format!("unknown native movement property {property}")),
        }
    }

    fn movement_event(&self, property: &str) -> Result<f64, String> {
        if matches!(self.encounter, Encounter::Dungeon { .. }) {
            return Err(format!(
                "DUNGEON_TTD_UNAVAILABLE: raid_event.movement.{property} unavailable"
            ));
        }
        if !matches!(
            property,
            "exists" | "remains" | "in" | "cooldown" | "duration"
        ) {
            return Err(format!("unknown native movement-event property {property}"));
        }
        // This explicit dummy fixture configures no movement raid event.
        fixture_add_value(None, property, self.world.resource::<Time>().elapsed())
    }

    fn targeted_value(&self, name: &str, action: Option<&Action>) -> Option<Result<f64, String>> {
        if let Some(property) = name.strip_prefix("target.") {
            return Some(self.target_fact(property, action));
        }
        if let Some(operand) = name.strip_prefix("dot.") {
            return Some(self.dot_value(operand, action));
        }
        name.strip_prefix("debuff.")
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
            "react" => reaction::reacted_stacks(self.world, self.actor, id, &active),
            "duration" => Ok(f64::from(crate::auras::fresh_duration_ms(
                self.world,
                self.actor,
                self.actor,
                self.spell(id)?,
            )) / 1000.0),
            // SimC76f8118 buff.cpp450–455: limit exists before the first proc.
            "max_stack" => Ok(f64::from(
                crate::auras::fresh_max_stacks(self.world, self.actor, self.spell(id)?).max(1),
            )),
            "at_max_stacks" => Ok(bool_value(
                active
                    .iter()
                    .any(|aura| aura.stacks >= aura.max_stacks.max(1)),
            )),
            _ => Err(format!("unknown native APL buff property {property}")),
        }
    }

    fn hero_tree(&self, alias: &str) -> Result<f64, String> {
        // SimC76f8118 player.cpp12209–12224: selected hero tree, not learned spells.
        let classes = self.world.resource::<crate::spell_info::ClassSpells>();
        let tree = classes
            .classes
            .iter()
            .flat_map(|class| &class.traits.sub_trees)
            .find(|tree| tree.name.to_ascii_lowercase().replace(' ', "_") == alias)
            .ok_or_else(|| format!("unknown native APL hero tree {alias}"))?;
        let traits = self
            .world
            .get::<crate::class_progression::TraitLoadout>(self.actor)
            .ok_or("native APL selected TraitLoadout unavailable")?;
        Ok(bool_value(
            traits.selected_sub_tree(classes) == Some(tree.id),
        ))
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
        self.cooldown_for_spell(self.spell(id)?, property)
    }

    fn action_spell(&self, alias: &str) -> Result<&crate::spell_info::SpellInfo, String> {
        let id = bound_spell(&self.bindings.actions, alias, "action")?;
        self.spell(id)
    }

    fn action_property(
        &self,
        alias: &str,
        property: &str,
        action: Option<&Action>,
    ) -> Result<f64, String> {
        let spell = self.action_spell(alias)?;
        match property {
            // SimC76f8118 action.cpp3502-3503: snapshot one target and average crit.
            "damage" => crate::spell_cast::effects::expected_direct_damage(
                self.world,
                self.actor,
                self.selected_target(action)?,
                spell,
            )
            .map(f64::from),
            // SimC76f8118 action.cpp3287-3288 and dot.cpp448 expose metadata, not remains.
            "cooldown" => self.cooldown_for_spell(spell, "duration"),
            "duration" => Ok(f64::from(crate::auras::fresh_duration_ms(
                self.world, self.actor, self.actor, spell,
            )) / 1000.0),
            // SimC76f8118 action.cpp3590: action.<name>.channeling reflects the
            // actor's actual active channel, never an aura or a normal cast.
            "channeling" => Ok(bool_value(
                self.world
                    .get::<shared::casting::CastState>(self.actor)
                    .is_some_and(|cast| {
                        cast.spell_id == spell.id
                            && cast.cast_type == shared::casting::CastType::Channel
                    }),
            )),
            "in_flight" | "in_flight_remains" => {
                crate::spell_cast::native_apl::flight_value(self.world, self.actor, spell, property)
            }
            "usable_in" => crate::spell_cast::cooldown_query::cooldown_value(
                self.world, self.actor, spell, "remains",
            ),
            "ready" => crate::spell_cast::native_apl::action_ready(self.world, self.actor, spell),
            "execute_time" => Ok(crate::spell_cast::native_apl::execute_time(
                self.world, self.actor, spell,
            )),
            // SimC76f8118 action.cpp3263–3273: cast time excludes GCD occupancy.
            "cast_time" => Ok(f64::from(
                crate::spell_cast::cast_time_ms(self.world, self.actor, spell).max(0),
            ) / 1000.0),
            "cost" => self.action_cost(spell),
            _ => Err(format!("unknown native APL action property {property}")),
        }
    }

    fn action_cost(&self, spell: &crate::spell_info::SpellInfo) -> Result<f64, String> {
        // SimC76f8118 action.cpp1192–1230: current-resource cost, not affordability.
        let costs = crate::spell_cast::applicable_costs(self.world, self.actor, spell)
            .map_err(|error| format!("native APL spell{} cost: {error:?}", spell.id))?;
        let Some(&(pool, amount, _)) = costs.first() else {
            return Ok(0.0);
        };
        let display = match pool {
            crate::spell_cast::CostPool::Health => 1,
            crate::spell_cast::CostPool::Power(power) => {
                self.world
                    .resource::<crate::spell_cast::powers::PowerTypeTable>()
                    .get(power)
                    .ok_or_else(|| format!("native APL cost metadata absent: {power:?}"))?
                    .display_modifier
            }
        };
        if display <= 0 {
            return Err("native APL cost display modifier must be positive".into());
        }
        Ok(f64::from(amount) / f64::from(display))
    }

    fn pet(&self, operand: &str) -> Result<f64, String> {
        let (alias, property) = split_property(operand, "pet")?;
        let spell = bound_spell(&self.bindings.actions, alias, "pet summon")?;
        self.spell(spell)?;
        if property != "active" {
            return Err(format!("unknown native APL pet property {property}"));
        }
        let now = crate::spell_cast::now(self.world);
        // SimC76f8118 player.cpp12458-12460: active reads an actual awake owned pet.
        Ok(bool_value(self.world.iter_entities().any(|entity| {
            entity
                .get::<crate::spell_cast::summon::SummonedCreature>()
                .is_some_and(|summon| {
                    summon.owner == self.actor
                        && summon.spell_id == spell
                        && summon.expires_at.is_none_or(|expires| expires > now)
                        && crate::spell_cast::targets::is_alive(self.world, entity.id())
                        && crate::spell_cast::targets::same_map(self.world, self.actor, entity.id())
                })
        })))
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
                crate::spell_cast::cooldown_query::cooldown_value(
                    self.world, self.actor, spell, name,
                )
            }
            "execute_time" => Ok(crate::spell_cast::native_apl::execute_time(
                self.world, self.actor, spell,
            )),
            "cast_time" | "cost" => self.action_property(&action.name, name, Some(action)),
            "ticks_remain" => self.ticks_remaining(spell.id),
            // SimC76f8118 dot.cpp540: ticking refers to this proposed action's dot.
            "ticking" => {
                let target = self.selected_target(Some(action))?;
                Ok(bool_value(
                    !self.read_owned_auras(target, spell.id).is_empty(),
                ))
            }
            _ => Err(format!("unknown native APL contextual property {name}")),
        }
    }

    fn ticks_remaining(&self, spell: u32) -> Result<f64, String> {
        let target = match self.world.get::<shared::casting::CastState>(self.actor) {
            Some(cast) => Entity::try_from_bits(cast.target)
                .ok_or("native APL ticks_remain has an invalid actual cast target")?,
            None => self.selected_target(None)?,
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
        if let Some(value) = self.targeted_value(name, None) {
            return value;
        }
        if let Some(property) = name.strip_prefix("raid_event.pull.") {
            // SimC76f8118 raid_event.cpp3068–3141: no configured event.
            // Patchwerk fixtures have no pull events; dungeon schedules stay explicit.
            if matches!(self.encounter, Encounter::Dungeon { .. }) {
                return Err(format!("DUNGEON_PULL_EVENT_UNAVAILABLE: {name}"));
            }
            return fixture_add_value(None, property, self.world.resource::<Time>().elapsed());
        }
        if let Some(property) = name.strip_prefix("raid_event.adds.") {
            return self.adds(property);
        }
        if let Some(property) = name.strip_prefix("raid_event.movement.") {
            return self.movement_event(property);
        }
        if let Some(alias) = name.strip_prefix("active_dot.") {
            return self.active_dot(alias);
        }
        // SimC76f8118 action.cpp3817–3896: this named action's available
        // targets, not the currently proposed action's target count.
        if let Some(alias) = name.strip_prefix("spell_targets.") {
            let action = Action {
                name: alias.to_owned(),
                source_row: None,
                target: None,
                options: Default::default(),
            };
            return self.target_ids(&action).map(|targets| targets.len() as f64);
        }
        match name {
            "time" => Ok(self.elapsed()),
            // dot.cpp671: tick_time is the channel's current native tick period.
            "tick_time" => self
                .world
                .get::<shared::casting::CastState>(self.actor)
                .filter(|cast| cast.cast_type == shared::casting::CastType::Channel)
                .map(|cast| f64::from(cast.channel_tick_interval))
                .ok_or_else(|| "tick_time requires an active native channel".into()),
            "expected_combat_length" => self.expected_combat_length(),
            "fight_remains" => self.remaining(name),
            "active_enemies" => Ok(self.active_enemies as f64),
            "spell_haste" => self.spell_haste(),
            "movement.distance" => self.movement_distance(),
            "movement.remains" => self.movement_value("remains"),
            "movement.speed" => self.movement_value("speed"),
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" | "charges" | "charges_fractional"
            | "full_recharge_time" | "execute_time" | "ticks_remain" | "ticking" => {
                Err(format!("native APL {name} requires action context"))
            }
            _ => {
                if let Some(value) = name.strip_prefix("buff.") {
                    return self.buff(value);
                }
                if let Some(value) = name.strip_prefix("hero_tree.") {
                    return self.hero_tree(value);
                }
                if let Some(value) = name.strip_prefix("talent.") {
                    return self.talent(value);
                }
                if let Some(value) = name.strip_prefix("cooldown.") {
                    return self.cooldown(value);
                }
                if let Some(value) = name.strip_prefix("action.") {
                    let (alias, property) = split_property(value, "action")?;
                    return self.action_property(alias, property, None);
                }
                if let Some(value) = name.strip_prefix("pet.") {
                    return self.pet(value);
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
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        // SimC76f8118 hunter_marksmanship.simc18: retarget scores real
        // candidates without requiring a corresponding spell definition.
        let spell = match action.name.as_str() {
            "run_action_list"
            | "call_action_list"
            | "auto_attack"
            | "retarget_auto_attack"
            | "retarget" => None,
            _ => Some(self.action_spell(&action.name)?),
        };
        self.ordered_targets()?
            .iter()
            .filter_map(|&target| match self.candidate_ready(spell, target) {
                Ok(true) => Some(Ok(target.to_bits())),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if let Some(value) = self.targeted_value(name, Some(action)) {
            return value;
        }
        if let Some(value) = name.strip_prefix("action.") {
            let (alias, property) = split_property(value, "action")?;
            return self.action_property(alias, property, Some(action));
        }
        match name {
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => self.gcd(action, name),
            "charges" | "charges_fractional" | "full_recharge_time" | "execute_time"
            | "cast_time" | "cost" | "ticks_remain" | "ticking" => {
                self.contextual_property(action, name)
            }
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
        // raid_event.cpp3097–3108: configured events are up only during an active wave.
        "up" => bool_value(
            adds.and_then(|adds| adds.expires_at)
                .is_some_and(|at| at > now),
        ),
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
mod expected_length_tests;
#[cfg(test)]
mod movement_distance_tests;
#[cfg(test)]
mod refresh_context_tests;
#[cfg(test)]
mod target_context_tests;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;

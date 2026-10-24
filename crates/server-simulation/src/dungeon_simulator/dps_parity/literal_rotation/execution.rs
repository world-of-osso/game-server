//! Native eligibility is sampled before immutable expression evaluation.
use super::equipment::{Equipment, ItemUse};
use bevy::prelude::*;
use server::simc_apl::Action;
use shared::components::PowerType;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "execution_options_tests.rs"]
mod options_tests;

#[derive(Clone)]
pub(crate) enum Execution {
    Snapshot,
    AutoAttack(Entity),
    RetargetAutoAttack(Entity),
    Spell(u32, Entity),
    EmpoweredSpell(u32, Entity, u8),
    ReplacingSpell(u32, Entity),
    Item(ItemUse, Entity),
}

pub(crate) struct NativeAction<'a> {
    pub name: &'a str,
    pub spell: u32,
    pub talent: Option<u32>,
    pub self_target: bool,
}

// None is the action's native default target, not an explicit-target substitute.
type ActionKey = (String, Option<Entity>);
type ReadyActions = BTreeMap<ActionKey, Execution>;

struct NativeReadiness {
    ready: bool,
    limited: Option<PowerType>,
}

pub(crate) struct Prepared {
    ready: ReadyActions,
    limits: BTreeMap<ActionKey, PowerType>,
    declared: BTreeSet<String>,
}

impl Prepared {
    pub fn read<'a>(
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        equipment: &Equipment,
        actions: impl IntoIterator<Item = NativeAction<'a>>,
        needs_snapshot: bool,
    ) -> Result<Self, String> {
        Self::read_with_replacement(
            world,
            actor,
            targets,
            equipment,
            actions,
            needs_snapshot,
            false,
        )
    }

    fn read_with_replacement<'a>(
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        equipment: &Equipment,
        actions: impl IntoIterator<Item = NativeAction<'a>>,
        needs_snapshot: bool,
        replace_current: bool,
    ) -> Result<Self, String> {
        let targets = native_targets(world, actor, targets);
        let mut prepared = Self {
            ready: BTreeMap::new(),
            limits: BTreeMap::new(),
            declared: ["snapshot_stats", "auto_attack", "retarget_auto_attack"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };
        if needs_snapshot {
            prepared
                .ready
                .insert(("snapshot_stats".into(), None), Execution::Snapshot);
        }
        read_attacks(world, actor, &targets, &mut prepared.ready);
        read_spells(
            world,
            actor,
            &targets,
            actions,
            &mut prepared,
            replace_current,
        )?;
        for item in equipment.uses() {
            read_item(world, actor, &targets, item, &mut prepared)?;
        }
        Ok(prepared)
    }

    /// Sample replacement admission without mutating or stopping the current cast.
    pub fn read_interrupting_self<'a>(
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        equipment: &Equipment,
        actions: impl IntoIterator<Item = NativeAction<'a>>,
        needs_snapshot: bool,
    ) -> Result<Self, String> {
        Self::read_with_replacement(
            world,
            actor,
            targets,
            equipment,
            actions,
            needs_snapshot,
            true,
        )
    }

    /// True only when this selected pool at its native maximum would admit the
    /// action. Other pools/health and every other native check remain unchanged.
    pub fn resource_limited(
        &self,
        action: &Action,
        equipment: &Equipment,
        resource: PowerType,
    ) -> Result<bool, String> {
        let Some(key) = action_key(action, equipment)? else {
            return Ok(false);
        };
        if !self.declared.contains(&key.0) {
            return Err(format!(
                "unknown native resource-query action {}",
                action.name
            ));
        }
        Ok(self
            .limits
            .get(&key)
            .is_some_and(|power| *power == resource))
    }

    pub fn execution(
        &self,
        action: &Action,
        equipment: &Equipment,
    ) -> Result<Option<Execution>, String> {
        let Some(key) = action_key(action, equipment)? else {
            return Ok(None);
        };
        self.ready
            .get(&key)
            .cloned()
            .map(|execution| select_empower_stage(action, execution))
            .transpose()
    }
}

fn select_empower_stage(action: &Action, execution: Execution) -> Result<Execution, String> {
    let Some(source) = action.options.get("empower_to") else {
        return Ok(execution);
    };
    let Execution::Spell(id, target) = execution else {
        return Err("empower_to requires an ordinary native spell selection".into());
    };
    let stage = source
        .parse::<u8>()
        .map_err(|error| format!("invalid literal empower_to={source}: {error}"))?;
    Ok(Execution::EmpoweredSpell(id, target, stage))
}

fn action_key(action: &Action, equipment: &Equipment) -> Result<Option<ActionKey>, String> {
    let name = if action.name == "use_item" {
        let Some(slot) = equipment.slot_for_action(action)? else {
            return Ok(None);
        };
        format!("item:{slot}")
    } else {
        action.name.clone()
    };
    let target = action
        .target
        .map(|bits| {
            Entity::try_from_bits(bits)
                .ok_or_else(|| format!("invalid explicit literal target {bits}"))
        })
        .transpose()?;
    Ok(Some((name, target)))
}

fn native_targets(world: &World, actor: Entity, supplied: &[Entity]) -> Vec<Entity> {
    if !supplied.is_empty() {
        return supplied.to_vec();
    }
    // A harmful precast still uses the actor's actual selected enemy, even
    // before the adapter supplies its combat candidate order.
    world
        .get::<crate::networking::CombatTarget>(actor)
        .and_then(|target| target.0)
        .into_iter()
        .collect()
}

fn record_ready(
    ready: &mut ReadyActions,
    name: &str,
    target: Entity,
    default: bool,
    execution: Execution,
) {
    ready.insert((name.into(), Some(target)), execution.clone());
    if default {
        ready.insert((name.into(), None), execution);
    }
}

fn read_attacks(world: &World, actor: Entity, targets: &[Entity], ready: &mut ReadyActions) {
    let current = world.get::<crate::combat::AutoAttack>(actor);
    for (index, &target) in targets.iter().enumerate() {
        if current.is_none() {
            record_ready(
                ready,
                "auto_attack",
                target,
                index == 0,
                Execution::AutoAttack(target),
            );
        }
        // SimC76f8118 player.cpp10824–10860: only a running, different
        // weapon victim can be retargeted; the actor's selection stays intact.
        if current.is_some_and(|attack| attack.victim != target) {
            record_ready(
                ready,
                "retarget_auto_attack",
                target,
                index == 0,
                Execution::RetargetAutoAttack(target),
            );
        }
    }
}

fn record_probe(
    prepared: &mut Prepared,
    key: ActionKey,
    default: bool,
    state: NativeReadiness,
    execution: Execution,
) {
    if state.ready {
        prepared.ready.insert(key.clone(), execution.clone());
        if default {
            prepared.ready.insert((key.0.clone(), None), execution);
        }
    }
    if let Some(power) = state.limited {
        prepared.limits.insert(key.clone(), power);
        if default {
            prepared.limits.insert((key.0, None), power);
        }
    }
}

fn read_spells<'a>(
    world: &mut World,
    actor: Entity,
    enemies: &[Entity],
    actions: impl IntoIterator<Item = NativeAction<'a>>,
    prepared: &mut Prepared,
    replace_current: bool,
) -> Result<(), String> {
    for action in actions {
        read_spell(world, actor, enemies, action, prepared, replace_current)?;
    }
    Ok(())
}

fn read_spell(
    world: &mut World,
    actor: Entity,
    enemies: &[Entity],
    action: NativeAction<'_>,
    prepared: &mut Prepared,
    replace_current: bool,
) -> Result<(), String> {
    let NativeAction {
        name,
        spell: id,
        talent,
        self_target,
    } = action;
    prepared.declared.insert(name.into());
    if !talent_permits(world, actor, talent)? {
        return Ok(());
    }
    let Some(id) = resolve_action(world, actor, id)? else {
        return Ok(());
    };
    let targets = if self_target {
        std::slice::from_ref(&actor)
    } else {
        enemies
    };
    for (index, &target) in targets.iter().enumerate() {
        let state = native_readiness(world, actor, id, target, None, replace_current)?;
        let execution = if replace_current {
            Execution::ReplacingSpell(id, target)
        } else {
            Execution::Spell(id, target)
        };
        record_probe(
            prepared,
            (name.into(), Some(target)),
            index == 0,
            state,
            execution,
        );
    }
    Ok(())
}

fn read_item(
    world: &mut World,
    actor: Entity,
    targets: &[Entity],
    item: (&str, ItemUse),
    prepared: &mut Prepared,
) -> Result<(), String> {
    let (slot, item) = item;
    let name = format!("item:{slot}");
    prepared.declared.insert(name.clone());
    for target in std::iter::once(actor).chain(targets.iter().copied()) {
        let state = native_readiness(
            world,
            actor,
            item.on_use.spell_id,
            target,
            Some(item),
            false,
        )?;
        record_probe(
            prepared,
            (name.clone(), Some(target)),
            target == actor,
            state,
            Execution::Item(item, target),
        );
    }
    Ok(())
}

fn talent_permits(world: &World, actor: Entity, talent: Option<u32>) -> Result<bool, String> {
    let Some(talent) = talent else {
        return Ok(true);
    };
    let traits = world
        .get::<crate::class_progression::TraitLoadout>(actor)
        .ok_or("native selected TraitLoadout absent")?;
    Ok(traits
        .active_talent_spells()
        .any(|spell| spell.spell_id == talent))
}

pub(crate) fn resolve_action(world: &World, actor: Entity, id: u32) -> Result<Option<u32>, String> {
    crate::spell_cast::knowledge::resolve_optional_actor_action(world, actor, id)
        .map_err(|reason| format!("native literal action knowledge: {reason:?}"))
}

fn native_readiness(
    world: &mut World,
    actor: Entity,
    id: u32,
    target: Entity,
    item: Option<ItemUse>,
    replace_current: bool,
) -> Result<NativeReadiness, String> {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .cloned()
        .ok_or_else(|| format!("native literal spell{id} absent"))?;
    if let Some(item) = item {
        spell.cast_item_level = Some(item.item_level);
        spell.cast_retail_item_id = Some(item.item_id);
    }
    let admitted = if replace_current {
        crate::spell_cast::read_native_cast_start_interrupting_self(world, actor, &spell, target)
    } else {
        crate::spell_cast::read_native_cast_start(world, actor, &spell, target)
    };
    match admitted {
        Ok(_) => Ok(NativeReadiness {
            ready: true,
            limited: None,
        }),
        Err(crate::spell_cast::CastFailure::NotEnoughPower(power)) => {
            match crate::spell_cast::read_native_cast_at_resource_max(
                world, actor, &spell, target, power,
            ) {
                Ok(_) => Ok(NativeReadiness {
                    ready: false,
                    limited: Some(power),
                }),
                Err(reason) => refused_admission(id, reason),
            }
        }
        Err(reason) => refused_admission(id, reason),
    }
}

fn refused_admission(
    id: u32,
    reason: crate::spell_cast::CastFailure,
) -> Result<NativeReadiness, String> {
    match reason {
        reason @ (crate::spell_cast::CastFailure::UnknownSpell
        | crate::spell_cast::CastFailure::Unsupported(_)) => Err(format!(
            "NATIVE_LITERAL_ADMISSION_GAP: spell{id}: {reason:?}"
        )),
        _ => Ok(NativeReadiness {
            ready: false,
            limited: None,
        }),
    }
}

pub(crate) fn execute(world: &mut World, actor: Entity, action: Execution) -> Result<(), String> {
    match action {
        Execution::Snapshot => Err("snapshot execution belongs to the native stat recorder".into()),
        Execution::AutoAttack(target) => {
            let current = world.get::<crate::combat::AutoAttack>(actor).cloned();
            crate::combat::start_attack(&mut world.commands(), actor, target, current.as_ref());
            world.flush();
            world
                .entity_mut(actor)
                .remove::<crate::creature_ai::FollowOwner>()
                .insert(crate::networking::CombatTarget(Some(target)));
            // Preserve native chase/facing when the literal fixture replaces the
            // kit controller. TC ChaseMovementGenerator::Update SetInFront;
            // without Pursuing, caster-facing cones miss the selected victim.
            if let Some(mut state) = world.get_mut::<crate::creature_ai::CreatureState>(actor) {
                *state = crate::creature_ai::CreatureState::Pursuing {
                    target: target.to_bits(),
                };
            }
            Ok(())
        }
        Execution::RetargetAutoAttack(target) => {
            if world.get_entity(target).is_err() {
                return Err("selected native AA target no longer exists".into());
            }
            let mut attack = world
                .get_mut::<crate::combat::AutoAttack>(actor)
                .ok_or("native AA stopped after retarget readiness")?;
            attack.victim = target;
            Ok(())
        }
        Execution::Spell(id, target) => {
            let result = if world.get::<shared::components::Player>(actor).is_some() {
                crate::spell_cast::cast_for_player_with_target(world, actor, id, Some(target))
            } else {
                crate::spell_cast::creature_cast(world, actor, id, Some(target), false)
            };
            result
                .map_err(|reason| format!("selected native literal spell{id} rejected: {reason:?}"))
        }
        Execution::EmpoweredSpell(id, target, stage) => {
            crate::spell_cast::creature_cast_empowered(world, actor, id, Some(target), stage)
                .map_err(|reason| format!("selected native empower spell{id} rejected: {reason:?}"))
        }
        Execution::ReplacingSpell(id, target) => {
            crate::spell_cast::creature_cast_interrupting_self(world, actor, id, Some(target))
                .map_err(|reason| {
                    format!("selected native replacement spell{id} rejected: {reason:?}")
                })
        }
        Execution::Item(item, target) => crate::spell_cast::cast_item_spell(
            world,
            actor,
            item.on_use.spell_id,
            Some(target),
            item.on_use.cooldown,
            item.item_level,
            Some(item.item_id),
        )
        .map_err(|reason| {
            format!(
                "selected native literal item{} rejected: {reason:?}",
                item.item_id
            )
        }),
    }
}

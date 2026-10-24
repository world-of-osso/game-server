//! Native eligibility is sampled before immutable expression evaluation.
use super::equipment::{Equipment, ItemUse};
use bevy::prelude::*;
use server::simc_apl::Action;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) enum Execution {
    Snapshot,
    AutoAttack(Entity),
    RetargetAutoAttack(Entity),
    Spell(u32, Entity),
    Item(ItemUse, Entity),
    Composite(Vec<Execution>),
}

#[derive(Clone, Copy)]
pub(crate) struct NativeAction<'a> {
    pub name: &'a str,
    pub spell: u32,
    pub talent: Option<u32>,
    pub self_target: bool,
}

pub(crate) struct NativeComposite<'a> {
    pub name: &'a str,
    pub steps: &'a [NativeAction<'a>],
}

// None is the action's native default target, not an explicit-target substitute.
type ReadyActions = BTreeMap<(String, Option<Entity>), Execution>;

pub(crate) struct Prepared {
    ready: ReadyActions,
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
        let targets = native_targets(world, actor, targets);
        let mut ready = BTreeMap::new();
        if needs_snapshot {
            ready.insert(("snapshot_stats".into(), None), Execution::Snapshot);
        }
        read_attacks(world, actor, &targets, &mut ready);
        read_spells(world, actor, &targets, actions, &mut ready)?;
        for (slot, item) in equipment.uses() {
            for target in std::iter::once(actor).chain(targets.iter().copied()) {
                if native_ready(world, actor, item.on_use.spell_id, target, Some(item))? {
                    record_ready(
                        &mut ready,
                        &format!("item:{slot}"),
                        target,
                        target == actor,
                        Execution::Item(item, target),
                    );
                }
            }
        }
        Ok(Self { ready })
    }

    /// Plans only; the precombat executor owns clock advancement and each native cast.
    pub fn with_composites<'a>(
        self,
        _world: &mut World,
        _actor: Entity,
        _targets: &[Entity],
        _composites: impl IntoIterator<Item = NativeComposite<'a>>,
    ) -> Result<Self, String> {
        Ok(self)
    }

    pub fn execution(
        &self,
        action: &Action,
        equipment: &Equipment,
    ) -> Result<Option<Execution>, String> {
        let key = if action.name == "use_item" {
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
        Ok(self.ready.get(&(key, target)).cloned())
    }
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

fn read_spells<'a>(
    world: &mut World,
    actor: Entity,
    enemies: &[Entity],
    actions: impl IntoIterator<Item = NativeAction<'a>>,
    ready: &mut ReadyActions,
) -> Result<(), String> {
    for NativeAction {
        name,
        spell: id,
        talent,
        self_target,
    } in actions
    {
        if !talent_permits(world, actor, talent)? || !knows_action(world, actor, id)? {
            continue;
        }
        let targets = if self_target {
            std::slice::from_ref(&actor)
        } else {
            enemies
        };
        for (index, &target) in targets.iter().enumerate() {
            if native_ready(world, actor, id, target, None)? {
                record_ready(
                    ready,
                    name,
                    target,
                    index == 0,
                    Execution::Spell(id, target),
                );
            }
        }
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

fn knows_action(world: &World, actor: Entity, id: u32) -> Result<bool, String> {
    let known = world
        .get::<crate::class_progression::KnownSpells>(actor)
        .ok_or("native literal actor lacks KnownSpells")?;
    if crate::auras::overrides::resolve_spell(world, actor, id) != id {
        return Ok(false);
    }
    Ok(known.contains(id)
        || known
            .iter()
            .any(|base| crate::auras::overrides::resolve_spell(world, actor, base) == id))
}

fn native_ready(
    world: &mut World,
    actor: Entity,
    id: u32,
    target: Entity,
    item: Option<ItemUse>,
) -> Result<bool, String> {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .cloned()
        .ok_or_else(|| format!("native literal spell{id} absent"))?;
    if let Some(item) = item {
        spell.cast_item_level = Some(item.item_level);
        spell.cast_retail_item_id = Some(item.item_id);
    }
    match crate::spell_cast::read_native_cast_start(world, actor, &spell, target) {
        Ok(_) => Ok(true),
        Err(
            reason @ (crate::spell_cast::CastFailure::UnknownSpell
            | crate::spell_cast::CastFailure::Unsupported(_)),
        ) => Err(format!(
            "NATIVE_LITERAL_ADMISSION_GAP: spell{id}: {reason:?}"
        )),
        Err(_) => Ok(false), // Actual native refusal (power/GCD/range/aura/etc).
    }
}

pub(crate) fn execute(world: &mut World, actor: Entity, action: Execution) -> Result<(), String> {
    match action {
        Execution::Snapshot => Err("snapshot execution belongs to the native stat recorder".into()),
        Execution::Composite(_) => {
            Err("composite execution requires an ordered precombat executor".into())
        }
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

//! Native eligibility is sampled before immutable expression evaluation.
use super::equipment::{Equipment, ItemUse};
use bevy::prelude::*;
use server::simc_apl::Action;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) enum Execution {
    Snapshot,
    AutoAttack(Entity),
    Spell(u32, Entity),
    Item(ItemUse, Entity),
}

pub(crate) struct NativeAction<'a> {
    pub name: &'a str,
    pub spell: u32,
    pub talent: Option<u32>,
    pub self_target: bool,
}

pub(crate) struct Prepared {
    ready: BTreeMap<String, Execution>,
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
        let mut ready = BTreeMap::new();
        if needs_snapshot {
            ready.insert("snapshot_stats".into(), Execution::Snapshot);
        }
        if let Some(&target) = targets.first() {
            if world
                .get::<crate::combat::AutoAttack>(actor)
                .is_none_or(|attack| attack.victim != target)
            {
                ready.insert("auto_attack".into(), Execution::AutoAttack(target));
            }
        }
        read_spells(world, actor, targets.first().copied(), actions, &mut ready)?;
        for (slot, item) in equipment.uses() {
            if native_ready(world, actor, item.on_use.spell_id, actor, Some(item))? {
                ready.insert(format!("item:{slot}"), Execution::Item(item, actor));
            }
        }
        Ok(Self { ready })
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
        Ok(self.ready.get(&key).cloned())
    }
}

fn read_spells<'a>(
    world: &mut World,
    actor: Entity,
    enemy: Option<Entity>,
    actions: impl IntoIterator<Item = NativeAction<'a>>,
    ready: &mut BTreeMap<String, Execution>,
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
        let target = if self_target { Some(actor) } else { enemy };
        let Some(target) = target else {
            continue;
        };
        if native_ready(world, actor, id, target, None)? {
            ready.insert(name.into(), Execution::Spell(id, target));
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
        Execution::AutoAttack(target) => {
            let current = world.get::<crate::combat::AutoAttack>(actor).cloned();
            crate::combat::start_attack(&mut world.commands(), actor, target, current.as_ref());
            world.flush();
            world
                .entity_mut(actor)
                .remove::<crate::creature_ai::FollowOwner>()
                .insert(crate::networking::CombatTarget(Some(target)));
            Ok(())
        }
        Execution::Spell(id, target) => {
            crate::spell_cast::creature_cast(world, actor, id, Some(target), false)
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

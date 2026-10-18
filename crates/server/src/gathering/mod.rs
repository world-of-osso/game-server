//! Retail gathering uses existing cast, personal loot, profession and pool systems.
//! Sources: TC a352b1fa CanOpenLock, EffectSkinning, UpdateGatherSkill; local DB2/TDB.
mod data;
#[cfg(test)]
mod tests;

use crate::maps::InMap;
use crate::profession::PlayerProfessions;
use crate::spell_cast::{CastFailure, effects::EffectCtx};
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;
use data::{Ability, OPEN_LOCK, SKIN, SKINNING};
pub(crate) use data::{GatheringData, load_data};
use rand::Rng;
use shared::components::{CombatStatus, Health, Npc, Player, Position};
use shared::protocol::{GameObjectInfo, GatherableResource};
use std::collections::HashSet;

#[derive(Component, Clone)]
pub(crate) struct NodeSource(pub crate::game_objects::GameObjectSpawn);
#[derive(Component, Default)]
struct NodeClaims(HashSet<u64>);
#[derive(Component)]
struct Depletion(f32);
#[derive(Component)]
struct NodeRespawn {
    source: NodeSource,
    remaining: f32,
}
#[derive(Component)]
struct Skinned;

pub(crate) struct GatheringPlugin;
impl Plugin for GatheringPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GatheringData>()
            .add_systems(Update, sync_skinning_corpses)
            .add_systems(FixedUpdate, update_node_timers);
    }
}

pub(crate) fn is_gathering(spell: &SpellInfo) -> bool {
    spell
        .effects
        .iter()
        .any(|effect| matches!(effect.effect, OPEN_LOCK | SKIN))
}

pub(crate) fn spawn_node(
    world: &mut World,
    spawn: &crate::game_objects::GameObjectSpawn,
    map: InMap,
) -> Result<Entity, String> {
    let parent =
        node_parent(world.resource::<GatheringData>(), spawn.info.entry).ok_or_else(|| {
            format!(
                "GATHERING GAP: node {} has no authored DB2 gathering skill",
                spawn.info.entry
            )
        })?;
    Ok(world
        .spawn((
            crate::game_objects::game_object_bundle(spawn),
            map,
            lightyear::prelude::Replicate::to_clients(lightyear::prelude::NetworkTarget::All),
            crate::interest::NetworkVisibility::default(),
            NodeSource(spawn.clone()),
            NodeClaims::default(),
            GatherableResource {
                parent_skill: parent,
            },
        ))
        .id())
}

fn node_parent(data: &GatheringData, entry: u32) -> Option<u32> {
    let node = data.nodes.get(&entry)?;
    let locks = data.locks.get(&node.lock)?;
    locks.iter().find_map(|requirement| {
        data.abilities
            .iter()
            .find(|ability| {
                ability.effect == OPEN_LOCK && ability.lock_type == requirement.lock_type
            })
            .map(|ability| ability.parent)
    })
}

pub(crate) fn spawn_selected_node(
    world: &mut World,
    spawn: crate::game_objects::GameObjectSpawn,
    map: InMap,
) {
    let pooled = world
        .resource::<crate::world_data::WorldData>()
        .pools
        .gameobject_pool(spawn.guid)
        .is_some();
    if pooled
        && !world
            .resource::<crate::spawn_pools::PoolSpawns>()
            .0
            .get(&map)
            .expect("gathering map pool state must precede node spawning")
            .spawns
            .gameobject_spawned(spawn.guid)
    {
        return;
    }
    let entity = match spawn_node(world, &spawn, map) {
        Ok(entity) => entity,
        Err(gap) => {
            warn!("{gap}");
            return;
        }
    };
    if pooled {
        world
            .resource_mut::<crate::spawn_pools::PoolSpawns>()
            .0
            .get_mut(&map)
            .expect("node pool state")
            .gameobjects
            .insert(spawn.guid, entity);
    }
}

fn sync_skinning_corpses(world: &mut World) {
    let mut query = world.query_filtered::<(Entity, &Npc, &InMap), (
        With<crate::combat::Corpse>,
        Without<Skinned>,
        Without<GatherableResource>,
    )>();
    let corpses = query
        .iter(world)
        .filter(|(entity, _, _)| crate::loot::corpse_loot_is_empty(world, *entity))
        .map(|(entity, npc, map)| (entity, npc.template_id, *map))
        .collect::<Vec<_>>();
    for (entity, entry, map) in corpses {
        let difficulty = world
            .get_resource::<crate::spawn_pools::PoolSpawns>()
            .and_then(|pools| pools.0.get(&map))
            .map_or(0, |pool| pool.difficulty_id);
        if world
            .resource::<GatheringData>()
            .skins
            .contains_key(&(entry, difficulty))
        {
            world.entity_mut(entity).insert(GatherableResource {
                parent_skill: SKINNING,
            });
        }
    }
}

fn choose_ability(world: &World, caster: Entity, target: Entity) -> Result<Ability, CastFailure> {
    let profs = world
        .get::<PlayerProfessions>(caster)
        .ok_or(CastFailure::NotKnown)?;
    let data = world.resource::<GatheringData>();
    if let Some(info) = world.get::<GameObjectInfo>(target) {
        let node = data
            .nodes
            .get(&info.entry)
            .ok_or(CastFailure::InvalidTarget)?;
        let locks = data
            .locks
            .get(&node.lock)
            .ok_or(CastFailure::InvalidTarget)?;
        return locks
            .iter()
            .find_map(|requirement| {
                data.abilities
                    .iter()
                    .find(|ability| {
                        ability.effect == OPEN_LOCK
                            && ability.lock_type == requirement.lock_type
                            && profs.rank(ability.skill) > 0
                    })
                    .cloned()
            })
            .ok_or(CastFailure::NotKnown);
    }
    let skin = skin_template(world, target)?;
    data.abilities
        .iter()
        .find(|ability| ability.spell == skin.spell && profs.rank(skin.skill) > 0)
        .cloned()
        .ok_or(CastFailure::NotKnown)
}

fn skin_template(world: &World, target: Entity) -> Result<data::SkinTemplate, CastFailure> {
    let npc = world.get::<Npc>(target).ok_or(CastFailure::InvalidTarget)?;
    let map = world
        .get::<InMap>(target)
        .ok_or(CastFailure::InvalidTarget)?;
    let difficulty = world
        .get_resource::<crate::spawn_pools::PoolSpawns>()
        .and_then(|pools| pools.0.get(map))
        .map_or(0, |pool| pool.difficulty_id);
    world
        .resource::<GatheringData>()
        .skins
        .get(&(npc.template_id, difficulty))
        .copied()
        .ok_or(CastFailure::InvalidTarget)
}

pub(crate) fn use_resource(world: &mut World, caster: Entity, target: Entity) {
    match choose_ability(world, caster, target) {
        Ok(ability) => {
            let _ = crate::spell_cast::cast_granted(world, caster, ability.spell, Some(target));
        }
        Err(failure) => crate::spell_cast::send_cast_rejection(world, caster, 0, &failure),
    }
}

pub(crate) fn check_target(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
) -> Result<(), CastFailure> {
    if !crate::spell_cast::targets::same_map(world, caster, target)
        || world.get::<GatherableResource>(target).is_none()
    {
        return Err(CastFailure::InvalidTarget);
    }
    let pending_materials = world.get::<crate::loot::ItemLoot>(caster).is_some()
        && world
            .get::<crate::loot::CorpseLoot>(caster)
            .is_some_and(|loot| {
                loot.looters
                    .values()
                    .any(|slots| slots.iter().any(Option::is_some))
            });
    if pending_materials {
        return Err(CastFailure::InProgress);
    }
    let from = world
        .get::<Position>(caster)
        .ok_or(CastFailure::InvalidTarget)?;
    let to = world
        .get::<Position>(target)
        .ok_or(CastFailure::InvalidTarget)?;
    let in_reach = if let Some(bounds) = world.get::<crate::game_objects::GameObjectBounds>(target)
    {
        bounds.reaches(
            crate::game_objects::wow_position(to),
            crate::game_objects::wow_position(from),
            crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
        )
    } else {
        Vec3::new(from.x, from.y, from.z).distance(Vec3::new(to.x, to.y, to.z))
            <= crate::npc_interaction::INTERACTION_DISTANCE
    };
    if !in_reach {
        return Err(CastFailure::OutOfRange);
    }
    let ability = choose_ability(world, caster, target)?;
    if ability.spell != spell.id {
        return Err(CastFailure::InvalidTarget);
    }
    if ability.effect == OPEN_LOCK {
        check_node(world, caster, target, &ability)
    } else {
        check_skin(world, target)
    }
}

fn check_node(
    world: &World,
    caster: Entity,
    target: Entity,
    ability: &Ability,
) -> Result<(), CastFailure> {
    let info = world
        .get::<GameObjectInfo>(target)
        .ok_or(CastFailure::InvalidTarget)?;
    let data = world.resource::<GatheringData>();
    let node = &data.nodes[&info.entry];
    if node.condition != 0 {
        return Err(CastFailure::Unsupported(format!(
            "Gathering PlayerCondition {} is not implemented",
            node.condition
        )));
    }
    if node.require_los && !crate::line_of_sight::sees_in_world(world, caster, target) {
        return Err(CastFailure::LineOfSight);
    }
    if node.no_combat
        && world
            .get::<CombatStatus>(caster)
            .is_some_and(|status| status.0)
    {
        return Err(CastFailure::AffectingCombat);
    }
    let id = world
        .get::<crate::persistence::ClientId>(caster)
        .ok_or(CastFailure::InvalidTarget)?
        .0;
    if world
        .get::<NodeClaims>(target)
        .is_some_and(|claims| claims.0.contains(&id) || claims.0.len() >= node.max_loots as usize)
    {
        return Err(CastFailure::InvalidTarget);
    }
    let required = data.locks[&node.lock]
        .iter()
        .find(|requirement| requirement.lock_type == ability.lock_type)
        .ok_or(CastFailure::InvalidTarget)?;
    if world
        .get::<PlayerProfessions>(caster)
        .expect("selected learned ability")
        .rank(ability.skill)
        < required.rank
    {
        return Err(CastFailure::NotKnown);
    }
    if !world
        .resource::<crate::loot::LootTables>()
        .gameobject
        .contains_key(&node.loot)
    {
        return Err(CastFailure::Unsupported(format!(
            "Gathering loot {} is unavailable",
            node.loot
        )));
    }
    Ok(())
}

fn check_skin(world: &World, target: Entity) -> Result<(), CastFailure> {
    if world.get::<Skinned>(target).is_some()
        || world
            .get::<Health>(target)
            .is_none_or(|health| health.current > 0.0)
    {
        return Err(CastFailure::InvalidTarget);
    }
    if !crate::loot::corpse_loot_is_empty(world, target) {
        return Err(CastFailure::Unsupported(
            "Corpse must be looted before Skinning".into(),
        ));
    }
    let skin = skin_template(world, target)?;
    if !world
        .resource::<crate::loot::LootTables>()
        .skinning
        .contains_key(&skin.loot)
    {
        return Err(CastFailure::Unsupported(format!(
            "Skinning loot {} is unavailable",
            skin.loot
        )));
    }
    Ok(())
}

pub(crate) fn effect_gather(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &SpellEffect,
    target: Entity,
) {
    if ctx.spell.id == shared::battleground::ALTERAC_OPENING_SPELL
        && crate::pvp::is_alterac_banner(world, target)
    {
        crate::pvp::complete_opening(world, ctx.caster, target);
        return;
    }
    if ctx.spell.id == shared::battleground::AZERITE_GATHERING_SPELL {
        crate::pvp::capture_azerite(world, ctx.caster, target);
        return;
    }
    // Cast completion revalidates; repeat at the mutation boundary for competing casts.
    if let Err(error) = check_target(world, ctx.caster, ctx.spell, target) {
        error!("Gathering completion refused: {error:?}");
        return;
    }
    let ability = choose_ability(world, ctx.caster, target).expect("validated gather ability");
    let (loot, progress) = gather_progress(world, ctx.caster, target, &ability);
    let mut rng = rand::thread_rng();
    let rolled = world.resource::<crate::loot::LootTables>().roll_gathering(
        loot,
        ability.effect == SKIN,
        world.resource::<crate::item_templates::ItemTemplates>(),
        &mut rng,
    );
    let id = world
        .get::<crate::persistence::ClientId>(ctx.caster)
        .expect("validated character")
        .0;
    let kind = if ability.effect == SKIN {
        crate::loot::LootKind::SKINNING
    } else {
        crate::loot::LootKind::CHEST
    };
    crate::loot::open_item_loot(world, ctx.caster, id, rolled, kind);
    advance_skill(world, ctx.caster, progress, rng.gen_range(1..=1000));
    mark_resource_collected(world, target, id, ability.effect);
}

fn mark_resource_collected(world: &mut World, target: Entity, character_id: u64, effect: u32) {
    if effect == SKIN {
        world
            .entity_mut(target)
            .insert(Skinned)
            .remove::<GatherableResource>();
    } else {
        let node = &world.resource::<GatheringData>().nodes
            [&world.get::<GameObjectInfo>(target).unwrap().entry];
        let delay = node.despawn_delay;
        let max_loots = node.max_loots as usize;
        let count = {
            let mut claims = world.get_mut::<NodeClaims>(target).unwrap();
            claims.0.insert(character_id);
            claims.0.len()
        };
        if world.get::<Depletion>(target).is_none() {
            world.entity_mut(target).insert(Depletion(delay));
        }
        if count >= max_loots {
            world.get_mut::<Depletion>(target).unwrap().0 = 0.0;
        }
    }
}

fn gather_progress(
    world: &World,
    caster: Entity,
    target: Entity,
    ability: &Ability,
) -> (u32, crate::profession::GatherProgress) {
    if ability.effect == OPEN_LOCK {
        let data = world.resource::<GatheringData>();
        let node = &data.nodes[&world.get::<GameObjectInfo>(target).unwrap().entry];
        let required = data.locks[&node.lock]
            .iter()
            .find(|requirement| requirement.lock_type == ability.lock_type)
            .unwrap()
            .rank;
        (
            node.loot,
            crate::profession::GatherProgress::node(ability.skill, required, node.low, node.high),
        )
    } else {
        let skin = skin_template(world, target).unwrap();
        let target_scaling = crate::level_scaling::UnitScaling::of(world, target);
        let level =
            target_scaling.level_against(&crate::level_scaling::UnitScaling::of(world, caster));
        let mut progress = crate::profession::GatherProgress::skinning(skin.skill, level);
        if skin.elite {
            progress.multiplier = 2;
        }
        (skin.loot, progress)
    }
}

fn advance_skill(
    world: &mut World,
    caster: Entity,
    progress: crate::profession::GatherProgress,
    roll: u32,
) {
    let player = world
        .get::<Player>(caster)
        .expect("player gatherer")
        .clone();
    world.resource_scope(|world, data: Mut<crate::profession::ProfessionData>| {
        world.resource_scope(|world, spells: Mut<crate::spell_info::SpellStore>| {
            let learner = crate::profession::Learner {
                data: &data,
                spells: &spells,
                race: player.race,
                class: player.class,
            };
            let mut profs = world
                .get_mut::<PlayerProfessions>(caster)
                .expect("learned gathering skill");
            learner.gather_skill_up(&mut profs, progress, roll);
        });
    });
}

fn update_node_timers(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    deplete_expired_nodes(world, dt);
    respawn_due_nodes(world, dt);
}

fn deplete_expired_nodes(world: &mut World, dt: f32) {
    let mut depleted = world.query::<(Entity, &mut Depletion, &NodeSource, &InMap)>();
    let expired = depleted
        .iter_mut(world)
        .filter_map(|(entity, mut timer, source, map)| {
            timer.0 -= dt;
            (timer.0 <= 0.0).then(|| (entity, source.clone(), *map))
        })
        .collect::<Vec<_>>();
    for (entity, source, map) in expired {
        let seconds = world
            .resource::<GatheringData>()
            .respawn_seconds
            .get(&source.0.guid)
            .copied();
        world.entity_mut(entity).despawn();
        match seconds {
            Some(remaining) => {
                world.spawn((NodeRespawn { source, remaining }, map));
            }
            None => warn!(
                "GATHERING GAP: node {} has no positive authored respawn timer",
                source.0.guid
            ),
        }
    }
}

fn respawn_due_nodes(world: &mut World, dt: f32) {
    let mut respawns = world.query::<(Entity, &mut NodeRespawn, &InMap)>();
    let ready = respawns
        .iter_mut(world)
        .filter_map(|(entity, mut timer, map)| {
            timer.remaining -= dt;
            (timer.remaining <= 0.0).then(|| (entity, timer.source.clone(), *map))
        })
        .collect::<Vec<_>>();
    for (entity, source, map) in ready {
        world.entity_mut(entity).despawn();
        crate::spawn_pools::respawn_gameobject(world, &source.0, map);
    }
}

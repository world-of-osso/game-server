use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    CreatureClassification, EquipmentAppearance, Health, ModelDisplay, Mounted, Npc, Position,
    Rotation, SheathState, UnitFactionTemplate, UnitFlags, UnitLevel, UnitPose, UnitPowers, Zone,
};
use shared::protocol::NpcFlags;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::combat::{CorpseDelay, RespawnDelay};
use crate::creature_ai::{FormationMember, SpawnPoint, WanderPhase, WanderState, WaypointState};
use crate::interest::NetworkVisibility;
use crate::level_scaling::{CreatureArmor, LevelStats, ScaledStats, TuningInputs};
use crate::spawn_pools::{MapPools, PoolSpawns, PooledSpawn};
use crate::world_data::summon_levels::GuardianLevelRange;
use crate::world_data::{
    CreatureAddon, CreatureModelInfo, CreatureSpawn, CreatureTemplate, GROUP_AI_FLAG_FOLLOW_LEADER,
    WorldData,
};
use shared::level_scaling::LevelScaling;

mod entry;
pub(crate) use entry::read_entry_projection;

/// Original ordinary population identity, independent of a temporary entry change.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureOrigin {
    pub guid: u32,
    pub original_entry: u32,
}

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostStartup,
            spawn_continent_creatures.after(crate::ground::open_ground_map),
        )
        .add_systems(FixedUpdate, apply_addon_auras);
    }
}

/// The hosted continents are created at startup with all their creatures.
pub(crate) fn spawn_continent_creatures(
    mut commands: Commands,
    world_data: Res<WorldData>,
    mut ground: Option<ResMut<crate::ground::GroundMap>>,
) {
    for map_id in crate::maps::HOSTED_CONTINENTS {
        let count = spawn_map_creatures(
            &mut commands,
            &world_data,
            MapContent::continent(map_id),
            true,
            ground.as_deref_mut(),
        );
        info!("Spawned {count} creatures on map {map_id}");
    }
}

/// A map copy to fill: its difficulty (`Map::GetDifficultyID`, 0 on a continent) and
/// the boss entries its instance lock keeps dead.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MapContent {
    pub map: crate::maps::InMap,
    pub difficulty_id: u32,
    pub dead_bosses: Vec<u32>,
}

impl MapContent {
    pub fn continent(map_id: u32) -> Self {
        Self {
            map: crate::maps::InMap::continent(map_id),
            difficulty_id: 0,
            dead_bosses: Vec::new(),
        }
    }
}

/// `Creature::m_stringIds`: template, spawn and script StringIds.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureStringIds {
    pub template: Option<String>,
    pub spawn: Option<String>,
    pub script: Option<String>,
}

impl CreatureStringIds {
    /// `Creature::HasStringId`.
    pub fn has(&self, id: &str) -> bool {
        [&self.template, &self.spawn, &self.script]
            .into_iter()
            .flatten()
            .any(|string_id| string_id == id)
    }
}

/// `Creature::SetScriptStringId`.
pub(crate) fn set_script_string_id(world: &mut World, creature: Entity, id: &str) {
    let mut ids = world
        .get::<CreatureStringIds>(creature)
        .cloned()
        .unwrap_or_default();
    ids.script = Some(id.to_owned());
    world.entity_mut(creature).insert(ids);
}

/// `Map::GetDifficultyID()` of `map`'s copy; open-world maps are difficulty 0.
pub(crate) fn map_difficulty_id(world: &World, map: crate::maps::InMap) -> u32 {
    world
        .get_resource::<crate::maps::InstanceManager>()
        .and_then(|instances| instances.copy_difficulty(map))
        .map_or(0, |difficulty| difficulty.difficulty_id)
}

/// `Map::SpawnGroupSpawn(groupId)` for a script-owned group withheld at copy fill.
pub(crate) fn spawn_group_spawn(
    world: &mut World,
    map: crate::maps::InMap,
    difficulty_id: u32,
    replicate: bool,
    group: u32,
) -> Vec<Entity> {
    let mut queue = bevy::ecs::world::CommandQueue::default();
    world.init_resource::<crate::ground::GroundMap>();
    let entities = world.resource_scope(|world, mut ground: Mut<crate::ground::GroundMap>| {
        let world_data = world.resource::<WorldData>();
        let Some(guids) = world_data.script_spawn_groups.get(&group) else {
            error!("SpawnGroupSpawn: group {group} is not a loaded script spawn group");
            return Vec::new();
        };
        let spawns = world_data.spawns_by_map[&(map.map_id as u16)]
            .iter()
            .map(|&idx| &world_data.spawns[idx])
            .filter(|spawn| guids.contains(&spawn.guid) && spawn.spawns_at(difficulty_id));
        let mut commands = Commands::new(&mut queue, world);
        spawn_creatures(
            &mut commands,
            world_data,
            map,
            difficulty_id,
            replicate,
            spawns,
            Some(&mut ground),
        )
        .into_values()
        .collect()
    });
    queue.apply(world);
    entities
}

/// The difficulty a creature's map copy was created at (`Map::GetDifficultyID`), which
/// picks its template's difficulty row and loot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDifficulty(pub u32);

/// Spawn every creature of the copy's map spawned at its difficulty,
/// with the template of that difficulty, except the bosses
/// kept dead. When `replicate` is true, adds `Replicate` (requires lightyear plugins).
/// Tests pass `false` to avoid that dependency.
pub(crate) fn spawn_map_creatures(
    commands: &mut Commands,
    world_data: &WorldData,
    content: MapContent,
    replicate: bool,
    ground: Option<&mut crate::ground::GroundMap>,
) -> u32 {
    let map = content.map;
    let indices = world_data.spawns_by_map.get(&(map.map_id as u16));
    let (pool_spawns, picked) = world_data
        .pools
        .init_map(map.map_id as u16, &mut rand::thread_rng());
    let picked: HashSet<u32> = picked.into_iter().collect();
    let withheld: HashSet<u32> = world_data
        .script_spawn_groups
        .values()
        .flatten()
        .copied()
        .collect();
    let spawns = indices
        .into_iter()
        .flatten()
        .map(|&idx| &world_data.spawns[idx])
        .filter(|spawn| {
            (world_data.pools.creature_pool(spawn.guid).is_none() || picked.contains(&spawn.guid))
                && !world_data.spawn_groups.is_member(spawn.guid)
                && spawn.spawns_at(content.difficulty_id)
                && !content.dead_bosses.contains(&spawn.template_entry)
                && !withheld.contains(&spawn.guid)
        });
    let entities = spawn_creatures(
        commands,
        world_data,
        map,
        content.difficulty_id,
        replicate,
        spawns,
        ground,
    );
    let mut map_pools = MapPools {
        spawns: pool_spawns,
        entities: HashMap::new(),
        gameobjects: HashMap::new(),
        difficulty_id: content.difficulty_id,
        replicate,
    };
    for guid in picked {
        if let Some(&entity) = entities.get(&guid) {
            commands.entity(entity).insert(PooledSpawn(guid));
            map_pools.entities.insert(guid, entity);
        }
    }
    commands.queue(move |world: &mut World| {
        world
            .get_resource_or_init::<PoolSpawns>()
            .0
            .insert(map, map_pools);
    });
    entities.len() as u32
}

/// The pooled creature spawns `guids` a pool picked in `map`'s copy; returns the entity
/// of each spawn guid.
pub(crate) fn spawn_pooled_creatures(
    commands: &mut Commands,
    world_data: &WorldData,
    map: crate::maps::InMap,
    map_pools: &MapPools,
    guids: &[u32],
    ground: Option<&mut crate::ground::GroundMap>,
) -> HashMap<u32, Entity> {
    let indices = &world_data.spawns_by_map[&(map.map_id as u16)];
    let spawns = indices
        .iter()
        .map(|&idx| &world_data.spawns[idx])
        .filter(|spawn| guids.contains(&spawn.guid) && spawn.spawns_at(map_pools.difficulty_id));
    let entities = spawn_creatures(
        commands,
        world_data,
        map,
        map_pools.difficulty_id,
        map_pools.replicate,
        spawns,
        ground,
    );
    for (&guid, &entity) in &entities {
        commands.entity(entity).insert(PooledSpawn(guid));
    }
    entities
}

/// `Map::SpawnGroupSpawn` of the condition-met members `guids` in `map`'s copy; each
/// carries its spawn guid.
pub(crate) fn spawn_grouped_creatures(
    commands: &mut Commands,
    world_data: &WorldData,
    map: crate::maps::InMap,
    difficulty_id: u32,
    guids: &std::collections::BTreeSet<u32>,
    ground: Option<&mut crate::ground::GroundMap>,
) {
    let spawns = world_data.spawns_by_map[&(map.map_id as u16)]
        .iter()
        .map(|&idx| &world_data.spawns[idx])
        .filter(|spawn| guids.contains(&spawn.guid) && spawn.spawns_at(difficulty_id));
    let entities = spawn_creatures(
        commands,
        world_data,
        map,
        difficulty_id,
        true,
        spawns,
        ground,
    );
    for (&guid, &entity) in &entities {
        commands
            .entity(entity)
            .insert(crate::spawn_groups::GroupedSpawn(guid));
    }
}

/// Spawn `spawns` with their AI, addon and formation components; returns
/// the entity of each spawn guid.
fn spawn_creatures<'a>(
    commands: &mut Commands,
    world_data: &WorldData,
    map: crate::maps::InMap,
    difficulty_id: u32,
    replicate: bool,
    spawns: impl Iterator<Item = &'a CreatureSpawn>,
    ground: Option<&mut crate::ground::GroundMap>,
) -> HashMap<u32, Entity> {
    let mut context = CreatureSpawnContext {
        map,
        difficulty_id,
        ground,
        temporary: false,
    };
    let mut entities = HashMap::new();
    let mut rng = rand::thread_rng();
    for spawn in spawns {
        let template = world_data
            .template_for(spawn.template_entry, difficulty_id)
            .unwrap_or_else(|| {
                panic!(
                    "creature spawn {} references missing template {}",
                    spawn.guid, spawn.template_entry
                )
            });
        let Some(entity) = spawn_creature(
            commands,
            world_data,
            template,
            &mut context,
            replicate,
            spawn,
            &mut rng,
        ) else {
            continue;
        };
        if difficulty_id != 0 {
            commands
                .entity(entity)
                .insert(CreatureDifficulty(difficulty_id));
        }
        entities.insert(spawn.guid, entity);
    }
    insert_formation_members(commands, world_data, &entities);
    entities
}

/// Map difficulty and actual terrain used before selecting a creature's native stats.
pub(crate) struct CreatureSpawnContext<'a> {
    pub map: crate::maps::InMap,
    pub difficulty_id: u32,
    pub ground: Option<&'a mut crate::ground::GroundMap>,
    /// TempSummon initialization/owner-level selection is outside ordinary population spawning.
    pub temporary: bool,
}

/// Spawn one creature with AI/addon/script state, or log and refuse unknown level context.
pub(crate) fn spawn_creature(
    commands: &mut Commands,
    world_data: &WorldData,
    template: &CreatureTemplate,
    context: &mut CreatureSpawnContext<'_>,
    replicate: bool,
    spawn: &CreatureSpawn,
    rng: &mut impl rand::Rng,
) -> Option<Entity> {
    let map = context.map;
    {
        let tuning = match read_spawn_tuning(world_data, template, spawn, context) {
            Ok(tuning) => tuning,
            Err(MissingSpawnLevelContext) => {
                error!(
                    "CREATURE_LEVEL_CONTEXT_GAP: refusing creature{} spawn{} map{} copy{} difficulty{}",
                    template.entry, spawn.guid, map.map_id, map.instance_id, context.difficulty_id,
                );
                return None;
            }
        };
        let scaling = tuning.map(|(range, stats)| (roll_spawn_scaling(range, rng), stats));
        let (level, health, armor) = match &scaling {
            Some((scaling, tuning)) => {
                let level = scaling.native_level();
                (level, tuning.max_health(level), tuning.armor(level))
            }
            None => (
                template.min_level,
                resolve_health(world_data, template),
                template.armor,
            ),
        };
        let (display_id, display_scale) = choose_display(template, spawn, rng);
        let bundle = creature_bundle(spawn, &template.name, health, display_id);

        let mut entity = commands.spawn((
            bundle,
            map,
            NativeObjectScale(display_scale),
            shared::components::UnitScale(display_scale),
            crate::creature_ai::HomeOrientation(spawn.orientation + std::f32::consts::FRAC_PI_2),
        ));
        let source_guid = spawn.guid;
        entity.queue(move |actor: EntityWorldMut| {
            crate::pvp::arena_dalaran::tag_spout(actor, source_guid)
        });
        if let Some((scaling, tuning)) = scaling {
            entity.insert((scaling, ScaledStats(tuning)));
        }
        // NPC power pools need creature class/level stats, which are not imported.
        entity.insert((
            UnitLevel(level),
            CreatureClassification::from_rank(template.rank)
                .expect("creature_template.rank is validated when loaded"),
            CreatureArmor(armor),
            UnitPowers::default(),
            UnitFactionTemplate(u32::from(template.faction)),
            UnitFlags(spawn.unit_flags.unwrap_or(template.unit_flags)),
            // The merged content view includes Retail-only services such as
            // Transmogrifier; an AzerothCore mask would discard their flags.
            NpcFlags(template.npc_flags),
            shared::components::CreatureMotion::Still,
            // Set by the movement systems while it moves (`creature_ai::Bearing`).
            shared::components::MovementSpeed(0.0),
        ));
        if let Some(&vignette) = world_data.vignettes.get(&template.entry) {
            crate::vignettes::show(&mut entity, vignette);
        }
        if replicate {
            entity.insert((
                Replicate::to_clients(NetworkTarget::All),
                NetworkVisibility::default(),
            ));
        }
        let addon = world_data.addon_for(spawn);
        let pose = unit_pose(addon);
        entity.insert((pose, SpawnPose(pose)));
        if let Some(equipment) = creature_equipment(world_data, spawn, rng) {
            entity.insert(equipment);
        }
        insert_ai_components(&mut entity, spawn, template, addon, world_data);
        crate::pvp::insert_track_switch(&mut entity, spawn);
        crate::creature_aggro::insert_aggro_components(&mut entity, template.aggro_mode);
        crate::creature_scripts::insert_script_components(&mut entity, &template.script_name);
        crate::scripted_ai::insert_script_name(&mut entity, &template.script_name, template.entry);
        let string_ids = CreatureStringIds {
            template: world_data.template_string_ids.get(&template.entry).cloned(),
            spawn: world_data.spawn_string_ids.get(&spawn.guid).cloned(),
            script: None,
        };
        if let Some(faction) = crate::pvp::isle::creature_faction(spawn.map, &string_ids) {
            entity.insert(UnitFactionTemplate(faction));
        }
        if string_ids != CreatureStringIds::default() {
            entity.insert(string_ids);
        }
        insert_mana(&mut entity, world_data, template, level);
        if template.regenerates_health {
            entity.insert(crate::spell_cast::powers::CreatureHealthRegenTimer::default());
        }
        if let Some(addon) = addon {
            insert_addon_components(&mut entity, addon);
        }
        Some(entity.id())
    }
}

/// Mana classes (paladin 2, mage 8: `ChrClasses.DisplayPower` mana) get
/// creature_classlevelstats `basemana` × `ManaModifier`, full
/// (Creature::UpdateLevelDependantStats → `SetCreateMana`, `SetInitialPowerValue`).
fn insert_mana(
    entity: &mut EntityCommands,
    world_data: &WorldData,
    template: &CreatureTemplate,
    level: u8,
) {
    const MANA_CLASSES: [u8; 2] = [2, 8];
    if !MANA_CLASSES.contains(&template.unit_class) {
        return;
    }
    let base = world_data
        .class_level_base_mana
        .get(&(level, template.unit_class))
        .copied()
        .unwrap_or(0);
    let max = (base as f32 * template.mana_modifier).round();
    entity.insert((
        shared::components::Mana { current: max, max },
        crate::spell_cast::powers::PowerPools {
            order: vec![shared::components::PowerType::Mana],
            ..Default::default()
        },
    ));
    if template.regenerates_power {
        entity.insert(crate::spell_cast::powers::CreatureRegenTimer::default());
    }
}

/// A creature summoned by `summoner` (`TempSummon`, `TEMPSUMMON_MANUAL_DESPAWN`):
/// it never respawns, and its corpse's removal or its summoner's `SummonList`
/// unsummons it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempSummon {
    pub summoner: Entity,
}

/// `WorldObject::SummonCreature(entry, pos)`: a creature of template `entry` at the
/// WoW position `[x, y, z, o]` in the summoner's map copy, replicated when the
/// summoner is.
pub(crate) fn summon_creature(
    world: &mut World,
    summoner: Entity,
    entry: u32,
    position: [f32; 4],
) -> Option<Entity> {
    let map = *world.get::<crate::maps::InMap>(summoner)?;
    let zone_id = world.get::<Zone>(summoner).map_or(0, |zone| zone.id as u16);
    let replicate = world.get::<Replicate>(summoner).is_some();
    let spawn = CreatureSpawn {
        guid: 0,
        template_entry: entry,
        map: map.map_id as u16,
        zone_id,
        position_x: position[0],
        position_y: position[1],
        position_z: position[2],
        orientation: position[3],
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 0,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: Vec::new(),
        // Creature::InitEntry: a summon loads its default equipment (LoadEquipment(1)).
        equipment_id: 1,
    };
    let difficulty_id = map_difficulty_id(world, map);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let summon = world.resource_scope(|world, world_data: Mut<WorldData>| {
        let Some(template) = world_data.template_for(entry, difficulty_id) else {
            error!("SummonCreature: creature template {entry} does not exist");
            return None;
        };
        let mut commands = Commands::new(&mut queue, world);
        let summon = spawn_creature(
            &mut commands,
            &world_data,
            template,
            &mut CreatureSpawnContext {
                map,
                difficulty_id,
                ground: None,
                temporary: true,
            },
            replicate,
            &spawn,
            &mut rand::thread_rng(),
        )?;
        commands
            .entity(summon)
            .insert(TempSummon { summoner })
            .remove::<RespawnDelay>();
        if difficulty_id != 0 {
            commands
                .entity(summon)
                .insert(CreatureDifficulty(difficulty_id));
        }
        Some(summon)
    });
    queue.apply(world);
    if let Some(summon) = summon {
        crate::scripted_ai::dispatch(
            world,
            summon,
            crate::scripted_ai::Hook::IsSummonedBy(summoner),
        );
        crate::scripted_ai::dispatch(
            world,
            summoner,
            crate::scripted_ai::Hook::JustSummoned(summon),
        );
        info!("{summoner} summoned creature {entry} as {summon} at {position:?}");
    }
    summon
}

/// `TempSummon::UnSummon`: the summoner's AI learns of it
/// (`SummonedCreatureDespawn`) and the summon leaves the world.
pub(crate) fn unsummon(world: &mut World, summon: Entity) {
    let Some(summoner) = world
        .get::<TempSummon>(summon)
        .map(|summon| summon.summoner)
    else {
        return;
    };
    crate::scripted_ai::boss_ai::notify_summoner_despawn(world, summoner, summon);
    if let Ok(entity) = world.get_entity_mut(summon) {
        entity.despawn();
    }
}

/// Creature::UpdateEntry draws melee weapons (SHEATH_STATE_MELEE); the addon then sets
/// its stand state, sheath state and emote state (Creature::LoadCreaturesAddon).
fn unit_pose(addon: Option<&CreatureAddon>) -> UnitPose {
    addon.map_or(
        UnitPose {
            sheath_state: SheathState::Melee,
            ..UnitPose::default()
        },
        |addon| UnitPose {
            stand_state: addon.stand_state,
            sheath_state: addon.sheath_state,
            emote_state: addon.emote_state,
        },
    )
}

/// Creature::LoadEquipment(creature.equipment_id): its entry's equipment of that ID,
/// a random one for -1 (ObjectMgr::GetEquipmentInfo), none for 0.
fn creature_equipment(
    world_data: &WorldData,
    spawn: &CreatureSpawn,
    rng: &mut impl rand::Rng,
) -> Option<EquipmentAppearance> {
    let equipment = world_data.equipment.get(&spawn.template_entry)?;
    let chosen = match spawn.equipment_id {
        0 => return None,
        -1 => equipment.values().nth(rng.gen_range(0..equipment.len())),
        id => equipment.get(&(id as u8)),
    }?;
    (!chosen.entries.is_empty()).then(|| chosen.clone())
}

/// `creature_addon` mount and auras (Creature::LoadCreaturesAddon).
fn insert_addon_components(entity: &mut EntityCommands, addon: &CreatureAddon) {
    if addon.mount != 0 {
        entity.insert(Mounted {
            mount_display_id: addon.mount,
            ..Default::default()
        });
    }
    if !addon.auras.is_empty() {
        entity.insert((
            AddonAuras(addon.auras.clone()),
            SpawnAddonAuras(addon.auras.clone()),
        ));
    }
}

/// The `creature_addon.auras` a creature spawned with, applied again when it gets
/// home from an evade (`HomeMovementGenerator::DoFinalize` → `LoadCreaturesAddon`).
#[derive(Component, Debug, Clone)]
pub(crate) struct SpawnAddonAuras(pub Vec<u32>);

/// The pose a creature's addon gives it at spawn, restored when it reaches home after an
/// evade (`HomeMovementGenerator::DoFinalize` → `Creature::LoadCreaturesAddon`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpawnPose(pub UnitPose);

/// Link spawned followers to their spawned leader (FormationMgr::AddCreatureToGroup).
fn insert_formation_members(
    commands: &mut Commands,
    world_data: &WorldData,
    entities: &HashMap<u32, Entity>,
) {
    for (guid, &member) in entities {
        let Some(info) = world_data.formations.get(guid) else {
            continue;
        };
        let Some(&leader) = entities.get(&info.leader_guid) else {
            continue;
        };
        commands
            .entity(member)
            .queue(move |mut entity: EntityWorldMut| {
                let mut leader_ids = entity
                    .world()
                    .get::<CreatureStringIds>(leader)
                    .cloned()
                    .unwrap_or_default();
                // With no live leader TC reads only authored spawn/template identity.
                leader_ids.script = None;
                entity.insert(CreatureGroupMember { leader, leader_ids });
            });
        if member == leader || info.group_ai & GROUP_AI_FLAG_FOLLOW_LEADER == 0 {
            continue;
        }
        commands.entity(member).insert(FormationMember {
            leader,
            follow_dist: info.follow_dist,
            follow_angle: info.follow_angle,
        });
    }
}

/// Group membership is independent of movement (`CreatureGroup::AddMember`).
#[derive(Component, Clone)]
struct CreatureGroupMember {
    leader: Entity,
    leader_ids: CreatureStringIds,
}

/// `Creature::setDeathState`: notify the copy only after its last living group member dies.
pub(crate) fn notify_group_depletion(world: &mut World, creature: Entity) {
    let Some(group) = world.get::<CreatureGroupMember>(creature).cloned() else {
        return;
    };
    let has_living_member = world
        .query_filtered::<(&CreatureGroupMember, &Health), Without<crate::combat::DeathState>>()
        .iter(world)
        .any(|(member, health)| member.leader == group.leader && health.current > 0.0);
    if has_living_member {
        return;
    }
    let Some(map) = world.get::<crate::maps::InMap>(creature).copied() else {
        return;
    };
    let leader_ids = world
        .get::<CreatureStringIds>(group.leader)
        .cloned()
        .unwrap_or(group.leader_ids);
    crate::scripted_ai::instance_script::creature_group_depleted(world, map, &leader_ids);
}

/// `creature_addon.auras` not yet applied to the spawned creature.
#[derive(Component, Debug)]
pub(crate) struct AddonAuras(pub Vec<u32>);

/// Self-cast addon auras once the creature exists (Creature::LoadCreaturesAddon `AddAura`).
pub(crate) fn apply_addon_auras(world: &mut World) {
    let pending: Vec<(Entity, Vec<u32>)> = world
        .query::<(Entity, &AddonAuras)>()
        .iter(world)
        .map(|(entity, auras)| (entity, auras.0.clone()))
        .collect();
    for (creature, spell_ids) in pending {
        world.entity_mut(creature).remove::<AddonAuras>();
        for spell_id in spell_ids {
            apply_addon_aura(world, creature, spell_id);
        }
    }
}

fn apply_addon_aura(world: &mut World, creature: Entity, spell_id: u32) {
    let Some(effect_mask) = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(spell_id))
        .map(crate::auras::aura_effect_mask)
    else {
        warn!("creature_addon aura spell {spell_id} not in SpellStore");
        return;
    };
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster: creature,
            target: creature,
            spell_id,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

/// Core creature components without replication marker.
fn creature_bundle(
    spawn: &crate::world_data::CreatureSpawn,
    name: &str,
    health: f32,
    display_id: u32,
) -> (Position, Npc, Health, ModelDisplay, Zone, Rotation) {
    // Spawn data is WoW coords; convert to Bevy space for ECS.
    let (bx, by, bz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    (
        Position {
            x: bx,
            y: by,
            z: bz,
        },
        Npc {
            template_id: spawn.template_entry,
            name: name.to_owned(),
        },
        Health {
            current: health,
            max: health,
        },
        ModelDisplay { display_id },
        Zone {
            id: spawn.zone_id as u32,
        },
        Rotation {
            x: 0.0,
            y: spawn.orientation + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    )
}

/// Native object multiplier retained when aura61 is applied or removed.
#[derive(Component, Clone, Copy)]
pub(crate) struct NativeObjectScale(pub f32);

/// `CreatureModel::DefaultInvisibleModel`: a trigger without an invisible model of its own.
const DEFAULT_INVISIBLE_DISPLAY: u32 = 11_686;

/// ObjectMgr::ChooseDisplayId (ObjectMgr.cpp:1601-1613): the spawn's modelid, else a model of the
/// template by Probability, except that triggers take their first invisible model.
fn choose_display(
    template: &CreatureTemplate,
    spawn: &CreatureSpawn,
    rng: &mut impl rand::Rng,
) -> (u32, f32) {
    match spawn.display_id {
        Some(display_id) => (display_id, 1.0),
        None => template_display(template, rng),
    }
}

/// `ObjectMgr::ChooseDisplayId` without spawn data: a model of the template by
/// Probability; triggers take their first invisible model.
pub(crate) fn template_display_id(template: &CreatureTemplate, rng: &mut impl rand::Rng) -> u32 {
    template_display(template, rng).0
}

fn template_display(template: &CreatureTemplate, rng: &mut impl rand::Rng) -> (u32, f32) {
    if !template.trigger
        && let Some(model) = random_valid_model(&template.models, rng)
    {
        return (model.display_id, model.display_scale);
    }
    template
        .models
        .iter()
        .find(|model| model.is_trigger)
        .map_or((DEFAULT_INVISIBLE_DISPLAY, 1.0), |model| {
            (model.display_id, model.display_scale)
        })
}

/// CreatureTemplate::GetRandomValidModel (Creature.cpp:113-128): one model regardless of its
/// Probability; otherwise weighted by Probability, uniform when every weight is 0
/// (SelectRandomWeightedContainerElement).
fn random_valid_model<'a>(
    models: &'a [CreatureModelInfo],
    rng: &mut impl rand::Rng,
) -> Option<&'a CreatureModelInfo> {
    if models.len() <= 1 {
        return models.first();
    }
    let index =
        match rand::distributions::WeightedIndex::new(models.iter().map(|model| model.probability))
        {
            Ok(weights) => rng.sample(weights),
            Err(rand::distributions::WeightedError::AllWeightsZero) => {
                rng.gen_range(0..models.len())
            }
            Err(error) => panic!("creature_template_model Probability: {error}"),
        };
    models.get(index)
}

struct MissingSpawnLevelContext;

/// Creature::ApplyLevelScaling: template, map difficulty, then actual terrain ancestry.
fn read_spawn_tuning(
    data: &WorldData,
    template: &CreatureTemplate,
    spawn: &CreatureSpawn,
    context: &mut CreatureSpawnContext<'_>,
) -> Result<Option<(GuardianLevelRange, Arc<LevelStats>)>, MissingSpawnLevelContext> {
    if data.authored_templates.contains(&template.entry) {
        return Ok(None);
    }
    if let Some(tuning) = &template.tuning {
        let range = GuardianLevelRange {
            content_tuning_id: tuning.content_tuning_id,
            range: (template.min_level, template.max_level),
            delta: tuning.delta,
        };
        return Ok(Some((range, tuning.stats.clone())));
    }
    if context.temporary {
        return Ok(None);
    }
    let range = read_regional_spawn_range(data, template, spawn, context)?;
    if range.content_tuning_id == 0 {
        return Ok(None);
    }
    let stats = build_regional_spawn_stats(data, template, range.content_tuning_id);
    Ok(Some((range, Arc::new(stats))))
}

fn build_regional_spawn_stats(
    data: &WorldData,
    template: &CreatureTemplate,
    content_tuning_id: u32,
) -> LevelStats {
    let inputs = TuningInputs {
        content_tuning_id,
        health_scaling_expansion: template.health_scaling_expansion,
        unit_class: template.unit_class,
        health_modifier: template.health_modifier,
        damage_modifier: template.damage_modifier,
        armor_modifier: template.armor_modifier,
    };
    LevelStats::build(&data.expected_stats, &inputs)
}

fn read_regional_spawn_range(
    data: &WorldData,
    template: &CreatureTemplate,
    spawn: &CreatureSpawn,
    context: &mut CreatureSpawnContext<'_>,
) -> Result<GuardianLevelRange, MissingSpawnLevelContext> {
    let map = context.map;
    let difficulty_id = context.difficulty_id;
    let resolve = |area| {
        data.summon_levels
            .guardian_range(template, map.map_id, difficulty_id, area)
    };
    // Do not load terrain when template/critter/map data already selects the source range.
    resolve(None)
        .or_else(|| resolve(read_spawn_area(context, spawn)))
        .ok_or(MissingSpawnLevelContext)
}

fn read_spawn_area(context: &mut CreatureSpawnContext<'_>, spawn: &CreatureSpawn) -> Option<u32> {
    let ground = context.ground.as_deref_mut()?;
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    ground.area_at(context.map.map_id, Vec3::new(x, y, z))
}

/// Creature::ApplyLevelScaling: roll the selected source delta exactly once per spawn.
fn roll_spawn_scaling(range: GuardianLevelRange, rng: &mut impl rand::Rng) -> LevelScaling {
    let (low, high) = (
        range.delta.0.min(range.delta.1),
        range.delta.0.max(range.delta.1),
    );
    LevelScaling {
        content_tuning_id: range.content_tuning_id,
        min_level: range.range.0,
        max_level: range.range.1,
        delta: if low == high {
            low
        } else {
            rng.gen_range(low..=high)
        },
    }
}

/// Untuned templates: Creature::SelectLevel / CreatureBaseStats::GenerateHealth:
/// max(1, ceil(basehp<exp> × HealthModifier)) at the spawn level.
fn resolve_health(world_data: &WorldData, template: &CreatureTemplate) -> f32 {
    let key = (template.min_level, template.unit_class);
    let base_hp = world_data.class_level_base_hp.get(&key).unwrap_or_else(|| {
        panic!(
            "creature template {} has no creature_classlevelstats row for level {} class {}",
            template.entry, key.0, key.1
        )
    });
    let base_hp = base_hp[usize::from(template.expansion)] as f32;
    (base_hp * template.health_modifier).ceil().max(1.0)
}

/// Every spawn gets a `SpawnPoint` and `RespawnDelay` so it respawns where it
/// was placed; MovementType 1 wanders, 2 patrols.
fn insert_ai_components(
    entity: &mut EntityCommands,
    spawn: &CreatureSpawn,
    template: &crate::world_data::CreatureTemplate,
    addon: Option<&CreatureAddon>,
    world_data: &WorldData,
) {
    let (bx, by, bz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    entity.insert((
        SpawnPoint {
            x: bx,
            y: by,
            z: bz,
            wander_distance: spawn.wander_distance,
            speed_walk: template.speed_walk,
            speed_run: template.speed_run,
        },
        RespawnDelay::for_spawn(spawn.spawntimesecs, template.rank),
        CorpseDelay::for_rank(template.rank),
    ));
    if spawn.movement_type == 1 {
        entity.insert(WanderState {
            phase: WanderPhase::Idle,
            target_x: bx,
            target_z: bz,
            idle_remaining: 1.0,
            seed: spawn.guid,
        });
    } else if spawn.movement_type == 2
        && let Some(path_id) = patrol_path(addon, world_data)
    {
        entity.insert(WaypointState::new(
            path_id,
            template.speed_walk,
            template.speed_run,
            (bx, bz),
        ));
    }
}

/// MovementType 2 walks the addon's `path_id` (Creature::LoadCreaturesAddon
/// sets `m_path_id`; WaypointMovementGenerator::LoadPath reads it). Without
/// an addon path AzerothCore idles the spawn (ObjectMgr::LoadCreatureAddons).
fn patrol_path(addon: Option<&CreatureAddon>, world_data: &WorldData) -> Option<u32> {
    addon
        .map(|addon| addon.path_id)
        .filter(|&id| id != 0 && world_data.waypoints.contains_key(&id))
}

#[cfg(test)]
#[path = "zones_world_db_tests.rs"]
mod world_db_tests;

#[cfg(test)]
#[path = "spawn_pools_world_db_tests.rs"]
mod spawn_pools_world_db_tests;

#[cfg(test)]
#[path = "zones_npc_combat_tests.rs"]
mod npc_combat_tests;

#[cfg(test)]
#[path = "zones_context_tuning_tests.rs"]
mod context_tuning_tests;

#[cfg(test)]
#[path = "zones_entry_tests.rs"]
mod entry_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_data::{CreatureModelInfo, CreatureSpawn, CreatureTemplate};
    use std::collections::HashMap;

    fn make_template(entry: u32, health_mod: f32, display_id: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: "Fixture wolf".into(),
            subname: String::new(),
            creature_type: 1,
            family: 0,
            type_flags: 0,
            flags_extra: 0,
            min_level: 1,
            max_level: 1,
            faction: 0,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            health_modifier: health_mod,
            unit_class: 1,
            expansion: 0,
            rank: 0,
            experience_modifier: 1.0,
            gives_experience: true,
            script_name: String::new(),
            aggro_mode: crate::creature_aggro::AggroMode::Aggressive,
            trigger: false,
            unit_flags: 0,
            models: vec![CreatureModelInfo {
                display_id,
                display_scale: 1.0,
                probability: 1.0,
                is_trigger: false,
            }],
            tuning: None,
            armor: 0.0,
            armor_modifier: 1.0,
            damage_modifier: 1.0,
            health_scaling_expansion: 0,
            mana_modifier: 1.0,
            regenerates_power: false,
            regenerates_health: true,
        }
    }

    fn make_world_data(templates: Vec<CreatureTemplate>, spawns: Vec<CreatureSpawn>) -> WorldData {
        let mut template_map = HashMap::new();
        for t in templates {
            template_map.insert(t.entry, t);
        }
        let mut spawns_by_map: HashMap<u16, Vec<usize>> = HashMap::new();
        for (i, s) in spawns.iter().enumerate() {
            spawns_by_map.entry(s.map).or_default().push(i);
        }
        WorldData {
            templates: template_map,
            spawns,
            spawns_by_map,
            waypoints: HashMap::new(),
            class_level_base_hp: HashMap::from([((1, 1), [100; 3]), ((7, 1), [100; 3])]),
            ..Default::default()
        }
    }

    fn known_untuned_ground(data: &WorldData) -> crate::ground::GroundMap {
        let samples: Vec<_> = data
            .spawns
            .iter()
            .map(|spawn| {
                let (x, y, z) = crate::terrain_height::wow_to_bevy(
                    spawn.position_x,
                    spawn.position_y,
                    spawn.position_z,
                );
                (u32::from(spawn.map), Vec3::new(x, y, z), 0)
            })
            .collect();
        crate::ground::GroundMap::with_area_samples(&samples)
    }

    /// ObjectMgr::GetEquipmentInfo: equipment_id 2 picks ID 2, -1 one of the entry's IDs
    /// (both, over 200 rolls), 0 and a missing ID none. Winterskorn Warrior 23664's
    /// AzerothCore row (1, 33320, 13922) and an invented ID 2.
    #[test]
    fn creature_equipment_follows_equipment_id() {
        use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
        let appearance = |item_id, display_info_id| EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::MainHand,
                item_id: Some(item_id),
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                display_info_id: Some(display_info_id),
                inventory_type: 13,
                hidden: false,
            }],
        };
        let world_data = WorldData {
            equipment: HashMap::from([(
                23_664,
                std::collections::BTreeMap::from([
                    (1, appearance(33_320, 45_000)),
                    (2, appearance(13_922, 21_000)),
                ]),
            )]),
            ..Default::default()
        };
        let spawn = |equipment_id| CreatureSpawn {
            equipment_id,
            ..make_spawn(23_664, 0.0, 0.0, 0.0)
        };
        let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(7);
        assert_eq!(
            creature_equipment(&world_data, &spawn(2), &mut rng),
            Some(appearance(13_922, 21_000))
        );
        assert_eq!(creature_equipment(&world_data, &spawn(0), &mut rng), None);
        assert_eq!(creature_equipment(&world_data, &spawn(3), &mut rng), None);
        let rolled: std::collections::HashSet<_> = (0..200)
            .map(|_| creature_equipment(&world_data, &spawn(-1), &mut rng).unwrap())
            .map(|equipment| equipment.entries[0].item_id)
            .collect();
        assert_eq!(rolled, [Some(33_320), Some(13_922)].into());
    }

    fn make_spawn(entry: u32, x: f32, y: f32, z: f32) -> CreatureSpawn {
        CreatureSpawn {
            guid: 1,
            template_entry: entry,
            map: 0,
            zone_id: 0,
            position_x: x,
            position_y: y,
            position_z: z,
            orientation: 0.0,
            wander_distance: 0.0,
            movement_type: 0,
            spawntimesecs: 120,
            display_id: None,
            unit_flags: None,
            spawn_difficulties: vec![0],
            equipment_id: 0,
        }
    }

    #[test]
    fn authored_sqlite_orientation_reaches_spawned_npc_model_facing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE zephras_row_source (table_name TEXT, row_key TEXT, kind TEXT);
             CREATE TABLE content_tuning_level (ID INTEGER, minlevel INTEGER, maxlevel INTEGER);
             CREATE TABLE map_difficulty (MapID INTEGER, DifficultyID INTEGER, ContentTuningID INTEGER);
             CREATE TABLE area_table (ID INTEGER, ParentAreaID INTEGER, ContentTuningID INTEGER);
             CREATE TABLE content_creature_template (entry INTEGER, minlevel INTEGER, maxlevel INTEGER,
                faction INTEGER, npcflag INTEGER, speed_walk REAL, HealthModifier REAL, name TEXT, rank INTEGER, exp INTEGER,
                ExperienceModifier REAL, type INTEGER, flags_extra INTEGER, speed_run REAL,
                ScriptName TEXT, unit_class INTEGER, detection_range REAL, unit_flags INTEGER,
                ContentTuningID INTEGER, HealthScalingExpansion INTEGER, LevelScalingDeltaMin INTEGER,
                LevelScalingDeltaMax INTEGER, DamageModifier REAL, ArmorModifier REAL, subname TEXT,
                ManaModifier REAL, unit_flags2 INTEGER, RegenHealth INTEGER, family INTEGER,
                type_flags INTEGER);
             CREATE TABLE content_pool_template (entry INTEGER NOT NULL PRIMARY KEY,
                max_limit INTEGER NOT NULL, description TEXT);
             CREATE TABLE content_pool_members (type INTEGER NOT NULL, spawnId INTEGER NOT NULL,
                poolSpawnId INTEGER NOT NULL, chance REAL NOT NULL, description TEXT,
                PRIMARY KEY (type, spawnId));
             CREATE TABLE content_gameobject (guid INTEGER PRIMARY KEY, map INTEGER);
             CREATE TABLE tdb_creature_template (entry INTEGER PRIMARY KEY, VignetteID INTEGER,
                StringId TEXT);
             CREATE TABLE tdb_creature (guid INTEGER PRIMARY KEY, id INTEGER, StringId TEXT);
             CREATE TABLE tdb_conditions (SourceTypeOrReferenceId INTEGER, SourceEntry INTEGER,
                ElseGroup INTEGER, ConditionTypeOrReference INTEGER, ConditionValue1 INTEGER,
                ConditionValue2 INTEGER, ConditionValue3 INTEGER, NegativeCondition INTEGER);
             CREATE TABLE tdb_spawn_group (groupId INTEGER, spawnType INTEGER, spawnId INTEGER);
             CREATE TABLE tdb_spawn_group_template (groupId INTEGER PRIMARY KEY, groupName TEXT,
                 groupFlags INTEGER);
             CREATE TABLE vignette (ID INTEGER PRIMARY KEY, Flags INTEGER);
             CREATE TABLE content_game_event_pool (eventEntry INTEGER NOT NULL,
                pool_entry INTEGER NOT NULL PRIMARY KEY);
             CREATE TABLE expected_stat (ID INTEGER, Lvl INTEGER, ExpansionID INTEGER,
                CreatureHealth REAL, CreatureAutoAttackDps REAL, CreatureArmor REAL,
                ArmorConstant REAL);
             CREATE TABLE expected_stat_mod (ID INTEGER, CreatureHealthMod REAL,
                CreatureAutoAttackDPSMod REAL, CreatureArmorMod REAL, ArmorConstantMod REAL);
             CREATE TABLE content_tuning_x_expected (ID INTEGER, ContentTuningID INTEGER,
                ExpectedStatModID INTEGER, MinMythicPlusSeasonID INTEGER);
             CREATE TABLE creature_classlevelstats (level INTEGER, class INTEGER,
                basehp0 INTEGER, basehp1 INTEGER, basehp2 INTEGER, basemana INTEGER);
             INSERT INTO creature_classlevelstats VALUES (1,1,42,42,42,0);
             CREATE TABLE content_creature_template_model (CreatureID INTEGER, Idx INTEGER,
                CreatureDisplayID INTEGER, DisplayScale REAL, Probability REAL);
             CREATE TABLE content_creature_template_difficulty (entry INTEGER NOT NULL,
            DifficultyID INTEGER NOT NULL, minlevel INTEGER NOT NULL, maxlevel INTEGER NOT NULL, exp INTEGER NOT NULL,
            ContentTuningID INTEGER NOT NULL, HealthScalingExpansion INTEGER NOT NULL, LevelScalingDeltaMin INTEGER NOT NULL,
            LevelScalingDeltaMax INTEGER NOT NULL, HealthModifier REAL NOT NULL, ManaModifier REAL NOT NULL,
            ArmorModifier REAL NOT NULL, DamageModifier REAL NOT NULL, lootid INTEGER NOT NULL DEFAULT 0,
            mingold INTEGER NOT NULL DEFAULT 0, maxgold INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (entry, DifficultyID));
             CREATE TABLE content_creature (guid INTEGER, id1 INTEGER, map INTEGER, zoneId INTEGER,
                position_x REAL, position_y REAL, position_z REAL, wander_distance REAL,
                MovementType INTEGER, orientation REAL, spawntimesecs INTEGER, modelid INTEGER,
                unit_flags INTEGER, spawnDifficulties TEXT NOT NULL DEFAULT '0', equipment_id INTEGER);
             CREATE TABLE creature_display_info (ID INTEGER PRIMARY KEY, ModelID INTEGER);
             CREATE TABLE creature_model_data (ID INTEGER PRIMARY KEY, FileDataID INTEGER);
             CREATE TABLE content_game_event_creature (guid INTEGER, eventEntry INTEGER);
             CREATE TABLE content_waypoint_data (id INTEGER, point INTEGER, position_x REAL,
                position_y REAL, position_z REAL, orientation REAL, delay INTEGER,
                move_type INTEGER);
             CREATE TABLE content_creature_addon (guid INTEGER, path_id INTEGER, mount INTEGER,
                auras TEXT, bytes1 INTEGER, bytes2 INTEGER, emote INTEGER);
             CREATE TABLE content_creature_template_addon (entry INTEGER, path_id INTEGER,
                mount INTEGER, auras TEXT, bytes1 INTEGER, bytes2 INTEGER, emote INTEGER);
             CREATE TABLE content_creature_equip_template (CreatureID INTEGER, ID INTEGER,
                ItemID1 INTEGER, AppearanceModID1 INTEGER, ItemID2 INTEGER, AppearanceModID2 INTEGER,
                ItemID3 INTEGER, AppearanceModID3 INTEGER);
             CREATE TABLE item (ID INTEGER PRIMARY KEY, InventoryType INTEGER);
             CREATE TABLE item_modified_appearance (ItemID INTEGER, ItemAppearanceModifierID INTEGER,
                ItemAppearanceID INTEGER);
             CREATE TABLE item_appearance (ID INTEGER PRIMARY KEY, ItemDisplayInfoID INTEGER);
             CREATE TABLE content_creature_formations (leaderGUID INTEGER, memberGUID INTEGER,
                dist REAL, angle REAL, groupAI INTEGER);",
        )
        .unwrap();
        let angles = [
            0.0f32,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            3.0 * std::f32::consts::FRAC_PI_2,
            1.606,
        ];
        for (index, angle) in angles.iter().enumerate() {
            let entry = 100 + index as u32;
            conn.execute(
                "INSERT INTO content_creature_template VALUES (?1,1,1,1,0,1.0,1.0,'Loup — Écorché',0,0,1.0,1,0,1.14,'',1,20.0,0,0,0,0,0,1.0,1.0,'',1.0,0,1,0,0)",
                [entry],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO content_creature_template_model VALUES (?1,0,3167,1.0,1.0)",
                [entry],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO content_creature VALUES (?1,?1,0,0,-9000,-100,80,0,0,?2,120,0,0,'0',0)",
                rusqlite::params![entry, angle],
            )
            .unwrap();
        }
        drop(conn);
        let data = crate::world_data::load_world_data(&path);
        let mut app = run_spawn_system(data);
        let mut npcs = app
            .world_mut()
            .query::<(&Npc, Option<&shared::components::Rotation>)>();
        let mut count = 0;
        for (npc, rotation) in npcs.iter(app.world()) {
            assert_eq!(serde_json::to_value(npc).unwrap()["name"], "Loup — Écorché");
            let rotation = rotation.expect("authored NPC orientation must be replicated");
            let angle = angles[(npc.template_id - 100) as usize];
            let expected = Vec3::new(angle.cos(), 0.0, -angle.sin());
            let model_forward =
                Quat::from_rotation_y(rotation.y - std::f32::consts::FRAC_PI_2) * Vec3::X;
            assert!((model_forward - expected).length() < 0.00001);
            let logical_forward = Vec3::new(rotation.y.sin(), 0.0, rotation.y.cos());
            assert!((logical_forward - expected).length() < 0.00001);
            count += 1;
        }
        assert_eq!(count, angles.len());
    }

    #[test]
    fn authored_model_scale_survives_spawn_and_aura_reconciliation() {
        use shared::components::UnitScale;
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE content_creature_template_model (
                CreatureID INTEGER, Idx INTEGER, CreatureDisplayID INTEGER,
                DisplayScale REAL, Probability REAL);
             CREATE TABLE creature_display_info (ID INTEGER, ModelID INTEGER);
             CREATE TABLE creature_model_data (ID INTEGER, FileDataID INTEGER);
             INSERT INTO content_creature_template_model VALUES (40923,0,116569,0.45,0);",
        )
        .unwrap();
        let spawn = make_spawn(40923, 0.0, 0.0, 0.0);
        let mut data = make_world_data(vec![make_template(40923, 1.0, 116569)], vec![spawn]);
        data.templates.get_mut(&40923).unwrap().models.clear();
        crate::world_data::load_template_models(&conn, &mut data.templates);
        let mut app = run_spawn_system(data);
        let (entity, display) = app
            .world_mut()
            .query::<(Entity, &ModelDisplay)>()
            .single(app.world())
            .unwrap();
        assert_eq!(display.display_id, 116569);
        assert_eq!(app.world().get::<UnitScale>(entity), Some(&UnitScale(0.45)));
        app.world_mut()
            .entity_mut(entity)
            .insert(crate::auras::ActiveAuras::default());
        crate::auras::auras_changed(app.world_mut(), entity);
        assert_eq!(app.world().get::<UnitScale>(entity), Some(&UnitScale(0.45)));
    }

    fn model(display_id: u32, probability: f32, is_trigger: bool) -> CreatureModelInfo {
        CreatureModelInfo {
            display_id,
            display_scale: 1.0,
            probability,
            is_trigger,
        }
    }

    fn choose_with_seed(template: &CreatureTemplate, spawn: &CreatureSpawn) -> u32 {
        let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(1);
        choose_display(template, spawn, &mut rng).0
    }

    #[test]
    fn a_single_model_is_chosen_even_with_probability_zero() {
        let mut template = make_template(42, 1.0, 0);
        template.models = vec![model(5555, 0.0, false)];
        assert_eq!(
            choose_with_seed(&template, &make_spawn(42, 0.0, 0.0, 0.0)),
            5555
        );
    }

    #[test]
    fn a_spawn_modelid_overrides_the_template_models() {
        let template = make_template(42, 1.0, 5555);
        let mut spawn = make_spawn(42, 0.0, 0.0, 0.0);
        spawn.display_id = Some(7777);
        assert_eq!(choose_with_seed(&template, &spawn), 7777);
    }

    /// CreatureTemplate::GetFirstInvisibleModel: CreatureModel::DefaultInvisibleModel when the
    /// trigger has no invisible model of its own.
    #[test]
    fn a_trigger_without_an_invisible_model_takes_the_default_invisible_display() {
        let mut template = make_template(42, 1.0, 0);
        template.trigger = true;
        template.models = vec![model(169, 1.0, false)];
        assert_eq!(
            choose_with_seed(&template, &make_spawn(42, 0.0, 0.0, 0.0)),
            11686
        );
        template.models.push(model(16925, 0.0, true));
        assert_eq!(
            choose_with_seed(&template, &make_spawn(42, 0.0, 0.0, 0.0)),
            16925
        );
    }

    #[test]
    fn health_is_base_hp_of_the_template_expansion_times_modifier() {
        let mut wd = make_world_data(vec![], vec![]);
        wd.class_level_base_hp.insert((1, 1), [42, 43, 44]);
        let mut template = make_template(10, 2.5, 0);
        assert_eq!(resolve_health(&wd, &template), 105.0);
        template.expansion = 2;
        assert_eq!(resolve_health(&wd, &template), 110.0);
        // Never below 1 hp.
        template.health_modifier = 0.0;
        assert_eq!(resolve_health(&wd, &template), 1.0);
    }

    /// Test-only system: spawns creatures without `Replicate` (which requires lightyear plugins).
    fn spawn_test(mut commands: Commands, world_data: Res<WorldData>) {
        spawn_map_creatures(
            &mut commands,
            &world_data,
            MapContent::continent(0),
            false,
            Some(&mut known_untuned_ground(&world_data)),
        );
    }

    #[test]
    fn follower_normal_population_spawns_normal_only_and_keeps_follower_difficulty() {
        let mut normal = make_spawn(42, 10.0, 20.0, 30.0);
        normal.map = 2521;
        normal.spawn_difficulties = vec![1, 2, 23];
        let mut mythic = make_spawn(43, 40.0, 50.0, 60.0);
        mythic.map = 2521;
        mythic.spawn_difficulties = vec![8, 23];
        let data = make_world_data(
            vec![make_template(42, 1.0, 5555), make_template(43, 1.0, 5555)],
            vec![normal, mythic],
        );
        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let map = crate::maps::InMap {
            map_id: 2521,
            instance_id: 7,
        };
        let content = MapContent {
            map,
            difficulty_id: 205,
            dead_bosses: Vec::new(),
        };
        assert_eq!(
            spawn_map_creatures(
                &mut commands,
                &data,
                content,
                false,
                Some(&mut known_untuned_ground(&data))
            ),
            1
        );
        queue.apply(&mut world);
        let units: Vec<_> = world
            .query::<(&Npc, &CreatureDifficulty, &crate::maps::InMap)>()
            .iter(&world)
            .collect();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].0.template_id, 42);
        assert_eq!(units[0].1.0, 205);
        assert_eq!(*units[0].2, map);
    }

    #[test]
    fn dungeon_population_spawns_outside_the_old_appearance_allowlist() {
        for map_id in [36, 43, 389, 643] {
            let mut spawn = make_spawn(42, 10.0, 20.0, 30.0);
            spawn.map = map_id;
            spawn.spawn_difficulties = vec![1];
            let data = make_world_data(vec![make_template(42, 1.0, 5555)], vec![spawn]);
            let mut world = World::new();
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let mut commands = Commands::new(&mut queue, &world);
            let content = MapContent {
                map: crate::maps::InMap {
                    map_id: u32::from(map_id),
                    instance_id: 1,
                },
                difficulty_id: 1,
                dead_bosses: Vec::new(),
            };
            assert_eq!(
                spawn_map_creatures(
                    &mut commands,
                    &data,
                    content,
                    false,
                    Some(&mut known_untuned_ground(&data))
                ),
                1
            );
            queue.apply(&mut world);
            assert_eq!(world.query::<&Npc>().iter(&world).count(), 1);
        }
    }

    #[test]
    #[ignore = "requires WORLD_POPULATION_DB pointing to a promoted private copy"]
    fn zone_population_quest_ender_keeps_authored_player_immunity() {
        const LORD_AUTUMNVALE: u32 = 127080;
        const AUTHORED_FLAGS: u32 = 320;
        let path = std::env::var("WORLD_POPULATION_DB").expect("promoted copy path");
        let data = crate::world_data::load_world_data(std::path::Path::new(&path));
        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        spawn_map_creatures(
            &mut commands,
            &data,
            MapContent::continent(1643),
            false,
            Some(&mut crate::test_data::ground_map()),
        );
        queue.apply(&mut world);
        let mut query = world.query::<(&Npc, &UnitFlags)>();
        let (_, flags) = query
            .iter(&world)
            .find(|(npc, _)| npc.template_id == LORD_AUTUMNVALE)
            .expect("authentic quest ender");
        assert_eq!(flags.0, AUTHORED_FLAGS);
        assert_ne!(
            flags.0 & 0x100,
            0,
            "authored player immunity survives loading"
        );
    }

    #[test]
    #[ignore = "requires WORLD_POPULATION_DB pointing to a promoted private copy"]
    fn world_population_real_copy_spawns_living_expansion_creatures() {
        let path = std::env::var("WORLD_POPULATION_DB").expect("promoted copy path");
        let data = crate::world_data::load_world_data(std::path::Path::new(&path));
        // Zandalar1642 has only one eligible source row: an inactive event12
        // Headless Ghost. Do not invent hub NPCs or activate that event.
        assert!(!data.spawns_by_map.contains_key(&1642));
        for map_id in [2444, 2552, 2601, 2222, 1643] {
            let mut world = World::new();
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let mut commands = Commands::new(&mut queue, &world);
            let count = spawn_map_creatures(
                &mut commands,
                &data,
                MapContent::continent(map_id),
                false,
                Some(&mut crate::test_data::ground_map()),
            );
            assert!(count > 0, "empty promoted map {map_id}");
            queue.apply(&mut world);
            let mut npcs = world.query::<(&Npc, &Health, &crate::maps::InMap)>();
            let actors: Vec<_> = npcs.iter(&world).collect();
            assert_eq!(actors.len(), count as usize);
            assert!(
                actors.iter().all(|(_, hp, map)| hp.current > 0.0
                    && *map == &crate::maps::InMap::continent(map_id))
            );
            println!("map {map_id}: {count} living continent NPCs");
        }
    }

    #[test]
    #[ignore = "requires DUNGEON_POPULATION_DB pointing to an imported database copy"]
    fn dungeon_population_real_copy_loads_all_maps_and_spawns_three_dungeons() {
        let path = std::env::var("DUNGEON_POPULATION_DB").expect("imported copy path");
        let data = crate::world_data::load_world_data(std::path::Path::new(&path));
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut statement = conn
            .prepare("SELECT map FROM tdb_instance_template ORDER BY map")
            .unwrap();
        let maps = statement.query_map([], |row| row.get::<_, u16>(0)).unwrap();
        for map in maps {
            let map = map.unwrap();
            let expected: usize = conn.query_row(
                "SELECT count(*) FROM content_creature c WHERE map=?1 AND
                 (NOT EXISTS (SELECT 1 FROM content_game_event_creature e WHERE e.guid=c.guid)
                  OR EXISTS (SELECT 1 FROM content_game_event_creature e WHERE e.guid=c.guid AND e.eventEntry<0))",
                [map], |row| row.get(0),
            ).unwrap();
            assert_eq!(
                data.spawns_by_map.get(&map).map_or(0, Vec::len),
                expected,
                "map {map}"
            );
        }
        for map_id in [36, 43, 389] {
            let mut world = World::new();
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let mut commands = Commands::new(&mut queue, &world);
            let content = MapContent {
                map: crate::maps::InMap {
                    map_id,
                    instance_id: 1,
                },
                difficulty_id: 1,
                dead_bosses: Vec::new(),
            };
            let expected = data.spawns_by_map[&(map_id as u16)]
                .iter()
                .filter(|&&i| data.spawns[i].spawns_at(1))
                .count();
            let count = spawn_map_creatures(
                &mut commands,
                &data,
                content,
                false,
                Some(&mut crate::test_data::ground_map()),
            );
            assert_eq!(count as usize, expected, "map {map_id}");
            queue.apply(&mut world);
            let npcs = world.query::<(&Npc, &Health, &ModelDisplay)>();
            let mut npcs = npcs;
            assert_eq!(npcs.iter(&world).count(), expected);
            assert!(
                npcs.iter(&world)
                    .all(|(_, health, display)| health.current > 0.0 && display.display_id > 0)
            );
            println!("map {map_id}: {count} living NPC entities at Normal difficulty");
        }
    }

    fn run_spawn_system(wd: WorldData) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(wd);
        app.add_systems(PostStartup, spawn_test);
        app.update();
        app
    }

    fn get_spawned(app: &mut App) -> Vec<(Position, Npc, Health, ModelDisplay, Zone)> {
        let mut query = app
            .world_mut()
            .query::<(&Position, &Npc, &Health, &ModelDisplay, &Zone)>();
        let mut results: Vec<_> = query
            .iter(app.world())
            .map(|(p, n, h, m, z)| (*p, n.clone(), *h, *m, *z))
            .collect();
        results.sort_by_key(|(_, npc, _, _, _)| npc.template_id);
        results
    }

    fn assert_creature(
        c: &(Position, Npc, Health, ModelDisplay, Zone),
        template_id: u32,
        wow_x: f32,
        health: f32,
        display: u32,
    ) {
        assert_eq!(c.1.template_id, template_id);
        // Position is now Bevy space; wow_x maps to bevy_x unchanged.
        assert_eq!(c.0.x, wow_x);
        assert_eq!(c.2.current, health);
        assert_eq!(c.2.max, health);
        assert_eq!(c.3.display_id, display);
    }

    #[test]
    fn spawn_system_creates_every_eastern_kingdoms_spawn() {
        let templates = vec![
            make_template(1, 1.5, 1001),
            make_template(2, 2.0, 1002),
            make_template(3, 1.0, 1003),
            make_template(4, 1.0, 1004),
        ];
        let kalimdor = CreatureSpawn {
            map: 1,
            ..make_spawn(4, -1000.0, 200.0, 90.0)
        };
        let spawns = vec![
            make_spawn(1, -9000.0, -100.0, 50.0), // Northshire
            make_spawn(2, -8800.0, 0.0, 60.0),
            make_spawn(3, -10500.0, 1000.0, 70.0), // Westfall
            kalimdor,
        ];
        let mut app = run_spawn_system(make_world_data(templates, spawns));
        let results = get_spawned(&mut app);

        assert_eq!(results.len(), 3, "every map 0 spawn, no map 1 spawn");
        assert_creature(&results[0], 1, -9000.0, 150.0, 1001);
        assert_creature(&results[1], 2, -8800.0, 200.0, 1002);
        assert_creature(&results[2], 3, -10500.0, 100.0, 1003);
    }

    #[test]
    fn spawned_npc_replicates_template_min_level_and_no_powers() {
        let mut template = make_template(1, 1.0, 100);
        template.min_level = 7;
        let spawns = vec![make_spawn(1, -9000.0, -100.0, 50.0)];
        let mut app = run_spawn_system(make_world_data(vec![template], spawns));
        let mut query = app.world_mut().query::<(&UnitLevel, &UnitPowers)>();
        let (level, powers) = query.single(app.world()).unwrap();
        assert_eq!(*level, UnitLevel(7));
        assert!(powers.entries.is_empty());
    }

    #[test]
    fn spawned_npc_carries_template_faction() {
        let mut template = make_template(1, 1.0, 100);
        template.faction = 11;
        let spawns = vec![make_spawn(1, -9000.0, -100.0, 50.0)];
        let mut app = run_spawn_system(make_world_data(vec![template], spawns));
        let mut query = app.world_mut().query::<&UnitFactionTemplate>();
        assert_eq!(*query.single(app.world()).unwrap(), UnitFactionTemplate(11));
    }

    #[test]
    fn spawn_system_no_map0_spawns() {
        let templates = vec![make_template(1, 1.0, 100)];
        let mut spawn = make_spawn(1, -9000.0, -100.0, 50.0);
        spawn.map = 1; // not Eastern Kingdoms
        let mut app = run_spawn_system(make_world_data(templates, vec![spawn]));
        assert_eq!(get_spawned(&mut app).len(), 0);
    }

    #[test]
    #[should_panic(expected = "creature spawn 1 references missing template 99")]
    fn spawn_system_rejects_missing_authoritative_template() {
        drop(run_spawn_system(make_world_data(
            vec![],
            vec![make_spawn(99, -9000.0, 0.0, 0.0)],
        )));
    }
}

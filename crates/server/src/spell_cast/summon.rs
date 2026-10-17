//! SPELL_EFFECT_SUMMON (28): TC a352b1fa Spell::EffectSummonType and
//! Map::SummonCreature. SummonProperties supplies control/title/faction/slot;
//! creature identity and lifetime come from SpellEffect and SpellDuration.
//! Temporary guardians share UnitSummonedBy / FollowOwner with hunter pets,
//! not their persisted stable slot (several guardians can coexist with a pet).
use super::effects::EffectCtx;
use crate::maps::InMap;
use crate::spell_info::SpellEffect;
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::components::{
    CreatureClassification, CreatureMotion, Health, Level, ModelDisplay, MovementSpeed, Npc,
    Player, Position, Rotation, UnitFactionTemplate, UnitFlags, UnitLevel, UnitSummonedBy, Zone,
};
use shared::level_scaling::LevelScaling;
use shared::protocol::NpcFlags;
use std::collections::HashMap;

pub const EFFECT_SUMMON: u32 = 28;
#[cfg(test)]
#[path = "summon_property_tests.rs"]
mod property_tests;

const CONTROL_WILD: u32 = 0;
const CONTROL_ALLY: u32 = 1;
const CONTROL_PET: u32 = 2;
const TITLE_PET: i32 = 1;
const TITLE_GUARDIAN: i32 = 2;
const TITLE_MINION: i32 = 3;
const TITLE_COMPANION: i32 = 5;
const TITLE_RUNEBLADE: i32 = 6;
/// TC EffectSummonType/Map::SummonCreature use the default TempSummon branch.
const TITLE_ASSISTANT: i32 = 35;
const IMMUNE_TO_PC_AND_NPC: u32 = 0x100 | 0x200;
const DANCING_RUNE_WEAPON: u32 = 49028;
const RUNE_WEAPON_PARRY: u32 = 81256;
/// TC a352b1fa DBCEnums.h / Spell::EffectSummonType private owner selection.
const ONLY_VISIBLE_TO_SUMMONER: u32 = 0x10;
const ONLY_VISIBLE_TO_SUMMONER_GROUP: u32 = 0x10000;
const JOIN_SUMMONER_SPAWN_GROUP: u32 = 0x200;
const USE_CREATURE_LEVEL: u32 = 0x100;
/// TC Spell::SummonGuardian scatters secondary actors within5yd of destTarget.
const GUARDIAN_SCATTER_RADIUS: f32 = 5.0;

#[derive(Debug, Clone, Copy)]
pub struct SummonProperties {
    pub control: u32,
    pub faction: u32,
    /// TC a352b1fa DB2Structure.h SummonPropertiesEntry: `int32 Title` (12.1.5 authors -1).
    pub title: i32,
    pub slot: i32,
    /// The first authored SummonProperties flag word; other flag semantics are separate.
    pub flags: u32,
}

impl SummonProperties {
    pub(crate) fn is_pet(self) -> bool {
        self.control == CONTROL_PET
    }

    fn follows_owner(self) -> bool {
        self.control == CONTROL_PET
            || self.joins_summoner_spawn_group()
            || matches!(
                self.title,
                TITLE_PET | TITLE_GUARDIAN | TITLE_MINION | TITLE_COMPANION | TITLE_RUNEBLADE
            )
    }

    fn joins_summoner_spawn_group(self) -> bool {
        self.flags & JOIN_SUMMONER_SPAWN_GROUP != 0
    }

    pub(super) fn is_guardian(self) -> bool {
        matches!(self.title, TITLE_GUARDIAN | TITLE_MINION | TITLE_RUNEBLADE)
            || (self.title == 0 && self.joins_summoner_spawn_group())
    }

    /// TC Map::SummonCreature: Guardian and Companion masks inherit Minion ownership.
    fn is_minion(self) -> bool {
        match self.control {
            CONTROL_PET => true,
            CONTROL_WILD | CONTROL_ALLY => match self.title {
                TITLE_GUARDIAN | TITLE_MINION | TITLE_RUNEBLADE | TITLE_COMPANION => true,
                _ => self.joins_summoner_spawn_group(),
            },
            _ => false,
        }
    }

    pub(crate) fn is_supported(self) -> bool {
        match self.control {
            CONTROL_PET => true,
            CONTROL_WILD | CONTROL_ALLY => matches!(
                self.title,
                0 | TITLE_PET
                    | TITLE_GUARDIAN
                    | TITLE_MINION
                    | TITLE_COMPANION
                    | TITLE_RUNEBLADE
                    | TITLE_ASSISTANT
            ),
            _ => false, // Puppet/vehicle controls and specialized titles require their own runtime.
        }
    }
}

#[derive(Resource, Default)]
pub struct SummonPropertiesStore(pub HashMap<u32, SummonProperties>);

pub fn load_summon_properties(conn: &Connection) -> rusqlite::Result<SummonPropertiesStore> {
    let mut stmt =
        conn.prepare("SELECT ID, Control, Faction, Title, Slot, Flags_0 FROM summon_properties")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            SummonProperties {
                control: row.get(1)?,
                faction: row.get(2)?,
                title: row.get(3)?,
                slot: row.get(4)?,
                flags: row.get::<_, i64>(5)? as u32,
            },
        ))
    })?;
    Ok(SummonPropertiesStore(
        rows.collect::<rusqlite::Result<_>>()?,
    ))
}

pub(super) fn load_summon_properties_startup(mut commands: Commands) {
    let path = std::path::Path::new(crate::spell_info::WORLD_DB_PATH);
    if !path.exists() {
        warn!("world.db not found: SummonProperties table empty");
        return;
    }
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open world.db for SummonProperties");
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='summon_properties')", [], |row| row.get(0))
        .expect("check SummonProperties table");
    if !exists {
        warn!(
            "SummonProperties is not imported: effect 28 cannot spawn creatures; extract/import SummonProperties with scripts/db2.py"
        );
        return;
    }
    commands.insert_resource(load_summon_properties(&conn).expect("load SummonProperties"));
}

#[derive(Component, Debug, Clone)]
pub struct SummonedCreature {
    pub owner: Entity,
    pub spell_id: u32,
    pub properties_id: u32,
    pub properties: SummonProperties,
    /// Template-model display multiplier; MOD_SCALE remains separate object scaling.
    pub display_scale: f32,
    pub expires_at: Option<f64>,
}

/// Same category/data gate for runtime and Toy Box preflight. Missing IDs remain
/// explicit; no creature or summon classification is inferred from spell names.
pub(crate) fn unsupported_reason(world: &World, effect: &SpellEffect) -> Option<String> {
    let properties_id = effect.misc_values[1] as u32;
    let properties = world
        .get_resource::<SummonPropertiesStore>()
        .and_then(|store| store.0.get(&properties_id));
    let Some(properties) = properties else {
        return Some(format!("missing SummonProperties {properties_id}"));
    };
    if !properties.is_supported() {
        return Some(format!(
            "summon control {} title {}",
            properties.control, properties.title
        ));
    }
    if properties.flags & ONLY_VISIBLE_TO_SUMMONER_GROUP != 0
        && !world.contains_resource::<crate::group::GroupRegistry>()
    {
        return Some(format!("summon {properties_id} requires group registry"));
    }
    let entry = effect.misc_values[0] as u32;
    let exists = world
        .get_resource::<crate::world_data::WorldData>()
        .is_some_and(|data| data.templates.contains_key(&entry));
    (!exists).then(|| format!("missing summon creature {entry}"))
}

pub(super) fn summon_creature(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let properties_id = effect.misc_values[1] as u32;
    let Some(properties) = world
        .resource::<SummonPropertiesStore>()
        .0
        .get(&properties_id)
        .copied()
    else {
        warn!(
            "Spell {}: missing SummonProperties {properties_id}",
            ctx.spell.id
        );
        return;
    };
    if !properties.is_supported() {
        warn!(
            "Spell {}: summon control {} / title {} unsupported",
            ctx.spell.id, properties.control, properties.title
        );
        return;
    }
    let Some(dest) = ctx.dest else {
        warn!("Spell {}: summon has no destination", ctx.spell.id);
        return;
    };
    let entry = effect.misc_values[0] as u32;
    let uses_default_lifecycle = matches!(
        (properties.control, properties.title),
        (CONTROL_WILD, 0) | (CONTROL_WILD | CONTROL_ALLY, TITLE_ASSISTANT)
    );
    if uses_default_lifecycle && !properties.is_guardian() {
        let count = roll_summon_count(world, ctx, effect, properties_id);
        for index in 0..count {
            let destination = if index == 0 {
                dest
            } else {
                scatter_wild_destination(world, ctx.caster, effect, dest)
            };
            spawn_wild_summon(world, ctx, properties_id, properties, entry, destination);
        }
        return;
    }
    spawn_owned_summons(world, ctx, effect, properties_id, properties, entry, dest);
}

fn spawn_owned_summons(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    properties_id: u32,
    properties: SummonProperties,
    entry: u32,
    dest: super::targets::Dest,
) {
    let count = if properties.is_guardian() {
        roll_summon_count(world, ctx, effect, properties_id)
    } else {
        1
    };
    for index in 0..count {
        let destination = if index == 0 {
            dest
        } else {
            scatter_summon_destination(world, ctx.caster, dest, 0.0, GUARDIAN_SCATTER_RADIUS)
        };
        let Some(template) =
            read_summon_template(world, ctx.caster, entry, properties, destination)
        else {
            warn!(
                "Spell {}: missing summon creature {entry} or source level inputs",
                ctx.spell.id
            );
            return;
        };
        if index == 0 {
            replace_slotted_summon(world, ctx.caster, properties);
        }
        spawn_summon(
            world,
            ctx,
            properties_id,
            properties,
            &template,
            destination,
        );
    }
}

/// TC EffectSummonType's explicit MiscValueB list, not all positive values.
fn roll_summon_count(
    world: &World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    properties_id: u32,
) -> usize {
    let uses_effect_count = matches!(
        properties_id,
        64 | 61 | 1101 | 66 | 648 | 2301 | 1061 | 1261 | 629 | 181 | 715 | 1562 | 833 | 1161 | 713
    );
    if !uses_effect_count {
        return 1;
    }
    let value = super::effects::roll_base_points(world, ctx.caster, ctx.spell, effect);
    (value as i32).max(1) as usize
}

/// TC GetRandomPoint: disk distribution in the primary radius, caster facing.
/// TargetB's radius selects the initial destination, not additional scatter.
fn scatter_wild_destination(
    world: &mut World,
    caster: Entity,
    effect: &SpellEffect,
    destination: super::targets::Dest,
) -> super::targets::Dest {
    let maximum = super::targets::radius(effect, 0);
    let minimum = effect.radius[0].map_or(0.0, |radius| radius.radius_min);
    scatter_summon_destination(world, caster, destination, minimum, maximum)
}

fn scatter_summon_destination(
    world: &mut World,
    caster: Entity,
    mut destination: super::targets::Dest,
    minimum: f32,
    maximum: f32,
) -> super::targets::Dest {
    if maximum > 0.0 {
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let distance = minimum + (maximum - minimum) * rand::random::<f32>().sqrt();
        destination.position.x += distance * angle.cos();
        destination.position.z -= distance * angle.sin();
        update_scattered_ground_height(world, caster, &mut destination);
    }
    destination.orientation = super::targets::orientation(world, caster);
    destination
}

/// Native UpdateGroundPositionZ retains the source Z when map height is invalid.
fn update_scattered_ground_height(
    world: &mut World,
    caster: Entity,
    destination: &mut super::targets::Dest,
) {
    let Some(map) = world.get::<InMap>(caster).copied() else {
        return;
    };
    let Some(mut ground) = world.get_resource_mut::<crate::ground::GroundMap>() else {
        return;
    };
    if let crate::ground::GroundProbe::Supported(height) =
        ground.ground_in(map, destination.position)
    {
        destination.position.y = height;
    }
}

/// TC EffectSummonType's wild/default branch uses Map::SummonCreature,
/// not guardian stats: keep the map difficulty and normal AI lifecycle.
fn spawn_wild_summon(
    world: &mut World,
    ctx: &EffectCtx,
    properties_id: u32,
    properties: SummonProperties,
    entry: u32,
    dest: super::targets::Dest,
) {
    let Some(stats) = read_wild_summon_stats(world, ctx.caster, entry, properties, dest) else {
        warn!(
            "Spell {}: wild creature {entry} lacks source level/stat inputs",
            ctx.spell.id
        );
        return;
    };
    let position = [
        dest.position.x,
        -dest.position.z,
        dest.position.y,
        dest.orientation,
    ];
    let Some(actor) = crate::zones::summon_creature(world, ctx.caster, entry, position) else {
        warn!(
            "Spell {}: unable to summon wild creature {entry}",
            ctx.spell.id
        );
        return;
    };
    initialize_wild_summon(world, ctx, properties_id, properties, actor, stats);
}

fn initialize_wild_summon(
    world: &mut World,
    ctx: &EffectCtx,
    properties_id: u32,
    properties: SummonProperties,
    actor: Entity,
    stats: WildSummonStats,
) {
    let display_scale = read_actor_display_scale(world, actor);
    let metadata = snapshot_summon_metadata(world, ctx, properties_id, properties, display_scale);
    let private_owner = private_summon_owner(world, ctx.caster, properties.flags);
    let mut entity = world.entity_mut(actor);
    entity.insert((
        // Native scale1 is separate from the model's DisplayScale multiplier.
        crate::zones::NativeObjectScale(1.0),
        UnitSummonedBy(ctx.caster.to_bits()),
        metadata,
        stats.level,
        stats.health,
        stats.armor,
    ));
    if properties.faction != 0 {
        entity.insert(UnitFactionTemplate(properties.faction));
    }
    if let Some(mana) = stats.mana {
        entity.insert(mana);
    }
    if let Some(scaling) = stats.scaling {
        entity.insert(scaling);
    } else {
        // InitStats owner tuning zero disables HasScalableLevels, even when
        // the ordinary spawn path selected template scaling components.
        entity.remove::<(LevelScaling, crate::level_scaling::ScaledStats)>();
    }
    if let Some(scaled_stats) = stats.scaled_stats {
        entity.insert(scaled_stats);
    }
    insert_summon_privacy(&mut entity, private_owner);
    drop(entity);
    super::log_summon(world, ctx.caster, actor, ctx.spell);
    crate::auras::auras_changed(world, actor);
}

struct WildSummonStats {
    level: UnitLevel,
    health: Health,
    armor: crate::level_scaling::CreatureArmor,
    mana: Option<shared::components::Mana>,
    scaling: Option<LevelScaling>,
    scaled_stats: Option<crate::level_scaling::ScaledStats>,
}

fn read_wild_summon_stats(
    world: &mut World,
    owner: Entity,
    entry: u32,
    properties: SummonProperties,
    dest: super::targets::Dest,
) -> Option<WildSummonStats> {
    let selected = roll_summon_level(world, owner, entry, properties, dest)?;
    // TempSummon::InitStats inherits the owner's tuning only without UseCreatureLevel.
    // Absent owner scaling represents the source's zero-initialized tuning field.
    let scaling = if properties.flags & USE_CREATURE_LEVEL != 0 {
        selected.scaling
    } else {
        world.get::<LevelScaling>(owner).copied()
    };
    let data = world.get_resource::<crate::world_data::WorldData>()?;
    let template = data.template_for(entry, selected.difficulty)?;
    calculate_wild_summon_stats(data, template, selected.level, scaling)
}

fn calculate_wild_summon_stats(
    data: &crate::world_data::WorldData,
    template: &crate::world_data::CreatureTemplate,
    level: u8,
    scaling: Option<LevelScaling>,
) -> Option<WildSummonStats> {
    let tuning = scaling.map_or(0, |scaling| scaling.content_tuning_id);
    let (base_health, base_armor) = data.expected_stats.creature_health_armor(
        level,
        template.health_scaling_expansion,
        tuning,
        template.unit_class,
    );
    let health = (base_health * template.health_modifier).ceil();
    Some(WildSummonStats {
        level: UnitLevel(level),
        health: Health {
            current: health,
            max: health,
        },
        armor: crate::level_scaling::CreatureArmor(base_armor * template.armor_modifier),
        mana: read_wild_summon_mana(data, template, level)?,
        scaling,
        scaled_stats: scaling.map(|scaling| build_wild_summon_scaling(data, template, scaling)),
    })
}

fn build_wild_summon_scaling(
    data: &crate::world_data::WorldData,
    template: &crate::world_data::CreatureTemplate,
    scaling: LevelScaling,
) -> crate::level_scaling::ScaledStats {
    use crate::level_scaling::{LevelStats, ScaledStats, TuningInputs};
    let inputs = TuningInputs {
        content_tuning_id: scaling.content_tuning_id,
        health_scaling_expansion: template.health_scaling_expansion,
        unit_class: template.unit_class,
        health_modifier: template.health_modifier,
        damage_modifier: template.damage_modifier,
        armor_modifier: template.armor_modifier,
    };
    ScaledStats(std::sync::Arc::new(LevelStats::build(
        &data.expected_stats,
        &inputs,
    )))
}

fn read_wild_summon_mana(
    data: &crate::world_data::WorldData,
    template: &crate::world_data::CreatureTemplate,
    level: u8,
) -> Option<Option<shared::components::Mana>> {
    if !matches!(template.unit_class, 2 | 8) {
        return Some(None);
    }
    let base = data
        .class_level_base_mana
        .get(&(level, template.unit_class))?;
    let max = (*base as f32 * template.mana_modifier).round();
    Some(Some(shared::components::Mana { current: max, max }))
}

fn snapshot_summon_metadata(
    world: &World,
    ctx: &EffectCtx,
    properties_id: u32,
    properties: SummonProperties,
    display_scale: f32,
) -> SummonedCreature {
    let duration = super::duration_ms(world, ctx.caster, ctx.spell);
    let expires_at =
        (duration > 0).then(|| crate::auras::aura_now(world) + f64::from(duration) / 1000.0);
    SummonedCreature {
        owner: ctx.caster,
        spell_id: ctx.spell.id,
        properties_id,
        properties,
        display_scale,
        expires_at,
    }
}

fn read_actor_display_scale(world: &World, actor: Entity) -> f32 {
    let entry = world
        .get::<Npc>(actor)
        .expect("spawned wild summon NPC")
        .template_id;
    let display = world
        .get::<ModelDisplay>(actor)
        .expect("spawned wild summon display");
    let difficulty = world
        .get::<crate::zones::CreatureDifficulty>(actor)
        .map_or(0, |difficulty| difficulty.0);
    let template = world
        .resource::<crate::world_data::WorldData>()
        .template_for(entry, difficulty)
        .expect("spawned wild summon template");
    model_display_scale(template, display.display_id)
}

/// TC Unit::SetDisplayId: GetModelWithDisplayId supplies the display multiplier;
/// DEFAULT_PLAYER_DISPLAY_SCALE is1 when no template model matches.
fn model_display_scale(template: &crate::world_data::CreatureTemplate, display_id: u32) -> f32 {
    template
        .models
        .iter()
        .find(|model| model.display_id == display_id)
        .map_or(1.0, |model| model.display_scale)
}

fn replace_slotted_summon(world: &mut World, owner: Entity, properties: SummonProperties) {
    // Slot 0 / -1 does not claim a fixed totem slot. No persistent pet slot is touched.
    if properties.slot <= 0 {
        return;
    }
    let previous: Vec<_> = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == owner && summon.properties.slot == properties.slot)
        .map(|(entity, _)| entity)
        .collect();
    for entity in previous {
        despawn_summon(world, entity);
    }
}

struct SummonTemplate {
    entry: u32,
    level: u8,
    name: String,
    health: f32,
    faction: u32,
    classification: CreatureClassification,
    display_id: u32,
    display_scale: f32,
}

fn read_summon_template(
    world: &mut World,
    owner: Entity,
    entry: u32,
    properties: SummonProperties,
    dest: super::targets::Dest,
) -> Option<SummonTemplate> {
    let selected = roll_summon_level(world, owner, entry, properties, dest)?;
    let level = selected.level;
    let data = world.get_resource::<crate::world_data::WorldData>()?;
    let template = data.template_for(entry, selected.difficulty)?;
    let display_id = crate::zones::template_display_id(template, &mut rand::thread_rng());
    let display_scale = model_display_scale(template, display_id);
    Some(SummonTemplate {
        entry,
        level,
        name: template.name.clone(),
        health: crate::hunter_pet::pet_max_health(data, template, level),
        faction: u32::from(template.faction),
        classification: CreatureClassification::from_rank(template.rank)
            .expect("validated creature rank"),
        display_id,
        display_scale,
    })
}

fn read_summoner_level(world: &World, owner: Entity) -> Option<u8> {
    if world.get::<Player>(owner).is_some() {
        world.get::<Level>(owner).map(|level| level.0)
    } else {
        world.get::<UnitLevel>(owner).map(|level| level.0)
    }
}

struct SelectedSummonLevel {
    difficulty: u32,
    level: u8,
    scaling: Option<LevelScaling>,
}

/// Select the source native level before reading dependent stats.
fn roll_summon_level(
    world: &mut World,
    owner: Entity,
    entry: u32,
    properties: SummonProperties,
    dest: super::targets::Dest,
) -> Option<SelectedSummonLevel> {
    let owner_level = read_summoner_level(world, owner)?;
    let uses_native_level =
        properties.is_guardian() || (properties.control == CONTROL_WILD && properties.title == 0);
    if !uses_native_level {
        return Some(SelectedSummonLevel {
            difficulty: 0,
            level: owner_level,
            scaling: None,
        });
    }
    let map = *world.get::<InMap>(owner)?;
    let difficulty = crate::zones::map_difficulty_id(world, map);
    let area = world
        .get_resource_mut::<crate::ground::GroundMap>()
        .and_then(|mut ground| ground.area_at(map.map_id, dest.position));
    let data = world.get_resource::<crate::world_data::WorldData>()?;
    let template = data.template_for(entry, difficulty)?;
    let range = data
        .summon_levels
        .guardian_range(template, map.map_id, difficulty, area)?;
    let (level, scaling) = roll_guardian_level(owner_level, range, properties.flags);
    Some(SelectedSummonLevel {
        difficulty,
        level,
        scaling,
    })
}

/// Creature::SelectLevel precedes TempSummon::InitStats and Guardian::InitStats.
fn roll_guardian_level(
    owner: u8,
    inputs: crate::world_data::summon_levels::GuardianLevelRange,
    flags: u32,
) -> (u8, Option<LevelScaling>) {
    use rand::Rng;
    let (range, delta) = (inputs.range, inputs.delta);
    let delta = if delta.0 == delta.1 {
        delta.0
    } else {
        rand::thread_rng().gen_range(delta.0.min(delta.1)..=delta.0.max(delta.1))
    };
    let minimum = i32::from(range.0) + i32::from(delta);
    let maximum = i32::from(range.1) + i32::from(delta);
    let level = if flags & USE_CREATURE_LEVEL != 0 {
        maximum.clamp(1, i32::from(u8::MAX)) as u8
    } else {
        // Source InitStats does not clamp these bounds to 1 after the authored delta.
        i32::from(owner).clamp(minimum, maximum) as u8
    };
    let scaling = (inputs.content_tuning_id != 0).then_some(LevelScaling {
        content_tuning_id: inputs.content_tuning_id,
        min_level: range.0,
        max_level: range.1,
        delta,
    });
    (level, scaling)
}

/// TC Unit::SetMinion sets PLAYER_CONTROLLED only for player owners.
/// Companion immunity augments ownership flags instead of replacing them.
fn calculate_summon_unit_flags(owner_is_player: bool, properties: SummonProperties) -> u32 {
    let ownership = if owner_is_player && properties.is_minion() {
        crate::faction::UNIT_FLAG_PLAYER_CONTROLLED
    } else {
        0
    };
    let immunity = if properties.title == TITLE_COMPANION {
        IMMUNE_TO_PC_AND_NPC | UnitFlags::NON_ATTACKABLE_2
    } else {
        0
    };
    ownership | immunity
}

fn spawn_summon(
    world: &mut World,
    ctx: &EffectCtx,
    properties_id: u32,
    properties: SummonProperties,
    template: &SummonTemplate,
    dest: super::targets::Dest,
) {
    let owner = ctx.caster;
    let Some(map) = world.get::<InMap>(owner).copied() else {
        return;
    };
    let level = template.level;
    let health = template.health;
    let faction = if properties.faction != 0 {
        properties.faction
    } else if properties.control == CONTROL_WILD && !properties.follows_owner() {
        template.faction
    } else {
        world
            .get::<UnitFactionTemplate>(owner)
            .map_or(template.faction, |faction| faction.0)
    };
    let metadata = snapshot_summon_metadata(
        world,
        ctx,
        properties_id,
        properties,
        template.display_scale,
    );
    let zone = world.get::<Zone>(owner).copied().unwrap_or(Zone { id: 0 });
    let replicate = world.get::<Replicate>(owner).is_some();
    let display_id = template.display_id;
    let private_owner = private_summon_owner(world, owner, properties.flags);
    let unit_flags = calculate_summon_unit_flags(world.get::<Player>(owner).is_some(), properties);
    let mut entity = world.spawn((
        (
            Position {
                x: dest.position.x,
                y: dest.position.y,
                z: dest.position.z,
            },
            Rotation {
                x: 0.0,
                y: dest.orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            Npc {
                template_id: template.entry,
                name: template.name.clone(),
            },
            Health {
                current: health,
                max: health,
            },
            ModelDisplay { display_id },
            shared::components::UnitScale(template.display_scale),
            zone,
            map,
        ),
        (
            UnitLevel(level),
            template.classification,
            UnitFactionTemplate(faction),
            UnitSummonedBy(owner.to_bits()),
            NpcFlags(0),
            CreatureMotion::Still,
            MovementSpeed(0.0),
            crate::networking::CombatTarget::default(),
            metadata,
        ),
    ));
    let actor = entity.id();
    entity.world_scope(|world| {
        super::guardian::initialize(world, actor);
        //49028's description explicitly grants81256 while its weapon is active.
        //Attach to actual spawn, not AfterCast: missing templates spawn nothing.
        if ctx.spell.id == DANCING_RUNE_WEAPON {
            super::cast_triggered(world, owner, RUNE_WEAPON_PARRY, Some(owner), ctx.depth + 1);
        }
    });
    if unit_flags != 0 {
        entity.insert(UnitFlags(unit_flags));
    }
    if properties.is_pet() {
        entity.insert(shared::components::UnitIsPet(true));
    }
    if properties.follows_owner() {
        entity.insert(crate::creature_ai::FollowOwner::new(owner));
    }

    insert_summon_privacy(&mut entity, private_owner);
    if replicate {
        entity.insert((
            Replicate::to_clients(NetworkTarget::All),
            crate::interest::NetworkVisibility::default(),
        ));
    }
    drop(entity);
    // TC TemporarySummon.cpp: Guardian::InitSummon calls TempSummon::InitSummon,
    // including the creature summoner's JustSummoned (BossAI's SummonList).
    world
        .entity_mut(actor)
        .insert(crate::zones::TempSummon { summoner: owner });
    crate::scripted_ai::dispatch(world, actor, crate::scripted_ai::Hook::IsSummonedBy(owner));
    crate::scripted_ai::dispatch(world, owner, crate::scripted_ai::Hook::JustSummoned(actor));
    super::log_summon(world, owner, actor, ctx.spell);
}

#[derive(Clone, Copy)]
enum PrivateSummonOwner {
    Player(Entity),
    Group(crate::group::GroupId),
}

fn private_summon_owner(world: &World, caster: Entity, flags: u32) -> Option<PrivateSummonOwner> {
    if flags & (ONLY_VISIBLE_TO_SUMMONER | ONLY_VISIBLE_TO_SUMMONER_GROUP) == 0 {
        return None;
    }
    // TC EffectSummonType: inherited privacy precedes either requested flag.
    if let Some(group) = world.get::<crate::interest::PrivateObjectGroup>(caster) {
        return Some(PrivateSummonOwner::Group(group.0));
    }
    if let Some(owner) = world.get::<crate::interest::PrivateObjectOwner>(caster) {
        return Some(PrivateSummonOwner::Player(owner.0));
    }
    if flags & ONLY_VISIBLE_TO_SUMMONER_GROUP != 0
        && let Some(group) = read_caster_group(world, caster)
    {
        return Some(PrivateSummonOwner::Group(group));
    }
    // Authored ungrouped case is the caster GUID, never public visibility.
    Some(PrivateSummonOwner::Player(caster))
}

fn read_caster_group(world: &World, caster: Entity) -> Option<crate::group::GroupId> {
    world.get::<shared::components::Player>(caster)?;
    let character = world.get::<crate::persistence::ClientId>(caster)?;
    world
        .resource::<crate::group::GroupRegistry>()
        .group_id(character.0)
}

fn insert_summon_privacy(entity: &mut EntityWorldMut<'_>, owner: Option<PrivateSummonOwner>) {
    match owner {
        Some(PrivateSummonOwner::Player(owner)) => {
            entity.insert(crate::interest::PrivateObjectOwner(owner));
        }
        Some(PrivateSummonOwner::Group(group)) => {
            entity.insert(crate::interest::PrivateObjectGroup(group));
        }
        None => {}
    }
}

pub(super) fn expire_summons(world: &mut World) {
    let now = crate::auras::aura_now(world);
    let expired: Vec<_> = world
        .query::<(Entity, &SummonedCreature, &InMap)>()
        .iter(world)
        .filter(|(_, summon, map)| {
            summon.expires_at.is_some_and(|expiry| now >= expiry)
                || world.get::<InMap>(summon.owner) != Some(*map)
                // TC TempSummon::Update retains wild summons' own lifetimes.
                // Only minion ownership ties this cleanup to owner death.
                || (summon.properties.is_minion()
                    && !super::targets::is_alive(world, summon.owner))
        })
        .map(|(entity, _, _)| entity)
        .collect();
    for entity in expired {
        despawn_summon(world, entity);
    }
}

pub(crate) fn despawn_summon(world: &mut World, entity: Entity) {
    if let Some(summon) = world.get::<SummonedCreature>(entity) {
        if summon.spell_id == DANCING_RUNE_WEAPON {
            let owner = summon.owner;
            crate::auras::remove_auras(
                world,
                owner,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == RUNE_WEAPON_PARRY && aura.caster == owner,
            );
        }
    }
    if let Some(summon) = world.get::<SummonedCreature>(entity) {
        debug!(
            "Despawn summon {entity}: spell {}, properties {}",
            summon.spell_id, summon.properties_id
        );
    }
    crate::combat::combat_stop(world, entity);
    world.entity_mut(entity).despawn();
}

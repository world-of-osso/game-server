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
    Position, Rotation, UnitFactionTemplate, UnitFlags, UnitLevel, UnitSummonedBy, Zone,
};
use shared::protocol::NpcFlags;
use std::collections::HashMap;

pub const EFFECT_SUMMON: u32 = 28;
const CONTROL_WILD: u32 = 0;
const CONTROL_ALLY: u32 = 1;
const CONTROL_PET: u32 = 2;
const TITLE_PET: u32 = 1;
const TITLE_GUARDIAN: u32 = 2;
const TITLE_MINION: u32 = 3;
const TITLE_COMPANION: u32 = 5;
const TITLE_RUNEBLADE: u32 = 6;
const IMMUNE_TO_PC_AND_NPC: u32 = 0x100 | 0x200;
const DANCING_RUNE_WEAPON: u32 = 49028;
const RUNE_WEAPON_PARRY: u32 = 81256;
/// TC a352b1fa DBCEnums.h / Spell::EffectSummonType private owner selection.
const ONLY_VISIBLE_TO_SUMMONER: u32 = 0x10;
const ONLY_VISIBLE_TO_SUMMONER_GROUP: u32 = 0x10000;

#[derive(Debug, Clone, Copy)]
pub struct SummonProperties {
    pub control: u32,
    pub faction: u32,
    pub title: u32,
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
            || matches!(
                self.title,
                TITLE_PET | TITLE_GUARDIAN | TITLE_MINION | TITLE_COMPANION | TITLE_RUNEBLADE
            )
    }

    pub(super) fn is_guardian(self) -> bool {
        matches!(self.title, TITLE_GUARDIAN | TITLE_MINION | TITLE_RUNEBLADE)
    }

    pub(crate) fn is_supported(self) -> bool {
        match self.control {
            CONTROL_PET => true,
            CONTROL_WILD | CONTROL_ALLY => matches!(
                self.title,
                0 | TITLE_PET | TITLE_GUARDIAN | TITLE_MINION | TITLE_COMPANION | TITLE_RUNEBLADE
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
    if properties.flags & ONLY_VISIBLE_TO_SUMMONER_GROUP != 0 {
        return Some(format!(
            "summon {properties_id} requires private group visibility"
        ));
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
    if properties.control == CONTROL_WILD && properties.title == 0 {
        spawn_wild_summon(world, ctx, properties_id, properties, entry, dest);
        return;
    }
    let level = world.get::<Level>(ctx.caster).map_or(1, |level| level.0);
    let Some(template) = read_summon_template(world, entry, level) else {
        warn!("Spell {}: missing summon creature {entry}", ctx.spell.id);
        return;
    };
    replace_slotted_summon(world, ctx.caster, properties);
    spawn_summon(world, ctx, properties_id, properties, &template, dest);
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
    let display_scale = read_actor_display_scale(world, actor);
    let metadata = snapshot_summon_metadata(world, ctx, properties_id, properties, display_scale);
    let private_owner = private_summon_owner(world, ctx.caster, properties.flags);
    let mut entity = world.entity_mut(actor);
    entity.insert((
        // Model DisplayScale is a separate factor, not native object scale.
        // Current authored wild templates have native object scale1, like the
        // existing guardian path; do not multiply the model factor twice.
        crate::zones::NativeObjectScale(1.0),
        UnitSummonedBy(ctx.caster.to_bits()),
        metadata,
    ));
    if properties.faction != 0 {
        entity.insert(UnitFactionTemplate(properties.faction));
    }
    if let Some(owner) = private_owner {
        entity.insert(crate::interest::PrivateObjectOwner(owner));
    }
    drop(entity);
    crate::auras::auras_changed(world, actor);
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
    name: String,
    health: f32,
    faction: u32,
    classification: CreatureClassification,
    display_id: u32,
    display_scale: f32,
}

fn read_summon_template(world: &World, entry: u32, level: u8) -> Option<SummonTemplate> {
    let data = world.get_resource::<crate::world_data::WorldData>()?;
    let template = data.template_for(entry, 0)?;
    let display_id = crate::zones::template_display_id(template, &mut rand::thread_rng());
    let display_scale = model_display_scale(template, display_id);
    Some(SummonTemplate {
        entry,
        name: template.name.clone(),
        health: crate::hunter_pet::pet_max_health(data, template, level),
        faction: u32::from(template.faction),
        classification: CreatureClassification::from_rank(template.rank)
            .expect("validated creature rank"),
        display_id,
        display_scale,
    })
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
    let level = world.get::<Level>(owner).map_or(1, |level| level.0);
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
    if properties.control == CONTROL_ALLY || properties.control == CONTROL_PET {
        entity.insert(UnitFlags(crate::faction::UNIT_FLAG_PLAYER_CONTROLLED));
    }
    if properties.follows_owner() {
        entity.insert(crate::creature_ai::FollowOwner::new(owner));
    }

    if properties.title == TITLE_COMPANION {
        // TC EffectSummonType: companions are minions immune to both players and NPCs.
        entity.insert(UnitFlags(
            IMMUNE_TO_PC_AND_NPC | UnitFlags::NON_ATTACKABLE_2,
        ));
    }
    if let Some(owner) = private_owner {
        entity.insert(crate::interest::PrivateObjectOwner(owner));
    }
    if replicate {
        entity.insert((
            Replicate::to_clients(NetworkTarget::All),
            crate::interest::NetworkVisibility::default(),
        ));
    }
}

fn private_summon_owner(world: &World, caster: Entity, flags: u32) -> Option<Entity> {
    // Group-private summons remain an explicit toy preflight gap; do not reinterpret
    // their group GUID as owner-only. Other class summon behavior is not changed here.
    if flags & ONLY_VISIBLE_TO_SUMMONER_GROUP != 0 || flags & ONLY_VISIBLE_TO_SUMMONER == 0 {
        return None;
    }
    // TC: an already-private caster passes its actual private owner to the child.
    let owner = match world.get::<crate::interest::PrivateObjectOwner>(caster) {
        Some(owner) => owner.0,
        None => caster,
    };
    Some(owner)
}

pub(super) fn expire_summons(world: &mut World) {
    let now = crate::auras::aura_now(world);
    let expired: Vec<_> = world
        .query::<(Entity, &SummonedCreature, &InMap)>()
        .iter(world)
        .filter(|(_, summon, map)| {
            summon.expires_at.is_some_and(|expiry| now >= expiry)
                || world.get::<InMap>(summon.owner) != Some(*map)
                || !super::targets::is_alive(world, summon.owner)
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

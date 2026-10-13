//! Effect50 (TRANS_DOOR / EffectTransmitted), not just summoning rituals.
//! TC a352b1fa SpellEffects.cpp: destination, template, owner and spell duration.
//! Decoration, chairs, mailboxes and source cooking-only foci have complete behavior.
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, Row};
use shared::components::{Position, Rotation};
use shared::protocol::{
    GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_GENERIC, GAMEOBJECT_TYPE_MAILBOX,
    GAMEOBJECT_TYPE_RITUAL, GAMEOBJECT_TYPE_SPELL_FOCUS, GameObjectInfo,
};

use super::GameObjectBounds;
use crate::interest::NetworkVisibility;
use crate::spell_cast::{duration_ms, effects::EffectCtx, now};
use crate::spell_info::SpellEffect;

pub(crate) const EFFECT_TRANSMITTED: u32 = 50;

#[derive(Debug, Clone)]
struct ObjectTemplate {
    info: GameObjectInfo,
    bounds: Option<GameObjectBounds>,
    chair: Option<crate::chair::Chair>,
    scripted: bool,
    spellcaster: Option<super::spellcaster::SpellcasterObject>,
    cooking_focus: Option<(u32, f32)>,
    extra_focus_payload: bool,
    linked_trap: u32,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct SummonedObjectTemplates(
    HashMap<u32, ObjectTemplate>,
    Option<super::cooking_traps::TrapTemplate>,
);

/// Owner and spell remain distinct from a ritual's channel-held ownership.
#[derive(Component, Debug)]
pub(crate) struct SummonedObject {
    pub owner: Entity,
    pub spell_id: u32,
    pub expires_at: Option<f64>,
}

fn template_from_row(row: &Row, conn: &Connection) -> rusqlite::Result<(u32, ObjectTemplate)> {
    let entry = row.get(0)?;
    let go_type = row.get(1)?;
    let scale = row.get::<_, f64>(4)? as f32;
    let geo = |i| row.get::<_, f64>(i).map(|v| v as f32);
    let bounds = row
        .get::<_, Option<u32>>(9)?
        .map(|_| -> rusqlite::Result<_> {
            Ok(GameObjectBounds {
                min: Vec3::new(geo(10)?, geo(11)?, geo(12)?),
                max: Vec3::new(geo(13)?, geo(14)?, geo(15)?),
                scale,
                orientation: 0.0,
            })
        })
        .transpose()?;
    Ok((
        entry,
        ObjectTemplate {
            info: GameObjectInfo {
                entry,
                go_type,
                display_id: row.get(2)?,
                name: row.get(3)?,
                scale,
            },
            bounds,
            chair: (go_type == GAMEOBJECT_TYPE_CHAIR)
                .then(|| -> rusqlite::Result<_> {
                    Ok(crate::chair::Chair::new(row.get(5)?, row.get(6)?))
                })
                .transpose()?,
            scripted: !row.get::<_, String>(7)?.is_empty() || !row.get::<_, String>(8)?.is_empty(),
            spellcaster: (go_type == super::spellcaster::GAMEOBJECT_TYPE_SPELLCASTER)
                .then(|| -> rusqlite::Result<_> {
                    Ok(super::spellcaster::SpellcasterObject {
                        spell: row.get(16)?,
                        charges: row.get(17)?,
                        party_only: row.get::<_, u32>(18)? != 0,
                        identity: super::identity_conditions::load_condition(conn, row.get(19)?)?,
                        radius_override: row.get(20)?,
                        uses: 0,
                    })
                })
                .transpose()?,
            cooking_focus: (go_type == GAMEOBJECT_TYPE_SPELL_FOCUS)
                .then(|| -> rusqlite::Result<_> { Ok((row.get(5)?, geo(6)?)) })
                .transpose()?,
            extra_focus_payload: row.get(21)?,
            // Column 18 is t.Data2: a type8 focus's linked trap entry.
            linked_trap: row.get(18)?,
        },
    ))
}

pub(crate) fn load_templates(conn: &Connection) -> rusqlite::Result<SummonedObjectTemplates> {
    let additional_focus_fields = (3..=34)
        .map(|index| format!("t.Data{index}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let focus_type = GAMEOBJECT_TYPE_SPELL_FOCUS;
    let sql = format!("SELECT t.entry,t.type,t.displayId,t.name,t.size,t.Data0,t.Data1,t.AIName,t.ScriptName,
        d.ID,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5,
        t.Data0,t.Data1,t.Data2,t.Data5,t.Data8,
        t.type={focus_type} AND ({additional_focus_fields})<>0
        FROM tdb_gameobject_template t LEFT JOIN game_object_display_info d ON d.ID=t.displayId
        WHERE t.type={focus_type} OR t.entry IN (SELECT EffectMiscValue_0 FROM spell_effect WHERE Effect=50 AND DifficultyID=0)");
    let mut query = conn.prepare(&sql)?;
    let templates = query
        .query_map([], |row| template_from_row(row, conn))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(SummonedObjectTemplates(
        templates,
        super::cooking_traps::load_template(conn)?,
    ))
}

pub(crate) fn unsupported_reason(world: &World, effect: &SpellEffect) -> Option<String> {
    let entry = effect.misc_values[0] as u32;
    if world
        .get_resource::<crate::meeting_stone::MeetingStoneData>()
        .and_then(|data| data.ritual(entry))
        .is_some()
    {
        return None;
    }
    let template = world
        .get_resource::<SummonedObjectTemplates>()
        .and_then(|t| t.0.get(&entry));
    let Some(template) = template else {
        return Some(format!("missing gameobject template {entry}"));
    };
    if template.info.go_type == GAMEOBJECT_TYPE_RITUAL {
        return world
            .get_resource::<crate::meeting_stone::MeetingStoneData>()
            .and_then(|data| data.ritual(entry))
            .is_none()
            .then(|| format!("missing ritual gameobject template {entry}"));
    }
    if template.scripted {
        return Some(format!("gameobject {entry} requires custom script"));
    }
    if let Some(spellcaster) = &template.spellcaster {
        return spellcaster
            .identity
            .as_ref()
            .err()
            .map(|reason| format!("gameobject {entry}: {reason}"))
            .or_else(|| {
                template.bounds.is_none().then(|| {
                    format!(
                        "gameobject {entry} missing display {}",
                        template.info.display_id
                    )
                })
            });
    }
    let cooking_only = template
        .cooking_focus
        .is_some_and(|(id, radius)| id == super::cooking_focus::COOKING_FIRE && radius > 0.0)
        && !template.extra_focus_payload;
    let ordinary_object = matches!(
        template.info.go_type,
        GAMEOBJECT_TYPE_GENERIC | GAMEOBJECT_TYPE_CHAIR | GAMEOBJECT_TYPE_MAILBOX
    );
    if cooking_only && template.linked_trap != 0 {
        let trap_supported = template.linked_trap == super::cooking_traps::BASIC_CAMPFIRE_TRAP
            && world.resource::<SummonedObjectTemplates>().1.is_some()
            && super::cooking_traps::spell_supported(world);
        if !trap_supported {
            return Some(format!(
                "gameobject {entry} requires unsupported linked cooking trap {}",
                template.linked_trap
            ));
        }
    }
    if !ordinary_object && !cooking_only {
        return Some(format!(
            "gameobject {entry} type {} requires interaction runtime",
            template.info.go_type
        ));
    }
    template.bounds.is_none().then(|| {
        format!(
            "gameobject {entry} missing display {}",
            template.info.display_id
        )
    })
}

pub(crate) fn cooking_focus_radius(world: &World, entry: u32) -> Option<f32> {
    let template = world
        .get_resource::<SummonedObjectTemplates>()?
        .0
        .get(&entry)?;
    template
        .cooking_focus
        .filter(|(id, _)| *id == super::cooking_focus::COOKING_FIRE)
        .map(|(_, radius)| radius)
}

pub(crate) fn spawn_transmitted(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    if let Some(reason) = unsupported_reason(world, effect) {
        warn!("Spell {}: {reason}", ctx.spell.id);
        return;
    }
    let entry = effect.misc_values[0] as u32;
    if world
        .get_resource::<crate::meeting_stone::MeetingStoneData>()
        .and_then(|data| data.ritual(entry))
        .is_some()
    {
        crate::meeting_stone::ritual::effect_trans_door(world, ctx, effect);
        return;
    }
    let template = world.resource::<SummonedObjectTemplates>().0[&entry].clone();
    let Some(dest) = ctx.dest else {
        warn!(
            "Spell {}: transmitted object has no destination",
            ctx.spell.id
        );
        return;
    };
    let Some(map) = world.get::<crate::maps::InMap>(ctx.caster).copied() else {
        return;
    };
    let duration = duration_ms(world, ctx.caster, ctx.spell);
    let expires_at = (duration > 0).then(|| now(world) + f64::from(duration / 1000));
    spawn_object(world, ctx, template, dest, map, expires_at);
}

fn spawn_object(
    world: &mut World,
    ctx: &EffectCtx,
    template: ObjectTemplate,
    dest: crate::spell_cast::targets::Dest,
    map: crate::maps::InMap,
    expires_at: Option<f64>,
) {
    let bounds = GameObjectBounds {
        orientation: dest.orientation,
        ..template
            .bounds
            .expect("supported template has display bounds")
    };
    let linked_trap = (template.linked_trap != 0 && template.cooking_focus.is_some())
        .then(|| world.resource::<SummonedObjectTemplates>().1.clone())
        .flatten();
    let mut object = world.spawn((
        Position {
            x: dest.position.x,
            y: dest.position.y,
            z: dest.position.z,
        },
        Rotation {
            x: 0.0,
            y: dest.orientation + FRAC_PI_2,
            z: 0.0,
        },
        map,
        template.info,
        bounds,
        SummonedObject {
            owner: ctx.caster,
            spell_id: ctx.spell.id,
            expires_at,
        },
        Replicate::to_clients(NetworkTarget::All),
        NetworkVisibility::default(),
    ));
    if let Some(chair) = template.chair {
        object.insert(chair);
    }
    if let Some(spellcaster) = template.spellcaster {
        object.insert(spellcaster);
    }
    let parent = object.id();
    if let Some(trap) = linked_trap {
        super::cooking_traps::spawn(world, parent, trap);
    }
}

pub(crate) fn expire_objects(world: &mut World) {
    let time = now(world);
    let mut query = world.query::<(Entity, &SummonedObject)>();
    let expired: Vec<_> = query
        .iter(world)
        .filter(|(_, object)| object.expires_at.is_some_and(|until| until <= time))
        .map(|(entity, object)| (entity, object.owner, object.spell_id))
        .collect();
    for (entity, owner, spell_id) in expired {
        debug!("Spell {spell_id}: object {entity} owned by {owner} expired");
        world.despawn(entity);
    }
    super::cooking_traps::update(world);
}

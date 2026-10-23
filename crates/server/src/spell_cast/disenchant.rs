//! Retail effect99: owned bag target, ItemDisenchantLoot eligibility, TDB loot.
//! TC a352b1fa Item.cpp:GetBaseDisenchantLoot; SpellEffects.cpp:EffectDisEnchant.
use super::{CastFailure, effects::EffectCtx};
use crate::inventory::{inventory_model::InventoryModel, load_model};
use crate::item_templates::ItemTemplates;
use crate::persistence::{ClientId, GameDatabase};
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;
use shared::item_data::ItemDefinitionSource::Retail;
use std::collections::HashMap;

#[cfg(test)]
#[path = "disenchant_cache_tests.rs"]
mod cache_tests;

pub const EFFECT_DISENCHANT: u32 = 99;
const ENCHANTING: u32 = 333;

/// One checked, nonempty roll held between completion validation and effect99.
#[derive(Component)]
struct PendingDisenchantLoot(crate::loot::RolledLoot);

#[derive(Clone, Copy)]
struct Eligibility {
    loot_id: u32,
    skill: u16,
}

#[cfg_attr(test, derive(Clone))]
#[derive(Resource, Default)]
pub(crate) struct DisenchantData {
    items: HashMap<u32, Eligibility>,
    loot: crate::loot::LootTables,
}

pub(crate) fn load_data(conn: &rusqlite::Connection) -> rusqlite::Result<DisenchantData> {
    #[cfg(test)]
    {
        crate::test_data::catalog_cache::load_connection(conn, || read_data(conn))
    }
    #[cfg(not(test))]
    {
        read_data(conn)
    }
}

fn read_data(conn: &rusqlite::Connection) -> rusqlite::Result<DisenchantData> {
    // Flags0: CONJURED=2, NO_DISENCHANT=0x8000. Never use legacy DisenchantID.
    let mut stmt = conn.prepare(
        "SELECT c.ID, d.ID, d.SkillRequired
         FROM content_item c JOIN item_sparse s ON s.ID=c.ID
         JOIN item_disenchant_loot d ON d.Class=c.ClassID
           AND (d.Subclass<0 OR d.Subclass=c.SubclassID)
           AND d.Quality=c.Quality
           AND c.ItemLevel BETWEEN d.MinLevel AND d.MaxLevel
           AND (d.ExpansionID=-2 OR d.ExpansionID=s.ExpansionID)
         WHERE (s.Flags_0 & 32770)=0 AND s.Bonding!=4 AND s.Stackable=1
           AND s.ZoneBound_0=0 AND s.ZoneBound_1=0 AND s.InstanceBound=0
           AND s.SellPrice>0 ORDER BY d.ID",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            Eligibility {
                loot_id: row.get(1)?,
                skill: row.get(2)?,
            },
        ))
    })?;
    let mut items = HashMap::new();
    for row in rows {
        let (item, eligibility) = row?;
        items.entry(item).or_insert(eligibility);
    }
    Ok(DisenchantData {
        items,
        loot: crate::loot::load_disenchant_tables(conn)?,
    })
}

pub(crate) fn load_startup(mut commands: Commands) {
    let result = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .and_then(|conn| load_data(&conn));
    match result {
        Ok(data) => commands.insert_resource(data),
        Err(error) => error!("Disenchant eligibility unavailable: {error}"),
    }
}

pub(crate) fn is_disenchant(spell: &SpellInfo) -> bool {
    spell
        .effects
        .iter()
        .any(|effect| effect.effect == EFFECT_DISENCHANT)
}

fn load_target(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(u64, InventoryModel, u64, Eligibility), CastFailure> {
    let guid = spell
        .cast_target_item_guid
        .ok_or(CastFailure::InvalidTarget)?;
    let id = world
        .get::<ClientId>(caster)
        .ok_or(CastFailure::InvalidTarget)?
        .0;
    let db = world.get_resource::<GameDatabase>().ok_or_else(|| {
        CastFailure::Unsupported("Disenchant inventory database unavailable".into())
    })?;
    let templates = world.resource::<ItemTemplates>();
    let model = load_model(&db.db, templates, id).map_err(|error| {
        CastFailure::Unsupported(format!("Disenchant inventory unavailable: {error:?}"))
    })?;
    let item = model.bag_item(guid).ok_or(CastFailure::InvalidTarget)?;
    if model.definition_source(guid) != Retail {
        return Err(CastFailure::CantBeDisenchanted);
    }
    let eligibility = world
        .resource::<DisenchantData>()
        .items
        .get(&item.item_id)
        .copied()
        .ok_or(CastFailure::CantBeDisenchanted)?;
    Ok((id, model, guid, eligibility))
}

pub(crate) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if !is_disenchant(spell) {
        return Ok(());
    }
    let professions = world
        .get::<crate::profession::PlayerProfessions>(caster)
        .filter(|profs| profs.knows(spell.id))
        .ok_or(CastFailure::DisenchantNotKnown)?;
    let (_, _, _, eligibility) = load_target(world, caster, spell)?;
    if professions.rank(ENCHANTING) < eligibility.skill {
        return Err(CastFailure::CantBeDisenchantedSkill);
    }
    if world.get::<crate::loot::ItemLoot>(caster).is_some()
        && world
            .get::<crate::loot::CorpseLoot>(caster)
            .is_some_and(|loot| !loot.looters.is_empty())
    {
        return Err(CastFailure::InProgress);
    }
    validate_materials(
        world.resource::<DisenchantData>(),
        world.resource::<ItemTemplates>(),
        eligibility.loot_id,
    )
}

fn validate_materials(
    data: &DisenchantData,
    items: &ItemTemplates,
    loot_id: u32,
) -> Result<(), CastFailure> {
    if !data.loot.disenchant.contains_key(&loot_id) {
        return Err(CastFailure::Unsupported(format!(
            "Disenchant loot {loot_id} unavailable"
        )));
    }
    if !data.loot.disenchant_has_materials(loot_id, items) {
        return Err(CastFailure::CantBeDisenchanted);
    }
    Ok(())
}

pub(crate) fn roll_cast_loot(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    rng: &mut impl rand::Rng,
) -> Result<(), CastFailure> {
    if !is_disenchant(spell) {
        return Ok(());
    }
    let (_, _, _, eligibility) = load_target(world, caster, spell)?;
    let rolled = world
        .resource::<DisenchantData>()
        .loot
        .roll_disenchant(eligibility.loot_id, world.resource::<ItemTemplates>(), rng)
        .ok_or(CastFailure::CantBeDisenchanted)?;
    world
        .entity_mut(caster)
        .insert(PendingDisenchantLoot(rolled));
    Ok(())
}

pub(crate) fn effect_disenchant(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    // Cast completion checked the same inventory immediately before effect resolution.
    let (id, mut model, guid, _) =
        load_target(world, ctx.caster, ctx.spell).expect("validated disenchant target");
    let item = model.bag_item(guid).expect("validated bag item").clone();
    let rolled = world
        .entity_mut(ctx.caster)
        .take::<PendingDisenchantLoot>()
        .expect("Disenchant completion checked its material roll")
        .0;
    model.collect_disposed_appearance(&item);
    model
        .take_bag_item(guid, 0, &mut || {
            unreachable!("single disenchant item is not split")
        })
        .expect("validated bag item");
    crate::persistence::inventory_store::save_inventory_record(
        &world.resource::<GameDatabase>().db,
        id,
        &model.into_record(),
    );
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(id);
    crate::loot::open_item_loot(
        world,
        ctx.caster,
        id,
        rolled,
        crate::loot::LootKind::DISENCHANTING,
    );
}

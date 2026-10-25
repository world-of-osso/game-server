//! TC a352b1fa spell_gen_upper_deck_create_foam_sword (64142), the Foam Sword Rack's
//! spell: the player gets one random foam sword unless they already hold one of the
//! five, bank included (`HasItemCount(item, 1, true)`).
use bevy::prelude::*;
use shared::item_data::ItemDefinitionSource::Retail;

use super::effects::EffectCtx;
use super::toy_scripts::{has_effect_shape, single_effect};
use crate::item_templates::ItemTemplates;
use crate::merchant::VendorCatalog;
use crate::persistence::{ClientId, GameDatabase};
use crate::spell_info::{SpellEffect, SpellInfo};

pub(super) const CREATE_FOAM_SWORD: u32 = 64142;
/// ITEM_FOAM_SWORD_GREEN, PINK, BLUE, RED, YELLOW.
const FOAM_SWORDS: [u32; 5] = [45061, 45176, 45177, 45178, 45179];
const EFFECT_SCRIPT: u32 = 77;
/// TARGET_UNIT_TARGET_ANY: the rack's user.
const TARGET_UNIT_TARGET_ANY: u32 = 25;

pub(super) fn is_script_effect(effect: &SpellEffect) -> bool {
    has_effect_shape(effect, EFFECT_SCRIPT, [TARGET_UNIT_TARGET_ANY, 0], 0)
}

pub(super) fn unsupported(world: &World, spell: &SpellInfo) -> Option<String> {
    if !single_effect(spell).is_some_and(is_script_effect) {
        return Some("foam sword source effect signature differs".into());
    }
    let (Some(catalog), Some(templates)) = (
        world.get_resource::<VendorCatalog>(),
        world.get_resource::<ItemTemplates>(),
    ) else {
        return Some("foam sword item data is unavailable".into());
    };
    let items = FOAM_SWORDS
        .iter()
        .all(|id| catalog.goods.contains_key(id) && templates.get(Retail, *id).is_some());
    (!items).then(|| "foam sword items are unavailable".into())
}

/// The rack casts on its user, so the caster is the hit player.
pub(super) fn create_one_foam_sword(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    match holds_foam_sword(world, ctx.caster) {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            warn!("Spell {}: {error}", ctx.spell.id);
            return;
        }
    }
    let choice = rand::Rng::gen_range(&mut rand::thread_rng(), 0..FOAM_SWORDS.len());
    if let Err(error) =
        crate::profession::store_created_item(world, ctx.caster, FOAM_SWORDS[choice], 1, None)
    {
        warn!("Spell {}: foam sword not created: {error}", ctx.spell.id);
    }
}

fn holds_foam_sword(world: &World, player: Entity) -> Result<bool, String> {
    let character_id = world
        .get::<ClientId>(player)
        .ok_or("user has no character")?
        .0;
    let db = &world.resource::<GameDatabase>().db;
    let model = crate::inventory::load_model(db, world.resource::<ItemTemplates>(), character_id)
        .map_err(|error| format!("{error:?}"))?;
    let carried = FOAM_SWORDS
        .iter()
        .any(|id| model.owned_count(Retail, *id) > 0);
    let banked = crate::persistence::banks::load_character_bank(db, character_id)
        .tabs
        .iter()
        .flat_map(|tab| tab.slots.iter().flatten())
        .any(|item| FOAM_SWORDS.contains(&item.item_id));
    Ok(carried || banked)
}

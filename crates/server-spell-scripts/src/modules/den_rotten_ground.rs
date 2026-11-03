//! INFERRED_WCL: Den Thornclaw Gatherer Rotten Ground.
//! DB2 1297700 creates area-trigger create-properties 43133 for 20 s. Client DB2 and
//! TDB lack 43133's shape and enter action, and no DB2 effect links Rotten Supplies
//! 1297699 to 1297700. In VxKH96NhkT487pnC/18, G8rCyV2w7L3mtQ1f/4 and
//! 6FJTa3cr2VDhypG1/30 each 1297699 impact is followed by 1297701 on players near the
//! hit player; fitted field centres lie at the impact position. Player-cast positions
//! separate holders from non-holders at 3.7-3.9 yd (14 fields, 795 samples, immune
//! players excluded); with tick positions the best band is 3.8-5.0 yd.
//! Retail applies the first entries about 1 s after impact; here the field starts at impact.
//! Retire when authoritative 43133 geometry and actions, or the likely real cast
//! 1297698 (DUMMY, one enemy within radius index 48), are verified.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    script_registry::{HitHook, SpellScriptHook},
    targets::Relation,
};

const ROTTEN_SUPPLIES: u32 = 1297699;
const ROTTEN_GROUND_FIELD: u32 = 1297700;
const ROTTEN_GROUND_PROPERTIES: i32 = 43133;
const ROTTEN_GROUND: u32 = 1297701;
const RADIUS: f32 = 3.8;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::Hit(hit_hook),
    SpellScriptHook::AreaTriggerBinding(binding),
];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == ROTTEN_SUPPLIES).then_some(create_rotten_ground)
}

fn binding(spell: u32, properties: i32) -> Option<(f32, Relation, u32)> {
    (spell == ROTTEN_GROUND_FIELD && properties == ROTTEN_GROUND_PROPERTIES).then_some((
        RADIUS,
        Relation::Enemy,
        ROTTEN_GROUND,
    ))
}

/// DB2 1297700 targets 63/138: the field lands at the explicit unit's ground position.
fn create_rotten_ground(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, ROTTEN_GROUND_FIELD, Some(target), depth + 1);
}

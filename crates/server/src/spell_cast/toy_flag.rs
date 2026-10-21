//! TC a352b1fa spell_item.cpp3603–3641, spell_item_taunt_flag_targeting.
//! The server represents a player's body in PlayerDeathState (as raid corpse selection does).
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::Player;

use super::{CastFailure, PlannedEffect, TargetPlan, targets};
use crate::death::PlayerDeathState;
use crate::maps::InMap;
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const TARGETING: u32 = 51640;
pub(super) const FLAG: u32 = 51657;
const EMOTE: u32 = 28008;

pub(super) fn source_signature(spell: &SpellInfo) -> bool {
    super::toy_scripts::single_effect(spell)
        .is_some_and(|effect| super::toy_scripts::has_effect_shape(effect, 3, [22, 93], 0))
}

pub(super) fn unsupported(world: &World, spell: &SpellInfo) -> Option<String> {
    if !source_signature(spell) {
        return Some("Flag of Ownership source signature differs".into());
    }
    if !super::toy_scripts::texts_available(world, &[EMOTE]) {
        return Some("Flag of Ownership BroadcastText28008 unavailable".into());
    }
    if world.resource::<SpellStore>().get(FLAG).is_none() {
        return Some("Flag of Ownership child51657 unavailable".into());
    }
    let missing = crate::toy_catalog::unsupported_types(world, FLAG);
    (!missing.is_empty()).then(|| format!("Flag of Ownership child: {missing:?}"))
}

fn body_position(world: &World, owner: Entity) -> Option<Vec3> {
    let death = world.get::<PlayerDeathState>(owner)?;
    if death.state == shared::death::DeathState::Alive {
        return None;
    }
    let body = death.corpse.as_ref()?;
    let map = world.get::<InMap>(owner)?;
    (u32::from(body.map_id) == map.map_id).then_some(Vec3::new(body.x, body.y, body.z))
}

fn enemy_bodies(world: &mut World, caster: Entity, spell: &SpellInfo) -> Vec<Entity> {
    let Some(origin) = targets::position(world, caster) else {
        return Vec::new();
    };
    let Some(effect) = spell.effects.first() else {
        return Vec::new();
    };
    let radius = targets::radius(effect, 1);
    let mut query = world.query_filtered::<Entity, With<Player>>();
    query
        .iter(world)
        .filter(|&owner| {
            targets::same_map(world, caster, owner)
                && targets::is_hostile(world, caster, owner)
                && body_position(world, owner)
                    .is_some_and(|position| origin.distance(position) <= radius)
        })
        .collect()
}

pub(super) fn check_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if let Some(reason) = unsupported(world, spell) {
        return Err(CastFailure::Unsupported(reason));
    }
    if enemy_bodies(world, caster, spell).is_empty() {
        return Err(CastFailure::NoValidTargets);
    }
    Ok(())
}

fn planned_effect(index: usize, units: Vec<Entity>, dest: Option<targets::Dest>) -> PlannedEffect {
    PlannedEffect {
        index,
        units,
        dest,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }
}

pub(super) fn targeting_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    _explicit: Option<Entity>,
) -> TargetPlan {
    let bodies = enemy_bodies(world, caster, spell);
    let units = bodies
        .choose(&mut rand::thread_rng())
        .copied()
        .into_iter()
        .collect();
    vec![planned_effect(0, units, None)]
}

pub(super) fn flag_plan(
    world: &mut World,
    caster: Entity,
    _spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let Some(target) = explicit else {
        return Vec::new();
    };
    let Some(position) = body_position(world, target) else {
        return Vec::new();
    };
    let destination = targets::Dest {
        position,
        orientation: targets::orientation(world, caster),
    };
    vec![
        planned_effect(0, Vec::new(), Some(destination)),
        planned_effect(1, vec![target], None),
    ]
}

pub(super) fn plant(
    world: &mut World,
    ctx: &super::effects::EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    super::toy_scripts::send_targeted_text_emote(world, ctx.caster, target, EMOTE);
    super::cast_triggered(world, ctx.caster, FLAG, Some(target), ctx.depth + 1);
}

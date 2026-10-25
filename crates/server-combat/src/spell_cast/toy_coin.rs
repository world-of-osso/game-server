//! TC a352b1fa spell_item_titanium_seal_of_dalaran_toss/_catch (60458/60476).
//! Toss: TextEmote32638 at launch; RelocateHeight puts the missile destination 20yd
//! above the caster. Catch: on the triggered 60476 hit, TextEmote heads32663 or tails32664.
use bevy::prelude::*;
use shared::components::{Player, Position};

use super::effects::EffectCtx;
use super::now;
use super::toy_scripts::{has_effect_shape, send_text_emote, single_effect};
use crate::maps::InMap;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};

pub(super) const COIN_TOSS: u32 = 60458;
const COIN_CATCH: u32 = 60476;
const TEXT_TOSS: u32 = 32638;
const TEXT_HEADS: u32 = 32663;
const TEXT_TAILS: u32 = 32664;
pub(super) const COIN_TEXTS: [u32; 3] = [TEXT_TOSS, TEXT_HEADS, TEXT_TAILS];
/// `RelocateHeight`: `dest.RelocateOffset({0, 0, 20})` on TARGET_DEST_CASTER.
const TOSS_HEIGHT: f32 = 20.0;
const EFFECT_TRIGGER_MISSILE: u32 = 32;
const EFFECT_SCRIPT: u32 = 77;
const TARGET_DEST_CASTER: u32 = 18;
const TARGET_UNIT_DEST_AREA_ENTRY: u32 = 8;

/// Tossed coins with the time their missile reaches the toss destination.
#[derive(Resource, Default)]
pub(crate) struct CoinsInFlight(Vec<(f64, Entity)>);

pub(super) fn has_source_signature(world: &World, spell: &SpellInfo) -> bool {
    let toss = single_effect(spell).is_some_and(|effect| {
        has_effect_shape(
            effect,
            EFFECT_TRIGGER_MISSILE,
            [TARGET_DEST_CASTER, 0],
            COIN_CATCH,
        )
    });
    let catch = world
        .resource::<SpellStore>()
        .get(COIN_CATCH)
        .and_then(single_effect)
        .is_some_and(|effect| {
            has_effect_shape(effect, EFFECT_SCRIPT, [TARGET_UNIT_DEST_AREA_ENTRY, 0], 0)
        });
    toss && catch && missile_speed(spell) > 0.0
}

pub(super) fn is_toss_effect(effect: &SpellEffect) -> bool {
    has_effect_shape(
        effect,
        EFFECT_TRIGGER_MISSILE,
        [TARGET_DEST_CASTER, 0],
        COIN_CATCH,
    )
}

fn missile_speed(spell: &SpellInfo) -> f32 {
    spell.misc.as_ref().map_or(0.0, |misc| misc.speed)
}

pub(super) fn toss_coin(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    send_text_emote(world, ctx.caster, TEXT_TOSS);
    let flight_seconds = TOSS_HEIGHT / missile_speed(ctx.spell);
    let lands_at = now(world) + f64::from(flight_seconds);
    world
        .get_resource_or_init::<CoinsInFlight>()
        .0
        .push((lands_at, ctx.caster));
}

/// The catch hit of each landed coin; a caster that left the world drops it.
pub(super) fn catch_landed_coins(world: &mut World) {
    let time = now(world);
    let landed: Vec<Entity> = {
        let Some(mut coins) = world.get_resource_mut::<CoinsInFlight>() else {
            return;
        };
        let (landed, flying) = std::mem::take(&mut coins.0)
            .into_iter()
            .partition(|(lands_at, _)| *lands_at <= time);
        coins.0 = flying;
        landed.into_iter().map(|(_, caster)| caster).collect()
    };
    for caster in landed {
        if is_located_player(world, caster) {
            let face = rand::Rng::gen_bool(&mut rand::thread_rng(), 0.5);
            let text = if face { TEXT_HEADS } else { TEXT_TAILS };
            send_text_emote(world, caster, text);
        }
    }
}

fn is_located_player(world: &World, caster: Entity) -> bool {
    world.get::<Player>(caster).is_some()
        && world.get::<Position>(caster).is_some()
        && world.get::<InMap>(caster).is_some()
}

//! Player resurrection offers, TC a352b1fa SpellEffects.cpp EffectResurrect/WithAura.
use super::PlayerDeathState;
use crate::auth::InWorld;
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::spell_cast::{effects::EffectCtx, now};
use crate::spell_info::SpellEffect;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, Mana, Player, Position};
use shared::death::DeathState;
use shared::protocol::{DeathChannel, DeathStateUpdate, ResurrectionOffer, ResurrectionResponse};

pub(crate) const EFFECT_RESURRECT: u32 = 18;
pub(crate) const EFFECT_RESURRECT_WITH_AURA: u32 = 172;
const OFFER_TIMEOUT_MS: u32 = 60_000;

#[derive(Component, Clone)]
pub(crate) struct PendingResurrection {
    offer: ResurrectionOffer,
    pub expires_at: f64,
    position: Position,
    map: InMap,
    health: f32,
    mana: f32,
    aura: u32,
}

pub(crate) fn is_resurrection(effect: &SpellEffect) -> bool {
    matches!(effect.effect, EFFECT_RESURRECT | EFFECT_RESURRECT_WITH_AURA)
}

fn connection(world: &mut World, player: Entity) -> Option<Entity> {
    let mut links = world.query_filtered::<(Entity, &PlayerLink), With<InWorld>>();
    links
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

pub(crate) fn send_offer(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    if world.get::<crate::followers::Follower>(target).is_some() {
        resurrect_follower(world, ctx, effect, target);
        return;
    }
    if !can_offer(world, target) {
        return;
    }
    let Some(pending) = build_offer(world, ctx, effect, target) else {
        return;
    };
    let Some(connection) = connection(world, target) else {
        return;
    };
    let Some(mut sender) = world.get_mut::<MessageSender<ResurrectionOffer>>(connection) else {
        return;
    };
    sender.send::<DeathChannel>(pending.offer.clone());
    world.entity_mut(target).insert(pending);
}

/// FOLLOWER_UTILITY_POLICY: a party companion accepts the ordinary native effect.
/// Never resurrect arbitrary NPCs or cross-owner/copy follower corpses.
fn resurrect_follower(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let follower = world.get::<crate::followers::Follower>(target).unwrap();
    let caster_owner = world
        .get::<crate::followers::Follower>(ctx.caster)
        .map_or(ctx.caster, |caster| caster.owner);
    let same_party = follower.owner == caster_owner;
    let same_map = world.get::<InMap>(target) == world.get::<InMap>(ctx.caster);
    let dead = world
        .get::<Health>(target)
        .is_some_and(|health| health.current <= 0.0);
    if !same_party || !same_map || !dead {
        return;
    }
    let Some(pending) = build_offer(world, ctx, effect, target) else {
        return;
    };
    world
        .entity_mut(target)
        .remove::<(crate::combat::DeathState, crate::combat::Corpse)>()
        .insert((pending.position, crate::creature_ai::CreatureState::Idle));
    if let Some(mut health) = world.get_mut::<Health>(target) {
        health.current = pending.health.min(health.max);
    }
    if let Some(mut mana) = world.get_mut::<Mana>(target) {
        mana.current = pending.mana.min(mana.max);
    }
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(target);
    }
    if pending.aura != 0 {
        crate::spell_cast::cast_triggered(world, target, pending.aura, Some(target), 0);
    }
}

fn can_offer(world: &World, target: Entity) -> bool {
    let dead = world
        .get::<PlayerDeathState>(target)
        .is_some_and(|state| matches!(state.state, DeathState::Dead | DeathState::Ghost));
    let pending = world
        .get::<PendingResurrection>(target)
        .is_some_and(|pending| pending.expires_at > now(world));
    world.get::<Player>(target).is_some() && dead && !pending
}

fn build_offer(
    world: &World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) -> Option<PendingResurrection> {
    let caster_name = match (
        world.get::<Player>(ctx.caster),
        world.get::<crate::followers::Follower>(ctx.caster),
    ) {
        (Some(player), _) => player.name.clone(),
        (_, Some(follower)) => follower.profile.name.clone(),
        _ => return None,
    };
    let position = *world.get::<Position>(ctx.caster)?;
    let map = *world.get::<InMap>(ctx.caster)?;
    let percent = effect.base_points.clamp(0.0, 100.0) / 100.0;
    let health = (world.get::<Health>(target)?.max * percent)
        .floor()
        .max(1.0);
    let mana = world
        .get::<Mana>(target)
        .map_or(0.0, |mana| (mana.max * percent).floor());
    Some(PendingResurrection {
        offer: ResurrectionOffer {
            caster: ctx.caster.to_bits(),
            caster_name,
            spell_id: ctx.spell.id,
            time_left_ms: OFFER_TIMEOUT_MS,
        },
        expires_at: now(world) + f64::from(OFFER_TIMEOUT_MS) / 1000.0,
        position,
        map,
        health,
        mana,
        aura: if effect.effect == EFFECT_RESURRECT_WITH_AURA {
            effect.trigger_spell
        } else {
            0
        },
    })
}

pub(crate) fn receive_responses(world: &mut World) {
    let mut receivers = world
        .query_filtered::<(&PlayerLink, &mut MessageReceiver<ResurrectionResponse>), With<InWorld>>(
        );
    let responses: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(link, mut receiver)| {
            receiver
                .receive()
                .map(|response| (link.0, response))
                .collect::<Vec<_>>()
        })
        .collect();
    for (player, response) in responses {
        respond(world, player, response);
        send_state(world, player);
    }
    expire_offers(world);
}

fn expire_offers(world: &mut World) {
    let time = now(world);
    let mut pending = world.query::<(Entity, &PendingResurrection)>();
    let expired: Vec<_> = pending
        .iter(world)
        .filter_map(|(player, offer)| (offer.expires_at <= time).then_some(player))
        .collect();
    for player in expired {
        world.entity_mut(player).remove::<PendingResurrection>();
    }
}

pub(crate) fn respond(world: &mut World, player: Entity, response: ResurrectionResponse) {
    let Some(pending) = world.get::<PendingResurrection>(player).cloned() else {
        return;
    };
    if response.caster != pending.offer.caster || response.spell_id != pending.offer.spell_id {
        return;
    }
    world.entity_mut(player).remove::<PendingResurrection>();
    if !response.accept || pending.expires_at <= now(world) {
        return;
    }
    let dead = world
        .get::<PlayerDeathState>(player)
        .is_some_and(|state| matches!(state.state, DeathState::Dead | DeathState::Ghost));
    if !dead || world.get::<InMap>(player) != Some(&pending.map) {
        return;
    }
    apply_resurrection(world, player, &pending);
}

fn apply_resurrection(world: &mut World, player: Entity, pending: &PendingResurrection) {
    world
        .entity_mut(player)
        .insert((pending.position, PlayerDeathState::default()));
    if let Some(mut health) = world.get_mut::<Health>(player) {
        health.current = pending.health.min(health.max);
    }
    if let Some(mut mana) = world.get_mut::<Mana>(player) {
        mana.current = pending.mana.min(mana.max);
    }
    crate::teleport::teleported_in(world, player);
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(player);
    }
    if pending.aura != 0 {
        crate::spell_cast::cast_triggered(world, player, pending.aura, Some(player), 0);
    }
}

fn send_state(world: &mut World, player: Entity) {
    let Some(position) = world.get::<Position>(player).copied() else {
        return;
    };
    let Some(state) = world.get::<PlayerDeathState>(player) else {
        return;
    };
    let snapshot = super::support::build_snapshot(&position, state);
    let Some(connection) = connection(world, player) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<DeathStateUpdate>>(connection) {
        sender.send::<DeathChannel>(DeathStateUpdate {
            snapshot: Some(snapshot),
            message: None,
            error: None,
        });
    }
}

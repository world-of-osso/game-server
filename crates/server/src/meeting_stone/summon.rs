//! Summon offers (TrinityCore `Player::SendSummonRequestFrom` /
//! `SummonIfPossible`, Player.cpp:26022-26098).
//!
//! `SPELL_EFFECT_SUMMON_PLAYER` hits the caster's selected player (not the
//! spell's target map, Spell.cpp:2027-2060) when it is another member of the
//! caster's party or raid without a pending summon (`CheckCast`,
//! Spell.cpp:6539-6552). The target then holds the summoner's position at that
//! moment for `MAX_PLAYER_SUMMON_DELAY` (2 min, Player.h:1041) and gets
//! `SummonRequest`. A new offer while one is pending is dropped.
//!
//! `SummonResponse` is ignored while the target is dead or in combat
//! (`HandleSummonResponseOpcode`, MovementHandler.cpp:768-774). A decline or a
//! late accept ends the offer; an accept in time ends a taxi flight and moves
//! the target to the held position (`teleport::teleported`). The server hosts
//! map 0 only, so summoner and target always share a map.

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, MovementControl, Player, Position, Zone};
use shared::protocol::{InteractionChannel, SummonRequest, SummonResponse};

use crate::auth::InWorld;
use crate::interest::InterestDirtyQueue;
use crate::networking::PlayerLink;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_cast::{now, targets};
use crate::spell_info::SpellEffect;

pub(crate) const EFFECT_SUMMON_PLAYER: u32 = 85;
/// `MAX_PLAYER_SUMMON_DELAY`.
const SUMMON_DELAY_MS: u32 = 120_000;

/// An open summon offer on the summoned player.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct PendingSummon {
    pub location: Position,
    pub expires_at: f64,
}

/// `Player::HasSummonPending`.
pub(crate) fn has_pending(world: &World, player: Entity) -> bool {
    world
        .get::<PendingSummon>(player)
        .is_some_and(|pending| pending.expires_at >= now(world))
}

/// `SPELL_EFFECT_SUMMON_PLAYER` at `target`, the caster's selection.
pub(crate) fn effect_summon_player(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &SpellEffect,
    target: Entity,
) {
    let summoner = ctx.caster;
    if target == summoner
        || world.get::<Player>(target).is_none()
        || !super::same_group(world, summoner, target)
        || has_pending(world, target)
    {
        debug!(
            "Spell {}: {target} cannot be summoned by {summoner}",
            ctx.spell.id
        );
        return;
    }
    send_summon_request(world, summoner, target);
}

fn send_summon_request(world: &mut World, summoner: Entity, target: Entity) {
    let Some(location) = world.get::<Position>(summoner).copied() else {
        return;
    };
    let request = SummonRequest {
        summoner: world
            .get::<Player>(summoner)
            .map_or_else(String::new, |player| player.name.clone()),
        zone_id: world.get::<Zone>(summoner).map_or(0, |zone| zone.id),
        time_left_ms: SUMMON_DELAY_MS,
    };
    let expires_at = now(world) + f64::from(SUMMON_DELAY_MS) / 1000.0;
    world.entity_mut(target).insert(PendingSummon {
        location,
        expires_at,
    });
    let Some(connection) = connection_of(world, target) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<SummonRequest>>(connection) {
        sender.send::<InteractionChannel>(request);
    }
}

fn connection_of(world: &mut World, player: Entity) -> Option<Entity> {
    let mut connections = world.query_filtered::<(Entity, &PlayerLink), With<InWorld>>();
    connections
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

pub(crate) fn receive_summon_responses(world: &mut World) {
    let mut receivers = world
        .query_filtered::<(&PlayerLink, &mut MessageReceiver<SummonResponse>), With<InWorld>>();
    let responses: Vec<(Entity, bool)> = receivers
        .iter_mut(world)
        .flat_map(|(link, mut receiver)| {
            let player = link.0;
            receiver
                .receive()
                .map(move |response| (player, response.accept))
                .collect::<Vec<_>>()
        })
        .collect();
    for (player, accept) in responses {
        respond_to_summon(world, player, accept);
    }
}

fn respond_to_summon(world: &mut World, player: Entity, accept: bool) {
    let in_combat = world
        .get::<CombatStatus>(player)
        .is_some_and(|status| status.0);
    if !targets::is_alive(world, player) || in_combat {
        return;
    }
    let Some(pending) = world.entity_mut(player).take::<PendingSummon>() else {
        return;
    };
    if !accept || pending.expires_at < now(world) {
        return;
    }
    crate::taxi::finish_flight(world.entity_mut(player));
    let Some(mut control) = world.get::<MovementControl>(player).copied() else {
        return;
    };
    world.entity_mut(player).insert(pending.location);
    crate::teleport::teleported(
        world.get_resource::<InterestDirtyQueue>(),
        player,
        &mut control,
    );
    world.entity_mut(player).insert(control);
    info!("{player} accepted a summon");
}

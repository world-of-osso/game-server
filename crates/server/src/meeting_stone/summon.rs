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
//! late accept ends the offer; an accept in time ends a taxi flight and teleports
//! the target to the held location (`TeleportTo(m_summon_location)`), onto another
//! map through a map transfer (`maps::teleport_to`).

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Player, Position, Rotation};
use shared::protocol::{InteractionChannel, SummonRequest, SummonResponse};

use crate::auth::InWorld;
use crate::maps::{InMap, TeleportOptions, WorldLocation};
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
    pub location: WorldLocation,
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

/// The summoner's `WorldLocation` (`Player::SendSummonRequestFrom` stores
/// `summoner->GetMapId()` and position).
fn summoner_location(world: &World, summoner: Entity) -> Option<WorldLocation> {
    let map = world.get::<InMap>(summoner)?;
    let position = world.get::<Position>(summoner)?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let orientation = world.get::<Rotation>(summoner)?.y - std::f32::consts::FRAC_PI_2;
    Some(WorldLocation {
        map_id: map.map_id,
        position: Vec3::new(x, y, z),
        orientation,
    })
}

/// Reuse city rest's terrain area lookup and top-level AreaTable ancestry.
fn read_summoner_zone_id(world: &mut World, summoner: Entity, map_id: u32) -> u32 {
    let position = world
        .get::<Position>(summoner)
        .expect("summoner location has position");
    let feet = Vec3::new(position.x, position.y, position.z);
    let area_id = world
        .resource_mut::<crate::ground::GroundMap>()
        .area_at(map_id, feet);
    area_id
        .and_then(|area| {
            world
                .resource::<crate::area_table::AreaTable>()
                .zone_id(area, map_id)
        })
        .unwrap_or(0)
}

pub(super) fn send_summon_request(world: &mut World, summoner: Entity, target: Entity) {
    let Some(location) = summoner_location(world, summoner) else {
        return;
    };
    let zone_id = read_summoner_zone_id(world, summoner, location.map_id);
    let request = SummonRequest {
        summoner: world
            .get::<Player>(summoner)
            .map_or_else(String::new, |player| player.name.clone()),
        zone_id,
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
    crate::achievement::record_accepted_summon(world, player);
    crate::maps::teleport_to(world, player, pending.location, TeleportOptions::default());
    info!(
        "{player} accepted a summon to map {}",
        pending.location.map_id
    );
}

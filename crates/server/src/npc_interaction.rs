//! NPC interaction (docs/wiki/systems/npc-interaction.md): validates
//! `InteractNpc`, opens a gossip menu or a role frame, routes gossip options, and
//! closes the interaction once the player could no longer open it.
//!
//! Validation is TrinityCore `Player::GetNPCIfCanInteractWith`: player alive, NPC
//! alive, NPC not hostile to the player, NPC not in combat, and within the NPC's
//! combat reach + 4 yd (from the client's `CGGameUI::SetInteractTarget`) measured
//! by `IsWithinDistInMap`, which also adds both units' combat reach. The same check
//! runs every frame while an interaction is open; TrinityCore repeats it on every
//! auction/vendor/bank request, so a player who walks out of range loses the frame.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, Npc, Position, UnitFactionTemplate};
use shared::protocol::{
    CloseInteraction, InteractNpc, InteractionChannel, InteractionClosed, InteractionError,
    InteractionFailed, InteractionKind, InteractionOpened, NpcFlags, NpcRole, SelectGossipOption,
};

use crate::auth::InWorld;
use crate::faction::{FactionTemplates, Reaction};
use crate::networking::PlayerLink;

mod gossip;

use gossip::{GossipAction, Opening};
pub use gossip::{GossipCatalog, load_gossip_catalog};

/// TrinityCore `DEFAULT_PLAYER_COMBAT_REACH` / creature default combat reach.
const DEFAULT_COMBAT_REACH: f32 = 1.5;
/// Largest NPC–player center distance that still interacts: creature reach + 4 yd,
/// plus both combat reaches added by `IsWithinDistInMap`.
pub(crate) const INTERACTION_DISTANCE: f32 =
    DEFAULT_COMBAT_REACH + 4.0 + 2.0 * DEFAULT_COMBAT_REACH;

/// Auction house byte stored on auctions and search queries (`AuctionData::faction`).
pub(crate) const AUCTION_HOUSE_ALLIANCE: u8 = 1;
pub(crate) const AUCTION_HOUSE_HORDE: u8 = 2;
pub(crate) const AUCTION_HOUSE_NEUTRAL: u8 = 3;

/// TrinityCore `FACTION_MASK_ALLIANCE` / `FACTION_MASK_HORDE` (FactionTemplate.FactionGroup).
const FACTION_MASK_ALLIANCE: u32 = 0x2;
const FACTION_MASK_HORDE: u32 = 0x4;

/// The NPC a connection is interacting with. Lives on the connection entity.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ActiveInteraction {
    pub npc: Entity,
    /// Role frame currently open; `None` while a gossip menu is shown.
    pub role: Option<NpcRole>,
    /// Auction house of the auctioneer, set while its frame is open.
    pub auction_house: Option<u8>,
    gossip_actions: Vec<(u32, GossipAction)>,
}

pub struct NpcInteractionPlugin;

impl Plugin for NpcInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GossipCatalog>();
        app.add_systems(
            Update,
            (
                process_interact_requests,
                process_gossip_selections,
                process_close_requests,
                close_invalid_interactions,
            )
                .chain(),
        );
    }
}

/// TrinityCore `AuctionHouseMgr::GetAuctionHouseEntry` by the auctioneer's
/// FactionTemplate: Alliance or Horde by faction group, otherwise neutral.
pub(crate) fn auction_house_for(templates: &FactionTemplates, faction_template: u32) -> u8 {
    let group = templates
        .0
        .get(&faction_template)
        .map_or(0, |entry| entry.faction_group);
    if group & FACTION_MASK_ALLIANCE != 0 {
        AUCTION_HOUSE_ALLIANCE
    } else if group & FACTION_MASK_HORDE != 0 {
        AUCTION_HOUSE_HORDE
    } else {
        AUCTION_HOUSE_NEUTRAL
    }
}

type NpcView<'a> = (
    &'a Npc,
    &'a NpcFlags,
    &'a Position,
    &'a Health,
    &'a UnitFactionTemplate,
    Option<&'a CombatStatus>,
);

#[derive(SystemParam)]
struct InteractionWorld<'w, 's> {
    players: Query<
        'w,
        's,
        (
            &'static Position,
            &'static Health,
            &'static UnitFactionTemplate,
        ),
    >,
    npcs: Query<'w, 's, NpcView<'static>>,
    templates: Res<'w, FactionTemplates>,
    catalog: Res<'w, GossipCatalog>,
}

impl InteractionWorld<'_, '_> {
    /// The NPC if `player` may interact with it now.
    fn interactable(&self, player: Entity, npc: Entity) -> Result<NpcView<'_>, InteractionError> {
        let npc_view = self
            .npcs
            .get(npc)
            .map_err(|_| InteractionError::InvalidTarget)?;
        let (player_position, player_health, player_faction) = self
            .players
            .get(player)
            .map_err(|_| InteractionError::InvalidTarget)?;
        let (_, _, npc_position, npc_health, npc_faction, combat) = npc_view;
        if player_health.current <= 0.0 {
            return Err(InteractionError::PlayerDead);
        }
        if npc_health.current <= 0.0 {
            return Err(InteractionError::TargetDead);
        }
        if self.templates.reaction(npc_faction.0, player_faction.0) == Reaction::Hostile {
            return Err(InteractionError::TargetHostile);
        }
        if combat.is_some_and(|status| status.0) {
            return Err(InteractionError::TargetInCombat);
        }
        if distance(player_position, npc_position) > INTERACTION_DISTANCE {
            return Err(InteractionError::TooFarAway);
        }
        Ok(npc_view)
    }

    fn auction_house(&self, npc: Entity) -> u8 {
        let faction = self.npcs.get(npc).map_or(0, |view| view.4.0);
        auction_house_for(&self.templates, faction)
    }
}

fn distance(a: &Position, b: &Position) -> f32 {
    Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z))
}

#[derive(SystemParam)]
struct InteractionSenders<'w, 's> {
    opened: Query<'w, 's, &'static mut MessageSender<InteractionOpened>>,
    failed: Query<'w, 's, &'static mut MessageSender<InteractionFailed>>,
    closed: Query<'w, 's, &'static mut MessageSender<InteractionClosed>>,
}

impl InteractionSenders<'_, '_> {
    fn opened(&mut self, connection: Entity, npc: Entity, kind: InteractionKind) {
        if let Ok(mut sender) = self.opened.get_mut(connection) {
            sender.send::<InteractionChannel>(InteractionOpened {
                npc: npc.to_bits(),
                kind,
            });
        }
    }

    fn failed(&mut self, connection: Entity, npc: u64, error: InteractionError) {
        if let Ok(mut sender) = self.failed.get_mut(connection) {
            sender.send::<InteractionChannel>(InteractionFailed { npc, error });
        }
    }

    fn closed(&mut self, connection: Entity, npc: Entity) {
        if let Ok(mut sender) = self.closed.get_mut(connection) {
            sender.send::<InteractionChannel>(InteractionClosed { npc: npc.to_bits() });
        }
    }
}

type InteractReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<InteractNpc>,
    ),
    With<InWorld>,
>;

fn process_interact_requests(
    mut receivers: InteractReceivers,
    world: InteractionWorld,
    mut senders: InteractionSenders,
    mut commands: Commands,
) {
    for (connection, link, current, mut receiver) in &mut receivers {
        let mut current = current.map(|interaction| interaction.npc);
        for request in receiver.receive() {
            let result = Entity::try_from_bits(request.npc)
                .ok_or(InteractionError::InvalidTarget)
                .and_then(|npc| open_interaction(&world, link.0, npc));
            let interaction = match result {
                Ok(interaction) => interaction,
                Err(error) => {
                    senders.failed(connection, request.npc, error);
                    continue;
                }
            };
            if let Some(previous) = current.filter(|previous| *previous != interaction.0.npc) {
                senders.closed(connection, previous);
            }
            current = Some(interaction.0.npc);
            senders.opened(connection, interaction.0.npc, interaction.1);
            commands.entity(connection).insert(interaction.0);
        }
    }
}

fn open_interaction(
    world: &InteractionWorld,
    player: Entity,
    npc: Entity,
) -> Result<(ActiveInteraction, InteractionKind), InteractionError> {
    let (npc_data, flags, ..) = world.interactable(player, npc)?;
    let opening = world
        .catalog
        .open(npc_data.template_id, *flags)
        .ok_or(InteractionError::InvalidTarget)?;
    Ok(apply_opening(world, npc, opening))
}

fn apply_opening(
    world: &InteractionWorld,
    npc: Entity,
    opening: Opening,
) -> (ActiveInteraction, InteractionKind) {
    let role = match opening.kind {
        InteractionKind::Role(role) => Some(role),
        InteractionKind::Gossip(_) => None,
    };
    let interaction = ActiveInteraction {
        npc,
        role,
        auction_house: (role == Some(NpcRole::AuctionHouse)).then(|| world.auction_house(npc)),
        gossip_actions: opening.actions,
    };
    (interaction, opening.kind)
}

type GossipReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<SelectGossipOption>,
    ),
    With<InWorld>,
>;

fn process_gossip_selections(
    mut receivers: GossipReceivers,
    world: InteractionWorld,
    mut senders: InteractionSenders,
    mut commands: Commands,
) {
    for (connection, link, current, mut receiver) in &mut receivers {
        let mut current = current.cloned();
        for request in receiver.receive() {
            let Some(interaction) = current.as_ref().filter(|i| i.npc.to_bits() == request.npc)
            else {
                continue;
            };
            let npc = interaction.npc;
            let Some(action) = interaction
                .gossip_actions
                .iter()
                .find_map(|(id, action)| (*id == request.option_id).then_some(*action))
            else {
                continue;
            };
            let flags = match world.interactable(link.0, npc) {
                Ok((_, flags, ..)) => *flags,
                Err(error) => {
                    senders.failed(connection, request.npc, error);
                    continue;
                }
            };
            let opening = match action {
                GossipAction::Submenu(menu_id) => world.catalog.submenu(menu_id, flags),
                GossipAction::Role(role) => Opening {
                    kind: InteractionKind::Role(role),
                    actions: Vec::new(),
                },
                GossipAction::Close => {
                    current = None;
                    senders.closed(connection, npc);
                    commands.entity(connection).remove::<ActiveInteraction>();
                    continue;
                }
            };
            let (interaction, kind) = apply_opening(&world, npc, opening);
            senders.opened(connection, npc, kind);
            commands.entity(connection).insert(interaction.clone());
            current = Some(interaction);
        }
    }
}

type CloseReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<CloseInteraction>,
    ),
    With<InWorld>,
>;

fn process_close_requests(mut receivers: CloseReceivers, mut commands: Commands) {
    for (connection, current, mut receiver) in &mut receivers {
        let closes_current = receiver
            .receive()
            .any(|request| current.is_some_and(|i| i.npc.to_bits() == request.npc));
        if closes_current {
            commands.entity(connection).remove::<ActiveInteraction>();
        }
    }
}

fn close_invalid_interactions(
    connections: Query<(Entity, &PlayerLink, &ActiveInteraction)>,
    world: InteractionWorld,
    mut senders: InteractionSenders,
    mut commands: Commands,
) {
    for (connection, link, interaction) in &connections {
        if world.interactable(link.0, interaction.npc).is_ok() {
            continue;
        }
        senders.closed(connection, interaction.npc);
        commands.entity(connection).remove::<ActiveInteraction>();
    }
}

#[cfg(test)]
#[path = "npc_interaction_tests.rs"]
mod tests;

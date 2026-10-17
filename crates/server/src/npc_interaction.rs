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
//!
//! `UseGameObject` opens a game object's frame the same way (TrinityCore
//! `WorldSession::HandleGameObjectUseOpcode` / `Player::GetGameObjectIfCanInteractWith`):
//! a Guild Vault opens the guild bank role for a living player within the object's
//! reach (`game_objects::GameObjectBounds::reaches`). Meeting stones and summoning
//! portals open no frame; `meeting_stone::use_object` casts their spell.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, Npc, Position, UnitFactionTemplate};
use shared::protocol::{
    CloseInteraction, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_MAILBOX,
    GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_RITUAL, GameObjectInfo, InteractNpc,
    InteractionChannel, InteractionClosed, InteractionError, InteractionFailed, InteractionKind,
    InteractionOpened, NpcFlags, NpcRole, SelectGossipOption, UseGameObject,
};

use crate::auth::InWorld;
use crate::faction::{FactionTemplates, Reaction};
use crate::game_objects::{GAME_OBJECT_INTERACT_DISTANCE, GameObjectBounds, wow_position};
use crate::networking::PlayerLink;

mod gossip;
mod spellclick;
pub(crate) use spellclick::{SpellClickCatalog, load_spellclick_catalog};

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
    /// The NPC or game object.
    pub npc: Entity,
    /// Role frame currently open; `None` while a gossip menu is shown.
    pub role: Option<NpcRole>,
    /// Auction house of the auctioneer, set while its frame is open.
    pub auction_house: Option<u8>,
    gossip_actions: Vec<(u32, GossipAction)>,
}

#[cfg(test)]
#[path = "auction_house_tests/regressions.rs"]
mod ahserver_regressions;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NpcInteractionsProcessed;

pub struct NpcInteractionPlugin;

impl Plugin for NpcInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GossipCatalog>();
        app.init_resource::<SpellClickCatalog>();
        app.add_systems(
            Update,
            (
                process_interact_requests,
                process_game_object_uses,
                process_gossip_selections,
                process_close_requests,
                close_invalid_interactions,
            )
                .chain()
                .in_set(NpcInteractionsProcessed),
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

fn validate_npc_view(
    (player_position, player_health, player_faction): (&Position, &Health, &UnitFactionTemplate),
    (_, _, npc_position, npc_health, npc_faction, combat): NpcView<'_>,
    templates: &FactionTemplates,
) -> Result<(), InteractionError> {
    if player_health.current <= 0.0 {
        return Err(InteractionError::PlayerDead);
    }
    if npc_health.current <= 0.0 {
        return Err(InteractionError::TargetDead);
    }
    if templates.reaction(npc_faction.0, player_faction.0) == Reaction::Hostile {
        return Err(InteractionError::TargetHostile);
    }
    if combat.is_some_and(|status| status.0) {
        return Err(InteractionError::TargetInCombat);
    }
    if distance(player_position, npc_position) > INTERACTION_DISTANCE {
        return Err(InteractionError::TooFarAway);
    }
    Ok(())
}

pub(crate) fn find_interactable_tabard_vendor(
    world: &World,
    player: Entity,
    npc: Entity,
) -> Result<&Npc, InteractionError> {
    find_interactable_flag(world, player, npc, NpcFlags::TABARD_DESIGNER)
}

/// Repeat the normal live-NPC checks on every registrar purchase/turn-in request.
pub(crate) fn find_interactable_petitioner(
    world: &World,
    player: Entity,
    npc: Entity,
) -> Result<&Npc, InteractionError> {
    find_interactable_flag(world, player, npc, NpcFlags::PETITIONER)
}

pub(crate) fn find_interactable_crafting_broker(
    world: &World,
    player: Entity,
    npc: Entity,
) -> Result<&Npc, InteractionError> {
    find_interactable_flag(world, player, npc, NpcFlags::CRAFTING_ORDERS)
}

pub(crate) fn find_interactable_trading_vendor(
    world: &World,
    player: Entity,
    npc: Entity,
) -> Result<&Npc, InteractionError> {
    find_interactable_flag(world, player, npc, NpcFlags::TRADING_POST)
}

fn find_interactable_flag(
    world: &World,
    player: Entity,
    npc: Entity,
    required_flag: u64,
) -> Result<&Npc, InteractionError> {
    let target = world
        .get::<Npc>(npc)
        .ok_or(InteractionError::InvalidTarget)?;
    let flags = world
        .get::<NpcFlags>(npc)
        .ok_or(InteractionError::InvalidTarget)?;
    if !flags.contains(required_flag) {
        return Err(InteractionError::InvalidTarget);
    }
    let player_view = (
        world
            .get::<Position>(player)
            .ok_or(InteractionError::InvalidTarget)?,
        world
            .get::<Health>(player)
            .ok_or(InteractionError::InvalidTarget)?,
        world
            .get::<UnitFactionTemplate>(player)
            .ok_or(InteractionError::InvalidTarget)?,
    );
    let npc_view = (
        target,
        flags,
        world
            .get::<Position>(npc)
            .ok_or(InteractionError::InvalidTarget)?,
        world
            .get::<Health>(npc)
            .ok_or(InteractionError::InvalidTarget)?,
        world
            .get::<UnitFactionTemplate>(npc)
            .ok_or(InteractionError::InvalidTarget)?,
        world.get::<CombatStatus>(npc),
    );
    validate_npc_view(player_view, npc_view, world.resource::<FactionTemplates>())?;
    Ok(target)
}

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
    maps: Query<'w, 's, &'static crate::maps::InMap>,
    guild_users: Query<
        'w,
        's,
        (
            &'static shared::components::GuildMembership,
            &'static shared::reputation::CharacterReputation,
        ),
    >,
    objects: Query<
        'w,
        's,
        (
            &'static GameObjectInfo,
            &'static Position,
            &'static GameObjectBounds,
            Has<crate::game_objects::GameObjectScript>,
            Has<crate::game_objects::GameObjectInUse>,
            Has<crate::game_objects::seaforium_bombs::SeaforiumFuse>,
            Has<crate::game_objects::summoned::SummonedObject>,
        ),
    >,
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
        let player_view = self
            .players
            .get(player)
            .map_err(|_| InteractionError::InvalidTarget)?;
        validate_npc_view(player_view, npc_view, &self.templates)?;
        Ok(npc_view)
    }

    /// What using a game object does if `player` may use it now.
    fn usable_object(&self, player: Entity, object: Entity) -> Result<ObjectUse, InteractionError> {
        let (info, position, bounds, scripted, in_use, seaforium, summoned) = self
            .objects
            .get(object)
            .map_err(|_| InteractionError::InvalidTarget)?;
        let role = match info.go_type {
            // Only buttons with a ported GameObjectAI; generic door/button use is not modelled.
            crate::game_objects::GAMEOBJECT_TYPE_BUTTON if scripted && !in_use => ObjectUse::Script,
            crate::game_objects::GAMEOBJECT_TYPE_TRAP if seaforium => ObjectUse::Seaforium,
            GAMEOBJECT_TYPE_GUILD_BANK => {
                // Installed Mobile Banking tooltip: guild members at Friendly1168
                // may USE the mobile chest. Static vaults keep their ordinary rules.
                if summoned {
                    let eligible = self.guild_users.get(player).is_ok_and(|(_, rep)| {
                        rep.has_standing(1168, shared::reputation::Standing::Friendly)
                    });
                    if !eligible {
                        return Err(InteractionError::InvalidTarget);
                    }
                }
                ObjectUse::Role(NpcRole::GuildBanker)
            }
            GAMEOBJECT_TYPE_MAILBOX => ObjectUse::Role(NpcRole::Mailbox),
            GAMEOBJECT_TYPE_MEETINGSTONE | GAMEOBJECT_TYPE_RITUAL => ObjectUse::Spell,
            crate::game_objects::spellcaster::GAMEOBJECT_TYPE_SPELLCASTER => ObjectUse::Spellcaster,
            GAMEOBJECT_TYPE_CHAIR => ObjectUse::Chair,
            shared::protocol::GAMEOBJECT_TYPE_KEYSTONE_RECEPTACLE => ObjectUse::Challenge,
            shared::protocol::GAMEOBJECT_TYPE_HOUSING_PLOT => ObjectUse::HousingUiLink,
            _ if info.is_battleground_flag()
                || info.is_battleground_orb()
                || info.is_battleground_capture_point()
                || info.is_battleground_banner() =>
            {
                ObjectUse::BattlegroundFlag
            }
            crate::game_objects::GAMEOBJECT_TYPE_CHEST
            | crate::game_objects::GAMEOBJECT_TYPE_GOOBER => ObjectUse::QuestObject,
            _ => return Err(InteractionError::InvalidTarget),
        };
        let (player_position, player_health, _) = self
            .players
            .get(player)
            .map_err(|_| InteractionError::InvalidTarget)?;
        if player_health.current <= 0.0 {
            return Err(InteractionError::PlayerDead);
        }
        let reaches = bounds.reaches(
            wow_position(position),
            wow_position(player_position),
            GAME_OBJECT_INTERACT_DISTANCE,
        );
        if !reaches {
            return Err(InteractionError::TooFarAway);
        }
        Ok(role)
    }

    /// Whether the open interaction with `target` (an NPC or a game object) still holds.
    fn still_interactable(&self, player: Entity, target: Entity) -> bool {
        if self.objects.contains(target) {
            self.usable_object(player, target).is_ok()
        } else {
            self.interactable(player, target).is_ok()
        }
    }

    fn auction_house(&self, npc: Entity) -> u8 {
        let faction = self.npcs.get(npc).map_or(0, |view| view.4.0);
        auction_house_for(&self.templates, faction)
    }
}

/// Result of using a game object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectUse {
    QuestObject,
    Spellcaster,
    Seaforium,
    /// Opens a role frame.
    Role(NpcRole),
    /// Casts the object's spell (`meeting_stone::use_object`).
    Spell,
    /// Seats the user (`chair::sit`).
    Chair,
    /// Retail GAMEOBJECT_TYPE_KEYSTONE_RECEPTACLE.
    Challenge,
    BattlegroundFlag,
    HousingUiLink,
    /// `GameObjectAI::OnGossipHello` of a ported object script.
    Script,
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
            if let Some(npc) = Entity::try_from_bits(request.npc)
                && let Ok(view) = world.npcs.get(npc)
                && view.1.contains(NpcFlags::SPELLCLICK)
            {
                let result = world.interactable(link.0, npc).and_then(|view| {
                    if world.maps.get(link.0).ok() != world.maps.get(npc).ok()
                        || world.maps.get(npc).is_err()
                    {
                        return Err(InteractionError::InvalidTarget);
                    }
                    Ok(view.0.template_id)
                });
                match result {
                    Ok(entry) => {
                        let player = link.0;
                        commands.queue(move |world: &mut World| {
                            spellclick::cast_spellclick(world, player, npc, entry)
                        });
                    }
                    Err(error) => senders.failed(connection, request.npc, error),
                }
                continue;
            }
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
            let player = link.0;
            let npc = interaction.0.npc;
            let entry = interaction.2;
            commands.queue(move |world: &mut World| {
                if !crate::scripted_ai::creature_gossip_hello(world, player, npc) {
                    send_opening(world, connection, interaction.0, interaction.1);
                }
                // Player::TalkedToCreature: only the actor receives TALKTO credit.
                crate::quest::event_credit(
                    world,
                    player,
                    npc,
                    crate::quest_data::ObjectiveKind::TalkTo,
                    entry,
                    false,
                );
            });
        }
    }
}

type UseReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<UseGameObject>,
    ),
    With<InWorld>,
>;

fn process_game_object_uses(
    mut receivers: UseReceivers,
    gatherables: Query<&shared::protocol::GatherableResource>,
    world: InteractionWorld,
    mut senders: InteractionSenders,
    mut commands: Commands,
) {
    for (connection, link, current, mut receiver) in &mut receivers {
        let mut current = current.map(|interaction| interaction.npc);
        for request in receiver.receive() {
            debug!(player = ?link.0, object = request.object, "UseGameObject received");
            if let Some(target) = Entity::try_from_bits(request.object)
                && gatherables.contains(target)
            {
                let player = link.0;
                commands.queue(move |world: &mut World| {
                    crate::gathering::use_resource(world, player, target)
                });
                continue;
            }
            let result = Entity::try_from_bits(request.object)
                .ok_or(InteractionError::InvalidTarget)
                .and_then(|object| Ok((object, world.usable_object(link.0, object)?)));
            let (object, role) = match result {
                Ok((object, ObjectUse::Role(role))) => (object, role),
                Ok((object, ObjectUse::Spell)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        crate::meeting_stone::use_object(world, player, object);
                    });
                    continue;
                }
                Ok((object, ObjectUse::Spellcaster)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        if let Err(error) =
                            crate::game_objects::spellcaster::use_object(world, player, object)
                            && let Some(mut sender) =
                                world.get_mut::<MessageSender<InteractionFailed>>(connection)
                        {
                            sender.send::<InteractionChannel>(InteractionFailed {
                                npc: object.to_bits(),
                                error,
                            });
                        }
                    });
                    continue;
                }
                Ok((object, ObjectUse::Seaforium)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        if let Err(error) =
                            crate::game_objects::seaforium_bombs::disarm(world, player, object)
                            && let Some(mut sender) =
                                world.get_mut::<MessageSender<InteractionFailed>>(connection)
                        {
                            sender.send::<InteractionChannel>(InteractionFailed {
                                npc: object.to_bits(),
                                error,
                            });
                        }
                    });
                    continue;
                }
                Ok((object, ObjectUse::QuestObject)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        if let Err(error) =
                            crate::game_objects::quest_object::use_object(world, player, object)
                            && let Some(mut sender) =
                                world.get_mut::<MessageSender<InteractionFailed>>(connection)
                        {
                            sender.send::<InteractionChannel>(InteractionFailed {
                                npc: object.to_bits(),
                                error,
                            });
                        }
                    });
                    continue;
                }
                Ok((object, ObjectUse::Script)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        crate::scripted_ai::game_object_gossip_hello(world, player, object);
                    });
                    continue;
                }
                Ok((object, ObjectUse::Chair)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        crate::chair::sit(world, player, object);
                    });
                    continue;
                }
                Ok((object, ObjectUse::Challenge)) => {
                    commands.queue(move |world: &mut World| {
                        crate::mythic_plus::open_font(world, connection, object)
                    });
                    continue;
                }
                Ok((object, ObjectUse::BattlegroundFlag)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        crate::pvp::use_battleground_object(world, connection, player, object);
                    });
                    continue;
                }
                Ok((object, ObjectUse::HousingUiLink)) => {
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        if let Err(error) =
                            crate::housing::use_ui_link(world, connection, player, object)
                            && let Some(mut sender) =
                                world.get_mut::<MessageSender<InteractionFailed>>(connection)
                        {
                            sender.send::<InteractionChannel>(InteractionFailed {
                                npc: object.to_bits(),
                                error,
                            });
                        }
                    });
                    continue;
                }
                Err(error) => {
                    debug!(player = ?link.0, object = request.object, error = ?error, "UseGameObject rejected before cast");
                    senders.failed(connection, request.object, error);
                    continue;
                }
            };
            if let Some(previous) = current.filter(|previous| *previous != object) {
                senders.closed(connection, previous);
            }
            current = Some(object);
            senders.opened(connection, object, InteractionKind::Role(role));
            commands.entity(connection).insert(ActiveInteraction {
                npc: object,
                role: Some(role),
                auction_house: None,
                gossip_actions: Vec::new(),
            });
        }
    }
}

fn open_interaction(
    world: &InteractionWorld,
    player: Entity,
    npc: Entity,
) -> Result<(ActiveInteraction, InteractionKind, u32), InteractionError> {
    let player_map = world
        .maps
        .get(player)
        .map_err(|_| InteractionError::InvalidTarget)?;
    let npc_map = world
        .maps
        .get(npc)
        .map_err(|_| InteractionError::InvalidTarget)?;
    if player_map != npc_map {
        return Err(InteractionError::InvalidTarget);
    }
    let (npc_data, flags, ..) = world.interactable(player, npc)?;
    let opening = world
        .catalog
        .open(npc_data.template_id, *flags)
        .ok_or(InteractionError::InvalidTarget)?;
    let (interaction, kind) = apply_opening(world, npc, opening);
    Ok((interaction, kind, npc_data.template_id))
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

fn player_connection(world: &mut World, player: Entity) -> Option<Entity> {
    world
        .query::<(Entity, &PlayerLink)>()
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

fn send_opening(
    world: &mut World,
    connection: Entity,
    interaction: ActiveInteraction,
    kind: InteractionKind,
) {
    if let Some(mut sender) = world.get_mut::<MessageSender<InteractionOpened>>(connection) {
        sender.send::<InteractionChannel>(InteractionOpened {
            npc: interaction.npc.to_bits(),
            kind,
        });
    }
    world.entity_mut(connection).insert(interaction);
}

/// Player::PrepareGossipMenu/SendPreparedGossip for a source script. Normal
/// ingress still validates authored NPC flags before invoking the callback.
pub(crate) fn prepare_gossip_menu(world: &mut World, player: Entity, npc: Entity, menu_id: u32) {
    let connection = player_connection(world, player).expect("gossip player connection");
    let flags = *world.get::<NpcFlags>(npc).expect("gossip NPC flags");
    let opening = world.resource::<GossipCatalog>().submenu(menu_id, flags);
    send_opening(
        world,
        connection,
        ActiveInteraction {
            npc,
            role: None,
            auction_house: None,
            gossip_actions: opening.actions,
        },
        opening.kind,
    );
}

pub(crate) fn close_gossip_menu(world: &mut World, player: Entity, npc: Entity) {
    let connection = player_connection(world, player).expect("gossip player connection");
    if let Some(mut sender) = world.get_mut::<MessageSender<InteractionClosed>>(connection) {
        sender.send::<InteractionChannel>(InteractionClosed { npc: npc.to_bits() });
    }
    world.entity_mut(connection).remove::<ActiveInteraction>();
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
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        crate::scripted_ai::creature_gossip_select(world, player, npc);
                        if world.get::<ActiveInteraction>(connection).is_some() {
                            close_gossip_menu(world, player, npc);
                        }
                    });
                    continue;
                }
            };
            let (interaction, kind) = apply_opening(&world, npc, opening);
            let player = link.0;
            let next = interaction.clone();
            commands.queue(move |world: &mut World| {
                crate::scripted_ai::creature_gossip_select(world, player, npc);
                if world.get::<ActiveInteraction>(connection).is_some() {
                    send_opening(world, connection, next, kind);
                }
            });
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
        if world.still_interactable(link.0, interaction.npc) {
            continue;
        }
        senders.closed(connection, interaction.npc);
        commands.entity(connection).remove::<ActiveInteraction>();
    }
}

#[cfg(test)]
#[path = "npc_interaction_tests.rs"]
mod tests;

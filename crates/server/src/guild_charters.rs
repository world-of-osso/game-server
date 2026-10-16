//! Retail registrar/petition requests. Actors come only from authenticated InWorld links.
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::charters as store;
use crate::persistence::{self, GameDatabase};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::GuildMembership;
use shared::guild_creation::CHARTER_COST;
use shared::protocol::{
    GUILD_CHARTER_ITEM, GuildChannel, GuildCharterReply, GuildCharterRequest, GuildRanksState,
    GuildRegistrarSnapshot, ItemLocation, NpcRole,
};

#[cfg(test)]
#[path = "guild_charter_mutation_fifo_tests.rs"]
mod mutation_fifo_tests;

#[path = "guild_charter_mutation_persistence.rs"]
mod mutation_actor;
pub(super) use mutation_actor::install;

#[derive(Clone, Copy)]
struct Actor {
    connection: Entity,
    character: u64,
    account: u64,
    player: Entity,
}

fn receive_requests(world: &mut World) -> Vec<(Actor, GuildCharterRequest)> {
    let mut query = world.query::<(
        Entity,
        &InWorld,
        &Authenticated,
        &PlayerLink,
        &mut MessageReceiver<GuildCharterRequest>,
    )>();
    query
        .iter_mut(world)
        .flat_map(|(connection, entered, auth, link, mut receiver)| {
            let actor = Actor {
                connection,
                character: entered.character_id,
                account: auth.account_id,
                player: link.0,
            };
            receiver
                .receive()
                .map(move |request| (actor, request))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn process_requests(world: &mut World) {
    for (actor, request) in receive_requests(world) {
        if let Err(message) = handle_request(world, actor, request) {
            send_reply(
                world,
                actor.connection,
                GuildCharterReply::Failed { message },
            );
        }
    }
}

fn send_reply(world: &mut World, connection: Entity, reply: GuildCharterReply) {
    if let Some(mut sender) = world.get_mut::<MessageSender<GuildCharterReply>>(connection) {
        sender.send::<GuildChannel>(reply);
    }
}

fn handle_request(
    world: &mut World,
    actor: Actor,
    request: GuildCharterRequest,
) -> Result<(), String> {
    match request {
        GuildCharterRequest::Registrar { npc } => send_registrar(world, actor, npc),
        GuildCharterRequest::Purchase { npc, guild_name } => {
            purchase(world, actor, npc, &guild_name)
        }
        GuildCharterRequest::Show { charter_guid } => {
            require_owner(world, actor, charter_guid)?;
            send_petition(world, actor, charter_guid)
        }
        GuildCharterRequest::Offer {
            charter_guid,
            character_name,
        } => offer(world, actor, charter_guid, &character_name),
        GuildCharterRequest::Sign { charter_guid } => sign(world, actor, charter_guid),
        GuildCharterRequest::Rename {
            charter_guid,
            guild_name,
        } => rename(world, actor, charter_guid, &guild_name),
        GuildCharterRequest::TurnIn { npc, charter_guid } => {
            turn_in(world, actor, npc, charter_guid)
        }
    }
}

fn require_petitioner(world: &World, actor: Actor, bits: u64) -> Result<String, String> {
    let npc = Entity::try_from_bits(bits).ok_or("Invalid petitioner.")?;
    let interaction = world
        .get::<ActiveInteraction>(actor.connection)
        .ok_or("You are not interacting with a petitioner.")?;
    if interaction.npc != npc || interaction.role != Some(NpcRole::Petitioner) {
        return Err("You are not interacting with that petitioner.".into());
    }
    crate::npc_interaction::find_interactable_petitioner(world, actor.player, npc)
        .map(|npc| npc.name.clone())
        .map_err(|error| error.message().to_string())
}
fn require_owner(world: &World, actor: Actor, guid: u64) -> Result<(), String> {
    require_persisted_owner(&world.resource::<GameDatabase>().db, actor.character, guid)
}

fn require_persisted_owner(db: &redb::Database, character: u64, guid: u64) -> Result<(), String> {
    if store::owner_of(db, guid)? != character {
        return Err("You are not the petition owner.".into());
    }
    let record = persistence::load_inventory(db, character);
    if !record
        .items
        .iter()
        .any(|item| item.item_guid == guid && item.item_id == GUILD_CHARTER_ITEM)
    {
        return Err("You don't own that guild charter.".into());
    }
    Ok(())
}

fn send_registrar(world: &mut World, actor: Actor, npc: u64) -> Result<(), String> {
    let npc_name = require_petitioner(world, actor, npc)?;
    let charter_guid =
        store::owned_charter_guid(&world.resource::<GameDatabase>().db, actor.character);
    send_reply(
        world,
        actor.connection,
        GuildCharterReply::Registrar(GuildRegistrarSnapshot {
            npc,
            npc_name,
            charter_cost: CHARTER_COST,
            charter_guid,
        }),
    );
    Ok(())
}

fn purchase(world: &mut World, actor: Actor, npc: u64, name: &str) -> Result<(), String> {
    require_petitioner(world, actor, npc)?;
    let guid = store::purchase(
        &world.resource::<GameDatabase>().db,
        world.resource::<crate::item_templates::ItemTemplates>(),
        world.resource::<crate::merchant::VendorCatalog>(),
        actor.character,
        name,
        crate::guild_bank::unix_now(),
    )?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(actor.character);
    send_reply(
        world,
        actor.connection,
        GuildCharterReply::Purchased { charter_guid: guid },
    );
    Ok(())
}

fn send_petition(world: &mut World, actor: Actor, guid: u64) -> Result<(), String> {
    let snapshot = store::snapshot(
        &world.resource::<GameDatabase>().db,
        guid,
        actor.character,
        actor.account,
    )?;
    send_reply(
        world,
        actor.connection,
        GuildCharterReply::Petition(snapshot),
    );
    Ok(())
}

fn find_online_actor(world: &mut World, character: u64) -> Option<Actor> {
    let mut query = world.query::<(Entity, &InWorld, &Authenticated, &PlayerLink)>();
    query
        .iter(world)
        .find_map(|(connection, entered, auth, link)| {
            (entered.character_id == character).then_some(Actor {
                connection,
                character,
                account: auth.account_id,
                player: link.0,
            })
        })
}

fn offer(world: &mut World, actor: Actor, guid: u64, name: &str) -> Result<(), String> {
    require_owner(world, actor, guid)?;
    let db = &world.resource::<GameDatabase>().db;
    let character =
        persistence::find_character_id_by_name(db, name.trim()).ok_or("Player not found.")?;
    if persistence::load_character_guild(db, character).is_some() {
        return Err("That player is already in a guild.".into());
    }
    if persistence::load_pending_guild_invite(db, character).is_some() {
        return Err("That player has already been invited into a guild.".into());
    }
    let target = find_online_actor(world, character).ok_or("That player is not online.")?;
    send_petition(world, target, guid)?;
    // The actor also receives an authoritative answer, releasing UI pacing.
    send_petition(world, actor, guid)
}

fn sign(world: &mut World, actor: Actor, guid: u64) -> Result<(), String> {
    mutation_actor::enqueue(world, actor, mutation_actor::Change::Sign(guid));
    Ok(())
}
fn rename(world: &mut World, actor: Actor, guid: u64, name: &str) -> Result<(), String> {
    mutation_actor::enqueue(
        world,
        actor,
        mutation_actor::Change::Rename {
            guid,
            name: name.into(),
        },
    );
    Ok(())
}

fn turn_in(world: &mut World, actor: Actor, npc: u64, guid: u64) -> Result<(), String> {
    require_petitioner(world, actor, npc)?;
    require_owner(world, actor, guid)?;
    let created = store::turn_in(
        &world.resource::<GameDatabase>().db,
        world.resource::<crate::item_templates::ItemTemplates>(),
        guid,
        actor.character,
        crate::guild_bank::unix_now(),
    )?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(actor.character);
    refresh_founders(world, &created);
    send_reply(
        world,
        actor.connection,
        GuildCharterReply::Created {
            guild_id: created.guild_id,
            guild_name: created.guild_name,
        },
    );
    Ok(())
}

fn refresh_founders(world: &mut World, created: &shared::guild_creation::CreatedGuild) {
    for character in
        std::iter::once(created.guild_master).chain(created.founding_members.iter().copied())
    {
        let Some(actor) = find_online_actor(world, character) else {
            continue;
        };
        world.entity_mut(actor.player).insert(GuildMembership {
            guild_id: created.guild_id,
            guild_name: created.guild_name.clone(),
        });
        let snapshot =
            super::guild_rank_snapshot(&world.resource::<GameDatabase>().db, character, None);
        if let Some(mut sender) = world.get_mut::<MessageSender<GuildRanksState>>(actor.connection)
        {
            sender.send::<GuildChannel>(snapshot);
        }
    }
}

/// The normal bag UseItem route opens the real owned charter before spell dispatch.
pub(crate) fn use_item(
    world: &mut World,
    connection: Entity,
    character: u64,
    location: ItemLocation,
) -> Result<bool, shared::protocol::InventoryError> {
    let (db, templates) = crate::inventory::use_store(world)?;
    let model = crate::inventory::load_model(db, templates, character)?;
    let Some(item) = model.view().get(&location).cloned().filter(|item| {
        item.item_id == GUILD_CHARTER_ITEM
            && item.definition_source == shared::item_data::ItemDefinitionSource::Retail
    }) else {
        return Ok(false);
    };
    let actor = Actor {
        connection,
        character,
        account: world
            .get::<Authenticated>(connection)
            .expect("authenticated item request")
            .account_id,
        player: world
            .get::<PlayerLink>(connection)
            .expect("in-world player link")
            .0,
    };
    match require_owner(world, actor, item.item_guid)
        .and_then(|()| send_petition(world, actor, item.item_guid))
    {
        Ok(()) => Ok(true),
        Err(message) => {
            send_reply(world, connection, GuildCharterReply::Failed { message });
            Ok(true)
        }
    }
}

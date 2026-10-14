//! Retail guild-tabard vendor: authenticated interaction, installed catalog, atomic payment.
use crate::{
    auth::{Authenticated, InWorld},
    networking::PlayerLink,
    persistence::{GameDatabase, guild_tabards as store},
};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::{
    components::GuildMembership,
    guild_creation::{GuildTabardDesign, TABARD_COST},
    guild_tabards::GuildTabardCatalog,
    protocol::*,
};

#[derive(Resource)]
struct Catalog(GuildTabardCatalog);

pub struct GuildTabardPlugin;
impl Plugin for GuildTabardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_catalog);
        app.add_systems(
            Update,
            (sync_membership_designs, process_requests)
                .chain()
                .after(crate::npc_interaction::NpcInteractionsProcessed),
        );
    }
}

fn load_catalog(mut commands: Commands) {
    let catalog = GuildTabardCatalog::load(std::path::Path::new("data/db2/active"))
        .expect("load installed guild tabard catalog");
    commands.insert_resource(Catalog(catalog));
}

fn sync_membership_designs(
    db: Res<GameDatabase>,
    mut commands: Commands,
    guilds: Query<(Entity, &GuildMembership), Changed<GuildMembership>>,
    mut removed: RemovedComponents<GuildMembership>,
) {
    for entity in removed.read() {
        if let Ok(mut entity) = commands.get_entity(entity) {
            entity.remove::<GuildTabardDesign>();
        }
    }
    for (entity, guild) in &guilds {
        match store::load_design(&db.db, guild.guild_id) {
            Some(design) => {
                commands.entity(entity).insert(design);
            }
            None => {
                commands.entity(entity).remove::<GuildTabardDesign>();
            }
        }
    }
}

fn process_requests(world: &mut World) {
    let mut query = world.query_filtered::<(Entity, &mut MessageReceiver<GuildTabardRequest>), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(entity, mut receiver)| {
            receiver
                .receive()
                .map(move |request| (entity, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, request) in requests {
        let reply = match handle_request(world, connection, request) {
            Ok(reply) => reply,
            Err(message) => GuildTabardReply::Failed { message },
        };
        let mut sender = world
            .get_mut::<MessageSender<GuildTabardReply>>(connection)
            .expect("authenticated guild tabard request requires reply sender");
        sender.send::<GuildChannel>(reply);
    }
}

fn require_vendor(world: &World, connection: Entity, bits: u64) -> Result<(u64, String), String> {
    let entered = world
        .get::<InWorld>(connection)
        .ok_or("You are not in the world.")?;
    world
        .get::<Authenticated>(connection)
        .ok_or("You are not authenticated.")?;
    let player = world
        .get::<PlayerLink>(connection)
        .ok_or("Player is unavailable.")?
        .0;
    let npc = Entity::try_from_bits(bits).ok_or("Invalid guild tabard vendor.")?;
    let interaction = world
        .get::<crate::npc_interaction::ActiveInteraction>(connection)
        .ok_or("You are not interacting with a guild tabard vendor.")?;
    if interaction.npc != npc || interaction.role != Some(NpcRole::TabardDesigner) {
        return Err("You are not interacting with that guild tabard vendor.".into());
    }
    let vendor = crate::npc_interaction::find_interactable_tabard_vendor(world, player, npc)
        .map_err(|error| error.message().to_string())?;
    Ok((entered.character_id, vendor.name.clone()))
}

fn handle_request(
    world: &mut World,
    connection: Entity,
    request: GuildTabardRequest,
) -> Result<GuildTabardReply, String> {
    let npc = match &request {
        GuildTabardRequest::Open { npc } | GuildTabardRequest::Save { npc, .. } => *npc,
    };
    let (character, name) = require_vendor(world, connection, npc)?;
    match request {
        GuildTabardRequest::Open { .. } => Ok(GuildTabardReply::Vendor(vendor_snapshot(
            world, character, npc, name,
        ))),
        GuildTabardRequest::Save { design, .. } => register_design(world, character, design),
    }
}

fn vendor_snapshot(
    world: &World,
    character: u64,
    npc: u64,
    npc_name: String,
) -> GuildTabardVendorSnapshot {
    let db = &world.resource::<GameDatabase>().db;
    let membership = crate::persistence::load_character_guild(db, character);
    let own_rank = membership.as_ref().map(|guild| {
        let ranks = crate::persistence::banks::load_guild(db, guild.guild_id);
        crate::persistence::banks::load_guild_member_rank(db, &ranks, character)
    });
    let design = membership.and_then(|guild| store::load_design(db, guild.guild_id));
    GuildTabardVendorSnapshot {
        npc,
        npc_name,
        own_rank,
        design,
        cost: u64::from(TABARD_COST),
    }
}

fn register_design(
    world: &mut World,
    character: u64,
    design: GuildTabardDesign,
) -> Result<GuildTabardReply, String> {
    world.resource::<Catalog>().0.validate(design)?;
    let db = &world.resource::<GameDatabase>().db;
    let guild_id = store::register(db, character, design)?;
    let gold = crate::persistence::load_inventory(db, character).gold;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record(character, gold);
    crate::guild::broadcast_registered_tabard(world, guild_id, design);
    Ok(GuildTabardReply::Saved { design })
}

//! TC Player::SetSpellFavorite: known spells only, persisted for the character GUID.
//! Separate storage leaves existing bitcode PlayerProfessions records unchanged.
use super::{PlayerProfessions, ProfessionData};
use crate::{auth::{Authenticated, InWorld}, networking::PlayerLink, persistence::{ClientId, GameDatabase}};
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::{Database, MultimapTableDefinition, ReadableDatabase, ReadableMultimapTable, WriteTransaction};
use shared::protocol::{ProfessionChannel, ProfessionSnapshot, SetRecipeFavorite};

const FAVORITES: MultimapTableDefinition<u64, u32> = MultimapTableDefinition::new("character_recipe_favorites");

pub(crate) fn initialize_store(txn: &WriteTransaction) {
    txn.open_multimap_table(FAVORITES).expect("create character recipe favorites");
}

fn load_favorites(db: &Database, character: u64) -> Vec<u32> {
    let txn = db.begin_read().expect("begin recipe favorites read");
    let table = txn.open_multimap_table(FAVORITES).expect("open recipe favorites");
    table.get(character).expect("read character recipe favorites")
        .map(|entry| entry.expect("read favorite spell ID").value()).collect()
}

fn persist_favorite(db: &Database, character: u64, request: &SetRecipeFavorite) {
    let txn = db.begin_write().expect("begin recipe favorite write");
    {
        let mut table = txn.open_multimap_table(FAVORITES).expect("open recipe favorites for write");
        if request.favorite {
            table.insert(character, request.spell_id).expect("insert character recipe favorite");
        } else {
            table.remove(character, request.spell_id).expect("remove character recipe favorite");
        }
    }
    txn.commit().expect("commit character recipe favorite");
}

pub(super) fn load_snapshot(db: &Database, character: u64, profs: &PlayerProfessions, data: &ProfessionData) -> ProfessionSnapshot {
    let mut snapshot = profs.snapshot(data);
    snapshot.favorites = load_favorites(db, character).into_iter().filter(|spell| profs.knows(*spell)).collect();
    snapshot
}

pub(super) fn receive_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(Entity, &PlayerLink, &mut MessageReceiver<SetRecipeFavorite>), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = receivers.iter_mut(world).flat_map(|(connection, link, mut receiver)| {
        receiver.receive().map(|request| (connection, link.0, request)).collect::<Vec<_>>()
    }).collect();
    for (connection, player, request) in requests {
        apply_and_send(world, connection, player, request);
    }
}

fn apply_and_send(world: &mut World, connection: Entity, player: Entity, request: SetRecipeFavorite) {
    let profs = world.get::<PlayerProfessions>(player).expect("authenticated favorite owner professions");
    let character = world.get::<ClientId>(player).expect("authenticated favorite owner ID").0;
    let db = world.resource::<GameDatabase>();
    if profs.knows(request.spell_id) {
        persist_favorite(&db.db, character, &request);
    } else {
        warn!(character, spell=request.spell_id, "Rejected favorite preference for an unlearned spell");
    }
    let snapshot = load_snapshot(&db.db, character, profs, world.resource::<ProfessionData>());
    world.get_mut::<MessageSender<ProfessionSnapshot>>(connection)
        .expect("registered profession snapshot sender").send::<ProfessionChannel>(snapshot);
}

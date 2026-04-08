use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::Mounted;
use shared::protocol::{
    CollectionChannel, CollectionMountSnapshot, CollectionPetSnapshot, CollectionSnapshot,
    CollectionStateUpdate, DismissMount, DismissPet, SummonMount, SummonPet,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCollectionState {
    pub mounts: Vec<CollectionMount>,
    pub pets: Vec<CollectionPet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionMount {
    pub mount_id: u32,
    pub name: String,
    pub known: bool,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionPet {
    pub pet_id: u32,
    pub name: String,
    pub known: bool,
    pub active: bool,
}

trait CollectionEntry {
    const KIND: &'static str;

    fn id(&self) -> u32;
    fn name(&self) -> &str;
    fn known(&self) -> bool;
    fn active(&self) -> bool;
    fn set_active(&mut self, active: bool);
}

impl CollectionEntry for CollectionMount {
    const KIND: &'static str = "mount";

    fn id(&self) -> u32 {
        self.mount_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn known(&self) -> bool {
        self.known
    }

    fn active(&self) -> bool {
        self.active
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

impl CollectionEntry for CollectionPet {
    const KIND: &'static str = "pet";

    fn id(&self) -> u32 {
        self.pet_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn known(&self) -> bool {
        self.known
    }

    fn active(&self) -> bool {
        self.active
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

impl PlayerCollectionState {
    pub fn active_mount_display_id(&self) -> Option<u32> {
        self.mounts
            .iter()
            .find(|mount| mount.active)
            .map(|mount| mount.mount_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MountStateChange {
    Mounted(u32),
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionChange {
    message: String,
    mount_state: Option<MountStateChange>,
}

impl CollectionChange {
    fn mounted(message: String, mount_id: u32) -> Self {
        Self {
            message,
            mount_state: Some(MountStateChange::Mounted(mount_id)),
        }
    }

    fn dismissed(message: String) -> Self {
        Self {
            message,
            mount_state: Some(MountStateChange::Dismissed),
        }
    }

    fn pet(message: String) -> Self {
        Self {
            message,
            mount_state: None,
        }
    }
}

trait CollectionRequest: lightyear::prelude::Message {
    fn apply(
        collection: &mut PlayerCollectionState,
        request: &Self,
    ) -> Result<CollectionChange, String>;
}

impl CollectionRequest for SummonMount {
    fn apply(
        collection: &mut PlayerCollectionState,
        request: &Self,
    ) -> Result<CollectionChange, String> {
        let message = summon_mount(collection, request.mount_id)?;
        Ok(CollectionChange::mounted(message, request.mount_id))
    }
}

impl CollectionRequest for DismissMount {
    fn apply(
        collection: &mut PlayerCollectionState,
        _request: &Self,
    ) -> Result<CollectionChange, String> {
        let message = dismiss_mount(collection)?;
        Ok(CollectionChange::dismissed(message))
    }
}

impl CollectionRequest for SummonPet {
    fn apply(
        collection: &mut PlayerCollectionState,
        request: &Self,
    ) -> Result<CollectionChange, String> {
        let message = summon_pet(collection, request.pet_id)?;
        Ok(CollectionChange::pet(message))
    }
}

impl CollectionRequest for DismissPet {
    fn apply(
        collection: &mut PlayerCollectionState,
        _request: &Self,
    ) -> Result<CollectionChange, String> {
        let message = dismiss_pet(collection)?;
        Ok(CollectionChange::pet(message))
    }
}

pub struct CollectionPlugin;

impl Plugin for CollectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, send_initial_collection_state);
        app.add_systems(Update, process_collection_requests::<SummonMount>);
        app.add_systems(Update, process_collection_requests::<DismissMount>);
        app.add_systems(Update, process_collection_requests::<SummonPet>);
        app.add_systems(Update, process_collection_requests::<DismissPet>);
    }
}

pub fn default_collection_state() -> PlayerCollectionState {
    PlayerCollectionState {
        mounts: vec![
            CollectionMount {
                mount_id: 101,
                name: "Swift Brown Steed".into(),
                known: true,
                active: false,
            },
            CollectionMount {
                mount_id: 102,
                name: "Black War Wolf".into(),
                known: false,
                active: false,
            },
        ],
        pets: vec![
            CollectionPet {
                pet_id: 202,
                name: "Brown Rabbit".into(),
                known: true,
                active: false,
            },
            CollectionPet {
                pet_id: 203,
                name: "Blue Moth".into(),
                known: false,
                active: false,
            },
        ],
    }
}

fn send_initial_collection_state(
    connections: Query<(Entity, &PlayerLink), (With<Authenticated>, Added<InWorld>)>,
    collections: Query<&PlayerCollectionState>,
    mut senders: Query<&mut MessageSender<CollectionStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
        let Ok(collection) = collections.get(link.0) else {
            send_collection_error(
                conn_entity,
                &mut senders,
                "collections unavailable for player",
            );
            continue;
        };
        send_collection_update(
            conn_entity,
            collection,
            Some("collections loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_collection_requests<R: CollectionRequest>(
    mut commands: Commands,
    mut receivers: Query<
        (Entity, &PlayerLink, &InWorld, &mut MessageReceiver<R>),
        With<Authenticated>,
    >,
    mut collections: Query<&mut PlayerCollectionState>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<CollectionStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in receivers.iter_mut() {
        for request in receiver.receive() {
            let result = collections
                .get_mut(link.0)
                .map_err(|_| "collections unavailable for player".to_string())
                .and_then(|mut collection| {
                    let change = R::apply(&mut collection, &request)?;
                    persistence::save_collections(&db.db, in_world.character_id, &collection);
                    Ok((change, collection.clone()))
                });
            apply_collection_change(&mut commands, conn_entity, link.0, result, &mut senders);
        }
    }
}

fn apply_collection_change(
    commands: &mut Commands,
    conn_entity: Entity,
    player_entity: Entity,
    result: Result<(CollectionChange, PlayerCollectionState), String>,
    senders: &mut Query<&mut MessageSender<CollectionStateUpdate>>,
) {
    match result {
        Ok((change, collection)) => {
            apply_mount_state_change(commands, player_entity, change.mount_state);
            send_collection_update(
                conn_entity,
                &collection,
                Some(change.message),
                None,
                senders,
            );
        }
        Err(error) => {
            send_collection_error(conn_entity, senders, error);
        }
    }
}

fn apply_mount_state_change(
    commands: &mut Commands,
    player_entity: Entity,
    mount_state: Option<MountStateChange>,
) {
    match mount_state {
        Some(MountStateChange::Mounted(mount_display_id)) => {
            commands
                .entity(player_entity)
                .insert(Mounted { mount_display_id });
        }
        Some(MountStateChange::Dismissed) => {
            commands.entity(player_entity).remove::<Mounted>();
        }
        None => {}
    }
}

pub(crate) fn summon_mount(
    collection: &mut PlayerCollectionState,
    mount_id: u32,
) -> Result<String, String> {
    summon_entry(&mut collection.mounts, mount_id)
}

pub(crate) fn dismiss_mount(collection: &mut PlayerCollectionState) -> Result<String, String> {
    dismiss_active_entry(&mut collection.mounts)
}

pub(crate) fn summon_pet(
    collection: &mut PlayerCollectionState,
    pet_id: u32,
) -> Result<String, String> {
    summon_entry(&mut collection.pets, pet_id)
}

pub(crate) fn dismiss_pet(collection: &mut PlayerCollectionState) -> Result<String, String> {
    dismiss_active_entry(&mut collection.pets)
}

fn summon_entry<T: CollectionEntry>(entries: &mut [T], entry_id: u32) -> Result<String, String> {
    let entry_name = entries
        .iter()
        .find(|entry| entry.id() == entry_id && entry.known())
        .map(|entry| entry.name().to_string())
        .ok_or_else(|| format!("unknown or unavailable {} {}", T::KIND, entry_id))?;
    clear_active_entries(entries);
    let entry = entries
        .iter_mut()
        .find(|entry| entry.id() == entry_id)
        .expect("known entry exists");
    entry.set_active(true);
    Ok(format!("summoned {}", entry_name))
}

fn dismiss_active_entry<T: CollectionEntry>(entries: &mut [T]) -> Result<String, String> {
    let active = entries
        .iter_mut()
        .find(|entry| entry.active())
        .ok_or_else(|| format!("no active {}", T::KIND))?;
    active.set_active(false);
    Ok(format!("dismissed {}", active.name()))
}

fn clear_active_entries<T: CollectionEntry>(entries: &mut [T]) {
    for entry in entries {
        entry.set_active(false);
    }
}

fn build_collection_snapshot(collection: &PlayerCollectionState) -> CollectionSnapshot {
    CollectionSnapshot {
        mounts: collection
            .mounts
            .iter()
            .map(|mount| CollectionMountSnapshot {
                mount_id: mount.mount_id,
                name: mount.name.clone(),
                known: mount.known,
                active: mount.active,
            })
            .collect(),
        pets: collection
            .pets
            .iter()
            .map(|pet| CollectionPetSnapshot {
                pet_id: pet.pet_id,
                name: pet.name.clone(),
                known: pet.known,
                active: pet.active,
            })
            .collect(),
    }
}

fn send_collection_update(
    conn_entity: Entity,
    collection: &PlayerCollectionState,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<CollectionStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<CollectionChannel>(CollectionStateUpdate {
        snapshot: Some(build_collection_snapshot(collection)),
        message,
        error,
    });
}

fn send_collection_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<CollectionStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<CollectionChannel>(CollectionStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
#[path = "collection_tests.rs"]
mod tests;

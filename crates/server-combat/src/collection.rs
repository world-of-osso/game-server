//! Legacy character collection serialization retained for companion snapshots.
//! Mount ownership and casting are account-authoritative in mount_journal.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use shared::protocol::{CollectionMountSnapshot, CollectionPetSnapshot, CollectionSnapshot};

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

pub struct CollectionPlugin;
impl Plugin for CollectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            crate::battle_pets::BattlePetPlugin,
            crate::mount_journal::MountJournalPlugin,
        ));
    }
}

pub fn default_collection_state() -> PlayerCollectionState {
    PlayerCollectionState {
        mounts: vec![],
        pets: vec![],
    }
}

pub(crate) fn build_collection_snapshot(collection: &PlayerCollectionState) -> CollectionSnapshot {
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

#[cfg(test)]
#[path = "collection_tests.rs"]
mod tests;

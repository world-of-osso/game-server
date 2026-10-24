//! One cast's shared applied-identity receipt, retained by delayed snapshots.
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use bevy::prelude::Entity;

#[derive(Debug, Clone, Default)]
pub struct AppliedSpellModifiers(Arc<Mutex<BTreeSet<(Entity, u32)>>>);

impl PartialEq for AppliedSpellModifiers {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for AppliedSpellModifiers {}

impl AppliedSpellModifiers {
    pub(crate) fn record(&self, owner: Entity, instance: u32) {
        self.0
            .lock()
            .expect("applied spellmod identities poisoned")
            .insert((owner, instance));
    }

    pub fn read(&self) -> Vec<(Entity, u32)> {
        self.0
            .lock()
            .expect("applied spellmod identities poisoned")
            .iter()
            .copied()
            .collect()
    }
}

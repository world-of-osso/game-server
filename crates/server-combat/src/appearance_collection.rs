//! Account-wide appearance collection (docs/specs/appearance-collection.md).
//! Inventory writes persist equip/bind/disposal grants atomically; quest rewards
//! also grant unchosen visuals. `AppearanceSourceItems` triggers this plugin's
//! refresh after a mutation, including removal of the last appearance-bearing
//! item. World entry also learns already-owned equipment and soulbound items.
//! The owner receives the complete `AppearanceCollectionUpdate` when it changes.

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{AppearanceCollectionUpdate, TooltipChannel};
use shared::transmog::AppearanceCollection;

use crate::auth::{Authenticated, InWorld};
use crate::inventory::InventoryDeltas;
use crate::item_templates::ItemTemplates;
use crate::persistence::GameDatabase;
use crate::persistence::appearance_store::{load_account_appearances, save_account_appearances};

/// Item ids whose appearance the connection's account learns: the in-world
/// character's equipped and soulbound items.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub(crate) struct AppearanceSourceItems(pub Vec<(shared::item_data::ItemDefinitionSource, u32)>);

/// The connection's account collection, loaded on first world entry.
#[derive(Component, Debug, Clone, PartialEq, Default)]
struct AccountAppearances(AppearanceCollection);

pub struct AppearanceCollectionPlugin;

impl Plugin for AppearanceCollectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            learn_appearances
                .after(InventoryDeltas)
                .before(ReplicationSystems::Send),
        );
    }
}

type Connections<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Authenticated,
        Ref<'static, InWorld>,
        Ref<'static, AppearanceSourceItems>,
        Option<&'static mut AccountAppearances>,
    ),
>;

/// Learn `items`' appearances; the number newly learned.
fn learn_items(
    collection: &mut AppearanceCollection,
    items: &[(shared::item_data::ItemDefinitionSource, u32)],
    templates: &ItemTemplates,
) -> usize {
    collection.learn_many(
        items
            .iter()
            .filter_map(|&(source, item_id)| templates.get(source, item_id)?.appearance_id),
    )
}

fn learn_appearances(
    mut connections: Connections,
    db: Option<Res<GameDatabase>>,
    templates: Option<Res<ItemTemplates>>,
    mut senders: Query<&mut MessageSender<AppearanceCollectionUpdate>>,
    mut commands: Commands,
) {
    let (Some(db), Some(templates)) = (db, templates) else {
        return;
    };
    for (connection, account, in_world, sources, loaded) in &mut connections {
        let account_id = account.account_id;
        let first = loaded.is_none();
        if !first && !sources.is_changed() && !in_world.is_added() {
            continue;
        }
        // Successful mutations already saved their grants, even when no item
        // survives in the final inventory. Reload instead of trusting the cache.
        let mut collection = load_account_appearances(&db.db, account_id);
        if learn_items(&mut collection, &sources.0, &templates) > 0 {
            save_account_appearances(&db.db, account_id, &collection);
        }
        let changed = match loaded {
            Some(mut loaded) => {
                let changed = loaded.0 != collection;
                loaded.0 = collection.clone();
                changed
            }
            None => {
                commands
                    .entity(connection)
                    .insert(AccountAppearances(collection.clone()));
                true
            }
        };
        if (changed || first || in_world.is_added())
            && let Ok(mut sender) = senders.get_mut(connection)
        {
            sender.send::<TooltipChannel>(AppearanceCollectionUpdate {
                appearances: collection.all().collect(),
            });
        }
    }
}

#[cfg(test)]
#[path = "appearance_collection_tests.rs"]
pub(crate) mod tests;

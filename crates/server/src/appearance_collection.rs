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

/// None marks a requested first load; Some contains only a committed collection.
#[derive(Component, Debug, Clone, PartialEq, Default)]
struct AccountAppearances(Option<AppearanceCollection>);

mod actor;
use actor::{PendingAppearanceRefreshes, RefreshRequest, publish_appearance_refreshes};

pub struct AppearanceCollectionPlugin;

impl Plugin for AppearanceCollectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingAppearanceRefreshes>()
            .add_systems(
                PostUpdate,
                (enqueue_appearance_refreshes, publish_appearance_refreshes)
                    .chain()
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
        Option<&'static AccountAppearances>,
    ),
>;

fn enqueue_appearance_refreshes(
    connections: Connections,
    db: Option<Res<GameDatabase>>,
    templates: Option<Res<ItemTemplates>>,
    mut pending: ResMut<PendingAppearanceRefreshes>,
    mut commands: Commands,
) {
    let (Some(db), Some(templates)) = (db, templates) else {
        return;
    };
    for (connection, account, in_world, sources, loaded) in &connections {
        let account_id = account.account_id;
        let first = loaded.is_none();
        if !first && !sources.is_changed() && !in_world.is_added() {
            continue;
        }
        if first {
            commands.entity(connection).insert(AccountAppearances(None));
        }
        let appearances = sources
            .0
            .iter()
            .filter_map(|&(source, item)| templates.get(source, item)?.appearance_id)
            .collect();
        pending.enqueue(
            &db,
            RefreshRequest {
                connection,
                account: account_id,
                character: in_world.character_id,
                force_send: in_world.is_added(),
                appearances,
            },
        );
    }
}

#[cfg(test)]
#[path = "appearance_collection_tests.rs"]
pub(crate) mod tests;

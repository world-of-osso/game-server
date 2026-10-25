//! Unit tooltip data (game-engine `docs/specs/unit-tooltip.md`): a
//! `CreatureTooltipQuery` for a creature template entry answers `CreatureTooltip`
//! with the template's name, subname and creature type, its loot table's drops with
//! their chances (`LootTables::drop_chances`, poor-quality items left out because
//! loot turns them into coins) and its vendor list (`VendorCatalog`). Every item
//! carries the `ItemAppearance` id it teaches, so the client can mark it against
//! the account's collection.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    CreatureTooltip, CreatureTooltipQuery, TooltipChannel, TooltipDrop, TooltipItem,
};

use crate::auth::{Authenticated, InWorld};
use crate::item_templates::ItemTemplates;
use crate::loot::LootTables;
use crate::merchant::VendorCatalog;
use crate::world_data::WorldData;

/// Retail `Enum.ItemQuality.Poor`.
const POOR_QUALITY: u8 = 0;

pub struct CreatureTooltipPlugin;

impl Plugin for CreatureTooltipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, answer_creature_tooltip_queries);
    }
}

/// World content a tooltip is built from, all loaded at startup.
#[derive(SystemParam)]
struct TooltipSources<'w> {
    world: Option<Res<'w, WorldData>>,
    loot: Option<Res<'w, LootTables>>,
    catalog: Option<Res<'w, VendorCatalog>>,
    items: Option<Res<'w, ItemTemplates>>,
}

impl TooltipSources<'_> {
    fn tooltip(&self, entry: u32) -> Option<CreatureTooltip> {
        let template = self.world.as_deref()?.templates.get(&entry)?;
        let (loot, catalog, items) = (
            self.loot.as_deref()?,
            self.catalog.as_deref()?,
            self.items.as_deref()?,
        );
        let item = |item_id| tooltip_item(item_id, catalog, items);
        let drops = loot
            .drop_chances(entry, 0, items)
            .into_iter()
            .filter_map(|(item_id, chance)| {
                Some(TooltipDrop {
                    item: item(item_id)?,
                    chance,
                })
            })
            .filter(|drop| drop.item.quality != POOR_QUALITY)
            .collect();
        let vendor_items = catalog
            .items(entry)
            .iter()
            .filter_map(|slot| item(slot.item_id))
            .collect();
        Some(CreatureTooltip {
            entry,
            name: template.name.clone(),
            subname: template.subname.clone(),
            creature_type: template.creature_type,
            drops,
            vendor_items,
        })
    }
}

fn tooltip_item(
    item_id: u32,
    catalog: &VendorCatalog,
    items: &ItemTemplates,
) -> Option<TooltipItem> {
    let goods = catalog.goods.get(&item_id)?;
    Some(TooltipItem {
        item_id,
        name: goods.name.clone(),
        quality: goods.quality,
        appearance_id: items
            .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
            .and_then(|template| template.appearance_id),
    })
}

type Queries<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut MessageReceiver<CreatureTooltipQuery>),
    (With<Authenticated>, With<InWorld>),
>;

fn answer_creature_tooltip_queries(
    mut queries: Queries,
    mut senders: Query<&mut MessageSender<CreatureTooltip>>,
    sources: TooltipSources,
) {
    for (connection, mut receiver) in &mut queries {
        for query in receiver.receive() {
            let Some(tooltip) = sources.tooltip(query.entry) else {
                warn!(
                    "CreatureTooltipQuery for unknown creature entry {}",
                    query.entry
                );
                continue;
            };
            if let Ok(mut sender) = senders.get_mut(connection) {
                sender.send::<TooltipChannel>(tooltip);
            }
        }
    }
}

#[cfg(test)]
#[path = "creature_tooltip_tests.rs"]
mod tests;

//! Real-DB tests against data/world.db.

use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};

use super::*;
use crate::item_templates::load_item_templates;
use crate::loot::load_loot_tables;
use crate::merchant::load_vendor_catalog;
use crate::world_data::{load_expected_stats, load_templates};

const DEFIAS_THUG: u32 = 38;
const CORINA_STEELE: u32 = 54;

#[derive(Resource, Default)]
struct Inbox(Vec<CreatureTooltip>);

fn collect(mut receivers: Query<&mut MessageReceiver<CreatureTooltip>>, mut inbox: ResMut<Inbox>) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

struct Harness {
    app: App,
    connection: Entity,
}

fn harness() -> Option<Harness> {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    app.init_resource::<Inbox>().add_systems(First, collect);
    app.insert_resource(WorldData {
        templates: load_templates(&conn, &load_expected_stats(&conn)).into(),
        ..Default::default()
    });
    app.insert_resource(load_loot_tables(&conn).unwrap());
    app.insert_resource(load_vendor_catalog(&conn).unwrap());
    app.insert_resource(load_item_templates(&path).unwrap());
    app.add_plugins(CreatureTooltipPlugin);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id: 1 },
            MessageSender::<CreatureTooltipQuery>::default(),
            MessageReceiver::<CreatureTooltipQuery>::default(),
            MessageSender::<CreatureTooltip>::default(),
            MessageReceiver::<CreatureTooltip>::default(),
        ))
        .id();
    app.update();
    Some(Harness { app, connection })
}

impl Harness {
    fn query(&mut self, entry: u32) -> Vec<CreatureTooltip> {
        self.app
            .world_mut()
            .get_mut::<MessageSender<CreatureTooltipQuery>>(self.connection)
            .unwrap()
            .send::<TooltipChannel>(CreatureTooltipQuery { entry });
        for _ in 0..3 {
            self.app.update();
        }
        std::mem::take(&mut self.app.world_mut().resource_mut::<Inbox>().0)
    }
}

fn item(item_id: u32, name: &str, quality: u8, appearance_id: Option<u32>) -> TooltipItem {
    TooltipItem {
        item_id,
        name: name.into(),
        quality,
        appearance_id,
    }
}

#[test]
fn defias_thug_tooltip_lists_its_loot_table_without_greys() {
    let Some(mut h) = harness() else { return };

    let tooltips = h.query(DEFIAS_THUG);

    assert_eq!(tooltips.len(), 1);
    let tooltip = &tooltips[0];
    assert_eq!(tooltip.entry, DEFIAS_THUG);
    assert_eq!(tooltip.name, "Defias Thug");
    assert_eq!(tooltip.subname, "");
    assert_eq!(tooltip.creature_type, 7, "Humanoid");
    let listed: Vec<(&str, f32)> = tooltip
        .drops
        .iter()
        .map(|drop| {
            (
                drop.item.name.as_str(),
                (drop.chance * 100.0).round() / 100.0,
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            ("Red Burlap Bandana", 60.0),
            ("Darnassian Bleu", 20.1),
            ("Refreshing Spring Water", 9.9),
            ("Pitted Defias Shortsword", 2.0),
            ("Small Red Pouch", 0.04),
            ("Small Blue Pouch", 0.04),
            ("Small Brown Pouch", 0.04),
            ("Small Black Pouch", 0.04),
            ("Small Green Pouch", 0.04),
        ],
        "the 23 grey items of reference 20000 are not listed"
    );
    let sword = tooltip
        .drops
        .iter()
        .find(|drop| drop.item.item_id == 2057)
        .unwrap();
    assert_eq!(
        sword.item,
        item(2057, "Pitted Defias Shortsword", 1, Some(646))
    );
    assert_eq!(tooltip.drops[0].item.appearance_id, None, "quest item");
    assert!(tooltip.vendor_items.is_empty());
}

#[test]
fn weaponsmith_tooltip_lists_the_items_she_sells() {
    let Some(mut h) = harness() else { return };

    let tooltips = h.query(CORINA_STEELE);

    let tooltip = &tooltips[0];
    assert_eq!(tooltip.name, "Corina Steele");
    assert_eq!(tooltip.subname, "Weaponsmith");
    assert!(tooltip.drops.is_empty());
    assert_eq!(
        tooltip.vendor_items,
        [
            // Retail (TDB1210) npc_vendor slot order.
            item(2489, "Two-Handed Sword", 1, Some(607)),
            item(2492, "Cudgel", 1, Some(858)),
            item(2491, "Large Axe", 1, Some(857)),
            item(2494, "Stiletto", 1, Some(860)),
            item(2495, "Walking Stick", 1, Some(861)),
            item(2488, "Gladius", 1, Some(154)),
            item(2490, "Tomahawk", 1, Some(388)),
            item(2493, "Wooden Mallet", 1, Some(859)),
        ]
    );
}

#[test]
fn an_unknown_entry_gets_no_answer() {
    let Some(mut h) = harness() else { return };

    assert!(h.query(999_999_999).is_empty());
}

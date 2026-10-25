//! Actual registered LFG loopback: catalog is queried, not a client-side list.
use super::*;
use crate::{auth::Authenticated, lfg::LfgPlugin, networking::PlayerLink};
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use shared::protocol::LfgChannel;
use shared::protocol::{FollowerDungeonEntry, FollowerDungeons, QueryFollowerDungeons};
use std::time::Duration;

#[derive(Component, Default)]
struct CatalogInbox(Vec<FollowerDungeons>);

#[test]
fn follower_catalog_query_returns_prepared_authored_entries_only() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<QueryFollowerDungeons>();
    app.register_message::<FollowerDungeons>();
    let mut catalog = crate::followers::FollowerCatalog::default();
    catalog.follower_ids.extend([2436, 2439]);
    catalog.dungeons.insert(
        2436,
        crate::followers::catalog::Dungeon {
            id: 2436,
            name: "Ruby Life Pools".into(),
            entrance: crate::maps::WorldLocation {
                map_id: 2521,
                position: Vec3::new(1386.2, -169.719, 131.0),
                orientation: 342.674_f32.to_radians(),
            },
        },
    );
    app.insert_resource(catalog);
    app.add_plugins(LfgPlugin);
    app.add_systems(
        Update,
        |mut inboxes: Query<(&mut CatalogInbox, &mut MessageReceiver<FollowerDungeons>)>| {
            for (mut inbox, mut receiver) in &mut inboxes {
                inbox.0.extend(receiver.receive());
            }
        },
    );
    app.finish();
    app.cleanup();
    let player = app.world_mut().spawn_empty().id();
    let conn = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(5761)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 5761 },
            PlayerLink(player),
            CatalogInbox::default(),
            MessageSender::<QueryFollowerDungeons>::default(),
            MessageReceiver::<QueryFollowerDungeons>::default(),
            MessageSender::<FollowerDungeons>::default(),
            MessageReceiver::<FollowerDungeons>::default(),
        ))
        .id();
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .get_mut::<MessageSender<QueryFollowerDungeons>>(conn)
        .unwrap()
        .send::<LfgChannel>(QueryFollowerDungeons);
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        app.world().get::<CatalogInbox>(conn).unwrap().0,
        vec![FollowerDungeons {
            entries: vec![FollowerDungeonEntry {
                id: 2436,
                name: "Ruby Life Pools".into(),
                map_id: 2521
            }],
        }]
    );
    app.world_mut()
        .resource_mut::<crate::followers::FollowerCatalog>()
        .dungeons
        .clear();
    app.world_mut()
        .get_mut::<MessageSender<QueryFollowerDungeons>>(conn)
        .unwrap()
        .send::<LfgChannel>(QueryFollowerDungeons);
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        app.world().get::<CatalogInbox>(conn).unwrap().0.last(),
        Some(&FollowerDungeons { entries: vec![] })
    );
}

//! Server-authoritative choices for LFDQueueFrame's follower list.
use super::*;
use shared::protocol::{FollowerDungeonEntry, FollowerDungeons, QueryFollowerDungeons};

pub(super) fn send_follower_catalog(
    mut connections: Query<
        (
            &PlayerLink,
            &mut MessageReceiver<QueryFollowerDungeons>,
            &mut MessageSender<FollowerDungeons>,
        ),
        With<Authenticated>,
    >,
    catalog: Res<crate::followers::FollowerCatalog>,
) {
    for (_, mut receiver, mut sender) in &mut connections {
        if receiver.receive().count() == 0 {
            continue;
        }
        let mut entries: Vec<_> = catalog
            .dungeons
            .values()
            .map(|dungeon| FollowerDungeonEntry {
                id: dungeon.id,
                name: dungeon.name.clone(),
                map_id: dungeon.entrance.map_id,
            })
            .collect();
        entries.sort_by_key(|entry| entry.id);
        sender.send::<LfgChannel>(FollowerDungeons { entries });
    }
}

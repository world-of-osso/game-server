use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Player, Zone};
use shared::protocol::{QueryWho, WhoChannel, WhoCharacterSnapshot, WhoSnapshot, WhoStateUpdate};

use crate::auth::{Authenticated, InWorld};

pub struct WhoPlugin;

impl Plugin for WhoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_query_requests);
    }
}

fn process_query_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryWho>), With<Authenticated>>,
    players: Query<(&Player, &Level, Option<&Zone>)>,
    mut senders: Query<&mut MessageSender<WhoStateUpdate>>,
) {
    let online_entries = collect_online_entries(&players);
    for (conn_entity, _in_world, mut receiver) in &mut receivers {
        let Some(request) = receiver.receive().last() else {
            continue;
        };
        let snapshot = build_who_snapshot(&request.query, online_entries.clone());
        let result_count = snapshot.entries.len();
        send_who_update(
            conn_entity,
            snapshot,
            Some(format!("who: {result_count} result(s)")),
            None,
            &mut senders,
        );
    }
}

fn collect_online_entries(
    players: &Query<(&Player, &Level, Option<&Zone>)>,
) -> Vec<WhoCharacterSnapshot> {
    players
        .iter()
        .map(|(player, level, zone)| WhoCharacterSnapshot {
            name: player.name.clone(),
            level: level.0 as u16,
            class_name: class_name_from_id(player.class),
            area: zone
                .map(|zone| format!("Zone {}", zone.id))
                .unwrap_or_else(|| "-".into()),
        })
        .collect()
}

fn build_who_snapshot(query: &str, mut entries: Vec<WhoCharacterSnapshot>) -> WhoSnapshot {
    let query = query.trim().to_string();
    entries.retain(|entry| matches_query(&entry.name, &query));
    entries.sort_by_cached_key(|entry| entry.name.to_ascii_lowercase());
    WhoSnapshot { query, entries }
}

fn matches_query(name: &str, query: &str) -> bool {
    query.is_empty()
        || name
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
}

fn class_name_from_id(class_id: u8) -> String {
    match class_id {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        6 => "Death Knight",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        10 => "Monk",
        11 => "Druid",
        12 => "Demon Hunter",
        13 => "Evoker",
        _ => "Unknown",
    }
    .into()
}

fn send_who_update(
    conn_entity: Entity,
    snapshot: WhoSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<WhoStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<WhoChannel>(WhoStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<WhoCharacterSnapshot> {
        vec![
            WhoCharacterSnapshot {
                name: "Zulu".into(),
                level: 42,
                class_name: "Mage".into(),
                area: "Zone 2".into(),
            },
            WhoCharacterSnapshot {
                name: "Alice".into(),
                level: 12,
                class_name: "Paladin".into(),
                area: "Zone 1".into(),
            },
            WhoCharacterSnapshot {
                name: "Theron".into(),
                level: 18,
                class_name: "Hunter".into(),
                area: "Zone 3".into(),
            },
        ]
    }

    #[test]
    fn build_who_snapshot_filters_case_insensitively() {
        let snapshot = build_who_snapshot("ali", sample_entries());

        assert_eq!(snapshot.query, "ali");
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].name, "Alice");
    }

    #[test]
    fn build_who_snapshot_sorts_results_by_name() {
        let snapshot = build_who_snapshot("", sample_entries());

        assert_eq!(snapshot.entries.len(), 3);
        assert_eq!(snapshot.entries[0].name, "Alice");
        assert_eq!(snapshot.entries[1].name, "Theron");
        assert_eq!(snapshot.entries[2].name, "Zulu");
    }
}

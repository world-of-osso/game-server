//! Dynamic per-character task list used by Mainline-style map pins.
use crate::persistence::quest_store::load_quest_log;
use crate::quest::{QuestStore, quest_player};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, Player};
use shared::protocol::{AvailableWorldQuest, QuestChannel, WorldQuestAvailability, WorldQuestPoi};

#[derive(Component)]
pub(crate) struct SentAvailability(WorldQuestAvailability);

pub(crate) fn publish(
    mut commands: Commands,
    store: QuestStore,
    players: Query<(&Player, &Level)>,
    mut connections: Query<(
        Entity,
        &crate::auth::InWorld,
        &crate::networking::PlayerLink,
        &mut MessageSender<WorldQuestAvailability>,
        Option<&mut SentAvailability>,
    )>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    for (connection, owner, link, mut sender, sent) in &mut connections {
        let Ok((player, level)) = players.get(link.0) else {
            continue;
        };
        let log = load_quest_log(db, owner.character_id);
        let snapshot = snapshot(quests, &log, quest_player(player, level));
        if sent.as_ref().is_some_and(|sent| sent.0 == snapshot) {
            continue;
        }
        sender.send::<QuestChannel>(snapshot.clone());
        if let Some(mut sent) = sent {
            sent.0 = snapshot;
        } else {
            commands
                .entity(connection)
                .insert(SentAvailability(snapshot));
        }
    }
}

fn snapshot(
    quests: &crate::quest_data::QuestDb,
    log: &crate::persistence::quest_store::QuestLogData,
    player: crate::quest_rules::QuestPlayer,
) -> WorldQuestAvailability {
    let mut active = Vec::new();
    for window in quests.world_quest_windows.values() {
        let Some(quest) = quests.quests.get(&window.quest_id) else {
            continue;
        };
        let completed = log
            .completed_at
            .get(&quest.id)
            .is_some_and(|&at| at >= window.starts_at);
        if completed
            || (log.active(quest.id).is_none()
                && crate::quest_rules::can_take(quests, log, player, quest).is_err())
        {
            continue;
        }
        let Some(areas) = quests.world_quest_areas.get(&quest.id) else {
            continue;
        };
        let pois = areas.iter().map(area_poi).collect();
        let data = quests
            .world_quest_rewards
            .get(&quest.id)
            .expect("imported world quest reward metadata");
        active.push(AvailableWorldQuest {
            quest_id: quest.id,
            title: quest.title.clone(),
            zone_id: quest.sort_id.max(0) as u32,
            pois,
            expires_at: window.expires_at,
            rewards: crate::quest_view::quest_rewards(quests, quest, player),
            xp: crate::world_quest_rewards::xp(quests, quest, player.level),
            currencies: data.currencies.clone(),
            reputation: data.reputation.clone(),
        });
    }
    WorldQuestAvailability {
        server_time: quests.now.expect("world quest clock initialized"),
        quests: active,
    }
}

fn area_poi(area: &crate::world_quests::Area) -> WorldQuestPoi {
    let count = area.vertices.len() as f32;
    WorldQuestPoi {
        map_id: area.map_id,
        ui_map_id: area.ui_map_id,
        x: (area.vertices.iter().map(|point| point.0).sum::<f32>() / count) as i32,
        y: (area.vertices.iter().map(|point| point.1).sum::<f32>() / count) as i32,
        z: (area.vertices.iter().map(|point| point.2).sum::<f32>() / count) as i32,
    }
}

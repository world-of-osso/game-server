use super::*;
use crate::achievement::PlayerAchievementState;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};

fn raid_roster(leader: u64, members: usize) -> GroupRegistry {
    let mut groups = GroupRegistry::default();
    if members == 0 {
        return groups;
    }
    let online: OnlineView = (leader..leader + members as u64)
        .map(|id| {
            (
                id,
                OnlineMember {
                    entity: id,
                    name: format!("Roster{id}"),
                    class: 1,
                    level: 80,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    for id in leader + 1..leader + members as u64 {
        groups.invite(leader, &format!("Roster{id}"), &online);
        groups.respond_invite(id, true);
        if id == leader + 1 {
            groups.convert_to_raid(leader);
        }
    }
    assert_eq!(groups.group_members(leader).len(), members);
    groups
}

#[test]
fn achievements_criteria_native_malygos_uses_full_roster_not_nearby_or_online_tappers() {
    for members in [0, 8, 9] {
        let (mut h, boss, map) = bound_boss_at_difficulty(28859, 616, 0, 3, None);
        h.app
            .insert_resource(crate::test_data::achievement_catalog());
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        let (character, _) =
            crate::persistence::create_character(&db, 1, "Rosterproof", 1, 1).unwrap();
        h.app
            .insert_resource(crate::persistence::GameDatabase::new(db));
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert((
            crate::persistence::ClientId(character),
            shared::components::Level(30),
            PlayerAchievementState::default(),
        ));
        let connection = h.players[0].1;
        h.app
            .world_mut()
            .entity_mut(connection)
            .insert(crate::auth::InWorld {
                character_id: character,
            });
        // Only the leader is in this copy. The accepted roster's other members are offline.
        h.app.insert_resource(raid_roster(character, members));
        h.engage(boss);
        h.advance(0.05);
        h.hit_pct(boss, 200.0);
        h.advance(0.05);
        assert_eq!(h.inbox(0).ends.last().map(|end| end.success), Some(true));
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1
        );
        let expected = (members < 9).then_some(1);
        let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
        assert_eq!(
            state.criteria_progress.get(&7174).copied(),
            expected,
            "full roster {members}"
        );
        assert!(
            !state.criteria_progress.contains_key(&7175),
            "10-player copy cannot credit25-player leaf"
        );
        let snapshot = h
            .app
            .world()
            .resource::<crate::achievement::AchievementCatalog>()
            .snapshot(state, 1);
        assert_eq!(snapshot.earned_ids.contains(&1869), expected.is_some());
        h.advance(0.05);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&7174)
                .copied(),
            expected,
            "no replay"
        );
        drop(h);
        let reopened = redb::Database::open(file.path()).unwrap();
        assert_eq!(
            crate::persistence::load_achievements(&reopened, character)
                .criteria_progress
                .get(&7174)
                .copied(),
            expected,
            "reopened roster{members}"
        );
    }
}

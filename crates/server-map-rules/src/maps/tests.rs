use super::instances::{INSTANCE_UNLOAD_DELAY_SECS, InstanceEntrant};
use super::test_fixtures::*;
use super::*;
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::protocol::TransferAbortReason;

#[test]
fn world_population_expansion_maps_admit_players_without_dungeon_copies() {
    let store = map_store().expect("authentic map metadata");
    let mut manager = InstanceManager::default();
    let player = entrant(9820, None);
    for map_id in [2444, 2552, 2601, 2222, 1643, 1642, 530, 571, 646, 860] {
        assert_eq!(
            manager.player_cannot_enter(store, map_id, player, 80, SUNDAY_NOON),
            None,
            "open-world map {map_id} refused"
        );
        let resolved = manager
            .create_map(store, map_id, player, at(0.0))
            .expect("hosted open-world map");
        assert_eq!(resolved.map, InMap::continent(map_id));
        assert!(!resolved.created);
    }
    // An unselected development/unused map remains refused.
    assert!(
        manager
            .player_cannot_enter(store, 13, player, 80, SUNDAY_NOON)
            .is_some()
    );
}

#[test]
fn arena_roster_reservation_admits_only_its_own_map_copy() {
    let store = map_store().expect("pinned Map DB2");
    let mut manager = InstanceManager::default();
    let copy = manager
        .create_battleground_map(store, 572, &[1, 2, 3, 4], 4, at(0.0))
        .expect("arena reservation");
    for id in 1..=4 {
        let resolved = manager
            .create_map(store, 572, entrant(id, None), at(0.0))
            .expect("reserved arena entrant");
        assert_eq!(resolved.map, copy);
    }
    assert!(
        manager
            .create_map(store, 572, entrant(5, None), at(0.0))
            .is_err()
    );
    assert!(
        manager
            .create_map(store, 617, entrant(1, None), at(0.0))
            .is_err()
    );
}

#[test]
fn housing_map_rows_keep_authored_identity_without_public_admission() {
    let store = map_store().expect("authentic world map catalog");
    let mut manager = InstanceManager::default();
    let entrant = InstanceEntrant {
        character_id: 30,
        leader: 30,
        group: None,
        difficulty_id: 0,
        ..Default::default()
    };
    let now = instances::Now {
        elapsed: 0.0,
        unix: 0,
    };
    for (id, name, wdt) in [
        (2735, "Founder's Point", 5660542),
        (2736, "Razorwind Shores", 5660543),
        (2783, "Home Interior", 5890928),
    ] {
        let entry = store.map(id).expect("housing map row must load");
        assert_eq!(entry.id, id);
        assert_eq!(entry.name, name);
        assert_eq!(entry.wdt_file_data_id, wdt);
        assert!(!entry.is_dungeon());
        assert_eq!(
            manager.create_map(store, id, entrant, now),
            Err(TransferAbortReason::MapNotAllowed)
        );
    }
}

#[test]
fn housing_copies_require_authority_and_isolate_properties_without_difficulty() {
    let store = map_store().expect("authentic world map catalog");
    let mut manager = InstanceManager::default();
    let owner = entrant(30, None);
    let visitor = entrant(31, None);
    let stranger = entrant(32, None);
    let first = manager
        .authorize_housing_entry(store, 2783, 1, owner.character_id)
        .unwrap();
    let second = manager
        .authorize_housing_entry(store, 2783, 2, visitor.character_id)
        .unwrap();
    assert_ne!(first.map, second.map);
    assert!(first.created && second.created);
    assert!(manager.copy_difficulty(first.map).is_none());
    assert!(manager.lock_copies_in_use().is_empty());
    assert_eq!(
        manager.create_map(store, 2783, owner, at(0.0)).unwrap().map,
        first.map
    );
    assert_eq!(
        manager
            .create_map(store, 2783, visitor, at(0.0))
            .unwrap()
            .map,
        second.map
    );
    assert_eq!(
        manager.create_map(store, 2783, stranger, at(0.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        manager.cannot_enter(first.map, stranger, SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        manager.cannot_enter(first.map, visitor, SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    let visit = manager
        .authorize_housing_entry(store, 2783, 1, visitor.character_id)
        .unwrap();
    assert_eq!(visit.map, first.map);
    assert!(!visit.created);
    assert_eq!(manager.cannot_enter(first.map, visitor, SUNDAY_NOON), None);
    assert_eq!(
        manager.player_cannot_enter(store, 2783, owner, 80, SUNDAY_NOON),
        None
    );
    assert_eq!(
        manager.player_cannot_enter(store, 2783, stranger, 80, SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    manager.revoke_housing_entry(owner.character_id);
    assert_eq!(
        manager.create_map(store, 2783, owner, at(0.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        manager.authorize_housing_entry(store, STOCKADE, 1, owner.character_id),
        Err(TransferAbortReason::MapNotAllowed)
    );
}

#[test]
fn stockade_entrance_trigger_leads_to_its_arrival_point_and_the_exit_back() {
    let Some(store) = map_store() else { return };
    let map = store.map(STOCKADE).expect("map 34");
    assert_eq!(map.directory, "stormwindjail");
    assert!(map.is_dungeon());
    assert_eq!(map.area_table_id, 717);

    let entrance = store
        .area_triggers_on(0)
        .iter()
        .find(|trigger| trigger.id == 101)
        .expect("area trigger 101 on map 0");
    assert!(entrance.contains(STOCKADE_ENTRANCE_TRIGGER));
    // Box 4.97 x 9.69 x 7.44 turned 0.663 rad: 6 yd off-center along X is outside.
    assert!(!entrance.contains(STOCKADE_ENTRANCE_TRIGGER + Vec3::new(6.0, 0.0, 0.0)));
    assert!(entrance.allow_while_ghost);
    assert_eq!(entrance.target.map_id, STOCKADE);
    assert!(close(entrance.target.position, STOCKADE_ARRIVAL));

    let back = store
        .go_back_location(STOCKADE)
        .expect("go-back trigger 503");
    assert_eq!(back.map_id, 0);
    assert!(close(back.position, STOCKADE_EXIT_TARGET));
    let inside = store.entrance_location(STOCKADE).expect("entrance trigger");
    assert!(close(inside.position, STOCKADE_ARRIVAL));
}

#[test]
fn a_stockade_death_releases_to_the_stockade_graveyard_in_stormwind() {
    let Some(store) = map_store() else { return };
    let inside = WorldLocation {
        map_id: STOCKADE,
        position: STOCKADE_ARRIVAL,
        orientation: 0.0,
    };
    let grave = graveyard::closest_graveyard_in_zone(store, &inside, 717).expect("graveyard");
    assert_eq!(grave.map_id, 0);
    assert!(close(grave.position, STOCKADE_GRAVEYARD));
}

#[test]
fn group_members_share_a_stockade_copy_and_a_solo_player_gets_its_own() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let first = instances
        .create_map(store, STOCKADE, entrant(1, Some(7)), at(0.0))
        .unwrap();
    let second = instances
        .create_map(store, STOCKADE, entrant(2, Some(7)), at(1.0))
        .unwrap();
    let solo = instances
        .create_map(store, STOCKADE, entrant(3, None), at(2.0))
        .unwrap();

    assert!(first.created);
    assert_eq!(
        second,
        instances::ResolvedMap {
            map: first.map,
            created: false,
        }
    );
    assert!(solo.created);
    assert_ne!(solo.map.instance_id, first.map.instance_id);
    // The solo player's copy is not the group's to enter, and the other way round.
    assert_eq!(
        instances.find_instance(store, STOCKADE, entrant(3, Some(7)), SUNDAY_NOON),
        Some(first.map)
    );
    assert_eq!(
        instances.find_instance(store, STOCKADE, entrant(1, None), SUNDAY_NOON),
        None
    );
}

#[test]
fn warsong_rosters_share_only_their_reserved_copy_without_dungeon_locks() {
    let store = map_store().expect("local map catalog");
    let template = shared::battleground::warsong_gulch();
    let mut instances = InstanceManager::default();
    let first_roster: Vec<u64> = (1..=10).collect();
    let second_roster: Vec<u64> = (11..=20).collect();
    let capacity = u32::from(template.team_size) * 2;
    assert_eq!(
        instances.create_map(store, template.map_id, entrant(1, None), at(0.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    let first = instances
        .create_battleground_map(store, template.map_id, &first_roster, capacity, at(0.0))
        .unwrap();
    let second = instances
        .create_battleground_map(store, template.map_id, &second_roster, capacity, at(0.0))
        .unwrap();
    assert_ne!(first, second);
    for (roster, expected) in [(&first_roster, first), (&second_roster, second)] {
        for character_id in roster {
            let resolved = instances
                .create_map(
                    store,
                    template.map_id,
                    entrant(*character_id, Some(7)),
                    at(1.0),
                )
                .unwrap();
            assert_eq!(resolved.map, expected);
            assert!(!resolved.created);
            assert_eq!(
                instances.player_cannot_enter(
                    store,
                    template.map_id,
                    entrant(*character_id, None),
                    90,
                    SUNDAY_NOON
                ),
                None
            );
        }
    }
    assert_eq!(
        instances.create_map(store, template.map_id, entrant(21, None), at(1.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        instances.cannot_enter(first, entrant(11, None), SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    assert!(instances.lock_copies_in_use().is_empty());
    assert!(
        instances
            .take_expired(INSTANCE_UNLOAD_DELAY_SECS + 1.0)
            .is_empty(),
        "no invented battleground reservation expiry"
    );
}

#[test]
fn a_full_stockade_copy_refuses_the_sixth_player_and_a_stranger_to_its_group() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, STOCKADE, entrant(1, Some(7)), at(0.0))
        .unwrap()
        .map;
    // MapDifficulty 8 (Normal) holds 5 players.
    for _ in 0..5 {
        instances.add_player(copy, world.spawn_empty().id());
    }
    assert_eq!(
        instances.player_cannot_enter(store, STOCKADE, entrant(6, Some(7)), 80, SUNDAY_NOON),
        Some(TransferAbortReason::MaxPlayers)
    );
    assert_eq!(
        instances.cannot_enter(copy, entrant(9, Some(8)), SUNDAY_NOON),
        Some(TransferAbortReason::MaxPlayers)
    );
}

#[test]
fn an_empty_copy_unloads_after_thirty_minutes_and_comes_back_fresh() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, STOCKADE, entrant(1, None), at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    instances.remove_player(copy, player, at(100.0));

    assert!(
        instances
            .take_expired(100.0 + INSTANCE_UNLOAD_DELAY_SECS - 1.0)
            .is_empty()
    );
    assert_eq!(
        instances.take_expired(100.0 + INSTANCE_UNLOAD_DELAY_SECS),
        vec![copy]
    );
    // The player re-enters its recent instance id as a new copy.
    let again = instances
        .create_map(store, STOCKADE, entrant(1, None), at(2000.0))
        .unwrap();
    assert_eq!(again.map, copy);
    assert!(again.created);
}

#[test]
fn world_maps_without_npc_data_are_not_hosted() {
    let Some(store) = map_store() else { return };
    let instances = InstanceManager::default();
    // Map.csv: Alliance PVP Barracks (449) is a world map without population.
    // Outland (530) now has promoted Retail NPCs and is explicitly hosted.
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (instance_type, spawns): (u32, u32) = conn
        .query_row(
            "SELECT InstanceType, (SELECT COUNT(*) FROM content_creature WHERE map=map.ID)
             FROM map WHERE ID=449",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((instance_type, spawns), (0, 0));
    for map_id in [449] {
        assert_eq!(
            instances.player_cannot_enter(store, map_id, entrant(1, None), 80, SUNDAY_NOON),
            Some(TransferAbortReason::MapNotAllowed),
            "map {map_id}"
        );
    }
    assert_eq!(
        instances.player_cannot_enter(store, 1, entrant(1, None), 1, SUNDAY_NOON),
        None
    );
}

#[test]
fn a_level_one_player_may_enter_a_dungeon_without_a_level_condition() {
    let Some(store) = map_store() else { return };
    let instances = InstanceManager::default();
    assert_eq!(
        instances.player_cannot_enter(
            store,
            CULLING_OF_STRATHOLME,
            entrant(1, None),
            1,
            SUNDAY_NOON
        ),
        None
    );
}

#[test]
fn grim_batol_offers_three_difficulties_and_the_stockade_downscales_to_normal() {
    let Some(store) = map_store() else { return };
    let heroic = store
        .downscaled_map_difficulty(GRIM_BATOL, DIFFICULTY_HEROIC)
        .unwrap();
    assert_eq!(
        (
            heroic.difficulty_id,
            heroic.reset_interval,
            heroic.is_instance_id_bound()
        ),
        (2, store::ResetInterval::Daily, true)
    );
    let mythic = store
        .downscaled_map_difficulty(GRIM_BATOL, DIFFICULTY_MYTHIC)
        .unwrap();
    assert_eq!(
        (
            mythic.difficulty_id,
            mythic.reset_interval,
            mythic.is_using_encounter_locks()
        ),
        (23, store::ResetInterval::Weekly, true)
    );
    let normal = store
        .downscaled_map_difficulty(GRIM_BATOL, store::DIFFICULTY_NORMAL)
        .unwrap();
    assert!(!normal.has_reset_schedule());
    // The Stockade has Normal only: Heroic and Mythic fall back (Mythic → Heroic → Normal).
    for difficulty in [DIFFICULTY_HEROIC, DIFFICULTY_MYTHIC] {
        assert_eq!(
            store
                .downscaled_map_difficulty(STOCKADE, difficulty)
                .unwrap()
                .difficulty_id,
            store::DIFFICULTY_NORMAL
        );
    }
    assert_eq!(store.difficulty_entries[&DIFFICULTY_MYTHIC].name, "Mythic");
    assert!(
        !store.difficulty_entries[&8].can_select(),
        "Mythic Keystone is not selectable"
    );
    assert_eq!(
        store
            .dungeon_encounter(1051, GRIM_BATOL, DIFFICULTY_HEROIC)
            .map(|e| e.bit),
        Some(3)
    );
}

#[test]
fn a_heroic_copy_is_created_at_heroic_and_normal_is_another_copy() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let heroic = instances
        .create_map(store, GRIM_BATOL, entrant_at(1, DIFFICULTY_HEROIC), at(0.0))
        .unwrap();
    // A change to Normal forgets the (unlocked) Heroic copy; even remembered, it is not entered.
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(1.0),
        )
        .unwrap();
    assert_ne!(heroic.map, normal.map);
    assert_eq!(
        instances.copy_difficulty(heroic.map).unwrap().difficulty_id,
        2
    );
    assert_eq!(
        instances.copy_difficulty(normal.map).unwrap().difficulty_id,
        1
    );
    // The Stockade at Heroic is a Normal copy.
    let stockade = instances
        .create_map(store, STOCKADE, entrant_at(1, DIFFICULTY_HEROIC), at(2.0))
        .unwrap();
    assert_eq!(
        instances
            .copy_difficulty(stockade.map)
            .unwrap()
            .difficulty_id,
        1
    );
}

#[test]
fn an_expired_lock_resets_its_empty_copy() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let heroic = entrant_at(1, DIFFICULTY_HEROIC);
    let copy = instances
        .create_map(store, GRIM_BATOL, heroic, at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    instances.update_instance_lock(copy, &[(player, 1)], 1, Some(3), SUNDAY_NOON);
    let after = instances::Now {
        elapsed: 100.0,
        unix: SUNDAY_RESET + 1,
    };
    // Occupied: the players are told, the copy stays.
    assert_eq!(instances.expire_locked_copies(after), vec![copy]);
    assert!(instances.take_expired(200.0).is_empty());
    // Empty after the reset: it unloads at once and the next entry is a fresh copy.
    instances.remove_player(copy, player, after);
    assert_eq!(instances.take_expired(101.0), vec![copy]);
    let fresh = instances
        .create_map(store, GRIM_BATOL, heroic, after)
        .unwrap();
    assert!(fresh.created);
    assert_ne!(fresh.map, copy);
    assert_eq!(instances.initial_done_bosses(fresh.map), 0);
}

#[test]
fn a_difficulty_change_resets_and_forgets_the_players_unlocked_copies() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(0.0),
        )
        .unwrap()
        .map;
    let results = instances.reset_instances(entrant_at(1, store::DIFFICULTY_NORMAL), true, at(5.0));
    assert_eq!(
        results,
        vec![(GRIM_BATOL, instances::InstanceResetResult::Success)]
    );
    assert_eq!(instances.take_expired(6.0), vec![normal]);
    assert!(
        instances
            .create_map(
                store,
                GRIM_BATOL,
                entrant_at(1, store::DIFFICULTY_NORMAL),
                at(7.0)
            )
            .unwrap()
            .created
    );
}

#[test]
fn raid_entry_legacy_raid_copy_and_weekly_bind_resume() {
    let store = map_store().expect("canonical map data");
    let mut instances = InstanceManager::default();
    let mut raider = entrant(71, None);
    raider.difficulty_id = 9; // Difficulty.db2: Molten Core 40 Player.
    assert_eq!(
        instances.player_cannot_enter(store, 409, raider, 80, SUNDAY_NOON),
        None
    );
    let copy = instances
        .create_map(store, 409, raider, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(copy).unwrap().difficulty_id, 9);
    let entries = *store.map_difficulty(409, 9).unwrap();
    assert_eq!(entries.reset_interval, store::ResetInterval::Weekly);
    let player = World::new().spawn_empty().id();
    instances.add_player(copy, player);
    assert_eq!(
        instances.update_instance_lock(copy, &[(player, 71)], 1, Some(0), SUNDAY_NOON),
        vec![player]
    );
    let saved = instances.locks.saved();
    let mut reloaded = InstanceManager::with_locks(locks::InstanceLocks::from_saved(saved));
    let resumed = reloaded
        .create_map(store, 409, raider, at(1.0))
        .unwrap()
        .map;
    assert_eq!(resumed.instance_id, copy.instance_id);
    assert_eq!(reloaded.initial_done_bosses(resumed), 1);
}

#[test]
fn raid_entry_current_requires_raid_but_legacy_allows_solo_and_resolves_separate_selections() {
    let store = map_store().unwrap();
    let instances = InstanceManager::default();
    let mut player = entrant(71, Some(7));
    assert_eq!(
        instances.player_cannot_enter(store, 2939, player, 90, SUNDAY_NOON),
        Some(TransferAbortReason::NeedGroup)
    );
    player.raid_group = true;
    // Actual pinned map condition, not a made-up map/level.
    assert_eq!(
        instances.player_cannot_enter(store, 2939, player, 90, SUNDAY_NOON),
        None
    );
    let mut instances = instances;
    player.raid_difficulty.primary = 15;
    player.raid_difficulty.legacy = 4;
    let modern = instances
        .create_map(store, 2522, player, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(modern).unwrap().difficulty_id, 15);
    let legacy = instances
        .create_map(store, 533, player, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(legacy).unwrap().difficulty_id, 4);
    let dungeon = instances
        .create_map(store, 34, player, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(dungeon).unwrap().difficulty_id, 1);
}

#[test]
fn raid_entry_weekly_reset_is_expired_at_tuesday_1500_not_a_second_later() {
    let store = map_store().unwrap();
    let entries = store.map_difficulty(409, 9).unwrap();
    let reset = locks::next_reset_time(entries.reset_interval, SUNDAY_NOON);
    let mut locks = locks::InstanceLocks::default();
    locks.create_for_new_instance(71, entries, 8, SUNDAY_NOON);
    locks.update_for_player(
        71,
        entries,
        &locks::LockUpdate {
            instance_id: 8,
            done_bosses: 1,
            instance_completed_encounters: 1,
            completed_encounter_bit: Some(0),
        },
        SUNDAY_NOON,
    );
    assert!(locks.find_active_lock(71, entries, reset - 1).is_some());
    assert!(locks.find_active_lock(71, entries, reset).is_none());
    locks.set_extended(71, entries, true, reset);
    let lock = locks.find_active_lock(71, entries, reset).unwrap();
    assert_eq!(lock.effective_expiry(entries, reset), reset + 604800);
    locks.set_extended(71, entries, false, reset);
    assert!(locks.find_active_lock(71, entries, reset).is_none());
}

#[test]
fn raid_entry_legacy_flexible_lock_uses_encounters_not_one_instance_id() {
    let store = map_store().unwrap();
    assert!(!store.map_difficulty(533, 3).unwrap().is_instance_id_bound());
    assert!(
        store
            .map_difficulty(2522, 16)
            .unwrap()
            .is_instance_id_bound()
    );
}

#[test]
fn raid_entry_extended_copy_does_not_expire_at_its_original_weekly_reset() {
    let store = map_store().unwrap();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, 409, entrant(71, None), at(0.0))
        .unwrap()
        .map;
    let player = World::new().spawn_empty().id();
    instances.add_player(copy, player);
    instances.update_instance_lock(copy, &[(player, 71)], 1, Some(0), SUNDAY_NOON);
    let entries = store.map_difficulty(409, 9).unwrap();
    let reset = locks::next_reset_time(entries.reset_interval, SUNDAY_NOON);
    instances.locks.set_extended(71, entries, true, SUNDAY_NOON);
    assert!(
        instances
            .expire_locked_copies(instances::Now {
                elapsed: 100.0,
                unix: reset
            })
            .is_empty()
    );
}

#[test]
fn raid_entry_flexible_saved_player_can_join_equal_progress_but_not_unkilled_encounters() {
    let store = map_store().unwrap();
    let entries = store.map_difficulty(533, 3).unwrap();
    let mut locks = locks::InstanceLocks::default();
    locks.create_for_new_instance(71, entries, 8, SUNDAY_NOON);
    locks.update_for_player(
        71,
        entries,
        &locks::LockUpdate {
            instance_id: 8,
            done_bosses: 1,
            instance_completed_encounters: 1,
            completed_encounter_bit: Some(0),
        },
        SUNDAY_NOON,
    );
    assert_eq!(locks.can_join(71, entries, 9, SUNDAY_NOON, 1), None);
    assert_eq!(locks.can_join(71, entries, 9, SUNDAY_NOON, 3), None);
    assert_eq!(
        locks.can_join(71, entries, 9, SUNDAY_NOON, 0),
        Some(TransferAbortReason::AlreadyCompletedEncounter)
    );
}

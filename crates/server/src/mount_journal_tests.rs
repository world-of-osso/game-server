use super::*;
use crate::spell_cast::test_harness::{Harness, drain, spawn_peer_connection, spawn_player};
use shared::components::{CombatStatus, Mounted};

fn install_connection(world: &mut World, connection: Entity) {
    world.entity_mut(connection).insert((
        MessageSender::<MountJournalUpdate>::default(),
        MessageReceiver::<MountJournalUpdate>::default(),
        MessageSender::<SummonMount>::default(),
        MessageReceiver::<SummonMount>::default(),
        MessageSender::<DismissMount>::default(),
        MessageReceiver::<DismissMount>::default(),
        MessageSender::<SetMountFavourite>::default(),
        MessageReceiver::<SetMountFavourite>::default(),
        MessageSender::<SummonRandomFavouriteMount>::default(),
        MessageReceiver::<SummonRandomFavouriteMount>::default(),
    ));
}
fn setup() -> Harness {
    let mut h = crate::auras::mount_tests::rider(150, true, &[6648]).expect("authentic mount data");
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app.insert_resource(load(&conn).unwrap());
    h.app
        .insert_resource(GameDatabase::new(crate::persistence::temp_database()));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![]));
    install_connection(h.app.world_mut(), h.connection);
    Plugin::build(&MountJournalPlugin, &mut h.app);
    h.advance(2);
    h
}
#[test]
fn mount_journal_initial_catalog_and_changed_rows_reach_account_without_repeating_catalog() {
    let mut h = setup();
    drain::<MountJournalUpdate>(&mut h.app, h.connection);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<PublishedMountSnapshot>();
    send_snapshot(h.app.world_mut(), h.connection, h.caster, None);
    h.advance(1);
    let initial = drain::<MountJournalUpdate>(&mut h.app, h.connection)
        .pop()
        .unwrap();
    assert!(initial.complete);
    assert_eq!(
        initial.mounts.len(),
        h.app.world().resource::<MountCatalog>().entries.len()
    );
    assert!(
        initial
            .mounts
            .iter()
            .any(|mount| mount.mount_id == 18 && mount.teaching_items.contains(&5655))
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    mount_store::learn_spells(db, 1, &[18]);
    broadcast_account(h.app.world_mut(), 1);
    h.advance(1);
    let learned = drain::<MountJournalUpdate>(&mut h.app, h.connection)
        .pop()
        .unwrap();
    assert!(!learned.complete);
    assert_eq!(learned.mounts.len(), 1);
    assert_eq!(
        (learned.mounts[0].mount_id, learned.mounts[0].collected),
        (18, true)
    );
    mount_store::favourite(&h.app.world().resource::<GameDatabase>().db, 1, 18, true).unwrap();
    broadcast_account(h.app.world_mut(), 1);
    h.advance(1);
    let favourite = drain::<MountJournalUpdate>(&mut h.app, h.connection)
        .pop()
        .unwrap();
    assert!(!favourite.complete);
    assert_eq!(favourite.mounts.len(), 1);
    assert!(favourite.mounts[0].favourite);
    // Character selection reuses its connection but replaces the player entity.
    let other = spawn_player(&mut h.app, 1, 0.0);
    let owned = mount_store::load(&h.app.world().resource::<GameDatabase>().db, 1);
    h.app
        .world_mut()
        .entity_mut(other)
        .insert(AccountMounts(owned));
    send_snapshot(h.app.world_mut(), h.connection, other, None);
    h.advance(1);
    assert!(
        drain::<MountJournalUpdate>(&mut h.app, h.connection)
            .iter()
            .any(|update| update.complete)
    );
}

#[test]
fn mount_journal_real_capability_types_distinguish_ground_flight_aquatic_and_passenger() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = load(&conn).unwrap();
    assert_eq!(catalog.entries[&18].journal_types, types::GROUND);
    assert_eq!(
        catalog.entries[&129].journal_types,
        types::FLYING | types::SKYRIDING
    );
    assert_eq!(catalog.entries[&373].journal_types, types::AQUATIC);
    assert_eq!(
        catalog.entries[&275].journal_types,
        types::GROUND | types::RIDE_ALONG
    );
    assert_eq!(catalog.entries[&2716].flags & 0x40, 0x40);
}

#[derive(Resource)]
struct LateEntry(Entity);

fn enter_after_mount_update(mut commands: Commands, entry: Res<LateEntry>) {
    commands.entity(entry.0).insert(InWorld { character_id: 2 });
}

#[test]
fn mount_journal_auth_entry_after_update_sync_loads_account_before_publish() {
    let mut h = setup();
    let player = spawn_player(&mut h.app, 1, 0.0);
    let connection = spawn_peer_connection(&mut h.app, player, 2);
    h.app.world_mut().entity_mut(connection).remove::<InWorld>();
    install_connection(h.app.world_mut(), connection);
    // Production authenticates and exchanges the roster before selecting a character;
    // let the HostClient channel components initialize while still out of world.
    h.advance(4);
    h.app.insert_resource(LateEntry(connection));
    h.app
        .add_systems(Update, enter_after_mount_update.after(update_mount_journal));
    h.advance(1);
    // HostClient delivers in Last of each frame and clears next frame's Last.
    let mut updates = drain::<MountJournalUpdate>(&mut h.app, connection);
    for _ in 0..4 {
        h.advance(1);
        updates.extend(drain::<MountJournalUpdate>(&mut h.app, connection));
    }
    assert!(h.app.world().get::<AccountMounts>(player).is_some());
    assert!(
        updates
            .iter()
            .any(|update| update.mounts.iter().any(|mount| mount.mount_id == 18))
    );
}

#[test]
fn mount_journal_request_at_world_entry_initializes_account_before_refusal() {
    let mut h = setup();
    let player = spawn_player(&mut h.app, 1, 0.0);
    let connection = spawn_peer_connection(&mut h.app, player, 2);
    h.app.world_mut().entity_mut(connection).remove::<InWorld>();
    install_connection(h.app.world_mut(), connection);
    h.advance(4);
    h.app
        .world_mut()
        .get_mut::<MessageSender<SummonMount>>(connection)
        .unwrap()
        .send::<CollectionChannel>(SummonMount { mount_id: 18 });
    h.advance(1);
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert(InWorld { character_id: 2 });
    h.advance(1);
    assert!(h.app.world().get::<AccountMounts>(player).is_some());
    let updates = drain::<MountJournalUpdate>(&mut h.app, connection);
    assert!(
        updates
            .iter()
            .any(|update| update.error.as_deref() == Some("You have not collected this mount."))
    );
    assert!(h.app.world().get::<Mounted>(player).is_none());
}

#[test]
fn mount_journal_random_favourite_uses_real_spell_and_rejects_combat_and_unowned_mounts() {
    let mut h = setup();
    let player = h.caster;
    assert!(
        summon(h.app.world_mut(), player, 18)
            .unwrap_err()
            .contains("not collected")
    );
    assert!(random_candidates(h.app.world(), player).is_empty());
    let db = &h.app.world().resource::<GameDatabase>().db;
    mount_store::learn_spells(db, 1, &[18]);
    mount_store::favourite(db, 1, 18, true).unwrap();
    broadcast_account(h.app.world_mut(), 1);
    assert_eq!(random_candidates(h.app.world(), player), vec![18]);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(CombatStatus(true));
    assert!(summon(h.app.world_mut(), player, 18).is_err());
    h.advance(35);
    assert!(h.app.world().get::<Mounted>(player).is_none());
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(CombatStatus(false));
    h.app
        .world_mut()
        .get_mut::<MessageSender<SummonRandomFavouriteMount>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(SummonRandomFavouriteMount);
    h.advance(40);
    assert_eq!(
        h.app
            .world()
            .get::<Mounted>(player)
            .unwrap()
            .mount_display_id,
        2405
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(player)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 6648)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<DismissMount>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(DismissMount);
    h.advance(4);
    assert!(h.app.world().get::<Mounted>(player).is_none());
}
#[test]
fn mount_journal_spell_learning_and_favourites_publish_only_to_same_account() {
    let mut h = setup();
    let same_player = spawn_player(&mut h.app, 1, 0.0);
    let same = spawn_peer_connection(&mut h.app, same_player, 2);
    let foreign_player = spawn_player(&mut h.app, 1, 0.0);
    let foreign = spawn_peer_connection(&mut h.app, foreign_player, 3);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(Authenticated { account_id: 2 });
    install_connection(h.app.world_mut(), same);
    install_connection(h.app.world_mut(), foreign);
    h.advance(4);
    h.app
        .world_mut()
        .get_mut::<crate::profession::PlayerProfessions>(h.caster)
        .unwrap()
        .spells
        .push(6648);
    h.advance(4);
    assert!(knows_spell(h.app.world(), same_player, 6648));
    assert!(!knows_spell(h.app.world(), foreign_player, 6648));
    h.app
        .world_mut()
        .get_mut::<MessageSender<SetMountFavourite>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(SetMountFavourite {
            mount_id: 18,
            favourite: true,
        });
    let mut same_updates = Vec::new();
    let mut foreign_updates = Vec::new();
    for _ in 0..7 {
        h.advance(1);
        same_updates.extend(drain::<MountJournalUpdate>(&mut h.app, same));
        foreign_updates.extend(drain::<MountJournalUpdate>(&mut h.app, foreign));
    }
    assert!(same_updates.iter().any(|update| {
        update
            .mounts
            .iter()
            .any(|mount| mount.mount_id == 18 && mount.collected && mount.favourite)
    }));
    assert!(foreign_updates.iter().all(|update| {
        update
            .mounts
            .iter()
            .all(|mount| !mount.collected && !mount.favourite)
    }));
}

#[test]
fn relog_crash_retires_old_world_link_before_mount_journal_next_frame() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameDatabase::new(crate::persistence::temp_database()));
    app.init_resource::<MountCatalog>();
    let old_player = app.world_mut().spawn(crate::persistence::ClientId(32)).id();
    let new_player = app.world_mut().spawn(crate::persistence::ClientId(32)).id();
    let old_connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 3 },
            InWorld { character_id: 32 },
            PlayerLink(old_player),
        ))
        .id();
    let new_connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 3 },
            InWorld { character_id: 32 },
            PlayerLink(new_player),
        ))
        .id();
    install_connection(app.world_mut(), old_connection);
    install_connection(app.world_mut(), new_connection);
    app.add_systems(Update, update_mount_journal);
    // Selection retires the old player while its transport is still delivering
    // ForcedDisconnect. Network retirement removes the player in PostUpdate.
    app.add_systems(
        PostUpdate,
        move |mut commands: Commands, mut retired: Local<bool>| {
            if !*retired {
                crate::networking::queue_player_despawn(&mut commands, old_player);
                *retired = true;
            } else {
                commands.entity(old_player).try_despawn();
            }
        },
    );
    app.update();
    app.update();
    // The next real exclusive reader must not dereference a dead player link.
    app.update();
    assert!(app.world().get_entity(old_player).is_err());
    assert!(app.world().get::<PlayerLink>(old_connection).is_none());
    assert!(app.world().get::<InWorld>(old_connection).is_none());
    assert_eq!(
        app.world().get::<PlayerLink>(new_connection).unwrap().0,
        new_player
    );
    assert!(app.world().get::<AccountMounts>(new_player).is_some());
    assert!(app.world().get::<InWorld>(new_connection).is_some());
}

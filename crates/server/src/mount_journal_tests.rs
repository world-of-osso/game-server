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
    h.app.insert_resource(GameDatabase {
        db: crate::persistence::temp_database(),
    });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![]));
    install_connection(h.app.world_mut(), h.connection);
    h.app.add_systems(
        Update,
        (
            sync_accounts,
            receive_favourites,
            receive_summons,
            receive_dismissals,
        )
            .chain(),
    );
    h.app.add_systems(PostUpdate, publish_mount_changes);
    h.advance(2);
    h
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

//! Recipient-scoped Mythic Mentor data, backed by persistent bag ownership.
use super::*;
use crate::group::GroupRegistry;

#[derive(Resource, Default)]
pub(super) struct PublishedKeys {
    next_refresh: f64,
    sent: HashMap<Entity, PartyKeystoneSnapshot>,
}

fn load_party_snapshot(world: &World, character: u64) -> PartyKeystoneSnapshot {
    let registry = world.resource::<GroupRegistry>();
    if registry.is_in_raid(character) {
        return PartyKeystoneSnapshot::default();
    }
    let mut members = registry.group_members(character);
    members.retain(|id| *id != character);
    members.insert(0, character);
    let leader = registry.leader_of(character);
    let db = &world.resource::<GameDatabase>().db;
    let summaries = &world.resource::<Challenges>().summaries;
    let members = members
        .into_iter()
        .map(|id| {
            let (name, class) = load_member_identity(db, registry, id);
            let key = owned_keys(db, id).into_iter().find_map(|key| {
                let summary = summaries.get(&key.challenge_id)?;
                (key.level > 0).then(|| PartyKeystone {
                    challenge_id: key.challenge_id,
                    level: key.level,
                    dungeon_name: summary.name.clone(),
                    time_limit_ms: summary.time_limit_ms,
                })
            });
            PartyKeystoneMember {
                character_id: id,
                name,
                class,
                is_leader: leader == Some(id),
                key,
            }
        })
        .collect();
    PartyKeystoneSnapshot { members }
}

fn load_member_identity(db: &redb::Database, registry: &GroupRegistry, id: u64) -> (String, u8) {
    if registry.group_id_of(id).is_some() {
        let (name, class) = registry
            .member_identity(id)
            .expect("group membership has a roster identity");
        return (name.to_owned(), class);
    }
    let data =
        persistence::load_character(db, id).expect("solo recipient is a persisted character");
    (data.name, data.class)
}

pub(super) fn send_changed_party_keys(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed_secs_f64();
    let mut published = world.resource_mut::<PublishedKeys>();
    if elapsed < published.next_refresh {
        return;
    }
    published.next_refresh = elapsed + 1.0;
    drop(published);
    let connections: Vec<_> = world
        .query_filtered::<(Entity, &InWorld), With<Authenticated>>()
        .iter(world)
        .map(|(entity, entry)| (entity, entry.character_id))
        .collect();
    world
        .resource_mut::<PublishedKeys>()
        .sent
        .retain(|entity, _| connections.iter().any(|(connected, _)| connected == entity));
    for (connection, character) in connections {
        let snapshot = load_party_snapshot(world, character);
        let changed = world.resource::<PublishedKeys>().sent.get(&connection) != Some(&snapshot);
        if changed {
            let mut sender = world
                .get_mut::<MessageSender<PartyKeystoneSnapshot>>(connection)
                .expect("authenticated connection has registered party keystone sender");
            sender.send::<InstanceChannel>(snapshot.clone());
            world
                .resource_mut::<PublishedKeys>()
                .sent
                .insert(connection, snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::{OnlineMember, OnlineView};

    fn put_key(h: &crate::bank_test_harness::Harness, owner: u64, guid: u64, level: u32) {
        let mut inventory = persistence::load_inventory(h.db(), owner);
        inventory.items.push(persistence::InventoryItemData {
            item_guid: guid,
            item_id: KEYSTONE_ITEM,
            name: "Mythic Keystone".into(),
            quality: 4,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 0,
            unique_auction: false,
        });
        persistence::save_inventory(h.db(), owner, &inventory);
        save_key(
            h.db(),
            &KeystoneInfo {
                item_guid: guid,
                item_id: KEYSTONE_ITEM,
                challenge_id: 401,
                level,
                affixes: vec![],
            },
        );
    }

    #[test]
    fn mythic_mentor_catalog_keys_are_not_misreported_as_no_key() {
        let mut h = crate::bank_test_harness::harness().unwrap();
        h.app.init_resource::<GroupRegistry>();
        let path = std::env::var("MYTHIC_PLUS_WORLD_COPY").unwrap();
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        h.app.insert_resource(Challenges {
            definition: load_azure(&conn).unwrap(),
            summaries: load_summaries(&conn).unwrap(),
            ..Default::default()
        });
        let txn = h.db().begin_write().unwrap();
        txn.open_table(KEYS).unwrap();
        txn.commit().unwrap();
        let owner = h.character_id;
        put_key(&h, owner, 4242, 8);
        // Installed BossHelper MythicTeleport.lua's eight maps, plus existing Azure401.
        for (challenge_id, name, time_limit_ms) in [
            (249, "Kings' Rest", 1_980_000),
            (250, "Temple of Sethraliss", 1_920_000),
            (399, "Ruby Life Pools", 1_680_000),
            (401, "The Azure Vault", 2_250_000),
            (584, "The Blinding Vale", 1_800_000),
            (585, "Voidscar Arena", 1_800_000),
            (586, "Den of Nalorakk", 1_920_000),
            (587, "Murder Row", 2_040_000),
            (588, "Altar of Fangs", 1_800_000),
        ] {
            save_key(
                h.db(),
                &KeystoneInfo {
                    item_guid: 4242,
                    item_id: KEYSTONE_ITEM,
                    challenge_id,
                    level: 8,
                    affixes: vec![],
                },
            );
            let snapshot = load_party_snapshot(h.app.world(), owner);
            let key = snapshot.members[0]
                .key
                .as_ref()
                .expect("Real bag key must be shared");
            assert_eq!((key.challenge_id, key.level), (challenge_id, 8));
            assert_eq!(
                (key.dungeon_name.as_str(), key.time_limit_ms),
                (name, time_limit_ms)
            );
        }
    }

    #[test]
    fn mythic_mentor_runtime_companions_have_no_key() {
        use shared::group::GroupRole;
        let mut h = crate::bank_test_harness::harness().unwrap();
        let owner = h.character_id;
        let data = persistence::load_character(h.db(), owner).unwrap();
        let mut online = OnlineView::from([(
            owner,
            OnlineMember {
                entity: h.player.to_bits(),
                name: data.name,
                class: data.class,
                level: 1,
                portrait: Default::default(),
                dungeon_difficulty: 205,
                raid_difficulty: Default::default(),
            },
        )]);
        let profiles = [
            ("Captain Garrick", 2, GroupRole::Tank),
            ("Crenna Earth-Daughter", 11, GroupRole::Healer),
            ("Meredy Huntswell", 8, GroupRole::Dps),
            ("Shuja Grimaxe", 7, GroupRole::Dps),
        ];
        let companions: Vec<_> = profiles
            .iter()
            .map(|(name, class, role)| {
                let entity = h.app.world_mut().spawn_empty().id();
                let id = crate::followers::Follower::member_id(entity);
                online.insert(
                    id,
                    OnlineMember {
                        entity: entity.to_bits(),
                        name: (*name).into(),
                        class: *class,
                        level: 80,
                        portrait: Default::default(),
                        dungeon_difficulty: 205,
                        raid_difficulty: Default::default(),
                    },
                );
                (id, *role)
            })
            .collect();
        let txn = h.db().begin_write().unwrap();
        txn.open_table(KEYS).unwrap();
        txn.commit().unwrap();
        h.app.init_resource::<GroupRegistry>();
        let path = std::env::var("MYTHIC_PLUS_WORLD_COPY").unwrap();
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        h.app.insert_resource(Challenges {
            definition: load_azure(&conn).unwrap(),
            summaries: load_summaries(&conn).unwrap(),
            ..Default::default()
        });
        put_key(&h, owner, 4242, 8);
        h.app
            .world_mut()
            .resource_mut::<GroupRegistry>()
            .create_companion_party(owner, GroupRole::Dps, &companions, &online)
            .unwrap();
        let snapshot = load_party_snapshot(h.app.world(), owner);
        assert_eq!(snapshot.members.len(), 5);
        assert_eq!(snapshot.members[0].character_id, owner);
        assert!(snapshot.members[0].is_leader);
        assert_eq!(snapshot.members[0].key.as_ref().unwrap().level, 8);
        for (row, (name, class, _)) in snapshot.members[1..].iter().zip(profiles) {
            assert_eq!((row.name.as_str(), row.class), (name, class));
            assert!(
                row.key.is_none(),
                "Runtime companions do not own persistent bag keys"
            );
        }
    }

    #[test]
    fn mythic_mentor_party_keys_follow_bags_roster_leadership_and_recipient() {
        let mut h = crate::bank_test_harness::harness().unwrap();
        let owner = h.character_id;
        let peer = h.create_alt("Mentorpeer");
        let outsider = h.create_alt("Outsider");
        let txn = h.db().begin_write().unwrap();
        txn.open_table(KEYS).unwrap();
        txn.commit().unwrap();
        let path = std::env::var("MYTHIC_PLUS_WORLD_COPY").unwrap();
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        h.app.insert_resource(Challenges {
            definition: load_azure(&conn).unwrap(),
            summaries: load_summaries(&conn).unwrap(),
            ..Default::default()
        });
        h.app.init_resource::<GroupRegistry>();
        put_key(&h, owner, 4242, 8);
        put_key(&h, peer, 4243, 11);
        put_key(&h, outsider, 4244, 20);
        let online: OnlineView = [owner, peer, outsider]
            .into_iter()
            .map(|id| {
                let data = persistence::load_character(h.db(), id).unwrap();
                (
                    id,
                    OnlineMember {
                        name: data.name,
                        class: data.class,
                        entity: id,
                        level: u8::try_from(data.level).unwrap(),
                        portrait: Default::default(),
                        dungeon_difficulty: 1,
                        raid_difficulty: Default::default(),
                    },
                )
            })
            .collect();
        {
            let mut registry = h.app.world_mut().resource_mut::<GroupRegistry>();
            registry.sync_presence(&online);
            registry.invite(owner, "Mentorpeer", &online);
            registry.respond_invite(peer, true);
        }
        let snapshot = load_party_snapshot(h.app.world(), owner);
        assert_eq!(
            snapshot
                .members
                .iter()
                .map(|m| m.character_id)
                .collect::<Vec<_>>(),
            vec![owner, peer]
        );
        assert!(snapshot.members[0].is_leader);
        assert_eq!(snapshot.members[0].key.as_ref().unwrap().level, 8);
        let key = snapshot.members[1].key.as_ref().unwrap();
        assert_eq!(
            (key.level, key.dungeon_name.as_str(), key.time_limit_ms),
            (11, "The Azure Vault", 2250000)
        );
        assert_eq!(
            load_party_snapshot(h.app.world(), peer).members[0].character_id,
            peer
        );
        assert_eq!(
            load_party_snapshot(h.app.world(), outsider).members.len(),
            1
        );
        let mut inventory = persistence::load_inventory(h.db(), peer);
        inventory.items.retain(|item| item.item_guid != 4243);
        persistence::save_inventory(h.db(), peer, &inventory);
        assert!(
            load_party_snapshot(h.app.world(), owner).members[1]
                .key
                .is_none(),
            "Persistent key record without bag item must not be shared"
        );
        h.app
            .world_mut()
            .resource_mut::<GroupRegistry>()
            .promote(owner, "Mentorpeer");
        assert!(load_party_snapshot(h.app.world(), owner).members[1].is_leader);
        h.app
            .world_mut()
            .resource_mut::<GroupRegistry>()
            .convert_to_raid(peer);
        assert!(load_party_snapshot(h.app.world(), owner).members.is_empty());
        h.app
            .world_mut()
            .resource_mut::<GroupRegistry>()
            .leave(owner);
        assert_eq!(load_party_snapshot(h.app.world(), owner).members.len(), 1);
    }
}

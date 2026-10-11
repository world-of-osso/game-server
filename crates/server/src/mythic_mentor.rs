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
    let definition = world.resource::<Challenges>().definition.as_ref();
    let members = members
        .into_iter()
        .map(|id| {
            let data = persistence::load_character(db, id).expect("group member character exists");
            let key = definition.and_then(|definition| {
                owned_keys(db, id)
                    .into_iter()
                    .find(|key| key.challenge_id == definition.challenge_id && key.level > 0)
                    .map(|key| PartyKeystone {
                        challenge_id: key.challenge_id,
                        level: key.level,
                        dungeon_name: definition.name.clone(),
                        time_limit_ms: definition.timers_ms[0],
                    })
            });
            PartyKeystoneMember {
                character_id: id,
                name: data.name,
                class: data.class,
                is_leader: leader == Some(id),
                key,
            }
        })
        .collect();
    PartyKeystoneSnapshot { members }
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
                    },
                )
            })
            .collect();
        {
            let mut registry = h.app.world_mut().resource_mut::<GroupRegistry>();
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

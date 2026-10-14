//! TC a352b1fa SpellEffects.cpp1890-1904: solo group-private summons retain
//! the caster GUID, not public visibility. Real learned UseToy path.
use super::*;

#[test]
fn toyfx28_group_private_source_summons_learn_use_and_expire() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = import_toyfx8_world(directory.path());
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, duration, count) in [
        (112324, 162979, 80682, 180000, 1),
        (120857, 179001, 125119, 180000, 1),
        (130214, 195753, 99535, 60000, 1),
        (142536, 231375, 116213, 180000, 1),
        (151877, 243248, 125120, 180000, 1),
        (160740, 273478, 139198, 45000, 5),
        (160751, 273491, 139204, 30000, 1),
        (163745, 280084, 143703, 30000, 1),
        (166704, 289536, 149954, 300000, 1),
        (174920, 317204, 163441, 120000, 1),
        (199894, 388259, 199054, 60000, 1),
        (229828, 465643, 231372, 300000, 1),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("group-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app
            .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
        h.app
            .insert_resource(crate::world_data::load_world_data(&path));
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        h.app.insert_resource(store.clone());
        learn_group_toy(&mut h, character, item);
        let destination = matches!(item, 160751 | 199894).then(|| {
            let origin = h.app.world().get::<Position>(h.caster).unwrap();
            [origin.x + 2.0, origin.y, origin.z]
        });
        assert_eq!(
            activate_at(&mut h, item, destination)[0].error,
            None,
            "item {item}"
        );
        h.advance(100);
        assert!(
            h.inbox.failed.is_empty(),
            "item {item}: {:?}",
            h.inbox.failed
        );
        assert!(
            h.inbox.go.iter().any(|go| go.spell_id == spell),
            "item {item}"
        );
        let world = h.app.world_mut();
        let actors: Vec<_> = world
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, npc, summon)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(actors.len(), count, "item {item}");
        for actor in &actors {
            assert!(world.get::<ModelDisplay>(*actor).unwrap().display_id > 0);
            assert_eq!(
                world.get::<UnitSummonedBy>(*actor).unwrap().0,
                h.caster.to_bits()
            );
            assert_eq!(
                world
                    .get::<crate::interest::PrivateObjectOwner>(*actor)
                    .unwrap()
                    .0,
                h.caster
            );
            assert_eq!(
                world.get::<crate::maps::InMap>(*actor),
                world.get::<crate::maps::InMap>(h.caster)
            );
            let expiry = world
                .get::<SummonedCreature>(*actor)
                .unwrap()
                .expires_at
                .unwrap();
            let remaining = expiry - world.resource::<Time>().elapsed_secs_f64();
            assert!(
                remaining > duration as f64 / 1000.0 - 10.0
                    && remaining <= duration as f64 / 1000.0
            );
        }
        let remaining = world
            .get::<SummonedCreature>(actors[0])
            .unwrap()
            .expires_at
            .unwrap()
            - world.resource::<Time>().elapsed_secs_f64();
        h.app
            .world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs_f64(remaining + 0.2));
        h.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(remaining + 0.2),
            ));
        h.advance(1);
        for actor in actors {
            assert!(
                h.app.world().get_entity(actor).is_err(),
                "item {item} did not expire"
            );
        }
    }
}

pub(super) fn learn_group_toy(h: &mut Harness, character: u64, item_id: u32) {
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::save_inventory(
        db,
        character,
        &InventoryData {
            gold: 0,
            items: vec![InventoryItemData {
                item_guid: persistence::next_id(db, "item_instance"),
                item_id,
                name: String::new(),
                quality: 1,
                required_level: 1,
                stack_count: 1,
                vendor_sell_price: 0,
                unique_auction: false,
            }],
        },
    );
    let mut results = learn(h, 0);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while results.is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
        results.extend(pump(h).0);
    }
    assert!(
        results
            .iter()
            .any(|result| result.operation == ToyOperation::Learn && result.error.is_none()),
        "physical toy {item_id} did not learn: {results:?}; errors: {:?}",
        drain::<InventoryError>(&mut h.app, h.connection)
    );
}

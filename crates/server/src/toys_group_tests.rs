//! TC a352b1fa SpellEffects.cpp1890-1904: solo group-private summons retain
//! the caster GUID, not public visibility. Real learned UseToy path.
use super::*;

#[path = "toys_group_cases.rs"]
mod source_cases;

fn assert_group_source_summon(item: u32, spell: u32, entry: u32, duration: u32, count: usize) {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    let class = if matches!(item, 120857 | 151877) {
        4
    } else {
        8
    };
    let (mut h, character) = setup_for_race_class(
        &directory.path().join(format!("group-{item}.redb")),
        1,
        class,
    );
    // Guardian properties require the actual destination's regional level source.
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    h.app.insert_resource(store.clone());
    let source_recovery = h.app.world().resource::<ToyCatalog>().0[&item].item_cooldown_ms;
    let owner_health = *h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    let owner_display = h.app.world().get::<ModelDisplay>(h.caster).copied();
    learn_group_toy(&mut h, character, item);
    let destination = (item == 160751).then(|| {
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
        // Native properties3535/4799 carry JoinSummonerSpawnGroup0x200.
        if matches!(item, 112324 | 120857 | 142536 | 151877 | 229828) {
            assert!(
                world
                    .get::<shared::components::UnitFlags>(*actor)
                    .is_some_and(
                        |flags| flags.0 & crate::faction::UNIT_FLAG_PLAYER_CONTROLLED != 0
                    ),
                "native player-owned spawn-group minion flag for {item}"
            );
            assert_eq!(
                world
                    .get::<crate::creature_ai::FollowOwner>(*actor)
                    .unwrap()
                    .owner,
                h.caster,
                "source spawn-group follow designation for {item}"
            );
        }
        assert!(
            !world
                .get::<shared::components::UnitIsPet>(*actor)
                .is_some_and(|pet| pet.0)
        );
        if item == 163745 {
            assert_eq!(
                world
                    .get::<shared::components::UnitScale>(*actor)
                    .unwrap()
                    .0,
                0.75
            );
        }
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
            remaining > duration as f64 / 1000.0 - 10.0 && remaining <= duration as f64 / 1000.0
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
    assert_eq!(
        h.app.world().get::<shared::components::Health>(h.caster),
        Some(&owner_health)
    );
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        owner_display
    );
    if source_recovery > duration as i32 + 10000 {
        assert_eq!(
            activate_at(&mut h, item, destination)[0].error.as_deref(),
            Some("OnCooldown")
        );
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
    let results = learn(h, 0);
    let owned = toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1);
    let inventory =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character);
    assert!(
        results
            .iter()
            .any(|result| result.operation == ToyOperation::Learn && result.error.is_none()),
        "physical toy {item_id} did not learn: {results:?}; owned: {owned:?}; inventory: {inventory:?}; errors: {:?}; spell failures: {:?}",
        h.inbox.inventory_errors,
        h.inbox.failed
    );
}

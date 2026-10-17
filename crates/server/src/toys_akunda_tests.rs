//! Akunda84: actual source root/child cursor chain and real linked Cozy Fire/cooking.
use super::*;

fn install_cooking_recipe_and_meat(h: &mut Harness, character: u64, conn: &rusqlite::Connection) {
    h.app
        .insert_resource(crate::profession::load_profession_data(conn).unwrap());
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(conn).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::persistence::ClientId(character),
        shared::components::UnitScale(1.0),
        crate::profession::PlayerProfessions {
            lines: vec![],
            spells: vec![2538],
        },
    ));
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::save_inventory(
        db,
        character,
        &InventoryData {
            gold: 0,
            items: vec![InventoryItemData {
                item_guid: persistence::next_id(db, "item_instance"),
                item_id: 2672,
                name: "Stringy Wolf Meat".into(),
                quality: 1,
                required_level: 1,
                stack_count: 2,
                vendor_sell_price: 0,
                unique_auction: false,
            }],
        },
    );
}

#[test]
fn toyfx84_akunda_real_learning_child_destination_visible_trap_cooking_and_expiry() {
    use crate::auras::ActiveAuras;
    use shared::components::Position;
    use shared::protocol::GameObjectInfo;
    const ITEM: u32 = 163211;
    const ROOT: u32 = 279076;
    const CHILD: u32 = 279077;
    const COZY_FIRE: u32 = 7353;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("akunda.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    learn_effect_toy(&mut h, character, ITEM);
    install_cooking_recipe_and_meat(&mut h, character, &conn);
    let caster = h.caster;
    let before =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character);
    let refused = crate::spell_cast::cast_for_player(h.app.world_mut(), caster, 2538);
    assert!(
        matches!(refused, Err(crate::spell_cast::CastFailure::Unsupported(ref reason)) if reason == "Requires Cooking Fire")
    );
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character),
        before
    );
    let origin = *h.app.world().get::<Position>(caster).unwrap();
    let point = [origin.x + 2.0, origin.y, origin.z];
    assert_eq!(
        activate_at(&mut h, ITEM, Some(point))[0].error,
        None,
        "actual Akunda learned-use"
    );
    h.advance(60); // Source2s cast completes; triggered child ignores its own cast time.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == ROOT));
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == ROOT && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, 300_000);
    let world = h.app.world_mut();
    let mut objects: Vec<_> = world
        .query::<(Entity, &GameObjectInfo)>()
        .iter(world)
        .filter(|(_, info)| [296450, 296451, 302746].contains(&info.entry))
        .map(|(entity, info)| (info.entry, entity, info.go_type, info.display_id))
        .collect();
    objects.sort_by_key(|object| object.0);
    assert_eq!(
        objects
            .iter()
            .map(|object| (object.0, object.2, object.3))
            .collect::<Vec<_>>(),
        [(296450, 8, 51512), (296451, 5, 45521), (302746, 6, 16675)]
    );
    let fire = objects[0].1;
    let trap = objects[2].1;
    let fire_position = *world.get::<Position>(fire).unwrap();
    assert!((fire_position.x - point[0]).abs() < 0.01);
    assert!((fire_position.z - point[2]).abs() < 0.01);
    assert_eq!(world.get::<Position>(trap), Some(&fire_position));
    for (entry, object, _, _) in &objects[..2] {
        let life = world
            .get::<crate::game_objects::summoned::SummonedObject>(*object)
            .unwrap();
        assert_eq!(life.owner, Some(caster));
        assert_eq!(life.spell_id, if *entry == 296450 { CHILD } else { ROOT });
        let remaining = life.expires_at.unwrap() - crate::spell_cast::now(world);
        assert!((295.0..=300.0).contains(&remaining));
    }
    let aura = world
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == COZY_FIRE)
        .unwrap();
    assert_eq!(
        aura.caster, trap,
        "environmental trap, not owner, casts Cozy Fire"
    );
    assert_eq!(aura.effects[0].amount, 1.0);
    assert!(crate::spell_cast::cast_for_player(h.app.world_mut(), caster, 2538).is_ok());
    h.advance(100);
    let cooked =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character);
    assert_eq!(
        cooked
            .items
            .iter()
            .find(|item| item.item_id == 2672)
            .unwrap()
            .stack_count,
        1
    );
    assert_eq!(
        cooked
            .items
            .iter()
            .find(|item| item.item_id == 2679)
            .unwrap()
            .stack_count,
        1
    );
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = Position {
        x: fire_position.x + 30.0,
        ..fire_position
    };
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == COZY_FIRE,
    );
    h.advance(100);
    assert!(
        !h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == COZY_FIRE)
    );
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = origin;
    h.advance(100);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == COZY_FIRE)
    );
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = Position {
        x: fire_position.x + 30.0,
        ..fire_position
    };
    let remaining = h
        .app
        .world()
        .get::<crate::game_objects::summoned::SummonedObject>(fire)
        .unwrap()
        .expires_at
        .unwrap()
        - crate::spell_cast::now(h.app.world());
    h.advance((remaining / 0.05).ceil() as usize + 2);
    for (_, object, _, _) in objects {
        assert!(h.app.world().get_entity(object).is_err());
    }
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = origin;
    assert!(crate::spell_cast::cast_for_player(h.app.world_mut(), caster, 2538).is_err());
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character),
        cooked
    );
}

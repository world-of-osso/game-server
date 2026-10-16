//! Direct grant fixtures exercise the same physical-item transaction and receipt publisher.
use super::*;

pub(crate) fn learn_companion_fixture(
    world: &mut World,
    player: Entity,
    spell: u32,
    level: u8,
) -> Result<bool, String> {
    let Some(species) = world.resource::<PetCatalog>().spells.get(&spell).copied() else {
        return Ok(false);
    };
    let account = world.get::<AccountPetJournal>(player).unwrap().account_id;
    let (connection, character) = find_or_spawn_fixture_session(world, player, account);
    world.init_resource::<PendingCompanionLearning>();
    world.init_resource::<crate::gold::GoldUpdateQueue>();
    if !world.contains_resource::<crate::item_templates::ItemTemplates>() {
        world.insert_resource(
            crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap(),
        );
    }
    let db = &world.resource::<GameDatabase>().db;
    let guid = crate::persistence::next_id(db, "item_instance");
    let mut inventory = crate::persistence::load_inventory(db, character);
    let slot = inventory.items.len() as u8;
    // Internal transaction fixture; native UseItem tests separately prove spell/species resolution.
    inventory.items.push(crate::persistence::InventoryItemData {
        item_guid: guid,
        item_id: 4401,
        name: String::new(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    crate::persistence::save_inventory(db, character, &inventory);
    enqueue(
        world,
        LearningRequest {
            connection,
            player,
            account,
            character,
            location: ItemLocation::Bag { bag: 0, slot },
            item: 4401,
            guid,
            species,
            level,
            retail_item: None,
        },
    );
    let receipt = world
        .resource::<PendingCompanionLearning>()
        .0
        .back()
        .unwrap()
        .receipt
        .clone();
    let (done, wait) = std::sync::mpsc::channel();
    world
        .resource::<GameDatabase>()
        .writer
        .enqueue("companion grant fixture barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
    let result = receipt.result().unwrap();
    publish_learning(world);
    result?.map_err(|error| format!("{error:?}"))?;
    Ok(true)
}

fn find_or_spawn_fixture_session(world: &mut World, player: Entity, account: u64) -> (Entity, u64) {
    let mut connections = world.query::<(Entity, &PlayerLink, &InWorld)>();
    if let Some((connection, _, entered)) = connections
        .iter(world)
        .find(|(_, link, _)| link.0 == player)
    {
        return (connection, entered.character_id);
    }
    let suffix = char::from(b'a' + (account % 26) as u8);
    let (character, _) = crate::persistence::create_character(
        &world.resource::<GameDatabase>().db,
        account,
        &format!("Pet{suffix}"),
        1,
        8,
    )
    .unwrap();
    let connection = world
        .spawn((
            Authenticated {
                account_id: account,
            },
            InWorld {
                character_id: character,
            },
            PlayerLink(player),
        ))
        .id();
    (connection, character)
}

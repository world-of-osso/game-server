//! Shared native ItemSet predicates: real memberships, flags and skill thresholds.
use super::*;
use shared::protocol::EquipmentSlot;

fn native_world() -> World {
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut world = World::new();
    world.insert_resource(load(&conn).unwrap());
    world.insert_resource(
        crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap(),
    );
    world
}

fn equip(world: &mut World, items: &[(EquipmentSlot, u32)]) -> Entity {
    world
        .spawn(crate::stats::EquippedItems {
            items: items
                .iter()
                .map(|&(slot, id)| (slot, ItemDefinitionSource::Retail, id))
                .collect(),
        })
        .id()
}

#[test]
#[ignore = "native ItemSparse membership counterexample; no invented catalog rows"]
fn native_item_set_predicate_counts_real_members_outside_the_display_array() {
    let mut world = native_world();
    // TC GetItemSet uses ItemSparse.ItemSet. Both Dark Arctic pieces belong to
    //754, but ItemSet754.ItemID_0..16 list the original Arctic pieces instead.
    let actor = equip(
        &mut world,
        &[(EquipmentSlot::Legs, 38591), (EquipmentSlot::Chest, 38592)],
    );
    assert_eq!(has_equipped_bonus(&world, actor, 51273), Ok(true));
    world
        .get_mut::<crate::stats::EquippedItems>(actor)
        .unwrap()
        .items
        .truncate(1);
    assert_eq!(has_equipped_bonus(&world, actor, 51273), Ok(false));
}

#[test]
#[ignore = "native legacy-inactive set must be disabled, not an unsupported predicate"]
fn native_item_set_predicate_disables_the_real_legacy_arcanist_set() {
    let mut world = native_world();
    let actor = equip(
        &mut world,
        &[
            (EquipmentSlot::Waist, 16802),
            (EquipmentSlot::Wrist, 16799),
            (EquipmentSlot::Head, 16795),
        ],
    );
    // Native201 flags1; ItemSetSpell2452 requires3 pieces for41657.
    assert_eq!(has_equipped_bonus(&world, actor, 41657), Ok(false));
}

#[test]
#[ignore = "native Blood Tiger skill165/rank300 source boundary"]
fn native_item_set_predicate_tracks_native_profession_requirement_changes() {
    let mut world = native_world();
    let actor = equip(
        &mut world,
        &[
            (EquipmentSlot::Chest, 19688),
            (EquipmentSlot::Shoulder, 19689),
        ],
    );
    let results: Vec<_> = [None, Some(299), Some(300), None]
        .into_iter()
        .map(|rank| {
            if let Some(rank) = rank {
                world
                    .entity_mut(actor)
                    .insert(crate::profession::PlayerProfessions {
                        lines: vec![shared::profession::ProfessionSkillLine {
                            skill_line: 165,
                            step: 1,
                            rank,
                            max_rank: 300,
                        }],
                        spells: Vec::new(),
                    });
            } else {
                world
                    .entity_mut(actor)
                    .remove::<crate::profession::PlayerProfessions>();
            }
            // Native ItemSet442 / ItemSetSpell1883: two pieces, Leatherworking300.
            has_equipped_bonus(&world, actor, 41670)
        })
        .collect();
    assert_eq!(results, vec![Ok(false), Ok(false), Ok(true), Ok(false)]);
}

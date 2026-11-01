use super::*;

#[test]
fn keystone_wcl_profile_equips_qualified_items_and_talents() {
    let base = super::super::super::class_profiles::read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == 250)
        .unwrap();
    let selected = class_profile(&base, 11).unwrap();
    assert_eq!(
        selected.talent_source().unwrap(),
        crate::trait_config::TalentSource::Wcl
    );
    assert_eq!(selected.race, 37);
    assert_eq!(selected.race_source, "RACE_FROM_SIMC_PROFILE");
    assert!(
        selected
            .talent_entries
            .as_ref()
            .unwrap()
            .contains(&(96167, 1))
    );
    let mut world = World::new();
    let actor = world.spawn_empty().id();
    equip(&mut world, actor, 250, 11).unwrap();
    let equipment = world.get::<crate::stats::EquippedItems>(actor).unwrap();
    assert!(equipment.items.contains(&(
        EquipmentSlot::Head,
        shared::item_data::ItemDefinitionSource::Retail,
        271474
    )));
    let modifiers = world
        .get::<crate::stats::EquippedItemModifiers>(actor)
        .unwrap();
    let head = &modifiers
        .items
        .iter()
        .find(|(slot, _)| *slot == EquipmentSlot::Head)
        .unwrap()
        .1;
    assert_eq!(head.enchant, 7991);
    assert!(head.bonus_list.contains(&13848));
    assert_eq!(head.gems, vec![240910]);
    assert_eq!(
        modifiers.temporary_enchants,
        vec![(EquipmentSlot::MainHand, 8052)]
    );
    assert!(find_profile(250, 1).is_err());
    assert!(find_profile(u32::MAX, 11).is_err());
}

#[test]
fn keystone_profiles_include_native_wcl_only_specializations() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let classes = crate::spell_info::load_class_spells(&db).unwrap();
    let profiles = class_profiles(&classes, 11).unwrap();
    let augmentation = profiles
        .iter()
        .find(|profile| profile.spec_id == 1473)
        .unwrap();
    assert_eq!(augmentation.class_id, 13);
    assert_eq!(augmentation.role, 2);
    assert_eq!(
        augmentation.talent_source().unwrap(),
        crate::trait_config::TalentSource::Wcl
    );
    assert!(class_profiles(&classes, 1).unwrap().is_empty());
}

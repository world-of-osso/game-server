//! Native ItemEffect/SpellStore operands: catalog facts, not proc execution proof.
use super::*;
use crate::simc_apl_native::{Encounter, FightStyle};
use shared::item_data::ItemDefinitionSource;
use std::path::PathBuf;

fn load_world() -> World {
    let path = PathBuf::from(std::env::var_os("DPS_PARITY_WORLD_DB").expect("private native DB"));
    let db =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut world = World::new();
    world.insert_resource(crate::spell_info::load_spell_data(&db).unwrap().0);
    world.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    world.insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    world.insert_resource(NativeItemInstances {
        effects: server::item_instance::load::load_item_effect_tables(&db).unwrap(),
        bonuses: server::item_instance::load::load_bonus_tables(&db).unwrap(),
        enchants: Default::default(),
    });
    world
}

fn equip(world: &mut World, first: u32, second: u32) -> Entity {
    let items = [
        (EquipmentSlot::MainHand, 19019),
        (EquipmentSlot::Trinket1, first),
        (EquipmentSlot::Trinket2, second),
    ];
    world
        .spawn((
            EquippedItems {
                items: items
                    .iter()
                    .map(|&(slot, id)| (slot, ItemDefinitionSource::Retail, id))
                    .collect(),
            },
            EquippedItemModifiers {
                items: items
                    .iter()
                    .map(|&(slot, _)| (slot, Default::default()))
                    .collect(),
            },
        ))
        .id()
}

fn read_facts(world: &World, actor: Entity, names: &[&str]) -> Vec<Result<f64, String>> {
    let mut bindings = Bindings::default();
    let equipment = Equipment::load(world, actor, &mut bindings).unwrap();
    let native = NativeFacts::new(
        world,
        actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    );
    names
        .iter()
        .map(|name| equipment.fact(name, &native))
        .collect()
}

#[test]
#[ignore = "requires private native ItemEffect/spell catalog; source facts only"]
fn literal_equipment_native_stat_and_cooldown_matrix() {
    let mut world = load_world();
    // Native ItemEffects 12410,170101,179495,30564,183893,234583.
    // Durations come from their direct stat aura or authored TriggerSpell child.
    let cases = [
        (61034, [1.0, 0.0, 0.0, 0.0, 0.0], 120.0, 20.0),
        (193701, [0.0, 1.0, 0.0, 0.0, 0.0], 120.0, 20.0),
        (207581, [0.0, 1.0, 1.0, 1.0, 1.0], 180.0, 20.0),
        (139329, [0.0, 0.0, 0.0, 1.0, 0.0], 0.0, 10.0),
        (219314, [1.0, 0.0, 0.0, 0.0, 0.0], 0.1, 60.0),
        (270173, [0.0, 0.0, 0.0, 0.0, 0.0], 0.0, 0.0),
    ];
    let names = [
        "trinket.1.has_buff.strength",
        "trinket.1.has_buff.mastery",
        "trinket.1.has_buff.versatility",
        "trinket.1.has_buff.haste",
        "trinket.1.has_buff.crit",
        "trinket.1.cooldown.duration",
        "trinket.1.proc.any_dps.duration",
        "trinket.1.has_cooldown",
    ];
    let observations: Vec<_> = cases
        .into_iter()
        .map(|(item, stats, cooldown, duration)| {
            let actor = equip(&mut world, item, 270173);
            let mut expected = stats.to_vec();
            expected.extend([cooldown, duration, if cooldown > 0.0 { 1.0 } else { 0.0 }]);
            (item, read_facts(&world, actor, &names), expected)
        })
        .collect();
    for (item, actual, expected) in observations {
        assert_eq!(
            actual,
            expected.into_iter().map(Ok).collect::<Vec<_>>(),
            "native item{item}"
        );
    }
}

#[test]
#[ignore = "requires private native item identities; includes non-trinket slot"]
fn literal_equipment_native_identity_matches_actual_equipment_not_catalog_membership() {
    let mut world = load_world();
    let actor = equip(&mut world, 193701, 270173);
    assert_eq!(
        read_facts(
            &world,
            actor,
            &[
                "equipped.193701",
                "equipped.algethar_puzzle_box",
                "equipped.zuljins_guillotine_technique",
                "equipped.19019",
                "equipped.61034",
                "equipped.vicious_gladiators_badge_of_victory",
            ]
        ),
        vec![Ok(1.0), Ok(1.0), Ok(1.0), Ok(1.0), Ok(0.0), Ok(0.0)]
    );
    world
        .get_mut::<EquippedItems>(actor)
        .unwrap()
        .items
        .retain(|(slot, _, _)| *slot != EquipmentSlot::Trinket1);
    world
        .get_mut::<EquippedItemModifiers>(actor)
        .unwrap()
        .items
        .retain(|(slot, _)| *slot != EquipmentSlot::Trinket1);
    assert_eq!(
        read_facts(
            &world,
            actor,
            &[
                "equipped.193701",
                "trinket.1.has_cooldown",
                "trinket.1.proc.any_dps.duration"
            ]
        ),
        vec![Ok(0.0), Ok(0.0), Ok(0.0)]
    );
}

#[test]
#[ignore = "requires private native Ele trinket identities; all names bind explicitly"]
fn literal_equipment_native_named_trinkets_are_slot_specific() {
    let mut world = load_world();
    for (item, name) in [
        (234217, "funhouse_lens"),
        (273649, "stormbound_emblem_of_dazar"),
        (270169, "hex_lords_dooming_idol"),
        (270168, "font_of_venomous_rage"),
    ] {
        let actor = equip(&mut world, item, 193701);
        let first = format!("trinket.1.is.{name}");
        let second = format!("trinket.2.is.{name}");
        assert_eq!(
            read_facts(
                &world,
                actor,
                &[&first, &second, "trinket.2.is.algethar_puzzle_box"]
            ),
            vec![Ok(1.0), Ok(0.0), Ok(1.0)],
            "native item{item}"
        );
        assert!(read_facts(&world, actor, &["trinket.1.is.not_a_native_item_name"])[0].is_err());
    }
}

#[test]
#[ignore = "requires private native ItemSet2062 and explicit pinned tier alias policy"]
fn literal_equipment_native_set_facts_follow_original_piece_counts_and_spec() {
    let mut world = load_world();
    let actor = equip(&mut world, 193701, 270173);
    world
        .entity_mut(actor)
        .insert(crate::class_progression::ActiveSpec(70));
    let pieces = [
        (EquipmentSlot::Head, 271465),
        (EquipmentSlot::Shoulder, 271463),
        (EquipmentSlot::Chest, 271468),
        (EquipmentSlot::Legs, 271464),
    ];
    for (count, expected) in [
        (0, [0.0, 0.0]),
        (1, [0.0, 0.0]),
        (2, [1.0, 0.0]),
        (4, [1.0, 1.0]),
        (3, [1.0, 0.0]),
    ] {
        world
            .get_mut::<EquippedItems>(actor)
            .unwrap()
            .items
            .retain(|(slot, _, _)| !pieces.iter().any(|(piece, _)| piece == slot));
        world.get_mut::<EquippedItems>(actor).unwrap().items.extend(
            pieces[..count]
                .iter()
                .map(|&(slot, id)| (slot, ItemDefinitionSource::Retail, id)),
        );
        assert_eq!(
            read_facts(&world, actor, &["set_bonus.mid2_2pc", "set_bonus.mid2_4pc"]),
            expected.map(Ok).to_vec(),
            "native set2062 count{count}"
        );
    }
    world
        .entity_mut(actor)
        .insert(crate::class_progression::ActiveSpec(71));
    assert_eq!(
        read_facts(&world, actor, &["set_bonus.mid2_2pc"]),
        vec![Ok(0.0)],
        "foreign spec cannot receive Paladin set"
    );
    assert!(read_facts(&world, actor, &["set_bonus.unknown_tier_4pc"])[0].is_err());
}

#[test]
#[ignore = "requires private native item spells; zero cooldown without item-specific exception"]
fn literal_equipment_native_passive_zero_cooldown_and_unknown_operand_boundary() {
    let mut world = load_world();
    let actor = equip(&mut world, 139329, 270173);
    assert_eq!(
        read_facts(
            &world,
            actor,
            &["trinket.1.cooldown.remains", "trinket.1.cooldown.ready"]
        ),
        vec![Ok(0.0), Ok(0.0)]
    );
    for name in [
        "trinket.3.has_cooldown",
        "trinket.1.has_buff.typo",
        "trinket.1.proc.any_dps.typo",
    ] {
        assert!(
            read_facts(&world, actor, &[name])[0].is_err(),
            "unknown operand {name}"
        );
    }
}

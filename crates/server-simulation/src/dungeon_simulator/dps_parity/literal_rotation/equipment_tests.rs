//! Native ItemEffect/SpellStore operands: catalog facts, not proc execution proof.
use super::*;
use crate::simc_apl_native::{Encounter, FightStyle};
use shared::item_data::ItemDefinitionSource;
use std::path::PathBuf;

#[path = "ground_target_tests.rs"]
mod ground_target_tests;

pub(super) fn load_world() -> World {
    let path = PathBuf::from(std::env::var_os("DPS_PARITY_WORLD_DB").expect("private native DB"));
    let db =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut world = World::new();
    world.insert_resource(crate::spell_info::load_spell_data(&db).unwrap().0);
    world.insert_resource(crate::item_proc_data::load(&db).unwrap());
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
                ..Default::default()
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

fn equip_reference_profile(world: &mut World, spec: u32) -> Entity {
    use crate::dungeon_simulator::dps_parity::inputs;
    let path = inputs::directory()
        .unwrap()
        .join(format!("{spec}.profile.json"));
    let profile: inputs::Profile = inputs::read_json(&path).unwrap();
    assert_eq!(profile.spec_id, spec);
    let actor = world.spawn(crate::class_progression::ActiveSpec(spec)).id();
    inputs::equip(world, actor, &profile).unwrap();
    actor
}

#[test]
#[ignore = "requires private native item catalogs; real Arcane reference trinkets"]
fn literal_equipment_absent_named_membership_and_use_are_unavailable() {
    let mut world = load_world();
    let actor = equip(&mut world, 250215, 281215);
    let membership = read_facts(
        &world,
        actor,
        &[
            "equipped.250215",
            "equipped.freightrunners_flask",
            "equipped.FREIGHTRUNNERS_FLASK",
            "equipped.ever_collapsing_void_fissure",
        ],
    );
    let mut bindings = Bindings::default();
    let equipment = Equipment::load(&world, actor, &mut bindings).unwrap();
    let absent = Action {
        source_row: None,
        name: "use_item".into(),
        target: None,
        options: [("name".into(), "ever_collapsing_void_fissure".into())].into(),
    };
    let named_use = equipment.slot_for_action(&absent);
    assert_eq!(
        (membership, named_use),
        (vec![Ok(1.0), Ok(1.0), Ok(1.0), Ok(0.0)], Ok(None))
    );
}

#[test]
#[ignore = "exact native Dev/Fire/Assassination profile instances; passive and bonus-derived levels"]
fn literal_equipment_native_profile_item_levels_include_passive_and_bonus_instances() {
    let mut world = load_world();
    // Exact reference gear: Dev249343/249346 at289, Fire273796/270167 at334,
    // Assassin270168 at344 and270175 bonuses13662/13848 resolving344.
    let observations: Vec<_> = [(1467, 289.0), (63, 334.0), (259, 344.0)]
        .into_iter()
        .map(|(spec, level)| {
            let actor = equip_reference_profile(&mut world, spec);
            (
                spec,
                read_facts(&world, actor, &["trinket.1.ilvl", "trinket.2.ilvl"]),
                vec![Ok(level), Ok(level)],
            )
        })
        .collect();
    for (spec, actual, expected) in observations {
        assert_eq!(actual, expected, "real native profile{spec}");
    }
}

#[test]
#[ignore = "real native profiles; pinned buff.default_value is distinct from stat magnitude"]
fn literal_equipment_native_profile_proc_default_values_follow_unset_buff_semantics() {
    let mut world = load_world();
    // Pinned item custom stat buffs do not set buff.default_value. The item
    // expression takes max(0, matching defaults), not their native stat amounts.
    let observations: Vec<_> = [1467, 63, 259]
        .into_iter()
        .map(|spec| {
            let actor = equip_reference_profile(&mut world, spec);
            (
                spec,
                read_facts(
                    &world,
                    actor,
                    &[
                        "trinket.1.proc.any_dps.default_value",
                        "trinket.2.proc.any_dps.default_value",
                    ],
                ),
            )
        })
        .collect();
    for (spec, actual) in observations {
        assert_eq!(actual, vec![Ok(0.0), Ok(0.0)], "real native profile{spec}");
    }
}

#[test]
#[ignore = "unqualified native script default must stay named rather than inventing a value"]
fn literal_equipment_native_unqualified_proc_default_value_is_explicit() {
    let mut world = load_world();
    let actor = equip(&mut world, 193701, 270173);
    let actual = read_facts(&world, actor, &["trinket.1.proc.any_dps.default_value"]);
    assert!(
        actual[0]
            .as_ref()
            .unwrap_err()
            .contains("NATIVE_ITEM_PROC_DEFAULT_VALUE_UNSOURCED"),
        "unqualified native script must identify its missing default-value source: {actual:?}"
    );
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
#[ignore = "native primary-stat trinkets for shared caster/melee APL operands"]
fn literal_equipment_native_primary_stat_names_match_the_real_trinket_aura() {
    let mut world = load_world();
    // Native effects10303/12409/12410 ->64712(Intellect)/84969(Agility)/84966(Strength).
    for (item, expected) in [
        (45148, [0.0, 0.0, 1.0]),
        (61033, [0.0, 1.0, 0.0]),
        (61034, [1.0, 0.0, 0.0]),
    ] {
        let actor = equip(&mut world, item, 270173);
        assert_eq!(
            read_facts(
                &world,
                actor,
                &[
                    "trinket.1.has_buff.strength",
                    "trinket.1.has_buff.agility",
                    "trinket.1.has_buff.intellect"
                ]
            ),
            expected.map(Ok).to_vec(),
            "native primary item{item}"
        );
        assert_eq!(
            read_facts(&world, actor, &["trinket.1.has_use_buff"]),
            vec![Ok(1.0)]
        );
        assert!(read_facts(&world, actor, &["trinket.1.has_buff.unknown_primary"])[0].is_err());
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
#[ignore = "actual Fire2060 four-piece profile; pinned long and short tier spellings"]
fn literal_equipment_native_fire_tier_long_and_short_aliases_share_threshold_state() {
    let mut world = load_world();
    let actor = equip_reference_profile(&mut world, 63);
    let original = world.get::<EquippedItems>(actor).unwrap().clone();
    let pieces = [271562, 271567, 271565, 271563];
    for (count, expected) in [(4, [1.0, 1.0]), (1, [0.0, 0.0]), (2, [1.0, 0.0])] {
        let mut equipped = original.clone();
        equipped
            .items
            .retain(|(_, _, id)| !pieces.contains(id) || pieces[..count].contains(id));
        world.entity_mut(actor).insert(equipped);
        assert_eq!(
            read_facts(
                &world,
                actor,
                &[
                    "set_bonus.midnight_season_2_2pc",
                    "set_bonus.midnight_season_2_4pc",
                    "set_bonus.mid2_2pc",
                    "set_bonus.mid2_4pc",
                ],
            ),
            expected
                .into_iter()
                .chain(expected)
                .map(Ok)
                .collect::<Vec<_>>(),
            "actual Fire2060 original pieces{count} select1296583/1296584",
        );
    }
    world.entity_mut(actor).insert(original);
    world
        .entity_mut(actor)
        .insert(crate::class_progression::ActiveSpec(71));
    assert_eq!(
        read_facts(&world, actor, &["set_bonus.midnight_season_2_2pc"]),
        vec![Ok(0.0)],
        "native spec selection remains authoritative",
    );
    assert!(read_facts(&world, actor, &["set_bonus.unqualified_fire_alias_2pc"])[0].is_err());
}

#[test]
#[ignore = "native on-use damage effects versus passive and stat-only real trinkets"]
fn literal_equipment_native_has_use_damage_follows_on_use_effects() {
    let mut world = load_world();
    // Lightning55039 is direct SCHOOL_DAMAGE; Caber99915 triggers99938 damage.
    // Dragonfire408671→408675 ends in aura42/Trigger0, not native damage.
    // Jewel215206 is rating-only, AraKara443541 passive, Guillotine1291728 passive.
    for (item, expected) in [
        (41121, 1.0),
        (70141, 1.0),
        (202610, 0.0),
        (137462, 0.0),
        (219314, 0.0),
    ] {
        let actor = equip(&mut world, item, 270173);
        assert_eq!(
            read_facts(
                &world,
                actor,
                &["trinket.1.has_use_damage", "trinket.2.has_use_damage"]
            ),
            vec![Ok(expected), Ok(0.0)],
            "native item{item} versus passive270173",
        );
    }
}

fn item_cooldown_harness(item: u32) -> crate::spell_cast::test_harness::Harness {
    let mut source = load_world();
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1).unwrap();
    h.app.insert_resource(
        source
            .remove_resource::<crate::spell_info::SpellStore>()
            .unwrap(),
    );
    h.app.insert_resource(
        source
            .remove_resource::<crate::item_proc_data::ItemProcInputs>()
            .unwrap(),
    );
    h.app
        .insert_resource(source.remove_resource::<ItemTemplates>().unwrap());
    h.app.insert_resource(
        source
            .remove_resource::<crate::item_stats::ItemScalingTables>()
            .unwrap(),
    );
    h.app
        .insert_resource(source.remove_resource::<NativeItemInstances>().unwrap());
    let temporary = equip(h.app.world_mut(), item, 270173);
    let equipment = h
        .app
        .world_mut()
        .entity_mut(temporary)
        .take::<EquippedItems>()
        .unwrap();
    let modifiers = h
        .app
        .world_mut()
        .entity_mut(temporary)
        .take::<EquippedItemModifiers>()
        .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((equipment, modifiers));
    h.app.world_mut().entity_mut(temporary).despawn();
    h
}

fn cast_native_item(h: &mut crate::spell_cast::test_harness::Harness, target: Entity) {
    let equipment = Equipment::load(h.app.world(), h.caster, &mut Bindings::default()).unwrap();
    let item = equipment.slots["trinket1"].on_use.unwrap();
    crate::spell_cast::cast_item_spell(
        h.app.world_mut(),
        h.caster,
        item.on_use.spell_id,
        Some(target),
        item.on_use.cooldown,
        item.item_level,
        Some(item.item_id),
    )
    .expect("actual native item cast, no fabricated history");
}

#[test]
#[ignore = "native Jewel137462 three-charge category1631 consumed by real item cast"]
fn literal_equipment_native_fractional_charges_follow_real_item_category_recharge() {
    let mut h = item_cooldown_harness(137462);
    let caster = h.caster;
    let initial = read_facts(
        h.app.world(),
        caster,
        &["trinket.1.cooldown.charges_fractional"],
    );
    cast_native_item(&mut h, caster);
    assert_eq!(initial, vec![Ok(3.0)]);
    let after = read_facts(
        h.app.world(),
        caster,
        &["trinket.1.cooldown.charges_fractional"],
    );
    assert_eq!(after, vec![Ok(2.0)]);
    h.advance(200); // Native manual50ms clock: ten seconds of authored20s recharge.
    let half = read_facts(
        h.app.world(),
        caster,
        &["trinket.1.cooldown.charges_fractional"],
    )[0]
    .clone()
    .unwrap();
    assert!((half - 2.5).abs() < 0.01, "native partial recharge: {half}");
    h.advance(200);
    assert_eq!(
        read_facts(
            h.app.world(),
            caster,
            &["trinket.1.cooldown.charges_fractional"]
        ),
        vec![Ok(3.0)]
    );
}

#[test]
#[ignore = "native Lightning41121 single-charge on-use cooldown consumed by real item cast"]
fn literal_equipment_native_fractional_single_charge_tracks_elapsed_cooldown() {
    let mut h = item_cooldown_harness(41121);
    let caster = h.caster;
    let target = h.npc;
    let initial = read_facts(
        h.app.world(),
        caster,
        &["trinket.1.cooldown.charges_fractional"],
    );
    cast_native_item(&mut h, target);
    h.advance(100); // Five seconds elapsed in the native spell/category cooldown.
    let remains = read_facts(h.app.world(), caster, &["trinket.1.cooldown.remains"])[0]
        .clone()
        .unwrap();
    let duration = read_facts(h.app.world(), caster, &["trinket.1.cooldown.duration"])[0]
        .clone()
        .unwrap();
    assert!(
        remains > 0.0 && remains < duration,
        "real running timer{remains}/{duration}"
    );
    let bindings = Bindings::default();
    let native = NativeFacts::new(
        h.app.world(),
        caster,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    );
    let equipment = Equipment::load(h.app.world(), caster, &mut Bindings::default()).unwrap();
    let spell = &equipment.slots["trinket1"].metadata.cooldowns[0].spell;
    let native_progress = native
        .cooldown_for_spell(spell, "charges_fractional")
        .unwrap();
    assert!(
        (native_progress - (1.0 - remains / duration)).abs() < 0.000001,
        "existing native helper must expose fractional progress{native_progress}"
    );
    assert_eq!(initial, vec![Ok(1.0)]);
    let fraction = read_facts(
        h.app.world(),
        caster,
        &["trinket.1.cooldown.charges_fractional"],
    )[0]
    .clone()
    .unwrap();
    assert!(
        (fraction - (1.0 - remains / duration)).abs() < 0.000001,
        "native single-charge progress{fraction}"
    );
}

fn scope_trigger_catalog(world: &mut World, remove_child: bool, cycle: bool) {
    let store = world.resource::<crate::spell_info::SpellStore>();
    let mut spells: Vec<_> = [21992, 27648, 221786, 221796, 1291728]
        .into_iter()
        .filter(|&id| !remove_child || id != 221796)
        .map(|id| store.get(id).unwrap().clone())
        .collect();
    let root = spells.iter_mut().find(|spell| spell.id == 221786).unwrap();
    if cycle {
        // Malformed test-owned source: a real proc root points to itself.
        root.effects[0].trigger_spell = root.id;
    } else {
        // Shared child DAG: two real proc effects reference the same stat spell.
        let mut second = root.effects[0].clone();
        second.index = 1;
        root.effects.push(second);
    }
    world.insert_resource(crate::spell_info::SpellStore::fixture(spells));
}

#[test]
#[ignore = "requires native item roots; malformed test-owned metadata must fail explicitly"]
fn literal_equipment_native_trigger_missing_child_and_cycle_are_errors() {
    let mut world = load_world();
    let actor = equip(&mut world, 139329, 270173);
    for (missing, cycle) in [(true, false), (false, true)] {
        // Restore unmodified native input before each independently malformed graph.
        let source = load_world();
        world.insert_resource(source.resource::<crate::spell_info::SpellStore>().clone());
        scope_trigger_catalog(&mut world, missing, cycle);
        let result = Equipment::load(&world, actor, &mut Bindings::default());
        assert!(
            result.is_err(),
            "missing={missing} cycle={cycle} must not bind silent metadata"
        );
    }
}

#[test]
#[ignore = "requires native item roots; shared child is not an active recursion cycle"]
fn literal_equipment_native_trigger_shared_child_dag_keeps_stat_and_duration() {
    let mut world = load_world();
    let actor = equip(&mut world, 139329, 270173);
    scope_trigger_catalog(&mut world, false, false);
    assert_eq!(
        read_facts(
            &world,
            actor,
            &[
                "trinket.1.has_buff.haste",
                "trinket.1.proc.any_dps.duration"
            ]
        ),
        vec![Ok(1.0), Ok(10.0)]
    );
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

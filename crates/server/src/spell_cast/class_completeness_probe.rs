//! Opt-in, native-row level-80 cast census; no UDP, database or persistence writes.
//! This records a controlled base-build cast, NOT full rotation/talent parity.
use std::collections::HashSet;
use std::io::Write;

use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use serde::Deserialize;
use serde_json::{Value, json};
use shared::components::{ActiveSpec, Health, Level, Mana, Player, Position};
use shared::protocol::{CombatChannel, SpellCastIntent};

use super::test_harness::*;
use crate::class_progression::KnownSpells;

#[derive(Deserialize)]
struct Manifest {
    specs: Vec<Spec>,
}

#[derive(Deserialize)]
struct Spec {
    id: u32,
    class: u8,
    spells: Vec<Spell>,
}

#[derive(Deserialize)]
struct Spell {
    id: u32,
    passive: bool,
    origin: Vec<String>,
}

fn source_passives(spec: &Spec) -> Vec<u32> {
    spec.spells
        .iter()
        .filter(|spell| {
            spell.passive
                && spell
                    .origin
                    .iter()
                    .any(|origin| !origin.contains("talent") && !origin.contains("race-gated"))
        })
        .map(|spell| spell.id)
        .collect()
}

fn spawn_case(h: &mut Harness, spec: &Spec, spell: &Spell) {
    h.caster = spawn_player(&mut h.app, spec.class, 0.0);
    h.npc = spawn_npc(&mut h.app, 3.0);
    let mut known = source_passives(spec);
    known.push(spell.id);
    known.extend(prerequisite_events(spell.id));
    if spell.id == 1244258 {
        // Chomp's native description names the passive controller1244292.
        known.push(1244292);
    }
    if let Some((form_spell, _)) = select_form_prerequisite(spec, spell, &world_db().unwrap().store)
    {
        known.push(form_spell);
    }
    h.app.world_mut().entity_mut(h.caster).insert((
        Level(80),
        crate::persistence::ClientId(u64::from(spell.id)),
        shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        ActiveSpec(spec.id),
        KnownSpells::fixture(known),
        Health {
            current: 500_000.0,
            max: 1_000_000.0,
        },
    ));
    let race = match spec.class {
        7 => 11,
        11 | 12 => 4,
        13 => 52,
        _ => 1,
    };
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = race;
    equip_native_armor(h, spec.class);
    h.app.world_mut().entity_mut(h.npc).insert((
        Level(80),
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::networking::PlayerLink(h.caster));
    h.advance(3);
    fill_resources(h);
    h.inbox = Inbox::default();
}

/// Item/ItemSparse 70448: complete Mystical, Scaled Leather, Glimmering Mail,
/// and Templar sets. No synthesized item templates or passive aura applications.
fn equip_native_armor(h: &mut Harness, class: u8) {
    use shared::item_data::ItemDefinitionSource::Retail;
    use shared::protocol::EquipmentSlot::*;
    let ids = match class {
        5 | 8 | 9 => [10175, 10172, 10181, 10180, 10177, 10179, 10173, 10176],
        4 | 10 | 11 | 12 => [10406, 9834, 9835, 9827, 9833, 9828, 9829, 9832],
        3 | 7 | 13 => [6389, 6388, 4071, 4712, 6386, 4073, 6387, 4072],
        1 | 2 | 6 => [10168, 10170, 10164, 10166, 10169, 10167, 10171, 10165],
        _ => panic!("no native census armor set for class {class}"),
    };
    let slots = [Head, Shoulder, Chest, Waist, Legs, Feet, Wrist, Hands];
    let player = h.app.world().get::<Player>(h.caster).unwrap();
    let catalog = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    for id in ids {
        let item = catalog
            .get(Retail, id)
            .expect("native armor item definition");
        assert!(item.allows(class, player.race) && item.required_level <= 80);
    }
    let items = slots
        .into_iter()
        .zip(ids)
        .map(|(slot, id)| (slot, Retail, id))
        .collect();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::stats::EquippedItems { items });
}

fn fill_resources(h: &mut Harness) {
    if let Some(mut pools) = h
        .app
        .world_mut()
        .get_mut::<super::powers::PowerPools>(h.caster)
    {
        for pool in &mut pools.pools {
            pool.current = pool.max;
        }
    }
    let mut mana = h.app.world_mut().get_mut::<Mana>(h.caster).unwrap();
    mana.current = mana.max;
}

fn resources(h: &Harness) -> Value {
    let powers: Vec<_> = h.app.world().get::<super::powers::PowerPools>(h.caster)
        .into_iter().flat_map(|pools| &pools.pools)
        .map(|pool| json!({"power": format!("{:?}", pool.power), "current": pool.current, "max": pool.max}))
        .collect();
    let mana = h.app.world().get::<Mana>(h.caster).unwrap();
    json!({"pools": powers, "mana": mana.current})
}

fn choose_case_target(h: &mut Harness, spell: &crate::spell_info::SpellInfo) -> (Entity, bool) {
    let resurrects = spell
        .effects
        .iter()
        .any(|effect| matches!(effect.effect, 18 | 172));
    let relation = super::scripts::target_plan(spell.id)
        .map(|(relation, _)| relation)
        .or_else(|| super::targets::explicit_requirement(super::handled_effects(spell)))
        .or_else(|| resurrects.then_some(super::targets::Relation::Ally));
    let hostile = relation == Some(super::targets::Relation::Enemy)
        || (relation == Some(super::targets::Relation::Any)
            && (spell.id == 361469 || spell.effects.iter().any(|effect| effect.effect == 2)));
    let target = if resurrects {
        let ally = spawn_player(&mut h.app, 1, 3.0);
        h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 0.0;
        ally
    } else if hostile {
        h.npc
    } else if super::attributes(spell, 1) & 0x0008_0000 != 0 {
        // SPELL_ATTR1_EXCLUDE_CASTER rejects explicit self before area selection.
        let ally = spawn_player(&mut h.app, 1, 3.0);
        h.app.world_mut().entity_mut(ally).insert((
            Level(80),
            Health {
                current: 500_000.0,
                max: 1_000_000.0,
            },
        ));
        ally
    } else {
        h.caster
    };
    (target, hostile)
}

fn send_case_cast(
    h: &mut Harness,
    spell: &crate::spell_info::SpellInfo,
    target: Entity,
    hostile: bool,
) {
    let (min, max) = super::targets::spell_range(spell, hostile);
    let distance = (min + 0.5).max(3.0).min(max.max(3.0));
    if target != h.caster {
        h.app.world_mut().entity_mut(target).insert(Position {
            x: distance,
            y: 0.0,
            z: 0.0,
        });
    }
    if super::attributes(spell, 0) & super::SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL != 0 {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(shared::components::CombatStatus(false));
    }
    let destination = spell
        .target_restrictions
        .as_ref()
        .filter(|restriction| restriction.targets & 0x40 != 0)
        .map(|_| [distance, 0.0, 0.0]); // Native TARGET_FLAG_DEST_LOCATION.
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(spell.id),
            spell: String::new(),
            target_entity: Some(target.to_bits()),
            target_item_guid: None,
            witness: None,
            destination,
        });
}

fn cast_case(h: &mut Harness, spell: &crate::spell_info::SpellInfo) {
    let (target, hostile) = choose_case_target(h, spell);
    send_case_cast(h, spell, target, hostile);
    h.advance(3);
    let cast_ms = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .map_or(0, |cast| cast.base_ms.max(0) as usize);
    h.advance(cast_ms / 50 + 200); // Native cast time plus ten seconds of ticks/flight.
}

fn place_ground_case(h: &mut Harness, spell: &crate::spell_info::SpellInfo) {
    if !spell
        .effects
        .iter()
        .any(|effect| effect.implicit_targets.contains(&62))
    {
        return;
    }
    // Generic zero-coordinate actors do not stand on an authored terrain tile.
    // Use the native Human arena location, not invented ground or a successful no-op.
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (x, y, z): (f32, f32, f32) = conn.query_row("SELECT position_x,position_y,position_z FROM tdb_playercreateinfo WHERE race=1 AND class=1", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(Position { x, y, z });
}

/// A controlled native form environment, not a legal talent-build certificate.
/// Cat/Bear are native Druid abilities; Moonkin is a separately declared talent setup.
/// Only a required, non-excluded form is selected. Other prerequisites remain exposed.
fn select_form_prerequisite(
    spec: &Spec,
    row: &Spell,
    store: &crate::spell_info::SpellStore,
) -> Option<(u32, i32)> {
    if spec.class != 11 || row.passive {
        return None;
    }
    let masks = store.get(row.id)?.shapeshift.as_ref()?;
    let required =
        u64::from(masks.shapeshift_mask[0]) | (u64::from(masks.shapeshift_mask[1]) << 32);
    let excluded =
        u64::from(masks.shapeshift_exclude[0]) | (u64::from(masks.shapeshift_exclude[1]) << 32);
    [768, 5487, 197625].into_iter().find_map(|id| {
        let form = store
            .get(id)?
            .effects
            .iter()
            .find(|effect| effect.aura == crate::auras::handlers::MOD_SHAPESHIFT)?
            .misc_values[0];
        let bit = u32::try_from(form - 1).ok()?;
        let stance = 1u64.checked_shl(bit)?;
        (stance & required != 0 && stance & excluded == 0).then_some((id, form))
    })
}

fn cast_form_prerequisite(h: &mut Harness, spec: &Spec, row: &Spell) -> Option<Value> {
    let store = &world_db().unwrap().store;
    let (id, expected_form) = select_form_prerequisite(spec, row, store)?;
    let spell = store.get(id).unwrap();
    cast_case(h, spell);
    let observed_form = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .and_then(crate::auras::shapeshift::current_form)
        .map(|(_, form)| form);
    let warnings = crate::auras::take_unhandled_auras(h.app.world_mut());
    let effects = h
        .app
        .world()
        .resource::<super::effects::UnhandledEffects>()
        .0
        .clone();
    let receipt = json!({"spell": id, "expected_form": expected_form,
        "observed_form": observed_form, "failed": h.inbox.failed, "go": h.inbox.go,
        "aura_warnings": warnings, "effect_warnings": effects,
        "combat_log": h.inbox.log, "resources_after_cast": resources(h)});
    h.inbox = Inbox::default();
    h.app
        .world_mut()
        .resource_mut::<super::effects::UnhandledEffects>()
        .0
        .clear();
    fill_resources(h);
    Some(receipt)
}

fn missing_store_receipt(spec: &Spec, id: u32) -> Value {
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut counts = serde_json::Map::new();
    for (table, key) in [
        ("spell_name", "ID"),
        ("spell", "ID"),
        ("spell_misc", "SpellID"),
        ("spell_effect", "SpellID"),
        ("tdb_serverside_spell", "Id"),
    ] {
        let query = format!("SELECT COUNT(*) FROM {table} WHERE {key}=?1");
        let count: u32 = conn.query_row(&query, [id], |r| r.get(0)).unwrap();
        counts.insert(table.into(), json!(count));
    }
    json!({"spec": spec.id, "spell": id, "missing_store": true,
        "source_rows": counts, "store_loader": "spell_info::load_spell_data (production)"})
}

/// Preceding player actions, never casts of hidden enabling auras. These may
/// expose an absent producer; a successful action alone does not certify its proc.
fn prerequisite_events(root: u32) -> &'static [u32] {
    match root {
        5143 => &[30451], // Clearcasting baseline; actual Arcane Blast mana spend.
        119898 => &[688], // Command Demon: summon a real Imp.
        152175 => &[107428, 113656], // Both native cooldowns before Dragon Punch.
        196277 | 264130 => &[105174], // Hand of Gul'dan, not a fabricated Wild Imp.
        196447 => &[348], // TC a352b1fa spell_warl_channel_demonfire_activator.
        247454 => &[203782], // Shear description: shatter a Lesser Soul Fragment.
        1244258 => &[5221, 5221], // Chomp controller: actual Cat energy expenditure.
        _ => &[],
    }
}

fn cast_preceding_event(h: &mut Harness, id: u32) -> Value {
    let spell = world_db()
        .unwrap()
        .store
        .get(id)
        .expect("native preceding event");
    let before = resources(h);
    let (target, hostile) = choose_case_target(h, spell);
    send_case_cast(h, spell, target, hostile);
    let misc = spell.misc.as_ref();
    let cast_ms = misc
        .and_then(|m| m.cast_time)
        .map_or(0, |c| c.base_ms.max(0));
    let channel_ms = misc
        .filter(|m| m.attributes[1] & 0x44 != 0)
        .and_then(|m| m.duration)
        .map_or(0, |d| d.duration_ms.max(0));
    // Finish the native cast/channel, delivery and the ordinary GCD.
    h.advance((cast_ms.max(channel_ms) as usize / 50) + 43);
    let receipt = json!({"spell": id, "failed": h.inbox.failed, "go": h.inbox.go,
        "combat_log": h.inbox.log, "resources_before": before, "resources_after": resources(h),
        "aura_warnings": crate::auras::take_unhandled_auras(h.app.world_mut()),
        "effect_warnings": h.app.world().resource::<super::effects::UnhandledEffects>().0});
    h.inbox = Inbox::default();
    h.app
        .world_mut()
        .resource_mut::<super::effects::UnhandledEffects>()
        .0
        .clear();
    receipt
}

fn prepare_caster_aura(h: &mut Harness, spell: &crate::spell_info::SpellInfo) -> Option<Value> {
    let restrictions = spell.aura_restrictions.as_ref()?;
    if restrictions.caster_aura_spell == 0 && restrictions.exclude_caster_aura_spell == 0 {
        return None;
    }
    let required = restrictions.caster_aura_spell;
    let excluded = restrictions.exclude_caster_aura_spell;
    let required_before = crate::auras::has_aura(h.app.world(), h.caster, required);
    let events: Vec<_> = prerequisite_events(spell.id)
        .iter()
        .map(|&id| cast_preceding_event(h, id))
        .collect();
    Some(json!({"required": required, "excluded": excluded,
        "required_before": required_before,
        "required_after": crate::auras::has_aura(h.app.world(), h.caster, required),
        "excluded_after": crate::auras::has_aura(h.app.world(), h.caster, excluded),
        "admitted": super::aura_spell_restrictions::check_caster(h.app.world(), h.caster, spell).is_ok(),
        "events": events}))
}

fn record_case(h: &mut Harness, spec: &Spec, row: &Spell) -> Value {
    let Some(spell) = world_db().unwrap().store.get(row.id).cloned() else {
        return missing_store_receipt(spec, row.id);
    };
    spawn_case(h, spec, row);
    record_spawned_case(h, spec, row, &spell)
}

fn record_spawned_case(
    h: &mut Harness,
    spec: &Spec,
    row: &Spell,
    spell: &crate::spell_info::SpellInfo,
) -> Value {
    place_ground_case(h, spell);
    let baseline_warnings = crate::auras::take_unhandled_auras(h.app.world_mut());
    h.app
        .world_mut()
        .resource_mut::<super::effects::UnhandledEffects>()
        .0
        .clear();
    let form_prerequisite = cast_form_prerequisite(h, spec, row);
    let caster_aura_prerequisite = prepare_caster_aura(h, spell);
    let before = resources(h);
    if !row.passive {
        cast_case(h, spell);
    }
    let after = resources(h);
    let warnings = crate::auras::take_unhandled_auras(h.app.world_mut());
    let effects: Vec<_> = h
        .app
        .world()
        .resource::<super::effects::UnhandledEffects>()
        .0
        .iter()
        .copied()
        .collect();
    let applied = crate::auras::has_aura(h.app.world(), h.caster, row.id);
    let form_after = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .and_then(crate::auras::shapeshift::current_form)
        .map(|(_, form)| form);
    json!({"spec": spec.id, "spell": row.id, "passive": row.passive,
        "passive_applied": applied, "form_prerequisite": form_prerequisite, "form_after": form_after,
        "caster_aura_prerequisite": caster_aura_prerequisite,
        "equipped_armor": h.app.world().get::<crate::stats::EquippedItems>(h.caster).unwrap().items,
        "failed": h.inbox.failed, "go": h.inbox.go,
        "baseline_aura_warnings": baseline_warnings, "aura_warnings": warnings,
        "effect_warnings": effects, "resources_before": before, "resources_after": after,
        "combat_log": h.inbox.log})
}

fn despawn_case(h: &mut Harness, retained: &HashSet<Entity>) {
    let entities: Vec<_> = h
        .app
        .world_mut()
        .query::<Entity>()
        .iter(h.app.world())
        .filter(|entity| !retained.contains(entity))
        .collect();
    for entity in entities {
        h.app.world_mut().despawn(entity);
    }
}

fn census_harness(class: u8, world_path: &std::path::Path) -> Harness {
    let data = world_db().unwrap();
    let mut h = harness(Vec::new(), &[], class).unwrap();
    h.app.insert_resource(data.store.clone());
    static ITEMS: std::sync::OnceLock<crate::item_templates::ItemTemplates> =
        std::sync::OnceLock::new();
    h.app.insert_resource(
        ITEMS
            .get_or_init(|| {
                crate::item_templates::load_item_templates(&data.powers_db)
                    .expect("production item catalogs")
            })
            .clone(),
    );
    h.app
        .insert_resource(crate::world_data::load_world_data(world_path));
    // Cargo executes tests from the crate directory; production's relative
    // data/world.db startup path is not this fixture's authored database.
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::teleport_positions::load_positions(&conn).unwrap());
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
    h.app
        .insert_resource(crate::meeting_stone::load_meeting_stone_data(&conn).unwrap());
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        maps.maps
            .values()
            .map(|map| (map.id, map.wdt_file_data_id))
            .collect(),
    ));
    h.app.insert_resource(maps);
    h.app.init_resource::<crate::maps::InstanceManager>();
    h.app.init_resource::<crate::group::GroupRegistry>();
    h.app
        .insert_resource(crate::conditions::load_spell_target_conditions(&data.powers_db).unwrap());
    h
}

#[test]
fn class_completeness_probe_druid_native_forms_precede_required_abilities() {
    let prepared = prepare_class_world();
    let mut h = census_harness(11, &prepared.path());
    let spec = Spec {
        id: 103,
        class: 11,
        spells: Vec::new(),
    };
    for (id, form_spell, form) in [(5221, 768, 1), (6795, 5487, 5), (164862, 197625, 35)] {
        let spell = Spell {
            id,
            passive: false,
            origin: vec!["SkillLineAbility".into()],
        };
        spawn_case(&mut h, &spec, &spell);
        // Qualify the replacement caster only here; the census stays unchanged.
        apply_native_enemy_dodge_reduction(&mut h);
        let native = world_db().unwrap().store.get(id).unwrap();
        let receipt = record_spawned_case(&mut h, &spec, &spell, native);
        assert_eq!(
            receipt["failed"],
            json!([]),
            "native ability {id}: {receipt}"
        );
        assert!(!receipt["go"].as_array().unwrap().is_empty(), "{receipt}");
        let prerequisite = &receipt["form_prerequisite"];
        assert_eq!(prerequisite["spell"], form_spell, "{receipt}");
        assert_eq!(prerequisite["observed_form"], form, "{receipt}");
        assert_eq!(prerequisite["failed"], json!([]), "{receipt}");
        assert!(
            !prerequisite["go"].as_array().unwrap().is_empty(),
            "{receipt}"
        );
        if id == 5221 {
            assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(id)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0));
        }
    }
}

#[test]
fn class_completeness_probe_druid_unrestricted_spells_do_not_gain_a_form() {
    let prepared = prepare_class_world();
    let mut h = census_harness(11, &prepared.path());
    let spec = Spec {
        id: 105,
        class: 11,
        spells: Vec::new(),
    };
    let spell = Spell {
        id: 774,
        passive: false,
        origin: vec!["SkillLineAbility".into()],
    };
    let receipt = record_case(&mut h, &spec, &spell);
    assert!(receipt["form_prerequisite"].is_null(), "{receipt}");
    assert_eq!(receipt["failed"], json!([]), "{receipt}");
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 774));
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .and_then(crate::auras::shapeshift::current_form)
            .is_none()
    );
}

#[test]
fn class_completeness_probe_native_warlock_rituals_use_retail_template_spells() {
    let prepared = prepare_class_world();
    for (id, entry, casters, completion) in [(698, 194108, 3, 62330), (342601, 177193, 5, 342607)] {
        let mut h = census_harness(9, &prepared.path());
        let spec = Spec {
            id: 265,
            class: 9,
            spells: Vec::new(),
        };
        let spell = Spell {
            id,
            passive: false,
            origin: vec!["talent".into()],
        };
        let receipt = record_case(&mut h, &spec, &spell);
        assert_eq!(
            receipt["failed"],
            json!([]),
            "native ritual {id}: {receipt}"
        );
        let world = h.app.world_mut();
        let rituals: Vec<_> = world
            .query::<(
                &shared::protocol::GameObjectInfo,
                &crate::meeting_stone::ritual::Ritual,
            )>()
            .iter(world)
            .filter(|(info, _)| info.entry == entry)
            .collect();
        assert_eq!(rituals.len(), 1);
        assert_eq!(rituals[0].1.casters, casters);
        assert_eq!(rituals[0].1.spell, completion);
    }
}

#[test]
fn class_completeness_probe_native_rewind_uses_ungrouped_player_state() {
    let prepared = prepare_class_world();
    let mut h = census_harness(13, &prepared.path());
    let spec = Spec {
        id: 1468,
        class: 13,
        spells: Vec::new(),
    };
    let spell = Spell {
        id: 363534,
        passive: false,
        origin: vec!["talent".into()],
    };
    let receipt = record_case(&mut h, &spec, &spell);
    assert_eq!(
        receipt["failed"],
        json!([]),
        "native Rewind cast: {receipt}"
    );
    assert!(!receipt["go"].as_array().unwrap().is_empty());
}

#[test]
fn class_completeness_probe_native_city_destination_is_not_a_fixture_failure() {
    let prepared = prepare_class_world();
    let mut h = census_harness(8, &prepared.path());
    let spec = Spec {
        id: 62,
        class: 8,
        spells: Vec::new(),
    };
    let spell = Spell {
        id: 3567,
        passive: false,
        origin: vec!["SkillLineAbility".into()],
    };
    let receipt = record_case(&mut h, &spec, &spell);
    assert_eq!(receipt["failed"], json!([]), "native city cast: {receipt}");
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let destinations = super::teleport_positions::load_positions(&conn).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::maps::MapTransfer>(h.caster)
            .unwrap()
            .destination,
        destinations.0[&(3567, 0)],
    );
}

#[test]
fn class_completeness_probe_fixture_refresh_equips_native_armor_for_passives() {
    let prepared = prepare_class_world();
    for (class, spec_id, id) in [
        (8, 62, 89744),
        (11, 103, 86092),
        (7, 262, 86100),
        (1, 71, 86101),
    ] {
        let mut h = census_harness(class, &prepared.path());
        let spec = Spec {
            id: spec_id,
            class,
            spells: Vec::new(),
        };
        let row = Spell {
            id,
            passive: true,
            origin: vec!["SpecializationSpells/mastery".into()],
        };
        let receipt = record_case(&mut h, &spec, &row);
        assert_eq!(receipt["passive_applied"], true, "{receipt}");
    }
}

#[test]
fn class_completeness_probe_fixture_refresh_exclude_caster_uses_eligible_ally() {
    let prepared = prepare_class_world();
    let mut h = census_harness(2, &prepared.path());
    let spec = Spec {
        id: 70,
        class: 2,
        spells: Vec::new(),
    };
    let row = Spell {
        id: 6940,
        passive: false,
        origin: vec!["talent".into()],
    };
    let receipt = record_case(&mut h, &spec, &row);
    assert_eq!(receipt["failed"], json!([]), "{receipt}");
    assert!(
        h.app
            .world_mut()
            .query::<(Entity, &crate::auras::ActiveAuras)>()
            .iter(h.app.world())
            .any(|(entity, auras)| entity != h.caster
                && auras.iter().any(|aura| aura.spell_id == 6940))
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 6940));
}

#[test]
fn class_completeness_probe_fixture_refresh_records_actual_preceding_events() {
    let prepared = prepare_class_world();
    let mut h = census_harness(9, &prepared.path());
    let spec = Spec {
        id: 267,
        class: 9,
        spells: Vec::new(),
    };
    let row = Spell {
        id: 196447,
        passive: false,
        origin: vec!["talent".into()],
    };
    let receipt = record_case(&mut h, &spec, &row);
    let prerequisite = &receipt["caster_aura_prerequisite"];
    assert_eq!(prerequisite["required"], 228312, "{receipt}");
    let event = &prerequisite["events"][0];
    assert_eq!(event["spell"], 348, "{receipt}");
    assert_eq!(event["failed"], json!([]), "{receipt}");
    assert!(!event["go"].as_array().unwrap().is_empty(), "{receipt}");
    assert!(
        event["combat_log"]
            .as_array()
            .unwrap()
            .iter()
            .any(|log| log["spell_id"] == 348),
        "{receipt}"
    );
    // Do not force the enabling aura if the actual Immolate path omits it.
    let observed = crate::auras::has_aura(h.app.world(), h.caster, 228312);
    assert_eq!(prerequisite["required_after"], observed, "{receipt}");
}

#[test]
fn class_completeness_probe_fixture_refresh_distinguishes_absent_native_definition() {
    let prepared = prepare_class_world();
    let mut h = census_harness(8, &prepared.path());
    let spec = Spec {
        id: 62,
        class: 8,
        spells: Vec::new(),
    };
    let row = Spell {
        id: 30482,
        passive: true,
        origin: vec!["SkillLineAbility".into()],
    };
    let receipt = record_case(&mut h, &spec, &row);
    assert_eq!(receipt["missing_store"], true, "{receipt}");
    assert_eq!(receipt["source_rows"]["spell_name"], 0, "{receipt}");
    assert_eq!(
        receipt["source_rows"]["tdb_serverside_spell"], 0,
        "{receipt}"
    );
    assert_eq!(receipt["source_rows"]["spell_effect"], 0, "{receipt}");
}

#[test]
#[ignore = "explicit read-only census with manifest/output paths; not a full suite"]
fn class_completeness_level80_probe() {
    let input = std::env::var("CLASS_COMPLETENESS_MANIFEST").expect("manifest path");
    let output = std::env::var("CLASS_COMPLETENESS_OUTPUT").expect("output path");
    let manifest: Manifest =
        serde_json::from_str(&std::fs::read_to_string(input).unwrap()).unwrap();
    let mut file = std::fs::File::create(output).unwrap();
    let prepared = prepare_class_world();
    let mut records = 0;
    for spec in &manifest.specs {
        let mut h = census_harness(spec.class, &prepared.path());
        let retained: HashSet<_> = h
            .app
            .world_mut()
            .query::<Entity>()
            .iter(h.app.world())
            .filter(|entity| *entity != h.caster && *entity != h.npc)
            .collect();
        despawn_case(&mut h, &retained);
        for spell in &spec.spells {
            let record = record_case(&mut h, spec, spell);
            writeln!(file, "{record}").unwrap();
            despawn_case(&mut h, &retained);
            records += 1;
        }
        file.flush().unwrap();
        println!(
            "spec {}: {} native spell cases recorded",
            spec.id,
            spec.spells.len()
        );
    }
    assert_eq!(
        records,
        manifest
            .specs
            .iter()
            .map(|spec| spec.spells.len())
            .sum::<usize>()
    );
}

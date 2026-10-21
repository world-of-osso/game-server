//! Completed immutable producer inputs; every installation clones app-owned resources.
use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type InstallResourceCopy = Box<dyn Fn(&mut App) + Send + Sync>;

#[derive(Default)]
pub(super) struct PreparedInputs {
    copies: Vec<InstallResourceCopy>,
    spawn: Option<CreatureSpawn>,
}

impl PreparedInputs {
    pub(super) fn push<T: Resource + Clone>(&mut self, resource: T) {
        self.copies.push(Box::new(move |app| {
            app.insert_resource(resource.clone());
        }));
    }

    pub(super) fn authored_spawn(&self, difficulty: u32) -> CreatureSpawn {
        let mut spawn = self.spawn.clone().expect("decoded authored fixture spawn");
        spawn.spawn_difficulties = vec![difficulty];
        spawn
    }

    pub(super) fn install(&self, app: &mut App) {
        for copy in &self.copies {
            copy(app);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct InputKey {
    pub producer: &'static str,
    pub entry: u32,
    pub map: u32,
    pub populate: bool,
}

pub(super) fn cached_inputs(
    key: InputKey,
    load: impl FnOnce() -> PreparedInputs,
) -> Arc<PreparedInputs> {
    static INPUTS: OnceLock<Mutex<HashMap<InputKey, Arc<PreparedInputs>>>> = OnceLock::new();
    let mut inputs = INPUTS.get_or_init(Default::default).lock().unwrap();
    inputs
        .entry(key)
        .or_insert_with(|| Arc::new(load()))
        .clone()
}

pub(super) fn encounter_inputs(entry: u32, map_id: u32, populate: bool) -> Arc<PreparedInputs> {
    let producer = match entry {
        234648 => Some("kystia"),
        135322 => Some("golden-serpent"),
        ADAROGG => Some("adarogg"),
        SLAGMAW => Some("slagmaw"),
        13282 => Some("noxxion"),
        12225 => Some("celebras"),
        12201 => Some("theradras"),
        17306 => Some("gargolmar"),
        17308 => Some("omor"),
        16807 => Some("nethekurse"),
        16808 => Some("kargath"),
        20923 => Some("porung"),
        186737 => Some("telash"),
        186739 => Some("azureblade"),
        186738 => Some("umbrelskul"),
        196482 => Some("ancient"),
        190609 => Some("doragosa"),
        188252 => Some("melidrussa"),
        189232 => Some("kokia"),
        184580 | 184581 | 184582 => Some("lost-dwarves"),
        _ => None,
    };
    match producer {
        Some(producer) => cached_inputs(
            InputKey {
                producer,
                entry,
                map: map_id,
                populate,
            },
            || {
                let (_directory, path) = prepare_encounter_database(producer);
                decode_encounter_inputs(entry, map_id, populate, &path, true)
            },
        ),
        None => Arc::new(decode_encounter_inputs(
            entry,
            map_id,
            populate,
            &world_db_path(),
            false,
        )),
    }
}

fn decode_encounter_inputs(
    entry: u32,
    map_id: u32,
    populate: bool,
    path: &std::path::Path,
    prepared: bool,
) -> PreparedInputs {
    let data = data().expect("world.db fixture");
    let mut inputs = PreparedInputs::default();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    if [
        20923, 196482, 190609, 188252, 189232, 184580, 184581, 184582,
    ]
    .contains(&entry)
    {
        // The authenticated primary binding belongs to this prepared copy, not canonical data.
        inputs.push(crate::maps::store::load_map_store(&conn).unwrap());
    }
    inputs.push(crate::conditions::load_spell_target_conditions(&path).unwrap());
    inputs.push(crate::scripted_ai::kings_rest::load_molten_gold_areas(&conn).unwrap());
    inputs.push(crate::scripted_ai::kings_rest::load_gust_slash_shape(&conn).unwrap());
    inputs.push(crate::spell_cast::area_trigger::load_aura_field_catalog(&conn).unwrap());
    inputs.push(crate::scripted_ai::ragefire_chasm::intro_triggers::load_triggers(&conn).unwrap());
    inputs.push(crate::scripted_ai::nethekurse_intro::load_trigger(&conn).unwrap());
    inputs.push(crate::scripted_ai::kings_rest::load_gold_model_reaches(&conn).unwrap());
    inputs.push(crate::scripted_ai::waypoint_path::load_script_paths(&conn).unwrap());
    inputs.push(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    inputs.push(crate::spell_cast::teleport_positions::load_positions(&conn).unwrap());
    // Decode imported authority before caching; never substitute canonical templates.
    let canonical = (!prepared).then(crate::test_data::world_data);
    let expected = canonical.map_or_else(
        || crate::world_data::load_expected_stats(&conn),
        |data| data.expected_stats.clone(),
    );
    let spawns: Vec<_> = if populate {
        crate::world_data::load_spawns(&conn)
            .into_iter()
            .filter(|spawn| u32::from(spawn.map) == map_id)
            .collect()
    } else {
        Vec::new()
    };
    let mut templates = canonical.map_or_else(
        || crate::world_data::load_templates(&conn, &expected),
        |data| data.templates.clone(),
    );
    templates.retain(|id, _| {
        *id == entry
            || (entry == 234648 && [234660, 255050].contains(id))
            || (entry == 135322 && [135406, 137020].contains(id))
            || (entry == ADAROGG && *id == DEMONIC_LEAP)
            || (entry == 7800 && *id == 7915)
            || (entry == 13282 && *id == 13456)
            || (entry == 12225 && [13743, 13716].contains(id))
            || (entry == 12203 && *id == 11783)
            || (entry == 12201 && *id == 12238)
            || (entry == 17306 && *id == 17309)
            || (entry == 17308 && *id == 17540)
            || (entry == 16807 && [17471, 18370].contains(id))
            || (entry == 16808 && [17611, 17695, 20709].contains(id))
            || (entry == 17381 && *id == 17370)
            || (entry == 186737 && *id == 197204)
            || (entry == 186739 && *id == 190187)
            || ([184580, 184581, 184582].contains(&entry) && [184580, 184581, 184582].contains(id))
            || (map_id == 43 && [3636, 3654, 3678, 3679, 5048, 5762, 5763].contains(id))
            || spawns.iter().any(|spawn| spawn.template_entry == *id)
    });
    let difficulty_templates = if let Some(data) = canonical {
        data.difficulty_templates
            .iter()
            .filter(|((entry, _), _)| templates.contains_key(entry))
            .map(|(key, template)| (*key, template.clone()))
            .collect()
    } else {
        crate::world_data::load_template_models(&conn, &mut templates);
        crate::world_data::load_difficulty_templates(&conn, &expected, &templates)
    };
    let hp = conn
        .prepare("SELECT level,class,basehp0,basehp1,basehp2 FROM creature_classlevelstats")
        .unwrap()
        .query_map([], |row| {
            Ok((
                (row.get(0)?, row.get(1)?),
                [row.get(2)?, row.get(3)?, row.get(4)?],
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let (template_string_ids, spawn_string_ids, script_spawn_groups, reset_spawn_groups) =
        if populate {
            (
                crate::world_data::load_template_string_ids(&conn).unwrap(),
                crate::world_data::load_spawn_string_ids(&conn).unwrap(),
                crate::world_data::load_script_spawn_groups(
                    &conn,
                    crate::scripted_ai::SCRIPT_SPAWN_GROUPS,
                    true,
                )
                .unwrap(),
                crate::world_data::load_script_spawn_groups(&conn, &[357], false).unwrap(),
            )
        } else {
            Default::default()
        };
    inputs.push(WorldData {
        templates,
        difficulty_templates,
        spawns_by_map: crate::world_data::build_spawns_by_map(&spawns),
        spawns,
        template_string_ids,
        spawn_string_ids,
        script_spawn_groups,
        reset_spawn_groups,
        // Nethekurse's source reactions pause the real default patrol. The
        // fissure-only producer leaves this canonical path catalog unchanged.
        waypoints: if entry == 16807 && populate {
            crate::test_data::world_data().waypoints.clone()
        } else {
            Default::default()
        },
        // The Azure intro depends on the real non-following tender formation.
        formations: if populate && map_id == 2515 {
            crate::test_data::world_data().formations.clone()
        } else {
            Default::default()
        },
        addons: crate::world_data::load_spawn_addons(&conn),
        template_addons: crate::world_data::load_template_addons(&conn),
        // Creature::LoadEquipment at spawn needs the canonical equip templates.
        equipment: canonical
            .map(|data| data.equipment.clone())
            .unwrap_or_default(),
        class_level_base_hp: hp,
        class_level_base_mana: data.base_mana.clone(),
        expected_stats: expected,
        summon_levels: crate::world_data::summon_levels::SummonLevels::load(&conn).unwrap(),
        jump_charge_params: crate::spell_cast::jump_charge::load_params(&conn).unwrap(),
        ..Default::default()
    });
    let mut scripts = smart_ai::load_smart_scripts(&path).unwrap();
    if [3674, 3669, 3670].contains(&entry) {
        // Pin the successful branch of the authored80% chance; keep the
        // selected source timer, spell and target. Production is unchanged.
        for row in std::sync::Arc::make_mut(scripts.0.get_mut(&entry).unwrap()) {
            if row.action_type == 11 && row.chance == 80 {
                row.chance = 100;
            }
        }
    }
    // Use production's complete DB2 store, including absence of legacy SAI IDs.
    // A script's unknown spell must fail normal admission, not panic the fixture.
    let all = crate::spell_cast::test_harness::world_db().unwrap();
    let mut store = all.store.clone();
    if [17306, 17308].contains(&entry) || store.get(119297).is_none() {
        // Production's loader over the prepared copy's server-side tables.
        store.append_serverside(&conn).unwrap();
    }
    inputs.push(store);
    inputs.push(scripts);
    if !populate {
        inputs.spawn = Some(load_authored_spawn(&conn, entry, map_id, 0));
    }
    inputs
}

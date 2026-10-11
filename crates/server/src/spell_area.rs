//! Authored TDB spell_area eligibility (TC a352b1fa SpellMgr/Player).
use crate::auras::{
    ActiveAuras, AuraApplication, AuraSource, RemoveReason, apply_aura, aura_effect_mask,
    remove_auras,
};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Rule {
    area: u32,
    start: u32,
    end: u32,
    aura: i32,
    race_mask: u64,
    gender: u8,
    flags: u8,
    start_status: u32,
    end_status: u32,
}

#[derive(Resource, Default)]
struct SpellAreas(HashMap<u32, Vec<Rule>>);

fn load(conn: &Connection) -> rusqlite::Result<SpellAreas> {
    let mut statement = conn.prepare("SELECT spell, area, quest_start, quest_end, aura_spell, racemask, gender, flags, quest_start_status, quest_end_status FROM tdb_spell_area")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            Rule {
                area: row.get(1)?,
                start: row.get(2)?,
                end: row.get(3)?,
                aura: row.get(4)?,
                race_mask: row.get::<_, i64>(5)? as u64,
                gender: row.get(6)?,
                flags: row.get(7)?,
                start_status: row.get(8)?,
                end_status: row.get(9)?,
            },
        ))
    })?;
    let mut areas = SpellAreas::default();
    for row in rows {
        let (spell, rule) = row?;
        areas.0.entry(spell).or_default().push(rule);
    }
    Ok(areas)
}

#[derive(Component, Clone, PartialEq, Eq)]
struct Inputs {
    area: u32,
    zone: u32,
    race: u8,
    gender: u8,
    statuses: HashMap<u32, u8>,
}

impl Rule {
    fn in_area(&self, inputs: &Inputs) -> bool {
        self.area == 0 || self.area == inputs.area || self.area == inputs.zone
    }

    fn fits(&self, inputs: &Inputs, auras: &[u32]) -> bool {
        let status_matches = |quest, mask| {
            quest == 0 || mask & (1 << inputs.statuses.get(&quest).copied().unwrap_or(0)) != 0
        };
        self.in_area(inputs)
            && (self.gender == 2 || self.gender == inputs.gender)
            && (self.race_mask == 0
                || inputs
                    .race
                    .checked_sub(1)
                    .and_then(|bit| 1u64.checked_shl(u32::from(bit)))
                    .is_some_and(|bit| self.race_mask & bit != 0))
            && status_matches(self.start, self.start_status)
            && status_matches(self.end, self.end_status)
            && (self.aura == 0
                || if self.aura > 0 {
                    auras.contains(&(self.aura as u32))
                } else {
                    !auras.contains(&self.aura.unsigned_abs())
                })
    }
}

fn reconcile(world: &mut World, player: Entity, inputs: &Inputs) {
    let area_changed = world
        .get::<Inputs>(player)
        .is_none_or(|old| old.area != inputs.area || old.zone != inputs.zone);
    let active: Vec<u32> = world
        .get::<ActiveAuras>(player)
        .map_or_else(Vec::new, |auras| auras.iter().map(|a| a.spell_id).collect());
    let actions: Vec<_> = world
        .resource::<SpellAreas>()
        .0
        .iter()
        .map(|(&spell, rules)| {
            let eligible: Vec<_> = rules
                .iter()
                .filter(|rule| rule.fits(inputs, &active))
                .collect();
            let apply = eligible
                .iter()
                .any(|rule| rule.flags & 1 != 0 && (area_changed || rule.flags & 4 == 0));
            let remove = eligible.is_empty()
                && rules
                    .iter()
                    .any(|rule| !rule.in_area(inputs) || rule.flags & 2 != 0);
            (spell, apply, remove)
        })
        .collect();
    for (spell_id, apply, remove) in actions {
        if remove {
            remove_auras(world, player, RemoveReason::Cancelled, |aura| {
                aura.spell_id == spell_id
            });
        } else if apply && !active.contains(&spell_id) {
            let spell = world
                .resource::<SpellStore>()
                .get(spell_id)
                .cloned()
                .expect("spell_area spell missing from spell catalog");
            apply_aura(
                world,
                &AuraApplication {
                    caster: player,
                    target: player,
                    spell_id,
                    effect_mask: aura_effect_mask(&spell),
                    source: AuraSource::SpellArea,
                },
            );
        }
    }
    world.entity_mut(player).insert(inputs.clone());
}

pub struct SpellAreaPlugin;
impl Plugin for SpellAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_startup).add_systems(
            PostUpdate,
            sync.after(crate::quest::mark_quest_inputs_changed)
                .before(crate::quest::send_quest_updates),
        );
    }
}

fn load_startup(mut commands: Commands) {
    let conn = Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open spell_area world database");
    let areas = load(&conn)
        .expect("Import TDB spell_area through mysql_dump.py --prefix tdb_ before starting server");
    info!(
        "Loaded {} spell_area spell requirement groups",
        areas.0.len()
    );
    commands.insert_resource(areas);
}

fn sync(world: &mut World) {
    use crate::auth::InWorld;
    use crate::maps::InMap;
    use crate::networking::PlayerLink;
    use shared::components::{Player, Position};
    let connections: Vec<_> = world
        .query::<(&PlayerLink, &InWorld)>()
        .iter(world)
        .map(|(link, state)| (link.0, state.character_id))
        .collect();
    for (entity, character) in connections {
        let Some((player, position, map)) = world
            .query::<(&Player, Ref<Position>, &InMap)>()
            .get(world, entity)
            .ok()
            .filter(|(_, position, _)| {
                position.is_changed()
                    || world.get::<Inputs>(entity).is_none()
                    || world
                        .resource::<crate::quest::QuestDirty>()
                        .0
                        .contains(&character)
                    || world
                        .entity(entity)
                        .get_ref::<ActiveAuras>()
                        .is_some_and(|auras| auras.is_changed())
            })
            .map(|(p, position, map)| (p.clone(), *position, *map))
        else {
            continue;
        };
        let area = world
            .resource_mut::<crate::ground::GroundMap>()
            .area_at(map.map_id, Vec3::new(position.x, position.y, position.z))
            .unwrap_or(0);
        let zone = world
            .resource::<crate::area_table::AreaTable>()
            .zone_id(area, map.map_id)
            .unwrap_or(0);
        let database = &world.resource::<crate::persistence::GameDatabase>().db;
        let log = crate::persistence::quest_store::load_quest_log(database, character);
        let inventory = crate::persistence::load_inventory(database, character);
        let quests = world.resource::<crate::quest_data::QuestDb>();
        let mut statuses = HashMap::new();
        for quest in &log.active {
            let status =
                if crate::quest_rules::is_complete_in_log(quests, &log, quest.quest_id, |item| {
                    crate::quest_view::bag_count(database, &inventory, item)
                }) {
                    1
                } else {
                    3
                };
            statuses.insert(quest.quest_id, status);
        }
        for quest in &log.rewarded {
            statuses.insert(*quest, 6);
        }
        reconcile(
            world,
            entity,
            &Inputs {
                area,
                zone,
                race: player.race,
                gender: player.appearance.sex,
                statuses,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spell_area_jaga_authored_position_resolves_required_area() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let maps = crate::maps::store::load_map_store(&conn).unwrap();
        let wdts = maps
            .maps
            .values()
            .map(|entry| (entry.id, entry.wdt_file_data_id))
            .collect();
        let mut ground = crate::ground::GroundMap::new(&crate::ground::ground_data_dir(), wdts);
        assert_eq!(
            ground.area_at(1, Vec3::new(1945.5, 24.1249, 4343.38)),
            Some(5171)
        );
    }

    #[test]
    fn spell_area_ignore_quest_change_still_autocasts_on_aura_change() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../../scripts/spell_area_schema.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO tdb_spell_area VALUES (100616,5171,29401,0,89305,0,2,5,8,0)",
            [],
        )
        .unwrap();
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[100616, 89305], 1).unwrap();
        h.app.insert_resource(load(&conn).unwrap());
        let mut inputs = Inputs {
            area: 5171,
            zone: 1637,
            race: 2,
            gender: 0,
            statuses: HashMap::new(),
        };
        reconcile(h.app.world_mut(), h.caster, &inputs);
        inputs.statuses.insert(29401, 3);
        reconcile(h.app.world_mut(), h.caster, &inputs);
        assert!(
            !h.app
                .world()
                .get::<ActiveAuras>(h.caster)
                .is_some_and(|auras| auras.iter().any(|a| a.spell_id == 100616))
        );
        let spell = h.app.world().resource::<SpellStore>().get(89305).unwrap();
        let mask = aura_effect_mask(spell);
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: 89305,
                effect_mask: mask,
                source: AuraSource::Cast,
            },
        );
        reconcile(h.app.world_mut(), h.caster, &inputs);
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(h.caster)
                .is_some_and(|auras| auras.iter().any(|a| a.spell_id == 100616)),
            "ignore-quest-status-change does not disable aura-driven eligibility"
        );
    }

    #[test]
    fn spell_area_balloon_eligibility_applies_and_removes_real_detection() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../../scripts/spell_area_schema.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO tdb_spell_area VALUES (100616,5171,0,29401,0,0,2,3,0,43)",
            [],
        )
        .unwrap();
        let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[100616], 1).unwrap();
        h.app.insert_resource(load(&conn).unwrap());
        let mut inputs = Inputs {
            area: 5171,
            zone: 1637,
            race: 2,
            gender: 0,
            statuses: HashMap::new(),
        };
        reconcile(h.app.world_mut(), h.caster, &inputs);
        let has = |world: &World| {
            world
                .get::<ActiveAuras>(h.caster)
                .is_some_and(|auras| auras.iter().any(|a| a.spell_id == 100616))
        };
        assert!(
            has(h.app.world()),
            "unaccepted quest status0 is eligible (mask43)"
        );
        inputs.statuses.insert(29401, 3); // incomplete
        reconcile(h.app.world_mut(), h.caster, &inputs);
        assert!(has(h.app.world()));
        inputs.statuses.insert(29401, 1); // complete, not rewarded
        reconcile(h.app.world_mut(), h.caster, &inputs);
        assert!(has(h.app.world()));
        inputs.statuses.insert(29401, 6); // rewarded
        reconcile(h.app.world_mut(), h.caster, &inputs);
        assert!(!has(h.app.world()), "reward removes the authored detector");
        inputs.statuses.clear();
        inputs.area = 1;
        reconcile(h.app.world_mut(), h.caster, &inputs);
        assert!(!has(h.app.world()), "wrong area does not grant detector");
    }
}

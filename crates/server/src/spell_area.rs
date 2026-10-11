//! Authored TDB spell_area eligibility (TC a352b1fa SpellMgr/Player).
use crate::auras::{
    ActiveAuras, AuraApplication, AuraSource, RemoveReason, apply_aura, aura_effect_mask,
    remove_auras,
};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::HashMap;

const AUTOCAST: u8 = 1;
const AUTOREMOVE: u8 = 2;
const IGNORE_QUEST_CHANGE: u8 = 4;
const GENDER_ANY: u8 = 2;
const QUEST_COMPLETE: u8 = 1;
const QUEST_INCOMPLETE: u8 = 3;
const QUEST_REWARDED: u8 = 6;

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
        self.in_area(inputs)
            && (self.gender == GENDER_ANY || self.gender == inputs.gender)
            && race_matches(self.race_mask, inputs.race)
            && status_matches(inputs, self.start, self.start_status)
            && status_matches(inputs, self.end, self.end_status)
            && aura_matches(self.aura, auras)
    }
}

fn race_matches(mask: u64, race: u8) -> bool {
    let race_bit = race
        .checked_sub(1)
        .and_then(|bit| 1u64.checked_shl(u32::from(bit)));
    mask == 0 || race_bit.is_some_and(|bit| mask & bit != 0)
}

fn status_matches(inputs: &Inputs, quest: u32, mask: u32) -> bool {
    let status = inputs.statuses.get(&quest).copied().unwrap_or(0);
    quest == 0 || mask & (1 << status) != 0
}

fn aura_matches(required: i32, auras: &[u32]) -> bool {
    match required.cmp(&0) {
        std::cmp::Ordering::Equal => true,
        std::cmp::Ordering::Greater => auras.contains(&(required as u32)),
        std::cmp::Ordering::Less => !auras.contains(&required.unsigned_abs()),
    }
}

fn reconcile(world: &mut World, player: Entity, inputs: &Inputs) {
    let previous = world.get::<Inputs>(player);
    let quest_only_change = previous.is_some_and(|old| {
        old.area == inputs.area && old.zone == inputs.zone && old.statuses != inputs.statuses
    });
    let active: Vec<u32> = world
        .get::<ActiveAuras>(player)
        .map_or_else(Vec::new, |auras| auras.iter().map(|a| a.spell_id).collect());
    let changes: Vec<_> = world
        .resource::<SpellAreas>()
        .0
        .iter()
        .filter_map(|(&spell, rules)| {
            decide_change(spell, rules, inputs, &active, quest_only_change)
        })
        .collect();
    for change in changes {
        apply_change(world, player, change);
    }
    if world.get::<Inputs>(player) != Some(inputs) {
        world.entity_mut(player).insert(inputs.clone());
    }
}

enum AuraChange {
    Apply(u32),
    Remove(u32),
}

fn decide_change(
    spell: u32,
    rules: &[Rule],
    inputs: &Inputs,
    active: &[u32],
    quest_only_change: bool,
) -> Option<AuraChange> {
    let eligible: Vec<_> = rules
        .iter()
        .filter(|rule| rule.fits(inputs, active))
        .collect();
    let remove = eligible.is_empty()
        && rules
            .iter()
            .any(|rule| !rule.in_area(inputs) || rule.flags & AUTOREMOVE != 0);
    if remove && active.contains(&spell) {
        return Some(AuraChange::Remove(spell));
    }
    let autocast = eligible.iter().any(|rule| {
        rule.flags & AUTOCAST != 0 && (!quest_only_change || rule.flags & IGNORE_QUEST_CHANGE == 0)
    });
    if autocast && !active.contains(&spell) {
        Some(AuraChange::Apply(spell))
    } else {
        None
    }
}

fn apply_change(world: &mut World, player: Entity, change: AuraChange) {
    match change {
        AuraChange::Remove(spell_id) => {
            remove_auras(world, player, RemoveReason::Cancelled, |aura| {
                aura.spell_id == spell_id
            });
        }
        AuraChange::Apply(spell_id) => {
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
    use crate::networking::PlayerLink;
    let connections: Vec<_> = world
        .query::<(&PlayerLink, &InWorld)>()
        .iter(world)
        .map(|(link, state)| (link.0, state.character_id))
        .collect();
    for (entity, character) in connections {
        if let Some(inputs) = load_changed_inputs(world, entity, character) {
            reconcile(world, entity, &inputs);
        }
    }
}

fn inputs_changed(world: &World, entity: Entity, character: u64, moved: bool) -> bool {
    moved
        || world.get::<Inputs>(entity).is_none()
        || world
            .resource::<crate::quest::QuestDirty>()
            .0
            .contains(&character)
        || world
            .entity(entity)
            .get_ref::<ActiveAuras>()
            .is_some_and(|auras| auras.is_changed())
}

fn load_changed_inputs(world: &mut World, entity: Entity, character: u64) -> Option<Inputs> {
    use shared::components::{Player, Position};
    let mut players = world.query::<(&Player, Ref<Position>, &crate::maps::InMap)>();
    let (player, position, map) = players.get(world, entity).ok()?;
    if !inputs_changed(world, entity, character, position.is_changed()) {
        return None;
    }
    let (race, gender, position, map) = (player.race, player.appearance.sex, *position, *map);
    let area = world
        .resource_mut::<crate::ground::GroundMap>()
        .area_at(map.map_id, Vec3::new(position.x, position.y, position.z))
        .unwrap_or(0);
    let zone = world
        .resource::<crate::area_table::AreaTable>()
        .zone_id(area, map.map_id)
        .unwrap_or(0);
    Some(Inputs {
        area,
        zone,
        race,
        gender,
        statuses: load_quest_statuses(world, character),
    })
}

fn load_quest_statuses(world: &World, character: u64) -> HashMap<u32, u8> {
    let database = &world.resource::<crate::persistence::GameDatabase>().db;
    let log = crate::persistence::quest_store::load_quest_log(database, character);
    let inventory = crate::persistence::load_inventory(database, character);
    let quests = world.resource::<crate::quest_data::QuestDb>();
    let active = log.active.iter().map(|quest| {
        let complete =
            crate::quest_rules::is_complete_in_log(quests, &log, quest.quest_id, |item| {
                crate::quest_view::bag_count(database, &inventory, item)
            });
        (
            quest.quest_id,
            if complete {
                QUEST_COMPLETE
            } else {
                QUEST_INCOMPLETE
            },
        )
    });
    let rewarded = log.rewarded.iter().map(|quest| (*quest, QUEST_REWARDED));
    active.chain(rewarded).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spell_area_requirements_respect_zone_race_gender_and_signed_auras() {
        let mut rule = Rule {
            area: 1637,
            start: 0,
            end: 0,
            aura: 89305,
            race_mask: 2,
            gender: 1,
            flags: 3,
            start_status: 0,
            end_status: 0,
        };
        let mut inputs = Inputs {
            area: 5171,
            zone: 1637,
            race: 2,
            gender: 1,
            statuses: HashMap::new(),
        };
        assert!(rule.fits(&inputs, &[89305]), "zone matches a child area");
        inputs.race = 1;
        assert!(!rule.fits(&inputs, &[89305]));
        inputs.race = 2;
        inputs.gender = 0;
        assert!(!rule.fits(&inputs, &[89305]));
        inputs.gender = 1;
        assert!(!rule.fits(&inputs, &[]), "positive prerequisite must exist");
        rule.aura = -89305;
        assert!(rule.fits(&inputs, &[]));
        assert!(
            !rule.fits(&inputs, &[89305]),
            "negative prerequisite must be absent"
        );
        inputs.zone = 0;
        assert!(!rule.fits(&inputs, &[]));
    }

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

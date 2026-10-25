//! Local Retail main-stage progression. Criteria semantics belong to achievement::catalog.
use super::*;
use rusqlite::Connection;
use shared::protocol::{InstanceChannel, ScenarioProgress, ScenarioStage};
use std::collections::HashMap;

#[cfg(test)]
#[path = "scenario_step82_tests.rs"]
mod step82_tests;
#[cfg(test)]
#[path = "scenario_tests.rs"]
mod tests;

#[derive(Clone, Debug)]
pub(super) struct ScenarioStep {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub root: u32,
}

#[derive(Clone, Debug)]
pub(super) struct ScenarioDefinition {
    pub id: u32,
    pub name: String,
    pub kind: u32,
    pub flags: u32,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Resource, Default)]
pub(crate) struct ScenarioCatalog {
    definitions: HashMap<u32, ScenarioDefinition>,
    bindings: HashMap<(u32, u32), (u32, u32)>,
}

#[derive(Default)]
pub(super) struct ScenarioRun {
    step: usize,
    pub criteria: PlayerAchievementState,
}

impl ScenarioRun {
    pub(super) fn apply(
        &mut self,
        def: &ScenarioDefinition,
        catalog: &AchievementCatalog,
        event: CriteriaEvent,
        conditions: &PlayerConditions,
    ) -> bool {
        let Some(step) = def.steps.get(self.step) else {
            return false;
        };
        if !catalog.update_tree_event(step.root, &mut self.criteria, event, conditions) {
            return false;
        }
        if catalog.tree_progress(step.root, &self.criteria).completed {
            self.step += 1;
        }
        true
    }

    pub(super) fn snapshot(
        &self,
        def: &ScenarioDefinition,
        catalog: &AchievementCatalog,
    ) -> ScenarioStage {
        let step = def.steps.get(self.step);
        let criteria = step.map_or_else(Vec::new, |step| {
            catalog
                .criterion_tree_ids(step.root)
                .into_iter()
                .map(|id| catalog.criterion_line(&self.criteria, id, true))
                .collect()
        });
        ScenarioStage {
            scenario_id: def.id,
            name: def.name.clone(),
            kind: def.kind,
            flags: def.flags,
            step_id: step.map_or(0, |s| s.id),
            stage: self.step as u32 + 1,
            num_stages: def.steps.len() as u32,
            title: step.map_or_else(String::new, |s| s.title.clone()),
            description: step.map_or_else(String::new, |s| s.description.clone()),
            completed: step.is_none(),
            criteria,
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct ScenarioRuns(HashMap<InMap, (u32, ScenarioRun)>);

#[derive(Component)]
struct LastScenarioMap(InMap);

pub(crate) fn load_scenarios(
    conn: &Connection,
    criteria: &AchievementCatalog,
) -> rusqlite::Result<ScenarioCatalog> {
    let mut catalog = ScenarioCatalog::default();
    let mut query = conn.prepare("SELECT ID, Name_lang, Type, Flags FROM scenario ORDER BY ID")?;
    let rows = query.query_map([], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, u32>(2)?,
            r.get::<_, u32>(3)?,
        ))
    })?;
    for row in rows {
        let (id, name, kind, flags) = row?;
        let steps = load_steps(conn, id)?;
        if !steps.is_empty()
            && steps
                .iter()
                .all(|s| criteria.supports_tree(s.root, &mut BTreeSet::new()))
        {
            catalog.definitions.insert(
                id,
                ScenarioDefinition {
                    id,
                    name,
                    kind,
                    flags,
                    steps,
                },
            );
        }
    }
    let mut query =
        conn.prepare("SELECT map, difficulty, scenario_A, scenario_H FROM scenario_map")?;
    catalog.bindings = query
        .query_map([], |r| Ok(((r.get(0)?, r.get(1)?), (r.get(2)?, r.get(3)?))))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(catalog)
}

fn load_steps(conn: &Connection, scenario: u32) -> rusqlite::Result<Vec<ScenarioStep>> {
    let mut query = conn.prepare("SELECT ID, Title_lang, Description_lang, CriteriatreeID, RelatedStep, Supersedes, VisibilityPlayerConditionID, WidgetSetID FROM scenario_step WHERE ScenarioID=?1 AND (Flags & 1)=0 ORDER BY OrderIndex, ID")?;
    let rows: Vec<_> = query
        .query_map([scenario], |r| {
            Ok((
                ScenarioStep {
                    id: r.get(0)?,
                    title: catalog::bound_text(&r.get::<_, String>(1)?, 256),
                    description: catalog::bound_text(&r.get::<_, String>(2)?, 2048),
                    root: r.get(3)?,
                },
                [r.get::<_, u32>(4)?, r.get(5)?, r.get(6)?, r.get(7)?],
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    if rows
        .iter()
        .any(|(_, conditional)| conditional.iter().any(|value| *value != 0))
    {
        return Ok(Vec::new());
    }
    Ok(rows.into_iter().map(|(step, _)| step).collect())
}

fn definition_for_map(world: &World, map: InMap, race: u8) -> Option<u32> {
    let difficulty = world
        .get_resource::<crate::maps::InstanceManager>()?
        .copy_difficulty(map)?
        .difficulty_id;
    let scenarios = world.get_resource::<ScenarioCatalog>()?;
    let &(alliance, horde) = scenarios.bindings.get(&(map.map_id, difficulty))?;
    let achievements = world.get_resource::<AchievementCatalog>()?;
    let id = if alliance == horde || achievements.faction_matches(1, race) {
        alliance
    } else {
        horde
    };
    scenarios.definitions.contains_key(&id).then_some(id)
}

pub(super) fn send_entry_progress(world: &mut World) {
    let mut connections = world.query_filtered::<
        (Entity, &PlayerLink, Ref<InWorld>, Option<&LastScenarioMap>),
        With<Authenticated>,
    >();
    let entries: Vec<_> = connections
        .iter(world)
        .filter_map(|(connection, link, in_world, last)| {
            let map = *world.get::<InMap>(link.0)?;
            let race = world.get::<Player>(link.0)?.race;
            let changed_map = last.is_none_or(|last| last.0 != map);
            (in_world.is_added() || changed_map).then_some((connection, map, race))
        })
        .collect();
    for (connection, map, race) in entries {
        if let Some(id) = definition_for_map(world, map, race) {
            world
                .resource_mut::<ScenarioRuns>()
                .0
                .entry(map)
                .or_insert_with(|| (id, ScenarioRun::default()));
        }
        send_snapshot(world, connection, map);
        world.entity_mut(connection).insert(LastScenarioMap(map));
    }
}

/// Current authored step identity; the run's vector position is never an ordinal.
pub(super) fn current_step_id(world: &World, map: InMap) -> Option<u32> {
    let (id, run) = world.get_resource::<ScenarioRuns>()?.0.get(&map)?;
    let definition = world
        .get_resource::<ScenarioCatalog>()?
        .definitions
        .get(id)?;
    definition.steps.get(run.step).map(|step| step.id)
}

fn snapshot(world: &World, map: InMap) -> ScenarioProgress {
    let difficulty_id = world
        .get_resource::<crate::maps::InstanceManager>()
        .and_then(|instances| instances.copy_difficulty(map))
        .map_or(0, |entry| entry.difficulty_id);
    let scenario = world
        .get_resource::<ScenarioRuns>()
        .and_then(|runs| runs.0.get(&map))
        .and_then(|(id, run)| {
            let def = world
                .get_resource::<ScenarioCatalog>()?
                .definitions
                .get(id)?;
            let criteria = world.get_resource::<AchievementCatalog>()?;
            Some(run.snapshot(def, criteria))
        });
    ScenarioProgress {
        map_id: map.map_id,
        instance_id: map.instance_id,
        difficulty_id,
        scenario,
    }
}

fn send_snapshot(world: &mut World, connection: Entity, map: InMap) {
    let update = snapshot(world, map);
    if let Some(mut sender) = world.get_mut::<MessageSender<ScenarioProgress>>(connection) {
        sender.send::<InstanceChannel>(update);
    }
}

pub(super) fn record(world: &mut World, player: Entity, event: CriteriaEvent) {
    if let Some(map) = world.get::<InMap>(player).copied() {
        let conditions = event_conditions(world, player, &[event]);
        record_map(world, map, event, &conditions);
    }
}

pub(super) fn record_map(
    world: &mut World,
    map: InMap,
    event: CriteriaEvent,
    conditions: &PlayerConditions,
) {
    let Some(scenarios) = world.get_resource::<ScenarioCatalog>() else {
        return;
    };
    let Some(criteria) = world.get_resource::<AchievementCatalog>() else {
        return;
    };
    let Some(runs) = world.get_resource::<ScenarioRuns>() else {
        return;
    };
    let Some((id, run)) = runs.0.get(&map) else {
        return;
    };
    let def = &scenarios.definitions[id];
    // Detach run before mutation, avoiding aliases across World resources.
    let mut run = ScenarioRun {
        step: run.step,
        criteria: run.criteria.clone(),
    };
    let was_complete = run.step >= def.steps.len();
    if !run.apply(def, criteria, event, conditions) {
        return;
    }
    let completed = !was_complete && run.step >= def.steps.len();
    let id = *id;
    world
        .resource_mut::<ScenarioRuns>()
        .0
        .insert(map, (id, run));
    let connections: Vec<_> = world
        .query_filtered::<(Entity, &PlayerLink), (With<Authenticated>, With<InWorld>)>()
        .iter(world)
        .filter(|(_, link)| world.get::<InMap>(link.0) == Some(&map))
        .map(|(connection, _)| connection)
        .collect();
    for connection in connections {
        send_snapshot(world, connection, map);
    }
    if completed {
        super::record_scenario_completion(world, map, id);
    }
}

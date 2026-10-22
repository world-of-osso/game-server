//! Authored boss-mod schedules; the AI's combat timers are not alert authority.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::protocol::{EncounterSeverity, EncounterTimelineEvent, EncounterTimer};

use crate::maps::InMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Trigger {
    Engage,
    CastStart(u32),
    CastSuccess(u32),
    Health(f64),
}
#[derive(Debug, Clone)]
pub enum Action {
    Timer {
        spell_id: u32,
        duration_ms: u32,
    },
    Warning {
        spell_id: u32,
        severity: u32,
        text: String,
    },
    Phase(f64),
}
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: u32,
    pub trigger: Trigger,
    pub required_phase: Option<f64>,
    pub action: Action,
}
#[derive(Debug, Default, Component)]
pub struct Schedule {
    rules: Vec<Rule>,
    timers: HashMap<u32, (u32, u32)>,
    phase: f64,
    consumed_health: HashSet<u32>,
    publish_age_ms: u32,
}
impl Schedule {
    fn new(rules: Vec<Rule>) -> Self {
        Self {
            rules,
            phase: 1.0,
            ..Default::default()
        }
    }
    fn trigger(&mut self, event: Trigger) -> Vec<Action> {
        // Match against the incoming phase once: changing stage must not discard
        // a sibling timer in the same authored health/cast handler.
        let selected: Vec<Rule> = self
            .rules
            .iter()
            .filter(|rule| {
                rule.trigger == event
                    && rule.required_phase.is_none_or(|phase| phase == self.phase)
                    && !self.consumed_health.contains(&rule.id)
            })
            .cloned()
            .collect();
        for rule in &selected {
            if matches!(event, Trigger::Health(_)) {
                self.consumed_health.insert(rule.id);
            }
            match rule.action {
                Action::Timer {
                    spell_id,
                    duration_ms,
                } => {
                    self.timers.insert(spell_id, (duration_ms, duration_ms));
                }
                Action::Phase(phase) => self.phase = phase,
                Action::Warning { .. } => {}
            }
        }
        selected.into_iter().map(|rule| rule.action).collect()
    }
    fn tick(&mut self, diff_ms: u32) {
        self.timers.retain(|_, (remaining, _)| {
            *remaining = remaining.saturating_sub(diff_ms);
            *remaining > 0
        });
        self.publish_age_ms = self.publish_age_ms.saturating_add(diff_ms);
    }
    fn health_triggers(&self, pct: f64) -> Vec<Trigger> {
        let mut thresholds: Vec<f64> = self
            .rules
            .iter()
            .filter_map(|rule| match rule.trigger {
                Trigger::Health(threshold) if pct < threshold => Some(threshold),
                _ => None,
            })
            .collect();
        thresholds.sort_by(|left, right| right.total_cmp(left));
        thresholds.dedup();
        thresholds.into_iter().map(Trigger::Health).collect()
    }
    fn projected_timers(&self) -> Vec<EncounterTimer> {
        let mut timers: Vec<_> = self
            .timers
            .iter()
            .map(|(&spell_id, &(remaining_ms, duration_ms))| EncounterTimer {
                event_id: spell_id,
                spell_id,
                remaining_ms,
                duration_ms,
                conditional: true,
            })
            .collect();
        timers.sort_by_key(|timer| timer.spell_id);
        timers
    }
}

#[derive(Resource, Default)]
pub struct Catalog(HashMap<u32, Vec<Rule>>);

fn read_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<(u32, Rule)> {
    let trigger_name: String = row.get(2)?;
    let trigger = match trigger_name.as_str() {
        "engage" => Trigger::Engage,
        "cast_start" => Trigger::CastStart(row.get(3)?),
        "cast_success" => Trigger::CastSuccess(row.get(3)?),
        "health" => Trigger::Health(row.get(4)?),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let action_name: String = row.get(6)?;
    let action = match action_name.as_str() {
        "timer" => Action::Timer {
            spell_id: row.get(7)?,
            duration_ms: row.get(8)?,
        },
        "warning" => Action::Warning {
            spell_id: row.get(7)?,
            severity: row.get(10)?,
            text: row.get(11)?,
        },
        "phase" => Action::Phase(row.get(9)?),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok((
        row.get(1)?,
        Rule {
            id: row.get(0)?,
            trigger,
            required_phase: row.get(5)?,
            action,
        },
    ))
}

pub fn load_catalog(conn: &Connection) -> rusqlite::Result<Catalog> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='bossmod_authority')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        warn!(
            "Boss-mod catalog absent: no alert schedules; import scripts/bossmod_data.py into a private world copy"
        );
        return Ok(Catalog::default());
    }
    let mut statement = conn.prepare("SELECT r.id,r.encounter_id,r.trigger,r.trigger_spell_id,r.health_pct,r.required_phase,r.action,r.spell_id,r.delay_ms,r.phase,r.severity,r.warning_text FROM bossmod_rule r JOIN bossmod_authority a ON a.encounter_id=r.encounter_id AND a.addon=r.addon ORDER BY r.id")?;
    let mut catalog = HashMap::<u32, Vec<Rule>>::new();
    for row in statement.query_map([], read_rule)? {
        let (encounter, rule) = row?;
        catalog.entry(encounter).or_default().push(rule);
    }
    info!(
        "Loaded boss-mod schedule authority for {} encounters",
        catalog.len()
    );
    Ok(Catalog(catalog))
}

pub fn startup_load_catalog(mut commands: Commands, injected: Option<Res<Catalog>>) {
    // Process fixtures supply their catalog explicitly before Startup. Production
    // has no injected catalog and must open its world copy successfully.
    if injected.is_some() {
        return;
    }
    let conn = Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open private world database for boss-mod catalog");
    let catalog = load_catalog(&conn).expect("load imported boss-mod catalog");
    commands.insert_resource(catalog);
}

pub fn engage(world: &mut World, unit: Entity) {
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let Some(encounter) = super::instance_script::active_encounter_id(world, map, unit) else {
        return;
    };
    // Explicit current-Lua scripts publish their selected EventMap requests.
    // An imported (possibly legacy) clock must not overwrite those snapshots.
    if super::scripted_alerts::owns_clock(encounter) {
        return;
    }
    // TC script schedule (partial; Retail 12.1 timeline inputs unavailable).
    // Lead-authorized TC combat does not authorize Golden Retail alert clocks.
    if encounter == super::kings_rest::GOLDEN_SERPENT_ENCOUNTER {
        return;
    }
    let rules = world
        .get_resource::<Catalog>()
        .and_then(|catalog| catalog.0.get(&encounter))
        .cloned();
    let Some(rules) = rules else { return };
    let mut schedule = Schedule::new(rules);
    let actions = schedule.trigger(Trigger::Engage);
    world.entity_mut(unit).insert(schedule);
    publish_actions(world, map, unit, actions);
    publish_timers(world, map, unit);
}

pub fn disengage(world: &mut World, unit: Entity) {
    super::scripted_alerts::disengage(world, unit);
    world.entity_mut(unit).remove::<Schedule>();
}

pub fn cast_event(world: &mut World, event: &shared::protocol::CombatLogEvent) {
    super::scripted_alerts::cast_event(world, event);
    use shared::protocol::CombatLogKind;
    let Some(spell_id) = event.spell_id else {
        return;
    };
    let trigger = match event.kind {
        CombatLogKind::CastStart => Trigger::CastStart(spell_id),
        CombatLogKind::CastSuccess => Trigger::CastSuccess(spell_id),
        _ => return,
    };
    let Some(unit) = event.source.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let Some(mut schedule) = world.get_mut::<Schedule>(unit) else {
        return;
    };
    let actions = schedule.trigger(trigger);
    if actions.is_empty() {
        return;
    }
    publish_actions(world, map, unit, actions);
    publish_timers(world, map, unit);
}

fn severity(value: u32) -> EncounterSeverity {
    match value {
        0 => EncounterSeverity::Low,
        1 => EncounterSeverity::Medium,
        2 => EncounterSeverity::High,
        _ => panic!("invalid imported encounter severity {value}"),
    }
}

fn publish_actions(world: &mut World, map: InMap, unit: Entity, actions: Vec<Action>) {
    for action in actions {
        let message = match action {
            Action::Warning {
                spell_id,
                severity: value,
                text,
            } => EncounterTimelineEvent::Cast {
                caster: unit.to_bits(),
                spell_id,
                severity: severity(value),
                warning_text: text,
            },
            Action::Phase(phase) if phase.fract() == 0.0 && phase >= 0.0 => {
                EncounterTimelineEvent::Phase {
                    caster: unit.to_bits(),
                    phase: phase as u32,
                }
            }
            Action::Phase(_) | Action::Timer { .. } => continue,
        };
        super::instance_script::send_to_players(world, map, message);
    }
}

fn publish_timers(world: &mut World, map: InMap, unit: Entity) {
    let Some(mut schedule) = world.get_mut::<Schedule>(unit) else {
        return;
    };
    schedule.publish_age_ms = 0;
    let timers = schedule.projected_timers();
    super::instance_script::send_to_players(
        world,
        map,
        EncounterTimelineEvent::Timers {
            caster: unit.to_bits(),
            sample_unix_ms: shared::protocol::read_encounter_unix_ms(),
            timers,
        },
    );
}

fn advance_schedule(schedule: &mut Schedule, diff_ms: u32, pct: f64) -> (bool, Vec<Action>) {
    let before = schedule.timers.len();
    schedule.tick(diff_ms);
    let actions: Vec<Action> = schedule
        .health_triggers(pct)
        .into_iter()
        .flat_map(|trigger| schedule.trigger(trigger))
        .collect();
    let publish =
        schedule.publish_age_ms >= 1000 || !actions.is_empty() || before != schedule.timers.len();
    (publish, actions)
}

pub fn advance_schedules(world: &mut World) {
    let diff_ms = world.resource::<Time>().delta().as_millis() as u32;
    let units: Vec<_> = world
        .query_filtered::<(Entity, &InMap, &shared::components::Health), With<Schedule>>()
        .iter(world)
        .map(|(unit, map, health)| (unit, *map, f64::from(health.current / health.max * 100.0)))
        .collect();
    for (unit, map, pct) in units {
        let mut schedule = world
            .get_mut::<Schedule>(unit)
            .expect("queried boss-mod schedule");
        let (publish, actions) = advance_schedule(&mut schedule, diff_ms, pct);
        if publish {
            publish_actions(world, map, unit, actions);
            publish_timers(world, map, unit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cookie() -> Schedule {
        // LittleWigs Classic/Deadmines/Cookie.lua:36,65: Retail Acid Splash.
        Schedule::new(vec![
            Rule {
                id: 1,
                trigger: Trigger::Engage,
                required_phase: None,
                action: Action::Timer {
                    spell_id: 6306,
                    duration_ms: 2400,
                },
            },
            Rule {
                id: 2,
                trigger: Trigger::CastStart(6306),
                required_phase: None,
                action: Action::Timer {
                    spell_id: 6306,
                    duration_ms: 13300,
                },
            },
            Rule {
                id: 3,
                trigger: Trigger::CastStart(6306),
                required_phase: None,
                action: Action::Warning {
                    spell_id: 6306,
                    severity: 1,
                    text: "Acid Splash".into(),
                },
            },
        ])
    }
    #[test]
    fn bossmod_timeline_cookie_initial_countdown_actual_start_reset_and_no_invented_repeats() {
        let mut schedule = cookie();
        assert_eq!(schedule.trigger(Trigger::Engage).len(), 1);
        schedule.tick(1000);
        assert_eq!(schedule.timers.get(&6306), Some(&(1400, 2400)));
        assert!(schedule.trigger(Trigger::CastSuccess(6306)).is_empty());
        assert_eq!(schedule.timers.get(&6306), Some(&(1400, 2400)));
        let actions = schedule.trigger(Trigger::CastStart(6306));
        assert!(actions.iter().any(|a| matches!(
            a,
            Action::Warning {
                spell_id: 6306,
                severity: 1,
                ..
            }
        )));
        assert_eq!(schedule.timers.get(&6306), Some(&(13300, 13300)));
        schedule.tick(13301);
        assert!(
            schedule.timers.is_empty(),
            "expiration is not an invented cast/repeat"
        );
    }
    #[test]
    fn bossmod_timeline_success_boundary_and_health_phase_are_once_per_pull() {
        let mut schedule = Schedule::new(vec![
            Rule {
                id: 1,
                trigger: Trigger::Health(35.0),
                required_phase: Some(1.0),
                action: Action::Phase(2.0),
            },
            Rule {
                id: 2,
                trigger: Trigger::Health(35.0),
                required_phase: Some(1.0),
                action: Action::Timer {
                    spell_id: 6435,
                    duration_ms: 5000,
                },
            },
            Rule {
                id: 3,
                trigger: Trigger::CastSuccess(6435),
                required_phase: Some(2.0),
                action: Action::Timer {
                    spell_id: 6435,
                    duration_ms: 6100,
                },
            },
        ]);
        assert!(schedule.trigger(Trigger::CastStart(6435)).is_empty());
        assert!(schedule.trigger(Trigger::CastSuccess(6435)).is_empty());
        assert_eq!(schedule.trigger(Trigger::Health(35.0)).len(), 2);
        assert_eq!(schedule.phase, 2.0);
        assert_eq!(schedule.timers.get(&6435), Some(&(5000, 5000)));
        assert!(schedule.trigger(Trigger::Health(35.0)).is_empty());
        schedule.trigger(Trigger::CastSuccess(6435));
        assert_eq!(schedule.timers.get(&6435), Some(&(6100, 6100)));
    }
}

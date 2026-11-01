//! Set up the native keystone lifecycle and collect one headless run.
#[cfg(test)]
#[path = "keystone_tests.rs"]
mod tests;
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};

use super::campaign::MoveIntent;
use super::graph::Graph;
use super::keystone_record::{PartyMember, RunRecord, Tracker};
use super::keystone_route::PlannedRoute;

#[derive(Clone, Copy)]
pub(super) struct Options {
    pub map_id: u32,
    pub level: u32,
    /// Tank, healer, two follower damage dealers, and the damage-dealer leader.
    pub specs: [u32; 5],
}

impl Options {
    fn parse(map: &str, level: &str, party: &str) -> Result<Self, String> {
        let map_id = map
            .parse::<u32>()
            .map_err(|error| format!("invalid dungeon map: {error}"))?;
        let level = level
            .parse::<u32>()
            .map_err(|error| format!("invalid keystone level: {error}"))?;
        if map_id == 0 || level == 0 {
            return Err("map and keystone level must be positive".into());
        }
        let specs: [u32; 5] = party
            .split(',')
            .map(|part| part.trim().parse::<u32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("invalid party specialization: {error}"))?
            .try_into()
            .map_err(|_| "party requires tank, healer and three damage specialization IDs")?;
        Ok(Self {
            map_id,
            level,
            specs,
        })
    }

    fn from_environment() -> Result<Self, String> {
        let read = |name| std::env::var(name).map_err(|error| format!("{name}: {error}"));
        Self::parse(
            &read("DUNGEON_SIMULATOR_MAP")?,
            &read("KEYSTONE_SIMULATOR_LEVEL")?,
            &read("KEYSTONE_SIMULATOR_PARTY")?,
        )
    }
}

pub(super) struct State {
    session: server::test_support::keystone::Session,
    tracker: Tracker,
    party: Vec<PartyMember>,
    route: PlannedRoute,
    scaling: super::keystone_record::Scaling,
    start_seconds: f64,
    countdown_ms: u64,
}

/// Plan from the actual challenge criteria and the selected dungeon's native level cap.
pub(super) fn prepare_challenge(
    graph: &Graph,
) -> Result<(server::mythic_plus_runtime::ChallengeDefinition, u8), String> {
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("open keystone catalog: {error}"))?;
    let definition = server::mythic_plus_catalog::load_azure(&db)
        .map_err(|error| format!("load challenge criteria: {error}"))?
        .filter(|definition| definition.map_id == graph.map_id)
        .ok_or("selected dungeon has no qualified keystone challenge")?;
    let mut statement = db.prepare("SELECT DISTINCT t.MaxLevelSquish FROM follower_lfg_dungeons d JOIN content_tuning t ON t.ID=d.content_tuning_id WHERE d.map_id=? AND d.difficulty_id=205")
        .map_err(|error| format!("read follower level range: {error}"))?;
    let levels = statement
        .query_map([graph.map_id], |row| row.get::<_, u8>(0))
        .map_err(|error| format!("query follower level range: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("decode follower level range: {error}"))?;
    let [level] = levels.as_slice() else {
        return Err(format!(
            "map{} has {} follower level caps",
            graph.map_id,
            levels.len()
        ));
    };
    if *level == 0 || u32::from(*level) > crate::spell_info::MAX_LEVEL {
        return Err(format!(
            "map{} has invalid follower level {level}",
            graph.map_id
        ));
    }
    Ok((definition, *level))
}

pub(super) fn start(
    app: &mut App,
    party: &crate::followers::simulator::Party,
    plan: PlannedRoute,
    options: Options,
) -> Result<State, String> {
    let session = server::test_support::keystone::Session::prepare(
        app.world_mut(),
        &crate::test_data::world_db_path(),
        party.owner,
        options.level,
    )?;
    let target = session.font_position(app.world())?;
    app.world_mut()
        .entity_mut(party.owner)
        .insert(MoveIntent(Vec3::new(target.x, target.y, target.z)));
    for _ in 0..super::campaign::MAX_TICKS {
        if session.in_font_range(app.world())? {
            break;
        }
        app.update();
    }
    if !session.in_font_range(app.world())? {
        return Err("headless party could not reach the authored Font".into());
    }
    app.world_mut()
        .entity_mut(party.owner)
        .remove::<MoveIntent>();
    session.activate(app.world_mut())?;
    let copy = *app
        .world()
        .get::<crate::maps::InMap>(party.owner)
        .ok_or("keystone owner left its copy")?;
    let scaled: Vec<_> = app
        .world_mut()
        .query::<(
            &crate::maps::InMap,
            &server::mythic_plus_scaling::KeystoneCreatureScale,
        )>()
        .iter(app.world())
        .filter_map(|(map, scale)| (*map == copy).then_some(scale.damage))
        .collect();
    if scaled.is_empty() {
        return Err("keystone activation scaled no native creatures".into());
    }
    let mut damage_multipliers = scaled.clone();
    damage_multipliers.sort_by(f32::total_cmp);
    damage_multipliers.dedup_by(|left, right| left.to_bits() == right.to_bits());
    let scaling = super::keystone_record::Scaling {
        scaled_creatures: scaled.len(),
        damage_multipliers,
    };
    let units: Vec<_> = std::iter::once(party.owner)
        .chain(party.members.iter().copied())
        .collect();
    let members = super::keystone_record::read_party(app.world(), &units)?;
    let identities = units
        .iter()
        .zip(&members)
        .map(|(unit, member)| (unit.to_bits(), member.spec_id))
        .collect::<Vec<_>>();
    let mut tracker = Tracker::new(&identities);
    let cursor = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    tracker.skip_existing(cursor);
    let countdown_ms = session
        .snapshot(app.world())
        .ok_or("started keystone snapshot unavailable")?
        .countdown_ms;
    let start_seconds = app.world().resource::<Time<Virtual>>().elapsed_secs_f64();
    Ok(State {
        session,
        tracker,
        party: members,
        route: plan,
        scaling,
        start_seconds,
        countdown_ms,
    })
}

impl State {
    pub fn elapsed_ms(&self, world: &World) -> u64 {
        ((world.resource::<Time<Virtual>>().elapsed_secs_f64() - self.start_seconds).max(0.0)
            * 1000.0)
            .round() as u64
    }

    pub fn update(&mut self, world: &mut World) -> Result<(), String> {
        let elapsed = self.elapsed_ms(world);
        self.session.advance(world, elapsed / 1000)?;
        let snapshot = self
            .session
            .snapshot(world)
            .ok_or("active keystone snapshot unavailable")?;
        if snapshot.phase == shared::protocol::ChallengePhase::Countdown {
            let cursor = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            self.tracker.skip_existing(cursor);
        } else {
            self.tracker
                .observe_world(world, elapsed.saturating_sub(self.countdown_ms));
        }
        Ok(())
    }

    pub fn completed(&self, world: &World) -> bool {
        self.session
            .snapshot(world)
            .is_some_and(|run| run.phase == shared::protocol::ChallengePhase::Completed)
    }

    pub fn finish(
        self,
        world: &World,
        seed: u64,
        options: Options,
        status: String,
        reason: String,
        diagnostics: serde_json::Value,
    ) -> Result<RunRecord, String> {
        let elapsed = self.elapsed_ms(world);
        let snapshot = self
            .session
            .snapshot(world)
            .ok_or("active keystone snapshot unavailable")?;
        let completed = snapshot.phase == shared::protocol::ChallengePhase::Completed;
        let resulting_key = self.session.resulting_key(world)?;
        let (actors, deaths, pulls) = self
            .tracker
            .finish(elapsed.saturating_sub(self.countdown_ms));
        Ok(RunRecord {
            schema_version: 1,
            seed,
            requested_specs: options.specs,
            map_id: snapshot.map_id,
            challenge_id: Some(snapshot.key.challenge_id),
            key_level: snapshot.key.level,
            affixes: Some(snapshot.key.affixes),
            party: self.party,
            route: Some(self.route),
            scaling: Some(self.scaling),
            outcome: super::keystone_record::Outcome {
                status,
                reason,
                elapsed_ms: Some(snapshot.elapsed_ms),
                timer_ms: Some(snapshot.time_limit_ms),
                in_time: completed.then_some(snapshot.elapsed_ms <= snapshot.time_limit_ms),
                upgrade_levels: completed.then_some(snapshot.upgrade_levels),
                resulting_key_level: Some(resulting_key.level),
                forces: Some(snapshot.forces),
                forces_required: Some(snapshot.forces_required),
                bosses: Some(snapshot.bosses),
            },
            deaths,
            pulls,
            actors,
            diagnostics,
        })
    }
}

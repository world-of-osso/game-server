//! Realm-backed Adventure Guide. Perks uses the existing achievement criteria engine.
#[path = "adventure_guide_catalog.rs"]
mod catalog;
#[path = "adventure_guide_conditions.rs"]
mod conditions;
#[path = "adventure_guide_month.rs"]
mod month;
#[path = "adventure_guide_publication.rs"]
mod publication;
use crate::{
    achievement::{AchievementCatalog, CriteriaEvent, PlayerAchievementState},
    auth::{Authenticated, InWorld},
    networking::PlayerLink,
    persistence::{self, GameDatabase},
};
use bevy::prelude::*;
use catalog::CatalogData;
use lightyear::prelude::*;
pub(crate) use month::MonthlyCriteriaState;
pub(crate) use publication::GuidePublication;
use shared::components::Level;
use shared::protocol::{
    AdventureGuideRequest, AdventureGuideSnapshot, InstanceChannel, PerksActivityProgress,
    PerksActivitySnapshot, SuggestionCensus,
};
use std::collections::BTreeSet;

#[derive(Resource, Debug)]
pub(crate) enum AdventureCatalog {
    Ready(CatalogData),
    Unavailable(String),
}
impl Default for AdventureCatalog {
    fn default() -> Self {
        Self::Unavailable("Adventure Guide source catalog not loaded".into())
    }
}
pub(crate) fn load_catalog(
    conn: &rusqlite::Connection,
    criteria: &AchievementCatalog,
    journal_csv: &std::path::Path,
) -> AdventureCatalog {
    match CatalogData::load(conn, criteria, journal_csv) {
        Ok(data) => AdventureCatalog::Ready(data),
        Err(error) => {
            error!("Adventure Guide catalog unavailable: {error}");
            AdventureCatalog::Unavailable(error)
        }
    }
}

#[derive(Component, Clone, Copy, Default)]
pub(crate) struct MonthlySummary {
    pub points: u32,
    pub start_unix: i64,
    pub end_unix: i64,
}

pub(crate) fn calendar_month(now: i64) -> shared::protocol::JournalMonth {
    month::month_at(now, 0, now)
}

pub(crate) struct AdventureGuidePlugin;
impl Plugin for AdventureGuidePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AdventureCatalog>()
            .init_resource::<crate::maps::ServerClock>()
            .init_resource::<crate::guild_bank::DailyReset>();
        app.add_systems(Update, receive_requests);
        app.add_systems(
            PostUpdate,
            publication::flush_updates
                .after(crate::achievement::flush_achievement_updates)
                .before(MessageSystems::Send),
        );
    }
}

fn receive_requests(world: &mut World) {
    let requests: Vec<_> = world
        .query_filtered::<(
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<AdventureGuideRequest>,
        ), With<Authenticated>>()
        .iter_mut(world)
        .flat_map(|(conn, link, owner, mut receiver)| {
            receiver
                .receive()
                .map(|request| (conn, link.0, owner.character_id, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, character, request) in requests {
        ensure_month_state(world, player, character);
        if let AdventureGuideRequest::TrackActivity {
            activity_id,
            tracked,
        } = request
        {
            update_tracking(world, player, activity_id, tracked);
        }
        send_snapshot(world, connection, player, character);
    }
}

fn ensure_month_state(world: &mut World, player: Entity, character: u64) {
    let now = world.resource::<crate::maps::ServerClock>().unix();
    let start = month::month_at(now, 0, now).start_unix;
    let mut state = match world.get::<PlayerAchievementState>(player) {
        Some(state) => state.clone(),
        None => persistence::load_achievements(&world.resource::<GameDatabase>().db, character),
    };
    match state.monthly.as_mut() {
        Some(monthly) => monthly.reset_at(start, now),
        None => state.monthly = Some(MonthlyCriteriaState::new(start, now)),
    }
    world.entity_mut(player).insert(state);
}

fn update_tracking(world: &mut World, player: Entity, id: u32, tracked: bool) {
    let valid = match world.resource::<AdventureCatalog>() {
        AdventureCatalog::Ready(data) => data.activities.iter().any(|a| a.id == id),
        AdventureCatalog::Unavailable(_) => false,
    };
    if !valid {
        return;
    }
    let Some(mut state) = world.get_mut::<PlayerAchievementState>(player) else {
        return;
    };
    let monthly = state
        .monthly
        .as_mut()
        .expect("journal request initializes month state");
    if tracked {
        monthly.tracked.insert(id);
    } else {
        monthly.tracked.remove(&id);
    }
}

fn persist_month_state(world: &World, connection: Entity, player: Entity, character: u64) {
    let state = world
        .get::<PlayerAchievementState>(player)
        .expect("monthly state initialized");
    let account = world
        .get::<Authenticated>(connection)
        .expect("authenticated monthly query")
        .account_id;
    crate::achievement::save_monthly_contribution(
        world.resource::<GameDatabase>(),
        account,
        character,
        state,
    );
}

fn send_snapshot(world: &mut World, connection: Entity, player: Entity, character: u64) {
    let snapshot = settle_and_project_month(world, connection, player, character);
    publication::queue_update(world, connection, snapshot, true);
}

fn settle_and_project_month(
    world: &mut World,
    connection: Entity,
    player: Entity,
    character: u64,
) -> AdventureGuideSnapshot {
    persist_month_state(world, connection, player, character);
    let now = world.resource::<crate::maps::ServerClock>().unix();
    let state = world
        .get::<PlayerAchievementState>(player)
        .expect("month initialized");
    let monthly = state.monthly.as_ref().expect("month initialized");
    let mut month = month::month_at(now, 0, monthly.tracking_since);
    let snapshot = match world.resource::<AdventureCatalog>() {
        AdventureCatalog::Unavailable(error) => AdventureGuideSnapshot {
            month,
            activities: Vec::new(),
            tags: Vec::new(),
            suggestions: Vec::new(),
            suggestion_census: SuggestionCensus::default(),
            points: 0,
            activity_scope_notice: None,
            source_error: Some(error.clone()),
        },
        AdventureCatalog::Ready(data) => {
            let level = world
                .get::<Level>(player)
                .expect("realm player has Level")
                .0;
            let criteria = world.resource::<AchievementCatalog>();
            let (projection, since) =
                monthly_projection(world, connection, player, month.start_unix, month.end_unix);
            month.tracking_since = since;
            let activities = data
                .activities
                .iter()
                .map(|activity| {
                    activity_snapshot(data, criteria, activity, monthly, &projection, level)
                })
                .collect::<Vec<_>>();
            let points = activities
                .iter()
                .filter(|a| {
                    matches!(
                        a.progress,
                        PerksActivityProgress::Available {
                            completed: true,
                            ..
                        }
                    )
                })
                .map(|a| a.points)
                .sum();
            let earned = criteria.iter_earned_achievement_ids(state).collect();
            let (suggestions, census) = data.suggestions(level, &earned);
            AdventureGuideSnapshot{month,activities,tags:data.tags.clone(),suggestions,suggestion_census:census,points,activity_scope_notice:Some("All source activities shown: interval/calendar binding unavailable. Osso default group1 Tender thresholds apply; seasonal reward binding unavailable.".into()),source_error:None}
        }
    };
    if let AdventureCatalog::Ready(data) = world.resource::<AdventureCatalog>() {
        let account = world
            .get::<Authenticated>(connection)
            .expect("authenticated monthly award")
            .account_id;
        crate::trading_post_store::settle(
            &world.resource::<GameDatabase>().db,
            account,
            snapshot.month.start_unix,
            snapshot.points,
            &data.thresholds,
        );
    }
    world.entity_mut(connection).insert(MonthlySummary {
        points: snapshot.points,
        start_unix: snapshot.month.start_unix,
        end_unix: snapshot.month.end_unix,
    });
    snapshot
}

fn activity_snapshot(
    data: &CatalogData,
    criteria: &AchievementCatalog,
    activity: &catalog::Activity,
    monthly: &MonthlyCriteriaState,
    state: &PlayerAchievementState,
    level: u8,
) -> PerksActivitySnapshot {
    let progress = match data.activity_eligibility(activity.id, level) {
        Err(reason) => PerksActivityProgress::Unavailable { reason },
        Ok(false) => PerksActivityProgress::Unavailable {
            reason: "PlayerCondition not met".into(),
        },
        Ok(true) if !data.supported_roots.contains(&activity.root) => {
            PerksActivityProgress::Unavailable {
                reason: format!(
                    "Criteria tree {} contains untracked criteria, modifiers or eligibility",
                    activity.root
                ),
            }
        }
        Ok(true) => {
            let progress = criteria.tree_progress(activity.root, state);
            let lines = criteria
                .criterion_tree_ids(activity.root)
                .into_iter()
                .map(|id| criteria.criterion_line(state, id, true))
                .collect();
            PerksActivityProgress::Available {
                current: progress.current,
                required: progress.required,
                completed: progress.completed,
                criteria: lines,
            }
        }
    };
    PerksActivitySnapshot {
        id: activity.id,
        name: activity.name.clone(),
        description: activity.description.clone(),
        points: activity.points,
        supersedes: activity.supersedes,
        priority: activity.priority,
        tags: data
            .activity_tags
            .get(&activity.id)
            .cloned()
            .unwrap_or_default(),
        tracked: monthly.tracked.contains(&activity.id),
        progress,
    }
}

fn monthly_projection(
    world: &World,
    connection: Entity,
    player: Entity,
    start: i64,
    end: i64,
) -> (PlayerAchievementState, i64) {
    let account = world
        .get::<Authenticated>(connection)
        .expect("authenticated journal query")
        .account_id;
    let current = world
        .get::<PlayerAchievementState>(player)
        .expect("month state");
    let db = world.resource::<GameDatabase>();
    let mut contributions = crate::achievement::monthly_contributions(db, account);
    let since = contributions
        .iter()
        .filter_map(|state| state.monthly.as_ref())
        .filter(|m| m.interval_start == start)
        .map(|m| m.tracking_since)
        .min()
        .expect("current monthly contribution exists");
    let ignored: BTreeSet<_> = contributions
        .iter()
        .filter_map(|state| state.monthly.as_ref())
        .filter(|m| m.interval_start == start)
        .flat_map(|m| m.ignored_login_achievements.iter().copied())
        .collect();
    // Current state is persisted before projection; only current-period contributions count.
    let criteria = world.resource::<AchievementCatalog>();
    for state in &mut contributions {
        state.criteria_progress = state
            .monthly
            .as_ref()
            .filter(|m| m.interval_start == start)
            .map(|m| m.counters.clone())
            .unwrap_or_default();
        state.earned_ids = state
            .earned_at
            .iter()
            .filter(|(id, date)| **date >= start && **date < end && !ignored.contains(id))
            .map(|(id, _)| *id)
            .collect();
        state.account = None;
        state.monthly = None;
    }
    let mut aggregate = criteria.aggregate(contributions.into_iter());
    if let Some(account) = &current.account {
        aggregate.account = Some(Box::new(PlayerAchievementState {
            earned_ids: account
                .earned_at
                .iter()
                .filter(|(id, date)| **date >= start && **date < end && !ignored.contains(id))
                .map(|(id, _)| *id)
                .collect(),
            ..Default::default()
        }));
    }
    (aggregate, since)
}

/// Only live committed event boundaries call this; historical login reconstruction does not.
pub(crate) fn record(world: &mut World, player: Entity, event: CriteriaEvent) {
    let character = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .find_map(|(link, owner)| (link.0 == player).then_some(owner.character_id));
    let Some(character) = character else { return };
    if !matches!(
        world.get_resource::<AdventureCatalog>(),
        Some(AdventureCatalog::Ready(_))
    ) {
        return;
    }
    ensure_month_state(world, player, character);
    let level = world.get::<Level>(player).expect("realm player Level").0;
    let conditions = crate::achievement::event_conditions(world, player, &[event]);
    let state = world
        .get::<PlayerAchievementState>(player)
        .expect("monthly player state");
    let mut progress = PlayerAchievementState {
        criteria_progress: state
            .monthly
            .as_ref()
            .expect("monthly state")
            .counters
            .clone(),
        ..Default::default()
    };
    let AdventureCatalog::Ready(data) = world.resource::<AdventureCatalog>() else {
        return;
    };
    let criteria = world.resource::<AchievementCatalog>();
    let roots = data
        .supported_indices
        .iter()
        .map(|index| &data.activities[*index])
        .filter(|activity| data.activity_eligibility(activity.id, level) == Ok(true))
        .map(|activity| activity.root);
    let changed = criteria.update_roots_event(roots, &mut progress, event, &conditions);
    if !changed {
        return;
    }
    world
        .get_mut::<PlayerAchievementState>(player)
        .expect("player state")
        .monthly
        .as_mut()
        .expect("month state")
        .counters = progress.criteria_progress;
    // Achievement send_updates persists/publishes after its own event evaluation.
}

pub(crate) fn refresh_for_connection(world: &mut World, connection: Entity) {
    let Some(snapshot) = settle_month_for_connection(world, connection) else {
        return;
    };
    publication::queue_update(world, connection, snapshot, false);
}

/// Settle gameplay credit without publishing the full Guide ahead of vendor replies.
pub(crate) fn settle_month_for_connection(
    world: &mut World,
    connection: Entity,
) -> Option<AdventureGuideSnapshot> {
    if !matches!(
        world.get_resource::<AdventureCatalog>(),
        Some(AdventureCatalog::Ready(_))
    ) {
        return None;
    }
    let Some((player, character)) = world
        .get::<PlayerLink>(connection)
        .zip(world.get::<InWorld>(connection))
        .map(|(link, owner)| (link.0, owner.character_id))
    else {
        return None;
    };
    ensure_month_state(world, player, character);
    Some(settle_and_project_month(
        world, connection, player, character,
    ))
}

pub(crate) fn ignore_login_achievements(
    world: &mut World,
    player: Entity,
    ids: impl Iterator<Item = u32>,
) {
    if !matches!(
        world.get_resource::<AdventureCatalog>(),
        Some(AdventureCatalog::Ready(_))
    ) {
        return;
    }
    let character = world
        .query::<(&PlayerLink, &InWorld)>()
        .iter(world)
        .find_map(|(link, owner)| (link.0 == player).then_some(owner.character_id));
    let Some(character) = character else { return };
    ensure_month_state(world, player, character);
    world
        .get_mut::<PlayerAchievementState>(player)
        .expect("monthly player")
        .monthly
        .as_mut()
        .expect("monthly state")
        .ignored_login_achievements
        .extend(ids);
}

#[cfg(test)]
#[path = "adventure_guide_tests.rs"]
mod tests;

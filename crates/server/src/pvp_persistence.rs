//! Fresh atomic PvP preferences share one ordered, commit-gated live publication queue.
use super::{PlayerPvpProgress, PlayerPvpState, pvp_support, pvp_talents, pvp_war_mode};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{GameDatabase, PVP_STATES};
use crate::persistence_writer::CommitReceipt;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use redb::ReadableTable;
use shared::components::{CombatStatus, Player, Position};
use shared::protocol::{PvpChannel, PvpStateUpdate, WarModeSnapshot};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) struct Owner {
    pub connection: Entity,
    pub player: Entity,
    pub account: u64,
    pub character: u64,
}

#[derive(Clone, Copy)]
pub(super) enum Change {
    WarMode,
    Talents,
}

struct PendingWrite {
    owner: Owner,
    change: Change,
    receipt: CommitReceipt<Result<PlayerPvpProgress, String>>,
}

#[derive(Resource, Default)]
pub(super) struct PendingPvpWrites(VecDeque<PendingWrite>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<PendingPvpWrites>()
        .add_systems(PostUpdate, publish_writes);
}

pub(super) fn enqueue(
    db: &GameDatabase,
    pending: &mut PendingPvpWrites,
    owner: Owner,
    change: Change,
    apply: impl FnOnce(&mut PlayerPvpProgress) -> Result<(), String> + Send + 'static,
) {
    let receipt = db.writer.enqueue("PvP preference mutation", move |db| {
        commit_change(db, owner.character, apply)
    });
    pending.0.push_back(PendingWrite {
        owner,
        change,
        receipt,
    });
}

fn commit_change(
    db: &redb::Database,
    character: u64,
    apply: impl FnOnce(&mut PlayerPvpProgress) -> Result<(), String>,
) -> Result<Result<PlayerPvpProgress, String>, String> {
    let txn = db
        .begin_write()
        .map_err(|error| format!("Begin PvP preference: {error}"))?;
    let mut progress = read_progress(&txn, character)?;
    if let Err(error) = apply(&mut progress) {
        return Ok(Err(error));
    }
    let bytes =
        serde_json::to_vec(&progress).map_err(|error| format!("Encode PvP preference: {error}"))?;
    txn.open_table(PVP_STATES)
        .map_err(|error| error.to_string())?
        .insert(character, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    crate::persistence_commit::commit(txn)
        .map_err(|error| format!("Commit PvP preference: {error}"))?;
    Ok(Ok(progress))
}

fn read_progress(
    txn: &redb::WriteTransaction,
    character: u64,
) -> Result<PlayerPvpProgress, String> {
    let table = txn
        .open_table(PVP_STATES)
        .map_err(|error| error.to_string())?;
    let row = table.get(character).map_err(|error| error.to_string())?;
    row.map(|row| {
        serde_json::from_slice(row.value())
            .map_err(|error| format!("Decode PvP character {character}: {error}"))
    })
    .transpose()
    .map(Option::unwrap_or_default)
}

type ContextPlayers<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static Player,
        &'static crate::maps::InMap,
        &'static crate::rest::PlayerRestState,
        &'static CombatStatus,
    ),
>;

#[derive(bevy::ecs::system::SystemParam)]
struct Publication<'w, 's> {
    contexts: ContextPlayers<'w, 's>,
    areas: pvp_war_mode::Areas<'w>,
    commands: Commands<'w, 's>,
}

impl Publication<'_, '_> {
    fn read_current_mode(&mut self, player: Entity, desired: bool) -> Option<WarModeSnapshot> {
        let (position, player, map, rest, combat) = self.contexts.get(player).ok()?;
        self.areas
            .context(position, player, map, rest, combat)
            .map(|context| pvp_war_mode::snapshot(context, desired))
    }
}

fn publish_writes(mut pending: ResMut<PendingPvpWrites>, mut publication: Publication) {
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed PvP preference receipt");
        let result = result.and_then(|result| result);
        let mode = match (&result, job.change) {
            (Ok(progress), Change::WarMode) => {
                publication.read_current_mode(job.owner.player, progress.war_mode_desired)
            }
            _ => None,
        };
        // Every row/cache/effect/reply publication follows this same receipt order.
        publication.commands.queue(move |world: &mut World| {
            publish_result(world, job.owner, job.change, result, mode)
        });
    }
}

fn owns_session(world: &World, owner: Owner) -> bool {
    let auth = world
        .get::<Authenticated>(owner.connection)
        .is_some_and(|auth| auth.account_id == owner.account);
    let entered = world
        .get::<InWorld>(owner.connection)
        .is_some_and(|entered| entered.character_id == owner.character);
    let linked = world
        .get::<PlayerLink>(owner.connection)
        .is_some_and(|link| link.0 == owner.player);
    auth && entered && linked
}

fn publish_result(
    world: &mut World,
    owner: Owner,
    change: Change,
    result: Result<PlayerPvpProgress, String>,
    mode: Option<WarModeSnapshot>,
) {
    if !owns_session(world, owner) {
        return;
    }
    match result {
        Ok(progress) => publish_committed(world, owner, change, progress, mode),
        Err(error) => send_state(world, owner.connection, owner.player, Some(error)),
    }
}

fn publish_committed(
    world: &mut World,
    owner: Owner,
    change: Change,
    progress: PlayerPvpProgress,
    mode: Option<WarModeSnapshot>,
) {
    let latest =
        crate::persistence::load_pvp(&world.resource::<GameDatabase>().db, owner.character);
    {
        let Some(mut state) = world.get_mut::<PlayerPvpState>(owner.player) else {
            return;
        };
        state.progress = merge_published_preferences(latest, &state.progress, progress, change);
        if matches!(change, Change::WarMode) {
            state.war_mode = mode.clone();
        }
    }
    match change {
        Change::WarMode => publish_war_mode_effect(world, owner, mode),
        Change::Talents => pvp_talents::sync_player(world, owner.connection, owner.player),
    }
    send_state(world, owner.connection, owner.player, None);
}

// Currency may already have published a later commit in Update. Refresh unrelated
// fields without advancing either preference ahead of its own FIFO publication.
fn merge_published_preferences(
    mut latest: PlayerPvpProgress,
    published: &PlayerPvpProgress,
    committed: PlayerPvpProgress,
    change: Change,
) -> PlayerPvpProgress {
    match change {
        Change::WarMode => {
            latest.war_mode_desired = committed.war_mode_desired;
            latest.pvp_talents = published.pvp_talents.clone();
        }
        Change::Talents => {
            latest.war_mode_desired = published.war_mode_desired;
            latest.pvp_talents = committed.pvp_talents;
        }
    }
    latest
}

fn publish_war_mode_effect(world: &mut World, owner: Owner, mode: Option<WarModeSnapshot>) {
    if mode.is_none() {
        warn!(
            character = owner.character,
            "War Mode preference committed; active mode awaits current territory context"
        );
    }
    let enlisted = mode.and_then(|mode| mode.enlisted_spell);
    pvp_war_mode::sync_enlisted_aura(world, owner.player, enlisted);
}

pub(super) fn send_state(
    world: &mut World,
    connection: Entity,
    player: Entity,
    error: Option<String>,
) {
    let state = world.get::<PlayerPvpState>(player).cloned();
    if let Some(mut sender) = world.get_mut::<MessageSender<PvpStateUpdate>>(connection) {
        sender.send::<PvpChannel>(PvpStateUpdate {
            snapshot: state.as_ref().map(pvp_support::build_pvp_snapshot),
            message: None,
            error,
        });
    }
}

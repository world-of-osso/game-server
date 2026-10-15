//! Durable War Mode preferences publish through their original authenticated session.
use super::*;
use crate::persistence;
use crate::persistence_writer::CommitReceipt;
use crate::pvp::PlayerPvpProgress;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) struct Toggle {
    pub connection: Entity,
    pub player: Entity,
    pub account: u64,
    pub character: u64,
    pub desired: bool,
}

struct PendingToggle {
    toggle: Toggle,
    receipt: CommitReceipt<PlayerPvpProgress>,
}

#[derive(Resource, Default)]
pub(super) struct PendingWarModeWrites(VecDeque<PendingToggle>);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingWarModeWrites>()
        .add_systems(PostUpdate, publish_writes);
}

pub(super) fn enqueue(db: &GameDatabase, pending: &mut PendingWarModeWrites, toggle: Toggle) {
    let receipt = db.writer.enqueue("War Mode preference", move |db| {
        let mut progress = persistence::load_pvp(db, toggle.character);
        progress.war_mode_desired = toggle.desired;
        persistence::save_pvp(db, toggle.character, &progress);
        Ok(progress)
    });
    pending.0.push_back(PendingToggle { toggle, receipt });
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
    sessions: Query<
        'w,
        's,
        (
            &'static Authenticated,
            &'static InWorld,
            &'static PlayerLink,
        ),
    >,
    states: Query<'w, 's, &'static mut PlayerPvpState>,
    contexts: ContextPlayers<'w, 's>,
    areas: Areas<'w>,
    commands: Commands<'w, 's>,
    senders: Query<'w, 's, &'static mut MessageSender<PvpStateUpdate>>,
}

impl Publication<'_, '_> {
    fn owns_session(&self, toggle: Toggle) -> bool {
        self.sessions
            .get(toggle.connection)
            .is_ok_and(|(auth, entered, link)| {
                auth.account_id == toggle.account
                    && entered.character_id == toggle.character
                    && link.0 == toggle.player
            })
    }

    fn read_current_context(&mut self, player: Entity) -> Option<Context> {
        let (position, player, map, rest, combat) = self.contexts.get(player).ok()?;
        self.areas.context(position, player, map, rest, combat)
    }

    fn publish(&mut self, toggle: Toggle, result: Result<PlayerPvpProgress, String>) {
        if !self.owns_session(toggle) {
            return;
        }
        let context = self.read_current_context(toggle.player);
        let Ok(mut state) = self.states.get_mut(toggle.player) else {
            return;
        };
        let progress = match result {
            Ok(progress) => progress,
            Err(error) => {
                pvp_support::send_pvp_update(
                    toggle.connection,
                    &state,
                    None,
                    Some(error),
                    &mut self.senders,
                );
                return;
            }
        };
        state.progress = progress;
        state.war_mode = context.map(|context| snapshot(context, state.progress.war_mode_desired));
        let enlisted = state
            .war_mode
            .as_ref()
            .and_then(|state| state.enlisted_spell);
        if context.is_none() {
            warn!(
                character = toggle.character,
                "War Mode preference committed; active mode awaits current territory context"
            );
        }
        self.commands
            .queue(move |world: &mut World| sync_enlisted_aura(world, toggle.player, enlisted));
        pvp_support::send_pvp_update(toggle.connection, &state, None, None, &mut self.senders);
    }
}

fn publish_writes(mut pending: ResMut<PendingWarModeWrites>, mut publication: Publication) {
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending.0.pop_front().expect("completed War Mode receipt");
        publication.publish(job.toggle, result);
    }
}

//! Breath and fatigue mirror timers and drowning (TrinityCore `Player::HandleDrowning`,
//! Player.cpp:725-853; `getMaxTimer` 680-704; `UpdateLiquidMirrorTimerFlagsOnPositionChange`
//! 26779-26823). Spec: docs/specs/breath-and-drowning.md.
//!
//! Every fixed tick each living player's terrain liquid status decides whether its head is
//! under water (breath) and whether it is in deep water (fatigue); `handle_drowning` runs
//! TrinityCore's timer rules and the system sends the owner the resulting
//! `MirrorTimerStart`/`MirrorTimerStop` and deals the environmental damage.

use bevy::prelude::*;
use lightyear::prelude::*;
use rand::Rng;
use shared::components::{Health, Player, Position};
use shared::death::DeathState;
use shared::protocol::{
    CombatLogEvent, CombatLogKind, EnvironmentalKind, MIRROR_TIMER_BREATH, MIRROR_TIMER_FATIGUE,
    MirrorTimerChannel, MirrorTimerStart, MirrorTimerStop,
};

use crate::auras::AuraStatMods;
use crate::ground::GroundMap;
use crate::liquid::{LiquidStatus, liquid_status};
use crate::maps::InMap;
use crate::networking::PlayerLink;

/// `DISABLED_MIRROR_TIMER` (Player.h).
pub const DISABLED_MIRROR_TIMER: i32 = -1;
/// `getMaxTimer(BREATH_TIMER)`: `3 * MINUTE * IN_MILLISECONDS`.
pub const BREATH_MAX_MS: i32 = 180_000;
/// `getMaxTimer(FATIGUE_TIMER)`: `MINUTE * IN_MILLISECONDS`.
pub const FATIGUE_MAX_MS: i32 = 60_000;
/// Out of the water a timer refills `10 * time_diff`.
const REFILL_SCALE: i32 = 10;
/// Below zero a timer deals damage and gains `1 * IN_MILLISECONDS` back.
const DAMAGE_INTERVAL_MS: i32 = 1_000;
/// `DEFAULT_COLLISION_HEIGHT` (Object.h:78): the head height over the feet. TrinityCore
/// takes it from the display's `CreatureModelData`; the server has no player model data.
pub const DEFAULT_COLLISION_HEIGHT: f32 = 2.03128;

/// `PlayerUnderwaterState` bits (Player.h:185-194).
const UNDERWATER_INWATER: u8 = 0x01;
const UNDERWATER_INDARKWATER: u8 = 0x08;
const UNDERWATER_EXIST_TIMERS: u8 = 0x10;

/// `Player::m_MirrorTimer`, `m_MirrorTimerFlags` and `m_MirrorTimerFlagsLast`, indexed by
/// `MirrorTimerType`. Server-only.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MirrorTimers {
    pub value: [i32; 3],
    flags: u8,
    flags_last: u8,
    /// `water_breathing` of the previous tick: gaining or losing it runs
    /// `UpdateMirrorTimers` (`AuraEffect::HandleWaterBreathing`).
    water_breathing: bool,
}

impl Default for MirrorTimers {
    fn default() -> Self {
        Self {
            value: [DISABLED_MIRROR_TIMER; 3],
            flags: 0,
            flags_last: 0,
            water_breathing: false,
        }
    }
}

/// Where the player's head and body are this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exposure {
    /// `UNDERWATER_INWATER`: `LIQUID_MAP_UNDER_WATER` in any liquid.
    pub under_water: bool,
    /// `UNDERWATER_INDARKWATER`: in contact with or over deep water.
    pub dark_water: bool,
}

/// What `getMaxTimer` depends on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breathing {
    pub alive: bool,
    /// `SPELL_AURA_WATER_BREATHING` 82: no breath timer.
    pub water_breathing: bool,
    /// `GetTotalAuraMultiplier(SPELL_AURA_MOD_WATER_BREATHING)` 155.
    pub breath_multiplier: f32,
}

impl Default for Breathing {
    fn default() -> Self {
        Self {
            alive: true,
            water_breathing: false,
            breath_multiplier: 1.0,
        }
    }
}

impl Breathing {
    fn max_timer(self, timer: u8) -> i32 {
        match timer {
            MIRROR_TIMER_FATIGUE => FATIGUE_MAX_MS,
            MIRROR_TIMER_BREATH if !self.alive || self.water_breathing => DISABLED_MIRROR_TIMER,
            MIRROR_TIMER_BREATH => (BREATH_MAX_MS as f32 * self.breath_multiplier) as i32,
            _ => 0,
        }
    }
}

/// A message `handle_drowning` sends the owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimerMessage {
    Start(MirrorTimerStart),
    Stop(MirrorTimerStop),
}

/// The messages and damage of one `handle_drowning` tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrowningTick {
    pub messages: Vec<TimerMessage>,
    pub damage: Vec<EnvironmentalKind>,
}

/// `Player::SendMirrorTimer`: a disabled maximum stops a running timer instead
/// (`StopMirrorTimer` disables it).
fn send_mirror_timer(tick: &mut DrowningTick, timer: u8, max: i32, value: &mut i32, scale: i32) {
    if max == DISABLED_MIRROR_TIMER {
        if *value != DISABLED_MIRROR_TIMER {
            *value = DISABLED_MIRROR_TIMER;
            tick.messages.push(stop(timer));
        }
        return;
    }
    tick.messages.push(TimerMessage::Start(MirrorTimerStart {
        timer,
        value_ms: *value,
        max_value_ms: max,
        scale: scale as f32,
        paused: false,
        spell_id: 0,
    }));
}

fn stop(timer: u8) -> TimerMessage {
    TimerMessage::Stop(MirrorTimerStop { timer })
}

/// One `Player::HandleDrowning(time_diff)` for `exposure` (breath then fatigue).
pub fn handle_drowning(
    timers: &mut MirrorTimers,
    exposure: Exposure,
    breathing: Breathing,
    time_diff_ms: i32,
) -> DrowningTick {
    timers.flags = (timers.flags & UNDERWATER_EXIST_TIMERS)
        | if exposure.under_water {
            UNDERWATER_INWATER
        } else {
            0
        }
        | if exposure.dark_water {
            UNDERWATER_INDARKWATER
        } else {
            0
        };
    if breathing.water_breathing != timers.water_breathing {
        timers.water_breathing = breathing.water_breathing;
        // `Player::UpdateMirrorTimers`: resend every timer on the next tick.
        if timers.flags != 0 {
            timers.flags_last = !timers.flags;
        }
    }
    let mut tick = DrowningTick::default();
    if timers.flags == 0 {
        return tick;
    }
    let timer_rules = [
        (
            MIRROR_TIMER_BREATH,
            UNDERWATER_INWATER,
            EnvironmentalKind::Drowning,
        ),
        (
            MIRROR_TIMER_FATIGUE,
            UNDERWATER_INDARKWATER,
            EnvironmentalKind::Fatigue,
        ),
    ];
    for (timer, flag, damage) in timer_rules {
        update_timer(
            timers,
            &mut tick,
            (timer, flag, damage),
            breathing,
            time_diff_ms,
        );
    }
    let any_running = timers
        .value
        .iter()
        .any(|&value| value != DISABLED_MIRROR_TIMER);
    timers.flags &= !UNDERWATER_EXIST_TIMERS;
    if any_running {
        timers.flags |= UNDERWATER_EXIST_TIMERS;
    }
    timers.flags_last = timers.flags;
    tick
}

/// The `// In water` / `// In dark water` blocks of `HandleDrowning`.
fn update_timer(
    timers: &mut MirrorTimers,
    tick: &mut DrowningTick,
    (timer, flag, damage): (u8, u8, EnvironmentalKind),
    breathing: Breathing,
    time_diff_ms: i32,
) {
    let index = usize::from(timer);
    let max = breathing.max_timer(timer);
    let value = &mut timers.value[index];
    if timers.flags & flag != 0 {
        if *value == DISABLED_MIRROR_TIMER {
            *value = max;
            if max == DISABLED_MIRROR_TIMER {
                return;
            }
            send_mirror_timer(tick, timer, max, value, -1);
            return;
        }
        *value -= time_diff_ms;
        if *value < 0 {
            *value += DAMAGE_INTERVAL_MS;
            // A dead player's fatigue repops a ghost at its graveyard in TrinityCore; ghosts
            // are not tracked here.
            if breathing.alive {
                tick.damage.push(damage);
            }
        } else if timers.flags_last & flag == 0 {
            send_mirror_timer(tick, timer, max, value, -1);
        }
    } else if *value != DISABLED_MIRROR_TIMER {
        *value += REFILL_SCALE * time_diff_ms;
        if *value >= max || !breathing.alive {
            *value = DISABLED_MIRROR_TIMER;
            tick.messages.push(stop(timer));
        } else if timers.flags_last & flag != 0 {
            send_mirror_timer(tick, timer, max, value, REFILL_SCALE);
        }
    }
}

/// `Player::StopMirrorTimers` (KillPlayer, BuildPlayerRepop): stop every running timer.
pub fn stop_mirror_timers(timers: &mut MirrorTimers) -> Vec<TimerMessage> {
    let mut messages = Vec::new();
    for (timer, value) in timers.value.iter_mut().enumerate() {
        if *value != DISABLED_MIRROR_TIMER {
            *value = DISABLED_MIRROR_TIMER;
            messages.push(stop(timer as u8));
        }
    }
    timers.flags = 0;
    timers.flags_last = 0;
    messages
}

/// `getEnvironmentalDamage` in `HandleDrowning`: 20% of maximum health for drowning and
/// exhaustion, plus `urand(0, 10^max(0, floor(log10(damage)) - 1))` from `roll(max)`.
pub fn environmental_damage(max_health: u32, roll: impl FnOnce(u32) -> u32) -> u32 {
    let damage = max_health * 20 / 100;
    let digits = if damage == 0 {
        0
    } else {
        damage.ilog10() as i32
    };
    damage + roll(10u32.pow((digits - 1).max(0) as u32))
}

/// Breath and fatigue exposure at `feet`: the terrain liquid status with the head
/// `DEFAULT_COLLISION_HEIGHT` over the feet.
pub fn exposure_at(ground: &mut GroundMap, map_id: u32, feet: Vec3) -> Exposure {
    let Some(liquid) = ground.liquid_at(map_id, feet) else {
        return Exposure::default();
    };
    let status = liquid_status(
        liquid.level,
        liquid.depth_level,
        feet.y,
        DEFAULT_COLLISION_HEIGHT,
    );
    Exposure {
        under_water: status == LiquidStatus::UnderWater,
        dark_water: liquid.deep && status != LiquidStatus::NoWater,
    }
}

type DrowningPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static InMap,
        &'static mut Health,
        &'static mut MirrorTimers,
        Option<&'static crate::death::PlayerDeathState>,
        Option<&'static AuraStatMods>,
        &'static shared::components::MovementControl,
    ),
    With<Player>,
>;

type TimerSenders<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerLink,
        &'static mut MessageSender<MirrorTimerStart>,
        &'static mut MessageSender<MirrorTimerStop>,
    ),
>;

/// Run `handle_drowning` for every player each fixed tick, send its messages to the
/// owner and deal its damage.
pub(crate) fn update_mirror_timers(
    time: Res<Time>,
    ground: Option<ResMut<GroundMap>>,
    mut players: DrowningPlayers,
    mut senders: TimerSenders,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut player_deaths: MessageWriter<crate::death::PlayerDeathTransition>,
    mut commands: Commands,
) {
    let Some(mut ground) = ground else {
        return;
    };
    let time_diff_ms = time.delta().as_millis() as i32;
    let mut outgoing = Vec::new();
    for (entity, pos, map, mut health, mut timers, death, auras, control) in &mut players {
        let alive =
            death.is_none_or(|death| death.state == DeathState::Alive) && health.current > 0.0;
        if !alive {
            let messages = stop_mirror_timers(&mut timers);
            if !messages.is_empty() {
                outgoing.push((entity, messages));
            }
            continue;
        }
        // A player the server flies (taxi) is not in the water (`IsInFlight`).
        let exposure = if control.controlled {
            Exposure::default()
        } else {
            exposure_at(&mut ground, map.map_id, Vec3::new(pos.x, pos.y, pos.z))
        };
        let breathing = Breathing {
            alive,
            water_breathing: auras.is_some_and(|mods| mods.water_breathing),
            breath_multiplier: auras.map_or(1.0, |mods| mods.water_breathing_mult),
        };
        let tick = handle_drowning(&mut timers, exposure, breathing, time_diff_ms);
        if !tick.damage.is_empty() {
            let logs = deal_environmental_damage(entity, &mut health, &tick.damage);
            let killed = health.current <= 0.0;
            if killed {
                queue_environmental_death(&mut commands, entity, &logs);
            }
            for log in logs {
                commands.queue(move |world: &mut World| crate::spell_cast::log_event(world, log));
            }
            report_damage(
                entity,
                killed,
                replication_dirty.as_deref(),
                &mut player_deaths,
            );
        }
        if !tick.messages.is_empty() {
            outgoing.push((entity, tick.messages));
        }
    }
    send_timer_messages(&mut senders, outgoing);
}

fn queue_environmental_death(commands: &mut Commands, player: Entity, logs: &[CombatLogEvent]) {
    let last = logs
        .last()
        .expect("lethal environmental damage has a source log");
    let CombatLogKind::Environmental(kind) = last.kind else {
        unreachable!("environmental producer emitted a different combat log kind")
    };
    // TC Player.h937-945: exhausted0, drowning1, fall2.
    let kind = match kind {
        EnvironmentalKind::Fatigue => 0,
        EnvironmentalKind::Drowning => 1,
        EnvironmentalKind::Falling => 2,
    };
    commands.queue(move |world: &mut World| {
        crate::achievement::record_environmental_death(world, player, kind);
    });
}

/// Replicate the damaged health; a lethal hit is a player death.
fn report_damage(
    player: Entity,
    killed: bool,
    replication_dirty: Option<&ReplicationDirtyQueue>,
    player_deaths: &mut MessageWriter<crate::death::PlayerDeathTransition>,
) {
    if killed {
        player_deaths.write(crate::death::PlayerDeathTransition { player });
    }
    if let Some(dirty) = replication_dirty {
        dirty.mark(player);
    }
}

/// `EnvironmentalDamage` once per `kinds` entry; the combat log lines of the dealt hits.
fn deal_environmental_damage(
    player: Entity,
    health: &mut Health,
    kinds: &[EnvironmentalKind],
) -> Vec<CombatLogEvent> {
    let mut rng = rand::thread_rng();
    kinds
        .iter()
        .filter_map(|&kind| {
            let damage = environmental_damage(health.max as u32, |max| rng.gen_range(0..=max));
            crate::death::deal_environmental_damage(player, kind, health, damage as f32)
        })
        .collect()
}

fn send_timer_messages(senders: &mut TimerSenders, outgoing: Vec<(Entity, Vec<TimerMessage>)>) {
    if outgoing.is_empty() {
        return;
    }
    for (link, mut starts, mut stops) in senders.iter_mut() {
        let Some((_, messages)) = outgoing.iter().find(|(player, _)| *player == link.0) else {
            continue;
        };
        for message in messages {
            match *message {
                TimerMessage::Start(start) => starts.send::<MirrorTimerChannel>(start),
                TimerMessage::Stop(stop) => stops.send::<MirrorTimerChannel>(stop),
            }
        }
    }
}

pub struct MirrorTimerPlugin;

impl Plugin for MirrorTimerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            update_mirror_timers.after(crate::networking::apply_terrain_gravity),
        );
    }
}

#[cfg(test)]
#[path = "mirror_timers_tests.rs"]
mod tests;

//! Retail's built-in damage meter (`Blizzard_DamageMeter`, `C_DamageMeter`), computed on
//! the server. Spec: docs/specs/damage-meter.md.
//!
//! Every damage line of the combat log (`spell_cast::log_event`) by a player is added to
//! the meter of that player and of each online member of its group. A meter keeps the
//! `Current` session, one per combat of the owner's group, and the `Overall` session, the
//! sum of all of them (`Enum.DamageMeterSessionType`). A session runs while the owner or
//! any online group member is in combat; its duration is that combat time, and a
//! source's DPS is its damage over the session's duration. The owner gets
//! `DamageMeterSnapshot` when a session starts or ends and every
//! `SEND_INTERVAL_SECS` while one runs or has new damage.

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Player};
use shared::protocol::{
    CombatChannel, CombatLogEvent, CombatLogKind, DamageMeterSession, DamageMeterSnapshot,
    DamageMeterSource, DamageMeterSpell,
};

use crate::auth::Authenticated;
use crate::group::{CharacterId, GroupRegistry};
use crate::networking::PlayerLink;
use crate::persistence::ClientId;

/// Seconds between the snapshots of a running session.
const SEND_INTERVAL_SECS: f64 = 1.0;

pub struct DamageMeterPlugin;

impl Plugin for DamageMeterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                attach_damage_meters,
                track_meter_sessions,
                send_damage_meters,
            )
                .chain(),
        );
    }
}

/// Who dealt damage, as the meter lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageSource {
    pub unit: Entity,
    pub name: String,
    pub class_id: u8,
}

#[derive(Clone, Debug, PartialEq)]
struct SourceTotals {
    source: DamageSource,
    total: u64,
    /// Damage by spell id (0 = melee).
    spells: HashMap<u32, u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Totals {
    sources: Vec<SourceTotals>,
}

impl Totals {
    fn add(&mut self, source: &DamageSource, spell_id: u32, amount: u64) {
        let index = match self
            .sources
            .iter()
            .position(|s| s.source.unit == source.unit)
        {
            Some(index) => index,
            None => {
                self.sources.push(SourceTotals {
                    source: source.clone(),
                    total: 0,
                    spells: HashMap::new(),
                });
                self.sources.len() - 1
            }
        };
        let totals = &mut self.sources[index];
        totals.total += amount;
        *totals.spells.entry(spell_id).or_default() += amount;
    }

    fn session(
        &self,
        session_id: u32,
        duration: f64,
        active: bool,
        local: Entity,
    ) -> DamageMeterSession {
        let per_second = |amount: u64| {
            if duration > 0.0 {
                (amount as f64 / duration) as f32
            } else {
                0.0
            }
        };
        let mut sources: Vec<DamageMeterSource> = self
            .sources
            .iter()
            .map(|totals| {
                let mut spells: Vec<DamageMeterSpell> = totals
                    .spells
                    .iter()
                    .map(|(&spell_id, &amount)| DamageMeterSpell {
                        spell_id,
                        total_amount: amount,
                        amount_per_second: per_second(amount),
                    })
                    .collect();
                spells.sort_by(|a, b| {
                    b.total_amount
                        .cmp(&a.total_amount)
                        .then(a.spell_id.cmp(&b.spell_id))
                });
                DamageMeterSource {
                    unit: totals.source.unit.to_bits(),
                    name: totals.source.name.clone(),
                    class_id: totals.source.class_id,
                    is_local_player: totals.source.unit == local,
                    total_amount: totals.total,
                    amount_per_second: per_second(totals.total),
                    spells,
                }
            })
            .collect();
        sources.sort_by(|a, b| {
            b.total_amount
                .cmp(&a.total_amount)
                .then_with(|| a.name.cmp(&b.name))
        });
        DamageMeterSession {
            session_id,
            duration_secs: duration as f32,
            active,
            total_amount: sources.iter().map(|source| source.total_amount).sum(),
            sources,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Session {
    id: u32,
    started_at: f64,
    ended_at: Option<f64>,
    totals: Totals,
}

impl Session {
    fn duration(&self, now: f64) -> f64 {
        self.ended_at.unwrap_or(now) - self.started_at
    }
}

/// One player's meter: its current (or last) session and the overall totals.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct DamageMeter {
    last_session_id: u32,
    current: Option<Session>,
    overall: Totals,
    /// Combat seconds of the finished sessions.
    finished_secs: f64,
    /// New damage since the last snapshot.
    dirty: bool,
    /// A session started or ended since the last snapshot.
    changed: bool,
    last_sent_at: Option<f64>,
}

impl DamageMeter {
    pub fn is_active(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|session| session.ended_at.is_none())
    }

    /// Start a new current session unless one is running.
    pub fn begin(&mut self, now: f64) {
        if self.is_active() {
            return;
        }
        self.last_session_id += 1;
        self.current = Some(Session {
            id: self.last_session_id,
            started_at: now,
            ended_at: None,
            totals: Totals::default(),
        });
        self.changed = true;
    }

    /// End the running session; it stays the current session until the next begins.
    pub fn end(&mut self, now: f64) {
        let Some(session) = self.current.as_mut().filter(|s| s.ended_at.is_none()) else {
            return;
        };
        session.ended_at = Some(now);
        self.finished_secs += session.duration(now);
        self.changed = true;
    }

    /// Add `amount` by `source` with `spell_id` (0 = melee) to the current session, which
    /// starts here if none is running, and to the overall totals.
    pub fn record(&mut self, now: f64, source: &DamageSource, spell_id: u32, amount: u64) {
        self.begin(now);
        let session = self.current.as_mut().expect("session begun");
        session.totals.add(source, spell_id, amount);
        self.overall.add(source, spell_id, amount);
        self.dirty = true;
    }

    fn overall_secs(&self, now: f64) -> f64 {
        let running = self
            .current
            .as_ref()
            .filter(|session| session.ended_at.is_none())
            .map_or(0.0, |session| session.duration(now));
        self.finished_secs + running
    }

    /// The owner's view at `now`; `local` is the owner's unit.
    pub fn snapshot(&self, now: f64, local: Entity) -> DamageMeterSnapshot {
        DamageMeterSnapshot {
            current: self.current.as_ref().map(|session| {
                session.totals.session(
                    session.id,
                    session.duration(now),
                    session.ended_at.is_none(),
                    local,
                )
            }),
            overall: self
                .overall
                .session(0, self.overall_secs(now), self.is_active(), local),
        }
    }

    /// A session started or ended, or it runs or has new damage and the last snapshot is
    /// `SEND_INTERVAL_SECS` old.
    fn should_send(&self, now: f64) -> bool {
        let due = self
            .last_sent_at
            .is_none_or(|sent| now - sent >= SEND_INTERVAL_SECS);
        self.changed || ((self.dirty || self.is_active()) && due)
    }

    fn mark_sent(&mut self, now: f64) {
        self.dirty = false;
        self.changed = false;
        self.last_sent_at = Some(now);
    }
}

/// Count a combat log line: damage dealt by a player to another unit goes to the meters
/// of the player and its online group members.
pub(crate) fn record_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage || event.amount <= 0 || event.source == event.target {
        return;
    }
    let Some(unit) = event.source.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(player) = world.get::<Player>(unit) else {
        return;
    };
    let source = DamageSource {
        unit,
        name: player.name.clone(),
        class_id: player.class,
    };
    let now = world.resource::<Time>().elapsed_secs_f64();
    let spell_id = event.spell_id.unwrap_or(0);
    for recipient in meter_recipients(world, unit) {
        let Ok(mut entity) = world.get_entity_mut(recipient) else {
            continue;
        };
        entity.entry::<DamageMeter>().or_default().get_mut().record(
            now,
            &source,
            spell_id,
            event.amount as u64,
        );
    }
}

/// `unit` and every online member of its group.
fn meter_recipients(world: &mut World, unit: Entity) -> Vec<Entity> {
    let members = world
        .get::<ClientId>(unit)
        .zip(world.get_resource::<GroupRegistry>())
        .map(|(character, registry)| registry.group_members(character.0))
        .unwrap_or_default();
    if members.len() <= 1 {
        return vec![unit];
    }
    world
        .query::<(Entity, &ClientId)>()
        .iter(world)
        .filter(|(_, character)| members.contains(&character.0))
        .map(|(entity, _)| entity)
        .collect()
}

fn attach_damage_meters(
    players: Query<Entity, (With<Player>, Without<DamageMeter>)>,
    mut commands: Commands,
) {
    for player in &players {
        commands.entity(player).insert(DamageMeter::default());
    }
}

/// Start and end each meter's session with its group's combat.
fn track_meter_sessions(
    time: Res<Time>,
    registry: Option<Res<GroupRegistry>>,
    mut meters: Query<(&mut DamageMeter, Option<&CombatStatus>, Option<&ClientId>)>,
    characters: Query<(&ClientId, &CombatStatus)>,
) {
    let now = time.elapsed_secs_f64();
    let in_combat: HashMap<CharacterId, bool> = characters
        .iter()
        .map(|(character, status)| (character.0, status.0))
        .collect();
    for (mut meter, status, character) in &mut meters {
        let group_in_combat =
            character
                .zip(registry.as_deref())
                .is_some_and(|(character, registry)| {
                    registry
                        .group_members(character.0)
                        .iter()
                        .any(|member| in_combat.get(member).copied().unwrap_or(false))
                });
        if status.is_some_and(|status| status.0) || group_in_combat {
            meter.begin(now);
        } else if meter.is_active() {
            meter.end(now);
        }
    }
}

fn send_damage_meters(
    time: Res<Time>,
    mut meters: Query<&mut DamageMeter>,
    mut connections: Query<
        (&PlayerLink, &mut MessageSender<DamageMeterSnapshot>),
        With<Authenticated>,
    >,
) {
    let now = time.elapsed_secs_f64();
    for (link, mut sender) in &mut connections {
        let Ok(mut meter) = meters.get_mut(link.0) else {
            continue;
        };
        if !meter.should_send(now) {
            continue;
        }
        sender.send::<CombatChannel>(meter.snapshot(now, link.0));
        meter.mark_sent(now);
    }
}

#[cfg(test)]
#[path = "damage_meter_tests.rs"]
mod tests;

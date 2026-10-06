//! Retail's built-in damage meter (`Blizzard_DamageMeter`, `C_DamageMeter`), computed on
//! the server. Spec: docs/specs/damage-meter.md.
//!
//! Combat-log category totals (`spell_cast::log_event`) and consumed shields are added
//! to the meter of their player actor and each online member of its group. A meter keeps the
//! `Current` session, one per combat of the owner's group, and the `Overall` session, the
//! sum of all of them (`Enum.DamageMeterSessionType`). A session runs while the owner or
//! any online group member is in combat; its duration is that combat time, and a
//! source's DPS is its damage over the session's duration. The owner gets
//! `DamageMeterSnapshot` when a session starts or ends and every
//! `SEND_INTERVAL_SECS` while one runs or has new category data.
//!
//! Retail's UI only reads aggregated sessions from `C_DamageMeter` and never the combat
//! log (DamageMeterSessionWindow.lua:561-579), but where Retail aggregates, server or
//! client engine, is not visible in its files. The server does it here because a client
//! never receives its group members' spell damage lines (`log_event` sends them only to
//! the source's and target's owners).

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Player};
use shared::protocol::{
    CombatChannel, CombatLogEvent, CombatLogKind, DAMAGE_METER_MAX_ACTION_SPELLS,
    DAMAGE_METER_MAX_DEATH_RECAPS, DAMAGE_METER_MAX_NAME_BYTES, DAMAGE_METER_MAX_RECAP_EVENTS,
    DAMAGE_METER_MAX_SOURCES, DAMAGE_METER_MAX_SPELLS, DamageMeterActionSpell,
    DamageMeterDeathRecap, DamageMeterSession, DamageMeterSnapshot, DamageMeterSource,
    DamageMeterSpell,
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

/// Player identity shared by damage, healing and action category totals.
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
    healing_done: u64,
    overhealing: u64,
    absorbs: u64,
    interrupts: u64,
    interrupt_spells: Vec<DamageMeterActionSpell>,
    dispels: u64,
    dispel_spells: Vec<DamageMeterActionSpell>,
    deaths: u64,
    death_recaps: VecDeque<DamageMeterDeathRecap>,
    recent_events: VecDeque<CombatLogEvent>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Totals {
    sources: Vec<SourceTotals>,
}

impl Totals {
    fn source_mut(&mut self, source: &DamageSource) -> &mut SourceTotals {
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
                    healing_done: 0,
                    overhealing: 0,
                    absorbs: 0,
                    interrupts: 0,
                    interrupt_spells: Vec::new(),
                    dispels: 0,
                    dispel_spells: Vec::new(),
                    deaths: 0,
                    death_recaps: VecDeque::new(),
                    recent_events: VecDeque::new(),
                });
                self.sources.len() - 1
            }
        };
        &mut self.sources[index]
    }

    fn add(&mut self, source: &DamageSource, spell_id: u32, amount: u64) {
        let totals = self.source_mut(source);
        totals.total += amount;
        *totals.spells.entry(spell_id).or_default() += amount;
    }

    fn record_event(&mut self, source: &DamageSource, event: &CombatLogEvent, actor: bool) {
        let totals = self.source_mut(source);
        if actor {
            match event.kind {
                CombatLogKind::Heal => {
                    let amount = event.amount.max(0) as u64;
                    let overheal = (event.overflow.max(0) as u64).min(amount);
                    totals.healing_done += amount - overheal;
                    totals.overhealing += overheal;
                }
                CombatLogKind::Interrupt => {
                    totals.interrupts += 1;
                    record_action_spell(&mut totals.interrupt_spells, event);
                }
                CombatLogKind::Dispel => {
                    totals.dispels += 1;
                    record_action_spell(&mut totals.dispel_spells, event);
                }
                _ => {}
            }
        }
        if event.target != Some(source.unit.to_bits()) {
            return;
        }
        if event.kind == CombatLogKind::Death {
            totals.deaths += 1;
            push_capped(
                &mut totals.death_recaps,
                DamageMeterDeathRecap {
                    timestamp_unix_ms: event.timestamp_unix_ms,
                    events: totals.recent_events.drain(..).collect(),
                },
                DAMAGE_METER_MAX_DEATH_RECAPS,
            );
        } else {
            push_capped(
                &mut totals.recent_events,
                event.clone(),
                DAMAGE_METER_MAX_RECAP_EVENTS,
            );
        }
    }

    fn session(
        &self,
        session_id: u32,
        duration: f64,
        active: bool,
        local: Entity,
        visible_units: &[Entity],
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
            .filter(|totals| {
                visible_units.is_empty() || visible_units.contains(&totals.source.unit)
            })
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
                spells.truncate(DAMAGE_METER_MAX_SPELLS);
                DamageMeterSource {
                    unit: totals.source.unit.to_bits(),
                    name: truncate_name(&totals.source.name),
                    class_id: totals.source.class_id,
                    is_local_player: totals.source.unit == local,
                    total_amount: totals.total,
                    amount_per_second: per_second(totals.total),
                    spells,
                    healing_done: totals.healing_done,
                    overhealing: totals.overhealing,
                    absorbs: totals.absorbs,
                    interrupts: totals.interrupts,
                    interrupt_spells: rank_action_spells(&totals.interrupt_spells),
                    dispels: totals.dispels,
                    dispel_spells: rank_action_spells(&totals.dispel_spells),
                    deaths: totals.deaths,
                    death_recaps: totals.death_recaps.iter().cloned().collect(),
                }
            })
            .collect();
        sources.sort_by(|a, b| {
            b.total_amount
                .cmp(&a.total_amount)
                .then_with(|| a.name.cmp(&b.name))
        });
        sources.truncate(DAMAGE_METER_MAX_SOURCES);
        DamageMeterSession {
            session_id,
            duration_secs: duration as f32,
            active,
            total_amount: sources.iter().map(|source| source.total_amount).sum(),
            sources,
        }
    }
}

fn record_action_spell(spells: &mut Vec<DamageMeterActionSpell>, event: &CombatLogEvent) {
    let Some(spell_id) = event.spell_id else {
        return;
    };
    let affected_spell_id = event.extra_spell_id;
    let existing = spells
        .iter_mut()
        .find(|entry| entry.spell_id == spell_id && entry.affected_spell_id == affected_spell_id);
    if let Some(entry) = existing {
        entry.total_amount += 1;
    } else if spells.len() < DAMAGE_METER_MAX_ACTION_SPELLS {
        spells.push(DamageMeterActionSpell {
            spell_id,
            affected_spell_id,
            total_amount: 1,
        });
    }
}

fn rank_action_spells(spells: &[DamageMeterActionSpell]) -> Vec<DamageMeterActionSpell> {
    let mut ranked = spells.to_vec();
    ranked.sort_by(|a, b| {
        b.total_amount
            .cmp(&a.total_amount)
            .then(a.spell_id.cmp(&b.spell_id))
            .then(a.affected_spell_id.cmp(&b.affected_spell_id))
    });
    ranked
}

fn push_capped<T>(queue: &mut VecDeque<T>, value: T, limit: usize) {
    if queue.len() == limit {
        queue.pop_front();
    }
    queue.push_back(value);
}

fn truncate_name(name: &str) -> String {
    let mut end = name.len().min(DAMAGE_METER_MAX_NAME_BYTES);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name[..end].to_owned()
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
    /// Latest online roster: accumulated former members do not grow the wire payload.
    visible_units: Vec<Entity>,
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

    fn record_event(
        &mut self,
        now: f64,
        source: &DamageSource,
        event: &CombatLogEvent,
        actor: bool,
    ) {
        if !self.is_active()
            && !matches!(
                event.kind,
                CombatLogKind::Damage
                    | CombatLogKind::Heal
                    | CombatLogKind::Interrupt
                    | CombatLogKind::Dispel
                    | CombatLogKind::Death
                    | CombatLogKind::Environmental(_)
            )
        {
            return;
        }
        if actor && event.kind == CombatLogKind::Damage {
            self.record(
                now,
                source,
                event.spell_id.unwrap_or(0),
                event.amount as u64,
            );
            return;
        }
        self.begin(now);
        self.current
            .as_mut()
            .expect("session begun")
            .totals
            .record_event(source, event, actor);
        self.overall.record_event(source, event, actor);
        self.dirty = true;
    }

    fn record_absorb(&mut self, now: f64, source: &DamageSource, amount: u64) {
        self.begin(now);
        self.current
            .as_mut()
            .expect("session begun")
            .totals
            .source_mut(source)
            .absorbs += amount;
        self.overall.source_mut(source).absorbs += amount;
        self.dirty = true;
    }

    fn include_roster(&mut self, roster: &[DamageSource]) {
        self.visible_units = roster.iter().map(|member| member.unit).collect();
        for member in roster {
            self.overall.source_mut(member);
            if let Some(current) = self.current.as_mut() {
                current.totals.source_mut(member);
            }
        }
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
                    &self.visible_units,
                )
            }),
            overall: self.overall.session(
                0,
                self.overall_secs(now),
                self.is_active(),
                local,
                &self.visible_units,
            ),
        }
    }

    /// A session started or ended, or it runs or has new data and the last snapshot is
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

/// Count categories on the actor's group, and retain victim events on the victim's
/// group. These sets may differ (PvP); the same player is processed only once.
pub(crate) fn record_damage(world: &mut World, event: &CombatLogEvent) {
    let actor = event
        .source
        .and_then(Entity::try_from_bits)
        .and_then(|unit| player_source(world, unit));
    let victim = event
        .target
        .and_then(Entity::try_from_bits)
        .and_then(|unit| player_source(world, unit));
    let counts_actor = matches!(
        event.kind,
        CombatLogKind::Heal | CombatLogKind::Interrupt | CombatLogKind::Dispel
    ) || (event.kind == CombatLogKind::Damage
        && event.amount > 0
        && event.source != event.target);
    let now = world.resource::<Time>().elapsed_secs_f64();
    if let Some(actor) = actor.as_ref().filter(|_| counts_actor) {
        record_for_group(world, now, actor, event, true);
    }
    if let Some(victim) = victim.filter(|victim| {
        !counts_actor || actor.as_ref().is_none_or(|actor| actor.unit != victim.unit)
    }) {
        record_for_group(world, now, &victim, event, false);
    }
    if event.kind == CombatLogKind::Death {
        if let Some(target) = event.target.and_then(Entity::try_from_bits) {
            if world.get::<Player>(target).is_some() {
                world.entity_mut(target).insert(MeterDeathRecorded);
            }
        }
    }
}

fn player_source(world: &World, unit: Entity) -> Option<DamageSource> {
    let unit = world
        .get::<crate::hunter_pet::HunterPet>(unit)
        .map_or(unit, |pet| pet.owner);
    let player = world.get::<Player>(unit)?;
    Some(DamageSource {
        unit,
        name: player.name.clone(),
        class_id: player.class,
    })
}

fn record_for_group(
    world: &mut World,
    now: f64,
    source: &DamageSource,
    event: &CombatLogEvent,
    actor: bool,
) {
    let recipients = meter_recipients(world, source.unit);
    let roster: Vec<_> = recipients
        .iter()
        .filter_map(|&unit| player_source(world, unit))
        .collect();
    for &recipient in &recipients {
        let mut entity = world.entity_mut(recipient);
        let mut entry = entity.entry::<DamageMeter>().or_default();
        let mut meter = entry.get_mut();
        meter.record_event(now, source, event, actor);
        meter.include_roster(&roster);
    }
}

#[derive(Component)]
struct MeterDeathRecorded;

/// Damage paths already emit Death. Environmental/admin transitions may not: count
/// their accepted transition once, without adding owner combat-log traffic.
pub(crate) fn record_player_death_transition(world: &mut World, player: Entity) {
    if world.get::<MeterDeathRecorded>(player).is_none() {
        let mut event = crate::combat::death_log_event(player, player);
        event.timestamp_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after 1970")
            .as_millis() as u64;
        record_damage(world, &event);
    }
    world.entity_mut(player).remove::<MeterDeathRecorded>();
}

/// Consumed shield points, attributed before the aura is removed. Incoming damage's
/// `absorbed` field cannot identify the shield caster and must not be counted again.
pub(crate) fn record_absorb(world: &mut World, caster: Entity, amount: f32) {
    let Some(source) = player_source(world, caster) else {
        return;
    };
    let amount = amount.max(0.0) as u64;
    if amount == 0 {
        return;
    }
    let now = world.resource::<Time>().elapsed_secs_f64();
    let recipients = meter_recipients(world, source.unit);
    let roster: Vec<_> = recipients
        .iter()
        .filter_map(|&unit| player_source(world, unit))
        .collect();
    for recipient in recipients {
        let mut entity = world.entity_mut(recipient);
        let mut entry = entity.entry::<DamageMeter>().or_default();
        let mut meter = entry.get_mut();
        meter.record_absorb(now, &source, amount);
        meter.include_roster(&roster);
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

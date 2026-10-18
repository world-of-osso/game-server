use std::time::Duration;

use shared::casting::CastState;
use shared::components::{CombatStatus, Health, Position};
use shared::protocol::{DamageMeterSnapshot, ProtocolPlugin};

use super::*;
use crate::group::{OnlineMember, OnlineView};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};

const FROSTBOLT: u32 = 116;
/// Frostbolt's missile: the spell its damage lines carry.
const FROSTBOLT_DAMAGE: u32 = 228_597;

fn unit(index: u32) -> Entity {
    Entity::from_raw_u32(index).unwrap()
}

fn mage() -> DamageSource {
    DamageSource {
        unit: unit(10),
        name: "Fbmage".into(),
        class_id: 8,
    }
}

fn warrior() -> DamageSource {
    DamageSource {
        unit: unit(11),
        name: "Fbwarrior".into(),
        class_id: 1,
    }
}

fn spell_totals(source: &DamageMeterSource) -> Vec<(u32, u64)> {
    source
        .spells
        .iter()
        .map(|spell| (spell.spell_id, spell.total_amount))
        .collect()
}

#[test]
fn a_session_lasts_its_combat_and_dps_is_damage_over_that_time() {
    let mut meter = DamageMeter::default();
    meter.begin(10.0);
    meter.record(12.0, &mage(), FROSTBOLT_DAMAGE, 186);
    meter.record(14.0, &mage(), 0, 14);
    meter.end(20.0);

    // Later snapshots keep the finished session's 10 s.
    let snapshot = meter.snapshot(25.0, mage().unit);
    let current = snapshot.current.expect("current session");
    assert_eq!(
        (current.session_id, current.duration_secs, current.active),
        (1, 10.0, false)
    );
    assert_eq!(current.total_amount, 200);
    let source = &current.sources[0];
    assert_eq!(
        (
            source.name.as_str(),
            source.class_id,
            source.is_local_player
        ),
        ("Fbmage", 8, true)
    );
    assert_eq!((source.total_amount, source.amount_per_second), (200, 20.0));
    assert_eq!(spell_totals(source), [(FROSTBOLT_DAMAGE, 186), (0, 14)]);
    assert_eq!(source.spells[0].amount_per_second, 18.6);
    assert_eq!(snapshot.overall.session_id, 0);
    assert_eq!(snapshot.overall.duration_secs, 10.0);
    assert_eq!(snapshot.overall.sources, current.sources);
}

#[test]
fn every_combat_is_a_new_session_and_overall_sums_them() {
    let mut meter = DamageMeter::default();
    meter.begin(0.0);
    meter.record(1.0, &mage(), FROSTBOLT_DAMAGE, 100);
    meter.end(10.0);
    meter.begin(30.0);
    meter.record(31.0, &mage(), FROSTBOLT_DAMAGE, 50);

    // While the second combat runs, both durations grow with it.
    let running = meter.snapshot(33.0, mage().unit);
    let current = running.current.unwrap();
    assert_eq!(
        (current.session_id, current.duration_secs, current.active),
        (2, 3.0, true)
    );
    assert_eq!(current.total_amount, 50);
    assert_eq!(
        (running.overall.duration_secs, running.overall.active),
        (13.0, true)
    );
    assert_eq!(running.overall.total_amount, 150);

    meter.end(35.0);
    let done = meter.snapshot(40.0, mage().unit);
    let current = done.current.unwrap();
    assert_eq!((current.duration_secs, current.active), (5.0, false));
    assert_eq!(current.sources[0].amount_per_second, 10.0);
    assert_eq!(
        (done.overall.duration_secs, done.overall.active),
        (15.0, false)
    );
    assert_eq!(done.overall.sources[0].total_amount, 150);
    assert_eq!(done.overall.sources[0].amount_per_second, 10.0);
}

#[test]
fn damage_out_of_combat_starts_a_session_and_sources_rank_by_damage() {
    let mut meter = DamageMeter::default();
    assert_eq!(meter.snapshot(0.0, mage().unit).current, None);
    meter.record(5.0, &mage(), FROSTBOLT_DAMAGE, 90);
    meter.record(6.0, &warrior(), 0, 120);
    meter.begin(7.0);
    meter.end(9.0);

    let current = meter.snapshot(9.0, mage().unit).current.unwrap();
    assert_eq!((current.session_id, current.duration_secs), (1, 4.0));
    let ranked: Vec<_> = current
        .sources
        .iter()
        .map(|source| {
            (
                source.name.as_str(),
                source.total_amount,
                source.is_local_player,
            )
        })
        .collect();
    assert_eq!(ranked, [("Fbwarrior", 120, false), ("Fbmage", 90, true)]);
    assert_eq!(current.total_amount, 210);
}

// --- ECS: the real Frostbolt path ---

fn frostbolt_harness() -> Option<Harness> {
    let mut h = harness_with_plugins(Vec::new(), &[FROSTBOLT, FROSTBOLT_DAMAGE], 8, true, |app| {
        app.add_plugins(DamageMeterPlugin);
    })?;
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<DamageMeterSnapshot>::default(),
        MessageReceiver::<DamageMeterSnapshot>::default(),
    ));
    // Out of combat until the first Frostbolt lands.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h.advance(1);
    // The harness spawned the caster in combat: start from a fresh meter.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(DamageMeter::default());
    Some(h)
}

/// The harness and the snapshots its connection received, drained every frame (a
/// `MessageReceiver` is cleared in `Last`).
struct Client {
    h: Harness,
    received: Vec<DamageMeterSnapshot>,
}

impl Client {
    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.h.advance(1);
            let sent: Vec<DamageMeterSnapshot> =
                crate::spell_cast::test_harness::drain(&mut self.h.app, self.h.connection);
            self.received.extend(sent);
        }
    }

    fn take(&mut self) -> Vec<DamageMeterSnapshot> {
        std::mem::take(&mut self.received)
    }

    /// Cast Frostbolt at the NPC and return the damage its line reports.
    fn frostbolt_hit(&mut self) -> u64 {
        self.h.cast(FROSTBOLT, Some(self.h.npc));
        self.advance(63);
        let hits: Vec<_> = std::mem::take(&mut self.h.inbox.log)
            .into_iter()
            .filter(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(FROSTBOLT_DAMAGE))
            .collect();
        assert_eq!(hits.len(), 1, "{:?}", self.h.inbox.failed);
        hits[0].amount as u64
    }

    /// Advance until the caster's combat ends (5 s after the hit against the harness
    /// NPC, which has no threat list).
    fn leave_combat(&mut self) {
        for _ in 0..200 {
            self.advance(1);
            if !self
                .h
                .app
                .world()
                .get::<CombatStatus>(self.h.caster)
                .unwrap()
                .0
            {
                // Loopback delivery of the final snapshot takes a few frames.
                self.advance(5);
                return;
            }
        }
        panic!("the caster never left combat");
    }
}

#[test]
fn frostbolt_damage_reaches_current_and_overall_and_a_second_combat_is_a_new_session() {
    let Some(h) = frostbolt_harness() else {
        return;
    };
    let mut client = Client {
        h,
        received: Vec::new(),
    };
    let first = client.frostbolt_hit();
    let sent = client.take();
    let latest = sent.last().expect("a snapshot after the hit");
    let current = latest.current.as_ref().unwrap();
    assert_eq!((current.session_id, current.active), (1, true));
    assert_eq!(current.total_amount, first);
    assert_eq!(current.sources[0].name, "Caster");
    assert!(current.sources[0].is_local_player);
    assert_eq!(
        spell_totals(&current.sources[0]),
        [(FROSTBOLT_DAMAGE, first)]
    );
    assert_eq!(latest.overall.total_amount, first);

    client.leave_combat();
    let ended = client.take();
    let ended = ended.last().expect("a snapshot when combat ends");
    let current = ended.current.as_ref().unwrap();
    assert_eq!((current.session_id, current.active), (1, false));
    // In combat from the hit until 5 s after it.
    assert!((5.0..6.5).contains(&current.duration_secs), "{current:?}");
    let dps = current.sources[0].amount_per_second;
    assert!((dps - first as f32 / current.duration_secs).abs() < 1e-3);
    // Nothing more is sent until the next combat.
    client.advance(40);
    assert!(client.take().is_empty());

    let second = client.frostbolt_hit();
    client.leave_combat();
    let latest = client.take().pop().unwrap();
    let current = latest.current.unwrap();
    assert_eq!((current.session_id, current.total_amount), (2, second));
    assert_eq!(latest.overall.total_amount, first + second);
    assert!(
        (latest.overall.duration_secs - (ended.overall.duration_secs + current.duration_secs))
            .abs()
            < 1e-3
    );
}

/// The harness NPC turned into a training dummy (`npc_training_dummy`) 20 yd away,
/// out of the caster's melee range.
fn make_distant_dummy(h: &mut Harness) {
    h.app.world_mut().entity_mut(h.npc).insert((
        crate::creature_scripts::TrainingDummy::default(),
        Position {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        },
    ));
}

#[test]
fn auto_attack_out_of_range_does_not_stretch_a_dummy_session_past_its_evade() {
    let Some(h) = frostbolt_harness() else {
        return;
    };
    let mut client = Client {
        h,
        received: Vec::new(),
    };
    make_distant_dummy(&mut client.h);
    // Tab-targeting starts auto-attack; the dummy is out of melee range.
    let npc = client.h.npc;
    client.h.attack(npc);
    client.frostbolt_hit();
    client.leave_combat();
    let latest = client.take().pop().expect("a snapshot when combat ends");
    let current = latest.current.unwrap();
    assert!(!current.active);
    // The dummy ends combat 5 s after its last hit (TC npc_training_dummy); nothing
    // keeps the caster in combat past that.
    assert!((5.0..6.5).contains(&current.duration_secs), "{current:?}");
}

/// The harness NPC with a threat list, so the fight with it lasts until it dies or
/// evades.
fn make_fighting_creature(h: &mut Harness) {
    h.app.world_mut().entity_mut(h.npc).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
        // Session timing and pushback require landed swings, not random misses.
        crate::auras::AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..Default::default()
        },
    ));
}

fn frostbolt_client() -> Option<Client> {
    Some(Client {
        h: frostbolt_harness()?,
        received: Vec::new(),
    })
}

fn caster_in_combat(client: &Client) -> bool {
    client
        .h
        .app
        .world()
        .get::<CombatStatus>(client.h.caster)
        .is_some_and(|status| status.0)
}

fn latest_current(client: &mut Client) -> DamageMeterSession {
    client
        .take()
        .pop()
        .and_then(|snapshot| snapshot.current)
        .expect("a snapshot with a current session")
}

#[test]
fn a_session_against_a_creature_ends_at_its_death_not_five_seconds_later() {
    let Some(mut client) = frostbolt_client() else {
        return;
    };
    make_fighting_creature(&mut client.h);
    let first = client.frostbolt_hit();
    // The next Frostbolt kills it.
    let npc = client.h.npc;
    client
        .h
        .app
        .world_mut()
        .get_mut::<Health>(npc)
        .unwrap()
        .current = 1.0;
    let second = client.frostbolt_hit();
    assert!(
        !caster_in_combat(&client),
        "the kill ends the caster's combat"
    );
    client.advance(5);

    let current = latest_current(&mut client);
    assert!(!current.active);
    assert_eq!(current.total_amount, first + second);
    // First launch to kill: 63-frame cycle + one 500 ms melee pushback on the
    // second cast (TrinityCore a352b1fa Spell::Delayed, max two per cast) + flight.
    // Flight at 3 yd uses the 5 yd minimum: ceil((5 / 35) / 0.05) = 3 ticks.
    // Thus (63 + 10 + 3) * 0.05 = 3.8 s, not another 5 s after the lethal hit.
    assert!((current.duration_secs - 3.8).abs() < 1e-4, "{current:?}");
    let dps = current.sources[0].amount_per_second;
    assert!((dps - (first + second) as f32 / current.duration_secs).abs() < 1e-3);
}

#[test]
fn damage_meter_melee_damage_delays_frostbolt_launch_by_ten_ticks() {
    let mut client = frostbolt_client().unwrap();
    make_fighting_creature(&mut client.h);
    client.frostbolt_hit();
    let mut h = client.h;
    let (caster, npc) = (h.caster, h.npc);
    let health_before = h.health(caster);

    h.cast_and_deliver(FROSTBOLT, Some(npc));
    let original_duration = h.app.world().get::<CastState>(caster).unwrap().duration;
    assert_eq!(original_duration, 1.75);
    // The real NPC swing arrives at frame 20, 0.9 s into the second cast.
    h.advance(17);
    // The swing can crit; pushback depends on positive damage, not its amount.
    assert!(h.health(caster) < health_before);
    let melee_hits = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(npc.to_bits())
                && event.target == Some(caster.to_bits())
                && event.spell_id.is_none()
                && event.amount > 0
        })
        .count();
    assert_eq!(melee_hits, 1);
    let cast = h.app.world().get::<CastState>(caster).unwrap();
    assert_eq!(cast.pushback_count, 1);
    // Native duration stays fixed; rewound progress postpones the real launch.
    assert_eq!(cast.duration, original_duration);

    // Without pushback it launches at frame 38. A damaging melee swing adds
    // 500 ms (TrinityCore a352b1fa Spell::Delayed): ten 50 ms simulation ticks.
    h.advance(18);
    assert!(h.app.world().get::<CastState>(caster).is_some());
    h.advance(9);
    assert!(h.app.world().get::<CastState>(caster).is_some());
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::CastSuccess)
    );
    h.advance(1);
    assert!(h.app.world().get::<CastState>(caster).is_none());
    let launches = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(FROSTBOLT)
        })
        .count();
    assert_eq!(launches, 1);
}

#[test]
fn a_one_shot_kill_lasts_the_missile_s_flight() {
    let Some(mut client) = frostbolt_client() else {
        return;
    };
    make_fighting_creature(&mut client.h);
    let npc = client.h.npc;
    let world = client.h.app.world_mut();
    *world.get_mut::<Position>(npc).unwrap() = Position {
        x: 20.0,
        y: 0.0,
        z: 0.0,
    };
    world.get_mut::<Health>(npc).unwrap().current = 1.0;
    client.frostbolt_hit();
    client.advance(5);
    let current = latest_current(&mut client);
    assert!(!current.active);
    // Combat from the launch to the kill: 20 yd / 35 yd/s = 0.571 s.
    assert!((0.5..=0.65).contains(&current.duration_secs), "{current:?}");
}

#[test]
fn a_lull_in_a_fight_with_a_creature_does_not_split_the_session() {
    let Some(mut client) = frostbolt_client() else {
        return;
    };
    make_fighting_creature(&mut client.h);
    client.frostbolt_hit();
    // 8 s without a hit: 11 s between the Frostbolts.
    client.advance(160);
    client.frostbolt_hit();
    let current = latest_current(&mut client);
    assert_eq!((current.session_id, current.active), (1, true));
    assert!(current.duration_secs > 11.0, "{current:?}");
}

#[test]
fn hits_on_a_dummy_a_few_seconds_apart_are_one_session_and_a_longer_gap_starts_another() {
    let Some(mut client) = frostbolt_client() else {
        return;
    };
    make_distant_dummy(&mut client.h);
    // Back to back: 3.15 s between the hits.
    client.frostbolt_hit();
    client.frostbolt_hit();
    // 2 s more: 3.9 s between the hits.
    client.advance(15);
    client.frostbolt_hit();
    let current = latest_current(&mut client);
    assert_eq!((current.session_id, current.active), (1, true));

    // 6.15 s between the hits: combat with the dummy ended 5 s after the last one.
    client.advance(60);
    client.frostbolt_hit();
    let current = latest_current(&mut client);
    assert_eq!((current.session_id, current.active), (2, true));
}

// --- ECS: groups ---

#[test]
fn groupmeter_three_players_receive_exact_categories_and_capped_recaps() {
    let (mut app, dps, tank, outsider) = group_app();
    let healer = spawn_member(&mut app, 4, "Healer", 5);
    let online: OnlineView = [
        (1, member(dps, "Fbmage", 8)),
        (2, member(tank, "Fbwarrior", 1)),
        (3, member(outsider, "Fbrogue", 4)),
        (4, member(healer, "Healer", 5)),
    ]
    .into();
    let mut registry = app.world_mut().resource_mut::<GroupRegistry>();
    registry.sync_presence(&online);
    registry.invite(1, "Healer", &online);
    registry.respond_invite(4, true);
    assert_eq!(registry.group_members(1).len(), 3);
    let players = [healer, tank, dps, outsider];
    let connections: Vec<_> = players
        .iter()
        .enumerate()
        .map(|(i, &player)| {
            let connection = crate::spell_cast::test_harness::spawn_peer_connection(
                &mut app,
                player,
                i as u64 + 1,
            );
            app.world_mut().entity_mut(connection).insert((
                MessageSender::<DamageMeterSnapshot>::default(),
                MessageReceiver::<DamageMeterSnapshot>::default(),
            ));
            connection
        })
        .collect();
    app.world_mut().entity_mut(tank).insert(CombatStatus(true));
    let mob = app.world_mut().spawn_empty().id();
    for (player, damage, healing, overheal, interrupts, dispels) in [
        (healer, 10, 100, 40, 1, 2),
        (tank, 250, 20, 0, 2, 1),
        (dps, 500, 0, 10, 3, 0),
    ] {
        crate::spell_cast::log_event(app.world_mut(), damage_line(player, mob, damage));
        crate::spell_cast::log_event(
            app.world_mut(),
            CombatLogEvent {
                kind: CombatLogKind::Heal,
                overflow: overheal,
                ..damage_line(player, tank, healing + overheal)
            },
        );
        for _ in 0..interrupts {
            crate::spell_cast::log_event(
                app.world_mut(),
                CombatLogEvent {
                    kind: CombatLogKind::Interrupt,
                    spell_id: Some(2139),
                    extra_spell_id: Some(116),
                    ..damage_line(player, mob, 0)
                },
            );
        }
        for _ in 0..dispels {
            crate::spell_cast::log_event(
                app.world_mut(),
                CombatLogEvent {
                    kind: CombatLogKind::Dispel,
                    spell_id: Some(527),
                    extra_spell_id: Some(589),
                    ..damage_line(player, tank, 0)
                },
            );
        }
    }
    super::record_absorb(app.world_mut(), healer, 75.0);
    for death in 0..3 {
        for hit in 0..12 {
            crate::spell_cast::log_event(
                app.world_mut(),
                damage_line(mob, tank, death * 100 + hit),
            );
        }
        crate::spell_cast::log_event(
            app.world_mut(),
            CombatLogEvent {
                kind: CombatLogKind::Death,
                ..damage_line(mob, tank, 0)
            },
        );
        // Accepted death transition must not count a combat-log death twice.
        super::record_player_death_transition(app.world_mut(), tank);
    }
    let mut received = vec![Vec::new(); players.len()];
    for _ in 0..30 {
        app.update();
        for (i, &connection) in connections.iter().enumerate() {
            received[i].extend(
                crate::spell_cast::test_harness::drain::<DamageMeterSnapshot>(&mut app, connection),
            );
        }
    }
    assert!(
        received[3].is_empty(),
        "outsider receives no group snapshots"
    );
    for (i, &recipient) in players[..3].iter().enumerate() {
        assert!(
            !received[i].is_empty(),
            "member must receive a wire snapshot"
        );
        let snapshot = received[i].last().unwrap();
        for session in [snapshot.current.as_ref().unwrap(), &snapshot.overall] {
            assert_eq!(session.sources.len(), 3);
            for (unit, expected) in [
                (healer, (10, 100, 40, 75, 1, 2, 0)),
                (tank, (250, 20, 0, 0, 2, 1, 3)),
                (dps, (500, 0, 10, 0, 3, 0, 0)),
            ] {
                let source = session
                    .sources
                    .iter()
                    .find(|source| source.unit == unit.to_bits())
                    .unwrap();
                let action_spells = |spell_id, affected_spell_id, count| {
                    if count == 0 {
                        Vec::new()
                    } else {
                        vec![shared::protocol::DamageMeterActionSpell {
                            spell_id,
                            affected_spell_id: Some(affected_spell_id),
                            total_amount: count,
                        }]
                    }
                };
                assert_eq!(
                    source.interrupt_spells,
                    action_spells(2139, 116, expected.4)
                );
                assert_eq!(source.dispel_spells, action_spells(527, 589, expected.5));
                assert_eq!(
                    (
                        source.total_amount,
                        source.healing_done,
                        source.overhealing,
                        source.absorbs,
                        source.interrupts,
                        source.dispels,
                        source.deaths
                    ),
                    expected
                );
                assert_eq!(source.is_local_player, unit == recipient);
            }
            let victim = session
                .sources
                .iter()
                .find(|source| source.unit == tank.to_bits())
                .unwrap();
            assert_eq!(victim.death_recaps.len(), DAMAGE_METER_MAX_DEATH_RECAPS);
            for (recap, death) in victim.death_recaps.iter().zip(1..3) {
                assert_eq!(recap.events.len(), DAMAGE_METER_MAX_RECAP_EVENTS);
                assert_eq!(
                    recap
                        .events
                        .iter()
                        .map(|event| event.amount)
                        .collect::<Vec<_>>(),
                    (4..12).map(|hit| death * 100 + hit).collect::<Vec<_>>()
                );
                assert!(
                    recap
                        .events
                        .iter()
                        .all(|event| event.target == Some(tank.to_bits()))
                );
            }
        }
    }
}

#[test]
fn groupmeter_export_caps_detail_without_losing_totals() {
    let mut meter = DamageMeter::default();
    for id in 0..DAMAGE_METER_MAX_SOURCES as u32 {
        let source = DamageSource {
            unit: unit(id),
            name: "é".repeat(40),
            class_id: 5,
        };
        for spell in 0..65 {
            meter.record(0.0, &source, spell, 1);
        }
    }
    let snapshot = meter.snapshot(1.0, unit(0));
    for session in [snapshot.current.as_ref().unwrap(), &snapshot.overall] {
        assert_eq!(session.sources.len(), DAMAGE_METER_MAX_SOURCES);
        assert_eq!(session.total_amount, DAMAGE_METER_MAX_SOURCES as u64 * 65);
        for source in &session.sources {
            assert_eq!(source.total_amount, 65);
            assert_eq!(source.spells.len(), DAMAGE_METER_MAX_SPELLS);
            assert_eq!(source.name.len(), DAMAGE_METER_MAX_NAME_BYTES);
            assert_eq!(source.name, "é".repeat(24));
        }
    }
}

#[test]
fn damage_meter_action_spell_caps_preserve_totals_and_existing_pairs() {
    use shared::protocol::DAMAGE_METER_MAX_ACTION_SPELLS;
    let source = DamageSource {
        unit: unit(1),
        name: "Mage".into(),
        class_id: 8,
    };
    let mut meter = DamageMeter::default();
    for kind in [CombatLogKind::Interrupt, CombatLogKind::Dispel] {
        for affected in 0..DAMAGE_METER_MAX_ACTION_SPELLS as u32 + 2 {
            meter.record_event(
                0.0,
                &source,
                &CombatLogEvent {
                    kind,
                    spell_id: Some(2139),
                    extra_spell_id: Some(affected),
                    ..damage_line(source.unit, unit(2), 0)
                },
                true,
            );
        }
        meter.record_event(
            0.0,
            &source,
            &CombatLogEvent {
                kind,
                spell_id: Some(2139),
                extra_spell_id: Some(0),
                ..damage_line(source.unit, unit(2), 0)
            },
            true,
        );
    }
    let snapshot = meter.snapshot(1.0, source.unit);
    for session in [snapshot.current.as_ref().unwrap(), &snapshot.overall] {
        let source = &session.sources[0];
        assert_eq!(source.interrupts, DAMAGE_METER_MAX_ACTION_SPELLS as u64 + 3);
        assert_eq!(source.dispels, DAMAGE_METER_MAX_ACTION_SPELLS as u64 + 3);
        for entries in [&source.interrupt_spells, &source.dispel_spells] {
            assert_eq!(entries.len(), DAMAGE_METER_MAX_ACTION_SPELLS);
            assert_eq!(entries[0].affected_spell_id, Some(0));
            assert_eq!(entries[0].total_amount, 2);
            assert!(entries.iter().all(|entry| entry.spell_id == 2139));
            assert_eq!(
                entries.iter().map(|entry| entry.total_amount).sum::<u64>(),
                DAMAGE_METER_MAX_ACTION_SPELLS as u64 + 1
            );
        }
    }
    meter.end(1.0);
    meter.begin(2.0);
    let snapshot = meter.snapshot(3.0, source.unit);
    assert!(snapshot.current.unwrap().sources.is_empty());
    assert_eq!(
        snapshot.overall.sources[0].interrupt_spells[0].total_amount,
        2
    );
}

#[test]
fn groupmeter_real_shield_consumption_credits_the_caster_group() {
    const SHIELD: u32 = 17;
    let mut h = harness_with_plugins(Vec::new(), &[SHIELD], 5, false, |app| {
        app.add_plugins(DamageMeterPlugin);
    })
    .expect("Power Word: Shield world spell data");
    h.app.world_mut().entity_mut(h.caster).insert(ClientId(1));
    let tank = spawn_member(&mut h.app, 2, "Tank", 1);
    let online: OnlineView = [
        (1, member(h.caster, "Caster", 5)),
        (2, member(tank, "Tank", 1)),
    ]
    .into();
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&online);
    registry.invite(1, "Tank", &online);
    registry.respond_invite(2, true);
    h.app.insert_resource(registry);
    h.cast_and_deliver(SHIELD, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        crate::auras::absorb_damage(h.app.world_mut(), h.caster, 1, 75.0),
        (0.0, 75.0)
    );
    // The damage line's mitigation is recap detail, never another caster credit.
    crate::spell_cast::log_event(
        h.app.world_mut(),
        CombatLogEvent {
            absorbed: 75,
            ..damage_line(h.npc, h.caster, 0)
        },
    );
    for recipient in [h.caster, tank] {
        let snapshot = h
            .app
            .world()
            .get::<DamageMeter>(recipient)
            .unwrap()
            .snapshot(0.0, recipient);
        let caster = snapshot
            .overall
            .sources
            .iter()
            .find(|source| source.unit == h.caster.to_bits())
            .unwrap();
        assert_eq!(caster.absorbs, 75);
    }
}

#[test]
fn groupmeter_first_absorb_exports_the_whole_roster() {
    let (mut app, healer, tank, outsider) = group_app();
    super::record_absorb(app.world_mut(), healer, 75.0);
    for recipient in [healer, tank] {
        let snapshot = app
            .world()
            .get::<DamageMeter>(recipient)
            .unwrap()
            .snapshot(0.0, recipient);
        for session in [snapshot.current.as_ref().unwrap(), &snapshot.overall] {
            assert_eq!(session.sources.len(), 2);
            let source = session
                .sources
                .iter()
                .find(|source| source.unit == healer.to_bits())
                .unwrap();
            assert_eq!(source.absorbs, 75);
        }
    }
    assert!(
        app.world()
            .get::<DamageMeter>(outsider)
            .unwrap()
            .snapshot(0.0, outsider)
            .current
            .is_none()
    );
}

#[test]
fn groupmeter_raid_receives_healing_without_counting_creature_damage() {
    let (mut app, healer, tank, outsider) = group_app();
    app.world_mut()
        .resource_mut::<GroupRegistry>()
        .convert_to_raid(1);
    let mob = app.world_mut().spawn_empty().id();
    crate::spell_cast::log_event(
        app.world_mut(),
        CombatLogEvent {
            kind: CombatLogKind::Heal,
            overflow: 40,
            ..damage_line(healer, tank, 140)
        },
    );
    crate::spell_cast::log_event(app.world_mut(), damage_line(mob, tank, 200));
    for recipient in [healer, tank] {
        let snapshot = app
            .world()
            .get::<DamageMeter>(recipient)
            .unwrap()
            .snapshot(0.0, recipient);
        assert_eq!(snapshot.overall.sources.len(), 2);
        assert_eq!(snapshot.overall.total_amount, 0);
        let source = snapshot
            .overall
            .sources
            .iter()
            .find(|source| source.unit == healer.to_bits())
            .unwrap();
        assert_eq!((source.healing_done, source.overhealing), (100, 40));
    }
    assert!(
        app.world()
            .get::<DamageMeter>(outsider)
            .unwrap()
            .snapshot(0.0, outsider)
            .current
            .is_none()
    );
}

#[test]
fn groupmeter_categories_reset_current_but_preserve_overall() {
    let (mut app, healer, tank, _) = group_app();
    crate::spell_cast::log_event(
        app.world_mut(),
        CombatLogEvent {
            kind: CombatLogKind::Heal,
            overflow: 40,
            ..damage_line(healer, tank, 140)
        },
    );
    crate::spell_cast::log_event(
        app.world_mut(),
        CombatLogEvent {
            kind: CombatLogKind::Death,
            ..damage_line(tank, tank, 0)
        },
    );
    super::record_player_death_transition(app.world_mut(), tank);
    app.update(); // No member in combat: encounter one ends.
    crate::spell_cast::log_event(
        app.world_mut(),
        CombatLogEvent {
            kind: CombatLogKind::Heal,
            ..damage_line(healer, tank, 20)
        },
    );
    for recipient in [healer, tank] {
        let snapshot = app
            .world()
            .get::<DamageMeter>(recipient)
            .unwrap()
            .snapshot(0.0, recipient);
        let current = snapshot.current.unwrap();
        assert_eq!(current.session_id, 2);
        let current_healer = current
            .sources
            .iter()
            .find(|source| source.unit == healer.to_bits())
            .unwrap();
        assert_eq!(
            (current_healer.healing_done, current_healer.overhealing),
            (20, 0)
        );
        let overall_healer = snapshot
            .overall
            .sources
            .iter()
            .find(|source| source.unit == healer.to_bits())
            .unwrap();
        assert_eq!(
            (overall_healer.healing_done, overall_healer.overhealing),
            (120, 40)
        );
        let current_tank = current
            .sources
            .iter()
            .find(|source| source.unit == tank.to_bits())
            .unwrap();
        assert_eq!(current_tank.deaths, 0);
        assert!(current_tank.death_recaps.is_empty());
        let overall_tank = snapshot
            .overall
            .sources
            .iter()
            .find(|source| source.unit == tank.to_bits())
            .unwrap();
        assert_eq!(overall_tank.deaths, 1);
        assert_eq!(overall_tank.death_recaps.len(), 1);
    }
}

#[test]
fn groupmeter_unlogged_death_transition_counts_once() {
    let (mut app, mage, warrior, _) = group_app();
    super::record_player_death_transition(app.world_mut(), warrior);
    for recipient in [mage, warrior] {
        let snapshot = app
            .world()
            .get::<DamageMeter>(recipient)
            .unwrap()
            .snapshot(0.0, recipient);
        let victim = snapshot
            .overall
            .sources
            .iter()
            .find(|source| source.unit == warrior.to_bits())
            .unwrap();
        assert_eq!(victim.deaths, 1);
        assert_eq!(victim.death_recaps.len(), 1);
    }
}

#[test]
fn groupmeter_healing_is_visible_to_a_non_casting_group_member() {
    let (mut app, mage, warrior, outsider) = group_app();
    crate::spell_cast::log_event(
        app.world_mut(),
        CombatLogEvent {
            kind: CombatLogKind::Heal,
            overflow: 40,
            ..damage_line(mage, warrior, 140)
        },
    );
    let snapshot = app
        .world()
        .get::<DamageMeter>(warrior)
        .unwrap()
        .snapshot(0.0, warrior);
    assert_eq!(snapshot.overall.sources.len(), 2);
    let healer = snapshot
        .overall
        .sources
        .iter()
        .find(|source| source.unit == mage.to_bits())
        .unwrap();
    assert_eq!((healer.healing_done, healer.overhealing), (100, 40));
    assert!(
        app.world()
            .get::<DamageMeter>(outsider)
            .unwrap()
            .snapshot(0.0, outsider)
            .current
            .is_none()
    );
}

fn spawn_member(app: &mut App, character: CharacterId, name: &str, class: u8) -> Entity {
    app.world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class,
                appearance: default(),
            },
            ClientId(character),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            CombatStatus(false),
        ))
        .id()
}

fn member(entity: Entity, name: &str, class: u8) -> OnlineMember {
    OnlineMember {
        entity: entity.to_bits(),
        name: name.into(),
        class,
        level: 10,
        portrait: Default::default(),
        dungeon_difficulty: 1,
        raid_difficulty: crate::maps::RaidDifficulty::default(),
    }
}

fn damage_line(source: Entity, target: Entity, amount: i32) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(source.to_bits()),
        target: Some(target.to_bits()),
        spell_id: Some(FROSTBOLT_DAMAGE),
        school_mask: 16,
        amount,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind: CombatLogKind::Damage,
    }
}

/// A meter app with the mage (character 1) and the warrior (2) grouped, and an
/// ungrouped rogue (3).
fn group_app() -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(DamageMeterPlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.finish();
    app.cleanup();
    let mage = spawn_member(&mut app, 1, "Fbmage", 8);
    let warrior = spawn_member(&mut app, 2, "Fbwarrior", 1);
    let outsider = spawn_member(&mut app, 3, "Fbrogue", 4);
    let online: OnlineView = [
        (1, member(mage, "Fbmage", 8)),
        (2, member(warrior, "Fbwarrior", 1)),
        (3, member(outsider, "Fbrogue", 4)),
    ]
    .into();
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&online);
    registry.invite(1, "Fbwarrior", &online);
    registry.respond_invite(2, true);
    assert_eq!(registry.group_members(1).len(), 2);
    app.insert_resource(registry);
    app.update();
    (app, mage, warrior, outsider)
}

#[test]
fn a_group_member_s_damage_and_combat_reach_every_member_s_meter() {
    let (mut app, mage, warrior, outsider) = group_app();
    let mob = app.world_mut().spawn_empty().id();

    // The warrior fights; the mage stays out of combat.
    app.world_mut()
        .entity_mut(warrior)
        .insert(CombatStatus(true));
    app.update();
    let line = damage_line(warrior, mob, 250);
    record_damage(app.world_mut(), &line);
    // Healing, misses and self-damage do not count.
    record_damage(
        app.world_mut(),
        &CombatLogEvent {
            kind: CombatLogKind::Heal,
            ..line.clone()
        },
    );
    record_damage(app.world_mut(), &damage_line(warrior, warrior, 30));
    app.update();

    let world = app.world();
    let mage_view = world.get::<DamageMeter>(mage).unwrap().snapshot(0.0, mage);
    let current = mage_view
        .current
        .expect("the group's combat runs on the mage's meter");
    assert!(current.active);
    let sources: Vec<_> = current
        .sources
        .iter()
        .map(|s| (s.name.as_str(), s.total_amount, s.is_local_player))
        .collect();
    assert_eq!(sources, [("Fbwarrior", 250, false), ("Fbmage", 0, true)]);
    let warrior_view = world
        .get::<DamageMeter>(warrior)
        .unwrap()
        .snapshot(0.0, warrior);
    assert_eq!(warrior_view.overall.sources[0].total_amount, 250);
    assert!(warrior_view.overall.sources[0].is_local_player);
    assert_eq!(
        world
            .get::<DamageMeter>(outsider)
            .unwrap()
            .snapshot(0.0, outsider)
            .current,
        None
    );

    // The group leaves combat: the mage's session ends too.
    app.world_mut()
        .entity_mut(warrior)
        .insert(CombatStatus(false));
    app.update();
    assert!(!app.world().get::<DamageMeter>(mage).unwrap().is_active());
}

#[test]
fn a_group_member_s_fight_keeps_one_session_through_a_lull_until_the_creature_dies() {
    let (mut app, mage, warrior, _) = group_app();
    app.add_systems(Update, crate::combat_refs::expire_combat_refs_system);
    let wolf = app
        .world_mut()
        .spawn((
            Health {
                current: 100.0,
                max: 100.0,
            },
            shared::components::ThreatTable::default(),
        ))
        .id();
    crate::combat_refs::set_in_combat_with(app.world_mut(), warrior, wolf);
    record_damage(app.world_mut(), &damage_line(warrior, wolf, 250));
    // 8 s without a hit.
    for _ in 0..160 {
        app.update();
    }
    record_damage(app.world_mut(), &damage_line(warrior, wolf, 150));
    app.update();
    let meter = app.world().get::<DamageMeter>(mage).unwrap();
    let current = meter.snapshot(0.0, mage).current.unwrap();
    assert_eq!((current.session_id, current.total_amount), (1, 400));
    assert!(meter.is_active());

    // The wolf dies: `CombatStop` ends the warrior's fight, and the group's session.
    crate::combat::combat_stop(app.world_mut(), wolf);
    app.update();
    assert!(!app.world().get::<DamageMeter>(mage).unwrap().is_active());
    assert!(!app.world().get::<DamageMeter>(warrior).unwrap().is_active());
}

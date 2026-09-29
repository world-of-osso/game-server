use std::time::Duration;

use shared::components::{Health, Position};
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

    /// Advance until the caster's combat (5 s grace after the hit) ends.
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
    // In combat from the hit to the end of the 5 s grace.
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

// --- ECS: groups ---

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
        dungeon_difficulty: 1,
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
        periodic: false,
        kind: CombatLogKind::Damage,
    }
}

#[test]
fn a_group_member_s_damage_and_combat_reach_every_member_s_meter() {
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
    let mob = app.world_mut().spawn_empty().id();
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
    assert_eq!(sources, [("Fbwarrior", 250, false)]);
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

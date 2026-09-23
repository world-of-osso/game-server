//! Bevy App tests with real SpellInfo from data/world.db (skipped when absent).

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::casting::CastState;
use shared::components::{
    CombatStatus, Health, Level, Mana, Npc, Player, Position, PowerType, UnitPowers, UnitStats,
};
use shared::protocol::{
    CastFailed, CombatChannel, CombatLogEvent, CombatLogKind, ProtocolPlugin, SpellCastIntent,
    SpellChargesUpdate, SpellCooldownUpdate, StopSpellCast,
};
use shared::spell_data::CastFailReason;

use super::powers::{PowerPools, PowerTypeTable, load_power_type_table};
use crate::auth::{Authenticated, InWorld};
use crate::class_progression::KnownSpells;
use crate::networking::{Moving, PlayerLink};
use crate::spell_info::{
    ClassInfo, ClassSpells, SpellEffect, SpellInfo, SpellStore, load_spell_data,
};

const MORTAL_STRIKE: u32 = 12294;
const FIREBALL: u32 = 133;
const KICK: u32 = 1766;
const FLASH_HEAL: u32 = 2061;
const CHARGE: u32 = 100;

struct WorldData {
    store: SpellStore,
    classes: ClassSpells,
    powers_db: PathBuf,
}

fn world_db() -> Option<&'static WorldData> {
    static DATA: OnceLock<Option<WorldData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
        if !path.exists() {
            eprintln!("skipping: {} not found", path.display());
            return None;
        }
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let (store, classes) = load_spell_data(&conn).unwrap();
        Some(WorldData {
            store,
            classes,
            powers_db: path,
        })
    })
    .as_ref()
}

fn power_table(data: &WorldData) -> PowerTypeTable {
    let conn =
        Connection::open_with_flags(&data.powers_db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    load_power_type_table(&conn).unwrap()
}

fn class_spells(data: &WorldData) -> ClassSpells {
    ClassSpells {
        classes: data
            .classes
            .classes
            .iter()
            .map(|class| ClassInfo {
                class_id: class.class_id,
                skill_line: class.skill_line,
                power_types: class.power_types.clone(),
                learnable: Vec::new(),
                specs: Vec::new(),
            })
            .collect(),
    }
}

struct Harness {
    app: App,
    caster: Entity,
    connection: Entity,
    npc: Entity,
    inbox: Inbox,
}

/// Messages drained from the connection every frame (unread HostClient
/// messages do not survive later frames).
#[derive(Default)]
struct Inbox {
    failed: Vec<CastFailed>,
    cooldowns: Vec<SpellCooldownUpdate>,
    charges: Vec<SpellChargesUpdate>,
    log: Vec<CombatLogEvent>,
}

fn spawn_player(app: &mut App, class: u8, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            Player {
                name: "Caster".into(),
                race: 1,
                class,
                appearance: default(),
            },
            Position { x, y: 0.0, z: 0.0 },
            Health {
                current: 1000.0,
                max: 1000.0,
            },
            Level(10),
            Moving(false),
            // In combat so rage does not decay mid-test (rage test toggles it).
            CombatStatus(true),
            UnitStats {
                stamina: 0.0,
                strength: 100.0,
                agility: 0.0,
                intellect: 100.0,
                spirit: 0.0,
            },
            Mana {
                current: 10_000.0,
                max: 10_000.0,
            },
        ))
        .id()
}

fn spawn_npc(app: &mut App, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            Npc {
                template_id: 1,
                name: "Target".into(),
            },
            Position { x, y: 0.0, z: 0.0 },
            Health {
                current: 5000.0,
                max: 5000.0,
            },
        ))
        .id()
}

fn spawn_connection(app: &mut App, player: Entity) -> Entity {
    app.world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id: 1 },
            PlayerLink(player),
            (
                MessageSender::<SpellCastIntent>::default(),
                MessageReceiver::<SpellCastIntent>::default(),
                MessageSender::<StopSpellCast>::default(),
                MessageReceiver::<StopSpellCast>::default(),
            ),
            (
                MessageSender::<CastFailed>::default(),
                MessageReceiver::<CastFailed>::default(),
                MessageSender::<SpellCooldownUpdate>::default(),
                MessageReceiver::<SpellCooldownUpdate>::default(),
                MessageSender::<SpellChargesUpdate>::default(),
                MessageReceiver::<SpellChargesUpdate>::default(),
                MessageSender::<CombatLogEvent>::default(),
                MessageReceiver::<CombatLogEvent>::default(),
            ),
        ))
        .id()
}

/// Warrior caster (class 1) at the origin, NPC at 3 yd.
fn harness(extra: Vec<SpellInfo>, ids: &[u32], class: u8) -> Option<Harness> {
    let data = world_db()?;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(crate::combat::CombatPlugin);
    app.add_plugins(super::SpellCastPlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.finish();
    app.cleanup();
    // Startup runs first so the real table replaces the missing-DB default.
    app.update();
    let mut spells: Vec<SpellInfo> = ids
        .iter()
        .map(|&id| data.store.get(id).expect("spell in world.db").clone())
        .collect();
    spells.extend(extra);
    let known = KnownSpells::fixture(spells.iter().map(|spell| spell.id).collect());
    app.insert_resource(SpellStore::fixture(spells));
    app.insert_resource(power_table(data));
    app.insert_resource(class_spells(data));
    let caster = spawn_player(&mut app, class, 0.0);
    app.world_mut().entity_mut(caster).insert(known);
    let connection = spawn_connection(&mut app, caster);
    let npc = spawn_npc(&mut app, 3.0);
    app.update();
    Some(Harness {
        app,
        caster,
        connection,
        npc,
        inbox: Inbox::default(),
    })
}

fn drain<M: lightyear::prelude::Message>(app: &mut App, connection: Entity) -> Vec<M> {
    app.world_mut()
        .get_mut::<MessageReceiver<M>>(connection)
        .unwrap()
        .receive()
        .collect()
}

impl Harness {
    fn cast(&mut self, spell_id: u32, target: Option<Entity>) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<SpellCastIntent>>(self.connection)
            .unwrap()
            .send::<CombatChannel>(SpellCastIntent {
                spell_id: Some(spell_id),
                spell: String::new(),
                target_entity: target.map(Entity::to_bits),
            });
    }

    /// Send + HostClient loopback delivery takes 3 frames (as in the old
    /// cast presentation tests); the cast resolves in the receiving frame.
    fn cast_and_deliver(&mut self, spell_id: u32, target: Option<Entity>) {
        self.cast(spell_id, target);
        self.advance(3);
    }

    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
            let inbox = &mut self.inbox;
            inbox
                .failed
                .extend(drain::<CastFailed>(&mut self.app, self.connection));
            inbox
                .cooldowns
                .extend(drain::<SpellCooldownUpdate>(&mut self.app, self.connection));
            inbox
                .charges
                .extend(drain::<SpellChargesUpdate>(&mut self.app, self.connection));
            inbox
                .log
                .extend(drain::<CombatLogEvent>(&mut self.app, self.connection));
        }
    }

    fn health(&self, entity: Entity) -> f32 {
        self.app.world().get::<Health>(entity).unwrap().current
    }

    fn power(&self, power: PowerType) -> f32 {
        let pools = self.app.world().get::<PowerPools>(self.caster).unwrap();
        pools.get(power).unwrap().current
    }

    fn set_power(&mut self, power: PowerType, value: f32) {
        let mut pools = self
            .app
            .world_mut()
            .get_mut::<PowerPools>(self.caster)
            .unwrap();
        pools.get_mut(power).unwrap().current = value;
    }
}

#[test]
fn warrior_gets_rage_pool_replicated_as_unit_powers() {
    let Some(mut h) = harness(Vec::new(), &[], 1) else {
        return;
    };
    h.advance(1);
    let powers = h.app.world().get::<UnitPowers>(h.caster).unwrap();
    assert_eq!(powers.entries.len(), 1);
    assert_eq!(powers.entries[0].power, PowerType::Rage);
    assert_eq!(
        (powers.entries[0].current, powers.entries[0].max),
        (0, 1000)
    );
}

#[test]
fn instant_spell_resolves_same_tick_spending_cost_and_starting_gcd_and_cooldown() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    // AP coefficient 5.2012 × strength 100 = 520.
    assert_eq!(h.health(h.npc), 5000.0 - 520.0);
    assert_eq!(h.power(PowerType::Rage), 200.0, "30 rage = 300 raw spent");
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    h.advance(2);
    let mut cooldowns = std::mem::take(&mut h.inbox.cooldowns);
    cooldowns.sort_by_key(|update| update.is_gcd);
    assert_eq!(
        cooldowns,
        vec![
            SpellCooldownUpdate {
                spell_id: MORTAL_STRIKE,
                category: 0,
                duration_ms: 6000,
                remaining_ms: 6000,
                is_gcd: false,
            },
            SpellCooldownUpdate {
                spell_id: MORTAL_STRIKE,
                category: 133,
                duration_ms: 1500,
                remaining_ms: 1500,
                is_gcd: true,
            },
        ]
    );
    let log = std::mem::take(&mut h.inbox.log);
    assert!(log.iter().any(|e| e.kind == CombatLogKind::CastSuccess));
    let damage = log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage)
        .unwrap();
    assert_eq!((damage.amount, damage.overflow), (520, 0));

    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    let failed = std::mem::take(&mut h.inbox.failed);
    assert_eq!(failed[0].reason, CastFailReason::OnGlobalCooldown);
    h.advance(30);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed)[0].reason,
        CastFailReason::OnCooldown
    );
    assert_eq!(h.health(h.npc), 5000.0 - 520.0);
}

#[test]
fn cast_time_spell_completes_after_its_cast_time() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    let cast = h.app.world().get::<CastState>(h.caster).expect("cast bar");
    assert_eq!(cast.duration, 1.75);
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        10_000.0
    );
    h.advance(30);
    assert!(h.app.world().get::<CastState>(h.caster).unwrap().elapsed < 1.75);
    assert_eq!(h.health(h.npc), 5000.0);
    h.advance(10);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    // SP coefficient 3.8812 × intellect 100 = 388.
    assert_eq!(h.health(h.npc), 5000.0 - 388.0);
    // 1% of max mana (100) spent on completion, then at most 0.25 s of 1%/s regen.
    let mana = h.app.world().get::<Mana>(h.caster).unwrap().current;
    assert!((9_900.0..=9_925.0).contains(&mana), "mana {mana}");
    let kinds: Vec<_> = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            CombatLogKind::CastStart,
            CombatLogKind::CastSuccess,
            CombatLogKind::Damage
        ]
    );
}

#[test]
fn moving_cancels_cast_time_spell_without_effects() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.app.world_mut().get_mut::<Moving>(h.caster).unwrap().0 = true;
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    h.advance(40);
    assert_eq!(h.health(h.npc), 5000.0);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed)[0].reason,
        CastFailReason::CantCastWhileMoving
    );
}

#[test]
fn kick_interrupts_target_cast_and_locks_its_school() {
    let Some(mut h) = harness(Vec::new(), &[KICK, FIREBALL], 4) else {
        return;
    };
    // NPCs do not cast yet; give the NPC a Fireball cast bar directly.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CastState::normal(FIREBALL, 0, 1.75, true));
    h.cast_and_deliver(KICK, Some(h.npc));
    assert!(
        h.app.world().get::<CastState>(h.npc).is_none(),
        "cast interrupted"
    );
    let history = h.app.world().get::<super::SpellHistory>(h.npc).unwrap();
    let now = h.app.world().resource::<Time>().elapsed_secs_f64();
    let fire = 4;
    assert!(history.school_locked(fire, now + 5.5), "6 s Fire lockout");
    assert!(!history.school_locked(fire, now + 6.1));
    assert!(!history.school_locked(1, now), "Physical not locked");
    h.advance(2);
    let log = std::mem::take(&mut h.inbox.log);
    let interrupt = log
        .iter()
        .find(|e| e.kind == CombatLogKind::Interrupt)
        .unwrap();
    assert_eq!(interrupt.spell_id, Some(KICK));
    assert_eq!(interrupt.school_mask, fire);
}

#[test]
fn out_of_range_sends_cast_failed_to_owner() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 30.0;
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed),
        vec![CastFailed {
            spell_id: MORTAL_STRIKE,
            reason: CastFailReason::OutOfRange,
            detail: None,
        }]
    );
    assert_eq!(h.health(h.npc), 5000.0);
    assert_eq!(h.power(PowerType::Rage), 500.0);
}

#[test]
fn heal_clamps_to_max_and_logs_overheal() {
    let Some(mut h) = harness(Vec::new(), &[FLASH_HEAL], 5) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 900.0;
    // No explicit ally target: auto self-cast.
    h.cast_and_deliver(FLASH_HEAL, None);
    h.advance(32);
    assert_eq!(h.health(h.caster), 1000.0);
    let heal = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Heal)
        .unwrap();
    // SP coefficient 6.82 × intellect 100 = 682; 100 effective.
    assert_eq!((heal.amount, heal.overflow), (682, 582));
}

#[test]
fn energize_adds_raw_units() {
    let Some(mut h) = harness(Vec::new(), &[CHARGE], 1) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(true));
    h.cast_and_deliver(CHARGE, Some(h.npc));
    assert_eq!(h.power(PowerType::Rage), 200.0, "Charge: 20 rage = 200 raw");
    h.advance(2);
    let energize = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Energize)
        .unwrap();
    assert_eq!((energize.amount, energize.overflow), (200, 0));
    let charges = std::mem::take(&mut h.inbox.charges);
    assert_eq!(
        (charges[0].current, charges[0].max, charges[0].recharge_ms),
        (0, 1, 20_000)
    );
}

#[test]
fn rage_decays_out_of_combat_only() {
    let Some(mut h) = harness(Vec::new(), &[], 1) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(true));
    h.set_power(PowerType::Rage, 500.0);
    h.advance(20);
    assert_eq!(h.power(PowerType::Rage), 500.0, "no decay in combat");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h.advance(40);
    // RegenPeace -12.5 raw/s over 2 s.
    let rage = h.power(PowerType::Rage);
    assert!((474.0..=476.0).contains(&rage), "rage {rage}");
    let powers = h.app.world().get::<UnitPowers>(h.caster).unwrap();
    assert_eq!(powers.entries[0].current, rage.floor() as i32);
}

/// Synthetic chain 900001 → 900002 → … each energizing 10 raw rage on the
/// caster and triggering the next.
fn trigger_chain(length: u32) -> Vec<SpellInfo> {
    (0..length)
        .map(|step| SpellInfo {
            id: 900_001 + step,
            name: format!("Chain {step}"),
            effects: vec![
                SpellEffect {
                    index: 0,
                    effect: super::effects::EFFECT_ENERGIZE,
                    base_points: 10.0,
                    misc_values: [1, 0],
                    implicit_targets: [1, 0],
                    ..default()
                },
                SpellEffect {
                    index: 1,
                    effect: super::effects::EFFECT_TRIGGER_SPELL,
                    trigger_spell: 900_002 + step,
                    implicit_targets: [1, 0],
                    ..default()
                },
            ],
            ..default()
        })
        .collect()
}

#[test]
fn triggered_spell_chain_is_bounded() {
    let Some(mut h) = harness(trigger_chain(8), &[], 1) else {
        return;
    };
    h.cast_and_deliver(900_001, None);
    // The requested spell plus MAX_TRIGGER_DEPTH (3) triggered hops.
    assert_eq!(h.power(PowerType::Rage), 40.0);
}

#[test]
fn unsupported_implicit_target_fails_the_cast() {
    let mut spell = trigger_chain(1).remove(0);
    spell.effects[0].implicit_targets = [87, 16];
    let Some(mut h) = harness(vec![spell], &[], 1) else {
        return;
    };
    h.cast_and_deliver(900_001, None);
    let failed = std::mem::take(&mut h.inbox.failed);
    assert_eq!(failed[0].reason, CastFailReason::NotSupported);
    assert_eq!(
        failed[0].detail.as_deref(),
        Some("implicit target 87 not supported")
    );
    assert_eq!(h.power(PowerType::Rage), 0.0);
}

#[test]
fn unknown_spell_is_rejected_as_not_known() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(Vec::new()));
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed),
        vec![CastFailed {
            spell_id: MORTAL_STRIKE,
            reason: CastFailReason::NotSupported,
            detail: Some("spell not known".into()),
        }]
    );
    assert_eq!(h.health(h.npc), 5000.0);
    assert_eq!(h.power(PowerType::Rage), 500.0);
}

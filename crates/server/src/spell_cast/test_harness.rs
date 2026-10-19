//! Real-DB Bevy App harness for cast and aura tests: a player caster with an
//! authenticated connection and an NPC target.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{
    CombatStatus, Health, Level, Mana, Npc, Player, Position, PowerType, UnitFactionTemplate,
};
use shared::protocol::{
    AttackStart, AttackStop, AttackStopped, AttackSwing, CancelAura, CastFailed, CombatChannel,
    CombatLogEvent, CombatLogKind, ProtocolPlugin, SetTarget, SpellCastIntent, SpellChargesUpdate,
    SpellCooldownUpdate, SpellFailure, SpellGo, StopSpellCast,
};

use super::powers::{PowerPools, PowerTypeTable, load_power_type_table};
use crate::auth::{Authenticated, InWorld};
use crate::class_progression::KnownSpells;
use crate::networking::{Moving, PlayerLink};
use crate::spell_info::{ClassInfo, ClassSpells, SpellInfo, SpellStore, load_spell_data};
use crate::stats::CharacterStats;

pub(crate) struct WorldData {
    pub(crate) store: SpellStore,
    pub(crate) classes: ClassSpells,
    pub(crate) powers_db: PathBuf,
}

pub(crate) fn place_toy_caster_on_source_area(
    h: &mut Harness,
    fdid: u32,
    point: [f32; 2],
    area: u32,
) {
    place_toy_caster_on_source_map_area(h, 0, fdid, point, area);
}

pub(crate) fn place_toy_caster_on_source_map_area(
    h: &mut Harness,
    map: u32,
    fdid: u32,
    point: [f32; 2],
    area: u32,
) {
    let data = std::fs::read(crate::ground::ground_data_dir().join(format!("terrain/{fdid}.adt")))
        .unwrap();
    let terrain = crate::terrain_height::parse_adt_heights(&data).unwrap();
    assert_eq!(terrain.area_at(point[0], point[1]), Some(area));
    let height = terrain.height_at(point[0], point[1]).unwrap();
    let position = Position {
        x: point[0],
        y: height,
        z: -point[1],
    };
    let tile = crate::ground::bevy_to_tile(position.x, position.z);
    h.app
        .insert_resource(crate::ground::GroundMap::with_terrain_on(
            map, tile, terrain,
        ));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::area_table::load_area_table(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((position, crate::maps::InMap::continent(map)));
}

/// What an effect with `Variance` 0.05 and an amount of `base` deals or
/// heals: `base` ± 2.5%, rounded (SpellEffectInfo::CalcValue).
pub(crate) fn varied(base: f32) -> std::ops::RangeInclusive<f32> {
    (base * 0.975).round()..=(base * 1.025).round()
}

pub(crate) fn world_db() -> Option<&'static WorldData> {
    static DATA: OnceLock<Option<WorldData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let path = crate::test_data::world_db_path();
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

/// Shared process input under target/, never mutable app state or canonical data.
pub(crate) struct PreparedClassWorld {
    directory: std::sync::Arc<tempfile::TempDir>,
}

impl PreparedClassWorld {
    pub(crate) fn path(&self) -> PathBuf {
        self.directory.path().join("world.class-summons.copy.db")
    }

    pub(crate) fn world_data(&self) -> crate::world_data::WorldData {
        static DATA: OnceLock<crate::world_data::WorldData> = OnceLock::new();
        DATA.get_or_init(|| crate::world_data::load_world_data(&self.path()))
            .clone()
    }
}

pub(crate) fn prepare_class_world() -> PreparedClassWorld {
    static DIRECTORY: OnceLock<std::sync::Arc<tempfile::TempDir>> = OnceLock::new();
    PreparedClassWorld {
        directory: DIRECTORY.get_or_init(import_class_world).clone(),
    }
}

fn import_class_world() -> std::sync::Arc<tempfile::TempDir> {
    // The process cache outlives individual fixtures; keep the 800MB snapshot off /tmp tmpfs.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/class-fixtures");
    std::fs::create_dir_all(&root).expect("disk-backed class fixture cache");
    let prepared = PreparedClassWorld {
        directory: std::sync::Arc::new(tempfile::tempdir_in(root).unwrap()),
    };
    let helper = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/prepare_class_summon_db.py");
    let output = std::process::Command::new("python3")
        .arg(helper)
        .arg(crate::test_data::world_db_path())
        .arg(prepared.path())
        .output()
        .expect("run native class fixture snapshot/import");
    assert!(
        output.status.success(),
        "class fixture import failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::File::open(prepared.path())
        .unwrap()
        .set_permissions(std::fs::Permissions::from_mode(0o444))
        .unwrap();
    prepared.directory
}

/// Explicit native template authority for fixtures whose spells restrict creature type.
pub(crate) fn load_native_creature_world(entries: &[u32]) -> crate::world_data::WorldData {
    let conn = Connection::open_with_flags(
        world_db().unwrap().powers_db.clone(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| entries.contains(entry));
    for entry in entries {
        assert!(
            templates.contains_key(entry),
            "missing source fixture creature template{entry}"
        );
    }
    crate::world_data::load_template_models(&conn, &mut templates);
    crate::world_data::WorldData {
        templates,
        expected_stats: expected,
        ..Default::default()
    }
}

pub(crate) fn power_table(data: &WorldData) -> PowerTypeTable {
    let conn =
        Connection::open_with_flags(&data.powers_db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    load_power_type_table(&conn).unwrap()
}

pub(crate) fn shapeshift_forms(data: &WorldData) -> crate::auras::shapeshift::ShapeshiftForms {
    let conn =
        Connection::open_with_flags(&data.powers_db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    crate::auras::shapeshift::load_shapeshift_forms(&conn).unwrap()
}

pub(crate) fn class_spells(data: &WorldData) -> ClassSpells {
    ClassSpells {
        classes: data
            .classes
            .classes
            .iter()
            .map(|class| ClassInfo {
                class_id: class.class_id,
                skill_line: class.skill_line,
                spell_class_set: class.spell_class_set,
                power_types: class.power_types.clone(),
                attack_power_per_strength: class.attack_power_per_strength,
                attack_power_per_agility: class.attack_power_per_agility,
                ranged_attack_power_per_agility: class.ranged_attack_power_per_agility,
                primary_stat_priority: class.primary_stat_priority,
                learnable: Vec::new(),
                specs: Vec::new(),
                traits: class.traits.clone(),
                default_skills: Vec::new(),
            })
            .collect(),
    }
}

pub(crate) struct Harness {
    pub(crate) app: App,
    pub(crate) caster: Entity,
    pub(crate) connection: Entity,
    pub(crate) npc: Entity,
    pub(crate) inbox: Inbox,
}

/// Messages drained from the connection every frame (unread HostClient
/// messages do not survive later frames).
#[derive(Default)]
pub(crate) struct Inbox {
    pub(crate) failed: Vec<CastFailed>,
    pub(crate) inventory_errors: Vec<shared::protocol::InventoryError>,
    pub(crate) go: Vec<SpellGo>,
    pub(crate) cooldowns: Vec<SpellCooldownUpdate>,
    pub(crate) charges: Vec<SpellChargesUpdate>,
    pub(crate) bars: Vec<shared::protocol::ActionBarSnapshot>,
    pub(crate) log: Vec<CombatLogEvent>,
    pub(crate) attack_starts: Vec<AttackStart>,
    pub(crate) attack_stops: Vec<AttackStopped>,
}

pub(crate) fn spawn_player(app: &mut App, class: u8, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            Player {
                name: "Caster".into(),
                race: 1,
                class,
                appearance: default(),
            },
            Position { x, y: 0.0, z: 0.0 },
            crate::maps::InMap::continent(0),
            Health {
                current: 1000.0,
                max: 1000.0,
            },
            Level(10),
            // Human.
            UnitFactionTemplate(1),
            shared::components::PlayerStandState::default(),
            Moving(false),
            crate::networking::CombatTarget::default(),
            // In combat so rage does not decay mid-test (rage test toggles it).
            CombatStatus(true),
            CharacterStats {
                spell_power: 100.0,
                healing_power: 100.0,
                attack_power: 100.0,
                ranged_attack_power: 100.0,
                base_mana: 10_000.0,
                ..default()
            },
            Mana {
                current: 10_000.0,
                max: 10_000.0,
            },
        ))
        .id()
}

pub(crate) fn spawn_npc(app: &mut App, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            crate::zones::CreatureFlagsExtra(
                crate::test_data::world_data().templates[&6].flags_extra,
            ),
            // Kobold Vermin: hostile to the Human caster.
            UnitFactionTemplate(25),
            Position { x, y: 0.0, z: 0.0 },
            crate::maps::InMap::continent(0),
            Health {
                current: 5000.0,
                max: 5000.0,
            },
        ))
        .id()
}

pub(crate) fn spawn_connection(app: &mut App, player: Entity) -> Entity {
    spawn_peer_connection(app, player, 1)
}

/// An authenticated in-world connection of local peer `peer` playing `player`.
pub(crate) fn spawn_peer_connection(app: &mut App, player: Entity, peer: u64) -> Entity {
    app.world_mut()
        .spawn((
            RemoteId(PeerId::Local(peer)),
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
                MessageSender::<CancelAura>::default(),
                MessageReceiver::<CancelAura>::default(),
                MessageSender::<shared::protocol::CancelMountAura>::default(),
                MessageReceiver::<shared::protocol::CancelMountAura>::default(),
                MessageSender::<shared::protocol::StandStateIntent>::default(),
                MessageReceiver::<shared::protocol::StandStateIntent>::default(),
            ),
            (
                MessageSender::<CastFailed>::default(),
                MessageReceiver::<CastFailed>::default(),
                MessageSender::<shared::protocol::InventoryError>::default(),
                MessageReceiver::<shared::protocol::InventoryError>::default(),
                MessageSender::<SpellCooldownUpdate>::default(),
                MessageReceiver::<SpellCooldownUpdate>::default(),
                MessageSender::<SpellChargesUpdate>::default(),
                MessageReceiver::<SpellChargesUpdate>::default(),
                MessageSender::<CombatLogEvent>::default(),
                MessageReceiver::<CombatLogEvent>::default(),
                MessageSender::<SpellGo>::default(),
                MessageReceiver::<SpellGo>::default(),
                MessageSender::<SpellFailure>::default(),
                MessageReceiver::<SpellFailure>::default(),
            ),
            (
                MessageSender::<AttackSwing>::default(),
                MessageReceiver::<AttackSwing>::default(),
                MessageSender::<AttackStop>::default(),
                MessageReceiver::<AttackStop>::default(),
                MessageSender::<AttackStart>::default(),
                MessageReceiver::<AttackStart>::default(),
                MessageSender::<AttackStopped>::default(),
                MessageReceiver::<AttackStopped>::default(),
                MessageSender::<SetTarget>::default(),
                MessageReceiver::<SetTarget>::default(),
            ),
            (
                MessageSender::<shared::protocol::ActionBarSnapshot>::default(),
                MessageReceiver::<shared::protocol::ActionBarSnapshot>::default(),
            ),
        ))
        .id()
}

/// Level 10 Human caster of `class` at the origin with fixed
/// `CharacterStats` (100 SP/AP, 10 000 base mana), hostile kobold at 3 yd.
pub(crate) fn harness(extra: Vec<SpellInfo>, ids: &[u32], class: u8) -> Option<Harness> {
    harness_with(extra, ids, class, false)
}

pub(crate) const SUNWELL_RADIANCE: u32 = 45769;

/// Opt-in coefficient/admission fixture, not a legal class loadout. Native
/// Sunwell Radiance45769 effect1 supplies permanent enemy-dodge reduction20
/// (published70391 SpellEffect37781, SpellDuration21). Keep ordinary NPC dodge
/// active in all other fixtures; never change RNG or spell defense attributes.
pub(crate) fn harness_with_native_enemy_dodge_reduction(
    extra: Vec<SpellInfo>,
    ids: &[u32],
    class: u8,
) -> Option<Harness> {
    let mut spells = ids.to_vec();
    spells.push(SUNWELL_RADIANCE);
    let mut h = harness(extra, &spells, class)?;
    apply_native_enemy_dodge_reduction(&mut h);
    Some(h)
}

/// Custom coefficient fixtures load SUNWELL_RADIANCE explicitly before opting in.
pub(crate) fn apply_native_enemy_dodge_reduction(h: &mut Harness) {
    apply_native_enemy_dodge_reduction_to(h, h.caster);
}

/// Pet and foreign-player damage fixtures must qualify the actual attacker.
pub(crate) fn apply_native_enemy_dodge_reduction_to(h: &mut Harness, attacker: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: attacker,
            target: attacker,
            spell_id: SUNWELL_RADIANCE,
            effect_mask: 1 << 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(attacker)
            .unwrap()
            .modifier(251),
        -20.0
    );
}

/// `computed_stats`: run `StatsPlugin` on a fresh caster (100/100 health,
/// 0/0 mana, as spawned on world entry) instead of the fixed stats.
pub(crate) fn harness_with(
    extra: Vec<SpellInfo>,
    ids: &[u32],
    class: u8,
    computed_stats: bool,
) -> Option<Harness> {
    harness_with_plugins(extra, ids, class, computed_stats, |_| {})
}

/// `harness_with` plus the plugins `add_plugins` adds before the app is finished.
pub(crate) fn harness_with_plugins(
    extra: Vec<SpellInfo>,
    ids: &[u32],
    class: u8,
    computed_stats: bool,
    add_plugins: impl FnOnce(&mut App),
) -> Option<Harness> {
    let data = world_db()?;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(crate::combat::CombatPlugin);
    app.add_plugins(super::SpellCastPlugin);
    app.add_plugins(crate::auras::AuraPlugin);
    app.add_plugins(crate::creature_scripts::CreatureScriptsPlugin);
    app.add_plugins(crate::stand_state::StandStatePlugin);
    app.add_systems(Update, crate::messaging::process_target_messages);
    if computed_stats {
        app.add_plugins(crate::stats::StatsPlugin);
    }
    add_plugins(&mut app);
    app.insert_resource(
        crate::conditions::load_spell_target_conditions(&crate::test_data::world_db_path())
            .expect("authored spell target conditions from fixture world.db"),
    );
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
    app.insert_resource(crate::item_stats::load_item_scaling_tables(&data.powers_db).unwrap());
    app.insert_resource(shapeshift_forms(data));
    let conn =
        Connection::open_with_flags(&data.powers_db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    app.insert_resource(crate::unit_creature_type::load_race_types(&conn).unwrap());
    app.insert_resource(
        crate::maps::store::load_map_store(&conn).expect("Actual passive map eligibility metadata"),
    );
    app.insert_resource(class_spells(data));
    app.insert_resource(crate::faction::world_db_faction_templates()?);
    let caster = spawn_player(&mut app, class, 0.0);
    if computed_stats {
        app.world_mut()
            .entity_mut(caster)
            .remove::<CharacterStats>()
            .insert((
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Mana {
                    current: 0.0,
                    max: 0.0,
                },
                shared::components::UnitStats::default(),
                shared::components::CombatRatings::default(),
                shared::components::DerivedStats::default(),
            ));
    }
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

/// Charge's target (its DUMMY effect, `TARGET_UNIT_TARGET_ENEMY`) must stand within
/// SpellRange 95, 8-25 yards: move the kobold to 10 yards.
pub(crate) fn charge_range(h: &mut Harness) {
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    });
}

pub(crate) fn drain<M: lightyear::prelude::Message>(app: &mut App, connection: Entity) -> Vec<M> {
    app.world_mut()
        .get_mut::<MessageReceiver<M>>(connection)
        .unwrap()
        .receive()
        .collect()
}

impl Harness {
    pub(crate) fn cast(&mut self, spell_id: u32, target: Option<Entity>) {
        self.cast_witnessed(spell_id, target, None);
    }

    /// A cast carrying the line of sight `witness` ray.
    pub(crate) fn cast_witnessed(
        &mut self,
        spell_id: u32,
        target: Option<Entity>,
        witness: Option<shared::protocol::WitnessRay>,
    ) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<SpellCastIntent>>(self.connection)
            .unwrap()
            .send::<CombatChannel>(SpellCastIntent {
                target_item_guid: None,
                destination: None,
                spell_id: Some(spell_id),
                spell: String::new(),
                target_entity: target.map(Entity::to_bits),
                witness,
            });
    }

    /// `CMSG_SET_SELECTION`: select `target`, or clear the selection.
    pub(crate) fn select(&mut self, target: Option<Entity>) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<SetTarget>>(self.connection)
            .unwrap()
            .send::<CombatChannel>(SetTarget {
                target_entity: target.map(Entity::to_bits),
            });
    }

    /// `CMSG_ATTACK_SWING` at `target`; like a cast, it arrives 3 frames later.
    pub(crate) fn attack(&mut self, target: Entity) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<AttackSwing>>(self.connection)
            .unwrap()
            .send::<CombatChannel>(AttackSwing {
                target: target.to_bits(),
            });
    }

    /// `CMSG_ATTACK_STOP`.
    pub(crate) fn stop_attack(&mut self) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<AttackStop>>(self.connection)
            .unwrap()
            .send::<CombatChannel>(AttackStop);
    }

    /// Send + HostClient loopback delivery takes 3 frames (as in the old
    /// cast presentation tests); the cast resolves in the receiving frame.
    pub(crate) fn cast_and_deliver(&mut self, spell_id: u32, target: Option<Entity>) {
        self.cast(spell_id, target);
        self.advance(3);
    }

    pub(crate) fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            crate::navigation::drain_fixture(&mut self.app);
            self.app.update();
            let inbox = &mut self.inbox;
            inbox
                .failed
                .extend(drain::<CastFailed>(&mut self.app, self.connection));
            inbox
                .inventory_errors
                .extend(drain::<shared::protocol::InventoryError>(
                    &mut self.app,
                    self.connection,
                ));
            inbox
                .go
                .extend(drain::<SpellGo>(&mut self.app, self.connection));
            inbox
                .cooldowns
                .extend(drain::<SpellCooldownUpdate>(&mut self.app, self.connection));
            inbox
                .charges
                .extend(drain::<SpellChargesUpdate>(&mut self.app, self.connection));
            inbox
                .bars
                .extend(drain::<shared::protocol::ActionBarSnapshot>(
                    &mut self.app,
                    self.connection,
                ));
            inbox
                .log
                .extend(drain::<CombatLogEvent>(&mut self.app, self.connection));
            inbox
                .attack_starts
                .extend(drain::<AttackStart>(&mut self.app, self.connection));
            inbox
                .attack_stops
                .extend(drain::<AttackStopped>(&mut self.app, self.connection));
        }
    }

    pub(crate) fn health(&self, entity: Entity) -> f32 {
        self.app.world().get::<Health>(entity).unwrap().current
    }

    pub(crate) fn power(&self, power: PowerType) -> f32 {
        let pools = self.app.world().get::<PowerPools>(self.caster).unwrap();
        pools.get(power).unwrap().current
    }

    pub(crate) fn set_power(&mut self, power: PowerType, value: f32) {
        let mut pools = self
            .app
            .world_mut()
            .get_mut::<PowerPools>(self.caster)
            .unwrap();
        pools.get_mut(power).unwrap().current = value;
    }
}

pub(crate) fn assert_combat_log_metadata(
    events: &[shared::protocol::CombatLogEvent],
    before: std::time::SystemTime,
) {
    let after = std::time::SystemTime::now();
    let unix_ms = |time: std::time::SystemTime| {
        time.duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    };
    assert!(!events.is_empty());
    for event in events {
        assert!(
            (unix_ms(before)..=unix_ms(after)).contains(&event.timestamp_unix_ms),
            "event must be stamped with wall-clock time at emission: {event:?}"
        );
        if !matches!(event.kind, CombatLogKind::Interrupt | CombatLogKind::Dispel) {
            assert_eq!(event.extra_spell_id, None, "{event:?}");
        }
    }
}

use std::path::PathBuf;
use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{
    AppMessageExt, Connected, MessageReceiver, MessageSender, PeerId, RemoteId,
};
use rusqlite::{Connection, OpenFlags};
use shared::protocol::{AuthChannel, EnterWorldResponse, LoadTerrain, SelectCharacter};

use super::*;
use crate::auth::Authenticated;
use crate::persistence::{self, ClientId, GameDatabase};

const HUMAN: u8 = 1;
const ORC: u8 = 2;
const WARRIOR: u8 = 1;
const PALADIN: u8 = 2;
const MAGE: u8 = 8;
const SPEC_HOLY: u32 = 65;

fn paladin_80_holy() -> StatInputs {
    StatInputs {
        class: PALADIN,
        race: HUMAN,
        level: 80,
        primary_stat: Some(PrimaryStat::Intellect),
        // Mastery: Lightbringer 183997 effect 0 EffectBonusCoefficient.
        mastery_coefficient: 1.5,
        attack_power_per_strength: 1.0,
        attack_power_per_agility: 0.0,
        gear_primary: UnitStats::default(),
        gear_ratings: CombatRatings::default(),
    }
}

#[test]
fn level_80_human_holy_paladin_base_stats() {
    let stats = compute_character_stats(&paladin_80_holy(), None).unwrap();
    // SimC class base L80 Paladin (159, 56, 2325, 159); Human +0.
    assert_eq!(
        stats.primary,
        UnitStats {
            strength: 159.0,
            agility: 56.0,
            stamina: 2325.0,
            intellect: 159.0,
            spirit: 0.0,
        }
    );
    // 2325 Sta × gtHpPerSta(80) 20.
    assert_eq!(stats.max_health, 46_500.0);
    assert_eq!(stats.base_mana, 50_000.0);
    assert_eq!((stats.spell_power, stats.attack_power), (159.0, 159.0));
    assert_eq!((stats.crit_pct, stats.haste_pct), (5.0, 0.0));
    // 8 base mastery points × 1.5.
    assert_eq!(stats.mastery_pct, 12.0);
}

#[test]
fn level_10_orc_warrior_base_stats_include_racial_mods() {
    let input = StatInputs {
        class: WARRIOR,
        race: ORC,
        level: 10,
        primary_stat: Some(PrimaryStat::Strength),
        mastery_coefficient: 0.0,
        attack_power_per_strength: 1.0,
        attack_power_per_agility: 0.0,
        gear_primary: UnitStats::default(),
        gear_ratings: CombatRatings::default(),
    };
    let stats = compute_character_stats(&input, None).unwrap();
    // Warrior L10 (33, 23, 555, 26) + Orc (+3, -3, +1, -1).
    assert_eq!(
        stats.primary,
        UnitStats {
            strength: 36.0,
            agility: 20.0,
            stamina: 556.0,
            intellect: 25.0,
            spirit: 0.0,
        }
    );
    // 556 × gtHpPerSta(10) 2.
    assert_eq!(stats.max_health, 1112.0);
    assert_eq!(stats.base_mana, 0.0);
    assert_eq!(stats.attack_power, 36.0);
    // Strength main stat: Intellect gives no spell power.
    assert_eq!(stats.spell_power, 0.0);
    assert_eq!(base_mana(WARRIOR, 10), None);
}

#[test]
fn gear_counts_only_the_spec_primary_stat_and_ratings_convert_per_level() {
    let mut input = paladin_80_holy();
    input.level = 90;
    input.gear_primary = UnitStats {
        strength: 100.0,
        intellect: 100.0,
        stamina: 100.0,
        ..default()
    };
    // gtCombatRatings L90: 46 crit, 44 haste, 46 mastery rating per 1% / point.
    input.gear_ratings = CombatRatings {
        crit: 460.0,
        haste: 440.0,
        mastery: 92.0,
        armor: 3000.0,
        ..default()
    };
    let stats = compute_character_stats(&input, None).unwrap();
    // Paladin L90 base (620, 217, 4600, 620): gear Str ignored for Holy.
    assert_eq!(stats.primary.strength, 620.0);
    assert_eq!(stats.primary.intellect, 720.0);
    assert_eq!(stats.max_health, 4700.0 * 20.0);
    assert_eq!(stats.base_mana, 250_000.0);
    assert!((stats.crit_pct - 15.0).abs() < 1e-4, "{}", stats.crit_pct);
    assert!((stats.haste_pct - 10.0).abs() < 1e-4, "{}", stats.haste_pct);
    assert!(
        (stats.mastery_pct - 15.0).abs() < 1e-4,
        "{}",
        stats.mastery_pct
    );
    assert_eq!(stats.armor, 3000.0);
}

#[test]
fn stat_modifiers_fold_in_after_base_and_gear() {
    let mods = StatModifiers {
        primary_flat: UnitStats {
            intellect: 41.0,
            ..default()
        },
        primary_pct: UnitStats {
            intellect: 10.0,
            stamina: 10.0,
            ..default()
        },
        crit_pct: 2.0,
        max_health_pct: 10.0,
        ..default()
    };
    let stats = compute_character_stats(&paladin_80_holy(), Some(&mods)).unwrap();
    // (159 + 41) × 1.1 = 220 Int.
    assert!((stats.spell_power - 220.0).abs() < 1e-3);
    // floor(2325 × 1.1 = 2557.5) × 20 × 1.1.
    assert_eq!(stats.max_health, 56_254.0);
    assert_eq!(stats.crit_pct, 7.0);
}

fn world_spell_data() -> Option<(SpellStore, ClassSpells)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return None;
    }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some(crate::spell_info::load_spell_data(&conn).unwrap())
}

/// App running the real character-select path plus `StatsPlugin`.
fn enter_world_app(db: GameDatabase, spells: SpellStore, classes: ClassSpells) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.insert_resource(db);
    app.insert_resource(spells);
    app.insert_resource(classes);
    app.insert_resource(crate::maps::MapStore::continents());
    app.init_resource::<crate::maps::InstanceManager>();
    app.add_systems(Update, crate::auth_character::process_select_character);
    app.add_plugins(StatsPlugin);
    app.add_systems(
        Update,
        crate::experience::refill_after_level_up.after(update_character_stats),
    );
    app.finish();
    app.cleanup();
    app
}

fn select_character(app: &mut App, account_id: u64, character_id: u64) -> Entity {
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
        ))
        .id();
    app.world_mut()
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter { character_id });
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .query::<(Entity, &ClientId)>()
        .iter(app.world())
        .find_map(|(entity, client)| (client.0 == character_id).then_some(entity))
        .expect("selected character spawns")
}

#[test]
fn entering_world_rescales_persisted_health_and_follows_spec_and_admin_level() {
    let Some((spells, classes)) = world_spell_data() else {
        return;
    };
    let db = GameDatabase {
        db: persistence::temp_database(),
    };
    let (character_id, _) = persistence::create_character(&db.db, 3, "Uther", HUMAN, PALADIN)
        .expect("create character");
    // An existing character saved with the old 100-health placeholder, at half health.
    let mut data = persistence::load_character(&db.db, character_id).unwrap();
    data.level = 80;
    data.health = 50.0;
    data.max_health = 100.0;
    persistence::save_character(&db.db, character_id, &data);

    let mut app = enter_world_app(db, spells, classes);
    let player = select_character(&mut app, 3, character_id);

    let world = app.world();
    assert_eq!(
        *world.get::<Health>(player).unwrap(),
        Health {
            current: 23_250.0,
            max: 46_500.0
        }
    );
    assert_eq!(
        *world.get::<Mana>(player).unwrap(),
        Mana {
            current: 50_000.0,
            max: 50_000.0
        }
    );
    assert_eq!(world.get::<UnitStats>(player).unwrap().intellect, 159.0);
    // No spec yet: the Paladin class main stat is Strength
    // (chr_classes.PrimaryStatPriority 4), so Intellect gives no spell power.
    let stats = world.get::<CharacterStats>(player).unwrap();
    assert_eq!((stats.spell_power, stats.mastery_pct), (0.0, 0.0));

    // class_progression resolves Holy at 80; spell power and the mastery
    // coefficient follow.
    app.world_mut()
        .entity_mut(player)
        .insert(ActiveSpec(SPEC_HOLY));
    app.update();
    let stats = app.world().get::<CharacterStats>(player).unwrap();
    assert_eq!((stats.spell_power, stats.mastery_pct), (159.0, 12.0));

    let response = app
        .world_mut()
        .run_system_once(|db: Res<GameDatabase>, mut commands: Commands| {
            crate::admin_handlers::handle_set_level(&db, &mut commands, "Uther", 10)
        })
        .unwrap();
    assert!(matches!(
        response,
        crate::admin_types::AdminResponse::LevelSet { level: 10, .. }
    ));
    app.update();
    // Paladin L10 Sta 555 × 2. The level change refills the half-health
    // character (Player::GiveLevel → SetFullHealth).
    assert_eq!(
        *app.world().get::<Health>(player).unwrap(),
        Health {
            current: 1110.0,
            max: 1110.0
        }
    );
    assert_eq!(
        *app.world().get::<Mana>(player).unwrap(),
        Mana {
            current: 1000.0,
            max: 1000.0
        }
    );
}

#[test]
fn equal_recompute_does_not_mark_health_changed() {
    let mut world = World::new();
    let entity = world
        .spawn((
            Player {
                name: "Probe".into(),
                race: HUMAN,
                class: WARRIOR,
                appearance: default(),
            },
            Level(10),
            UnitStats::default(),
            CombatRatings::default(),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    world.run_system_once(update_character_stats).unwrap();
    assert_eq!(world.get::<Health>(entity).unwrap().max, 1110.0);
    let tick = world
        .entity(entity)
        .get_ref::<Health>()
        .unwrap()
        .last_changed();
    world.get_mut::<Level>(entity).unwrap().set_changed();
    world.run_system_once(update_character_stats).unwrap();
    assert_eq!(
        world
            .entity(entity)
            .get_ref::<Health>()
            .unwrap()
            .last_changed(),
        tick
    );
}

/// A max change keeps the health share in whole points (TrinityCore health is integer):
/// 999/1000 becomes 1108/1110, not a fractional 1108.89 the client shows as 1108 forever;
/// an exact half stays exact.
#[test]
fn max_health_change_keeps_whole_health_points() {
    let mut world = World::new();
    let spawn = |world: &mut World, current: f32, max: f32| {
        world
            .spawn((
                Player {
                    name: "Probe".into(),
                    race: HUMAN,
                    class: WARRIOR,
                    appearance: default(),
                },
                Level(10),
                UnitStats::default(),
                CombatRatings::default(),
                Health { current, max },
            ))
            .id()
    };
    let damaged = spawn(&mut world, 999.0, 1000.0);
    let half = spawn(&mut world, 50.0, 100.0);
    world.run_system_once(update_character_stats).unwrap();
    assert_eq!(
        *world.get::<Health>(damaged).unwrap(),
        Health {
            current: 1108.0,
            max: 1110.0
        }
    );
    assert_eq!(world.get::<Health>(half).unwrap().current, 555.0);
}

#[test]
fn secondary_dr_follows_curve_21024() {
    assert_eq!(secondary_dr(10.0), 10.0);
    assert_eq!(secondary_dr(30.0), 30.0);
    assert_eq!(secondary_dr(45.0), 43.0);
    assert_eq!(secondary_dr(250.0), 126.0);
    // 46 crit rating = 1% at level 90: 2300 rating = 50% raw → 47% after DR.
    let mut input = paladin_80_holy();
    input.level = 90;
    input.gear_ratings.crit = 2300.0;
    let stats = compute_character_stats(&input, None).unwrap();
    assert!((stats.crit_pct - 52.0).abs() < 1e-3, "{}", stats.crit_pct);
}

/// TC master avoidance (StatSystem.cpp:490-715): a level 10 Warrior with a weapon, a
/// shield, Parry and Block parries 5%, blocks 5% and dodges nothing without rating;
/// 10% of dodge rating diminishes to 65.63144 × 10 / (10 + 65.63144 × 0.956) = 9.022%.
#[test]
fn player_avoidance_follows_trinitycore_master() {
    let inputs = DefenseInputs {
        can_parry: true,
        can_block: true,
        has_weapon: true,
        shield_armor: Some(100.0),
    };
    let none = player_defense(WARRIOR, 10, &CombatRatings::default(), inputs);
    assert_eq!(
        none,
        PlayerDefense {
            dodge: 0.0,
            parry: 5.0,
            block: 5.0,
            shield_block: 250.0,
        }
    );
    let rated = player_defense(
        WARRIOR,
        10,
        &CombatRatings {
            dodge: 10.0 * crate::character_stats_data::DODGE_RATING[9],
            ..default()
        },
        inputs,
    );
    assert!((rated.dodge - 9.0223).abs() < 1e-3, "{}", rated.dodge);
    // No Parry spell, no weapon or no parry cap (Mage) parries nothing; no Block spell
    // or no shield blocks nothing.
    let unarmed = DefenseInputs {
        has_weapon: false,
        ..inputs
    };
    assert_eq!(
        player_defense(WARRIOR, 10, &CombatRatings::default(), unarmed).parry,
        0.0
    );
    assert_eq!(
        player_defense(MAGE, 10, &CombatRatings::default(), inputs).parry,
        0.0
    );
    let no_block = DefenseInputs {
        can_block: false,
        ..inputs
    };
    assert_eq!(
        player_defense(WARRIOR, 10, &CombatRatings::default(), no_block).block,
        0.0
    );
}

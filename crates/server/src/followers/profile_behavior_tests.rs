//! Every authenticated local class/spec profile uses the production initializer and
//! real spell/aura/cost executor. No invented follower identities or spell rows.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins, spawn_npc, world_db};
use shared::components::{Health, Npc, UnitPowers};

pub(super) fn spawn_profile_fixture(entry: u32) -> (Harness, Entity, Profile) {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(entry, |_| {});
    h.attack(h.npc);
    h.advance(3);
    (h, unit, profile)
}

pub(super) fn spawn_profile_fixture_with_plugins(
    entry: u32,
    configure: impl FnOnce(&mut App),
) -> (Harness, Entity, Profile) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let profile = catalog::load_catalog(&db)
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.entry == entry)
        .expect("authenticated imported follower profile");
    let mut h = harness_with_plugins(vec![], &[], 8, false, configure).unwrap();
    // Full native metadata includes actual form/passive/trigger dependencies.
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[entry]);
    if let Some(mut data) = h
        .app
        .world_mut()
        .get_resource_mut::<crate::world_data::WorldData>()
    {
        data.templates.extend(native.templates);
        data.expected_stats = native.expected_stats;
    } else {
        h.app.insert_resource(native);
    }
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert((
        Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        },
        UnitPowers::default(),
    ));
    spawning::initialize(h.app.world_mut(), h.caster, unit, profile.clone()).unwrap();
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    (h, unit, profile)
}

#[test]
fn follower_selection_is_not_engagement_but_native_attack_is() {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(209059, |_| {});
    let enemy = h.npc;
    let before = h.health(enemy);
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(unit)
        .unwrap();
    for _ in 0..90 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(h.health(enemy), before, "selection alone must not pull");
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Position>(unit)
            .unwrap(),
        position
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(unit)
            .is_some()
    );
    h.attack(enemy);
    h.advance(3);
    assert!(
        h.inbox
            .attack_starts
            .iter()
            .any(|event| event.attacker == h.caster.to_bits())
    );
    for _ in 0..90 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.inbox.log.iter().any(|event| {
            event.source == Some(unit.to_bits())
                && event.target == Some(enemy.to_bits())
                && event.spell_id == Some(profile.attack_spell)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        }),
        "real owner attack must engage Meredy's authored attack spell"
    );
    for event in h.inbox.log.iter().filter(|event| {
        event.source == Some(unit.to_bits())
            && event.target == Some(enemy.to_bits())
            && event.spell_id == Some(profile.attack_spell)
            && event.kind == shared::protocol::CombatLogKind::Damage
    }) {
        assert!(event.amount > 0, "{event:?}");
        assert!((0..=event.amount).contains(&event.overflow), "{event:?}");
    }
}

#[test]
fn follower_party_threat_engages_without_selection_or_owner_auto_attack() {
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(209059, |_| {});
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    let mut threat = shared::components::ThreatTable::default();
    threat.add_threat(h.caster.to_bits(), 100.0);
    h.app.world_mut().entity_mut(enemy).insert(threat);
    for _ in 0..90 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(unit.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage),
        "party threat from landed combat must engage follower"
    );
}

#[test]
fn follower_lethal_melee_delivers_one_native_death_to_party_owner() {
    let (mut h, tank, _) = spawn_profile_fixture(209057);
    h.app.world_mut().get_mut::<Health>(tank).unwrap().current = 1.0;
    let enemy = h.npc;
    h.app.world_mut().entity_mut(enemy).insert((
        crate::networking::CombatTarget(Some(tank)),
        crate::combat::AutoAttack::new(tank),
        crate::auras::AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..Default::default()
        },
    ));
    h.advance(200);
    assert_eq!(h.health(tank), 0.0);
    let deaths: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Death
                && event.target == Some(tank.to_bits())
        })
        .collect();
    assert_eq!(
        deaths.len(),
        1,
        "owner must receive exactly one follower death"
    );
    assert_eq!(deaths[0].source, Some(enemy.to_bits()));
    assert!(deaths[0].timestamp_unix_ms > 0);
}

/// The real summon starts at the authored template's level, then the bridge
/// promotes the NPC to its owner's level. Its combat armor must use that level.
#[test]
fn follower_garrick_level32_uses_installed_level32_creature_armor() {
    let (mut h, original, profile) = spawn_profile_fixture(209057);
    h.app.world_mut().despawn(original);
    h.app.world_mut().entity_mut(h.caster).insert((
        shared::components::Level(32),
        shared::components::UnitLevel(32),
    ));
    let data = crate::world_data::load_world_data(&world_db().unwrap().powers_db);
    h.app.insert_resource(data);
    let unit = spawning::spawn(h.app.world_mut(), h.caster, profile).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitLevel>(unit)
            .unwrap()
            .0,
        32
    );
    let armor = h
        .app
        .world()
        .get::<crate::level_scaling::CreatureArmor>(unit)
        .unwrap()
        .0;
    // Installed70391 ExpectedStat32: CreatureArmor159, class-mod2=1 and
    // Garrick's authored ArmorModifier=1. TC GetBaseArmorForLevel uses these.
    assert_eq!(
        armor, 159.0,
        "owner-level32 follower retained template-level1 armor"
    );
}

fn assert_profile_combat(entry: u32) {
    let (mut h, unit, profile) = spawn_profile_fixture(entry);
    let _summon_source = if profile.spec_id == 266 {
        let prepared = crate::spell_cast::test_harness::prepare_class_world();
        let mut data = crate::world_data::load_world_data(&prepared.path());
        // Keep native defense metadata for the harness's live Kobold target too.
        data.templates.retain(|entry, _| [6, 17252].contains(entry));
        h.app.insert_resource(data);
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(crate::pet_control::load_pet_levelup_spells(&db).unwrap());
        Some(prepared)
    } else {
        None
    };
    let enemy = h.npc;
    if profile.role == GroupRole::Healer {
        h.app
            .world_mut()
            .get_mut::<Health>(h.caster)
            .unwrap()
            .current = 100.0;
    }
    if profile.role == GroupRole::Tank {
        // Taunts retain native creature-melee miss rolls. This success fixture
        // fixes only the hit modifier; the real miss boundary is tested below.
        h.app
            .world_mut()
            .get_mut::<crate::auras::AuraStatMods>(unit)
            .unwrap()
            .melee_hit_chance_pct = 100.0;
        h.app.world_mut().entity_mut(enemy).insert((
            shared::components::ThreatTable::default(),
            crate::creature_ai::CreatureState::Pursuing {
                target: h.caster.to_bits(),
            },
            crate::networking::CombatTarget(Some(h.caster)),
        ));
        crate::creature_aggro::add_threat(h.app.world_mut(), enemy, h.caster, 100.0);
        update_ai(h.app.world_mut());
        h.advance(4);
        assert_eq!(
            h.app
                .world()
                .get::<crate::networking::CombatTarget>(enemy)
                .unwrap()
                .0,
            Some(unit),
            "{} must taunt the ally's attacker with spell{}; casts {:?}; combat {:?}",
            profile.name,
            profile.taunt_spell,
            h.inbox.go,
            h.inbox.log
        );
    }
    let before = h.health(enemy);
    for _ in 0..200 {
        update_ai(h.app.world_mut());
        h.advance(1);
        if profile.role == GroupRole::Healer && h.health(h.caster) > 100.0 {
            break;
        }
    }
    if profile.role == GroupRole::Healer {
        assert!(
            h.health(h.caster) > 100.0,
            "{} must heal the injured owner through spell{}",
            profile.name,
            profile.heal_spell
        );
        assert!(
            !h.inbox
                .log
                .iter()
                .any(|event| event.source == Some(unit.to_bits())
                    && event.kind == shared::protocol::CombatLogKind::Damage),
            "{} must triage before damage",
            profile.name
        );
    } else {
        assert!(
            h.health(enemy) < before,
            "{} class{} spec{} must damage with its native priority spells{}/{}; actual casts: {:?}",
            profile.name,
            profile.class_id,
            profile.spec_id,
            profile.attack_spell,
            profile.cooldown_spell,
            h.inbox
                .go
                .iter()
                .filter(|event| event.caster == unit.to_bits())
                .map(|event| event.spell_id)
                .collect::<Vec<_>>()
        );
    }
    assert!(
        h.app
            .world()
            .get::<UnitPowers>(unit)
            .is_some_and(|powers| !powers.entries.is_empty()),
        "{} native class resources must be replicated",
        profile.name
    );
}

#[test]
fn follower_real_taunt_miss_keeps_the_attacked_ally_as_victim() {
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<crate::auras::AuraStatMods>(unit)
        .unwrap()
        .melee_hit_chance_pct = -100.0;
    h.app.world_mut().entity_mut(enemy).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Pursuing {
            target: h.caster.to_bits(),
        },
        crate::networking::CombatTarget(Some(h.caster)),
    ));
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, h.caster, 100.0);
    update_ai(h.app.world_mut());
    h.advance(4);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .unwrap()
            .0,
        Some(h.caster)
    );
    assert!(
        h.inbox.log.iter().any(|event| event.spell_id == Some(62124)
            && event.kind
                == shared::protocol::CombatLogKind::Miss(shared::protocol::MissKind::Miss)),
        "actual taunt miss log missing: {:?}",
        h.inbox.log
    );
}

macro_rules! profile_case {
    ($test:ident, $entry:literal) => {
        #[test]
        fn $test() {
            assert_profile_combat($entry);
        }
    };
}
profile_case!(follower_labelled_garrick_protection_paladin, 209057);
profile_case!(follower_labelled_breka_protection_warrior, 215517);
profile_case!(follower_labelled_crenna_restoration_druid, 209072);
profile_case!(follower_labelled_henry_holy_priest, 215011);
profile_case!(follower_labelled_meredy_fire_mage, 209059);
profile_case!(follower_labelled_shuja_elemental_shaman, 209053);
profile_case!(follower_labelled_austin_beast_mastery_hunter, 209065);
profile_case!(follower_labelled_kybel_devastation_evoker, 215240);
profile_case!(follower_labelled_bo_windwalker_monk, 227734);
profile_case!(follower_labelled_lantha_demonology_warlock, 230861);
profile_case!(follower_labelled_ralia_feral_druid, 232800);

#[test]
fn current_victim_garrick_primary_death_without_alternate_returns_to_owner() {
    use bevy::ecs::system::RunSystemOnce;
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    update_ai(h.app.world_mut());
    h.advance(3);
    let primary = h.npc;
    h.app
        .world_mut()
        .get_mut::<Health>(primary)
        .unwrap()
        .current = 0.0;
    crate::combat::mark_npc_dead_world(h.app.world_mut(), primary);
    update_ai(h.app.world_mut());
    assert_eq!(
        *h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(unit)
            .unwrap(),
        crate::creature_ai::CreatureState::Idle
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(unit)
            .unwrap()
            .owner,
        h.caster
    );
    let before = *h
        .app
        .world()
        .get::<shared::components::Position>(unit)
        .unwrap();
    h.app
        .world_mut()
        .run_system_once(crate::creature_ai::chase_system)
        .unwrap();
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Position>(unit)
            .unwrap(),
        before
    );
}

#[test]
fn current_victim_garrick_primary_death_preserves_live_alternate_party_defence() {
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    update_ai(h.app.world_mut());
    h.advance(3);
    let alternate = spawn_npc(&mut h.app, 5.0);
    h.app.world_mut().entity_mut(alternate).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
    ));
    let owner = h.caster;
    crate::creature_aggro::add_threat(h.app.world_mut(), alternate, owner, 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(alternate)
            .unwrap()
            .0,
        Some(owner),
        "fixture must establish real party engagement before primary death"
    );
    let primary = h.npc;
    h.app
        .world_mut()
        .get_mut::<Health>(primary)
        .unwrap()
        .current = 0.0;
    crate::combat::mark_npc_dead_world(h.app.world_mut(), primary);
    update_ai(h.app.world_mut());
    assert_eq!(
        *h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(unit)
            .unwrap(),
        crate::creature_ai::CreatureState::Pursuing {
            target: alternate.to_bits()
        }
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(unit)
            .unwrap()
            .0,
        Some(alternate)
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(unit)
            .is_none()
    );
    let before = h.health(alternate);
    for _ in 0..45 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.health(alternate) < before,
        "real sourced Garrick spells still defend the party after the primary dies"
    );
}

struct ChaseTraceWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for ChaseTraceWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn diagnostic_tank_chase_trace_records_victim_and_pre_stop_cached_route_at_same_tick() {
    use crate::navigation::{Goal, MoveRequest, Step};
    use bevy::ecs::system::RunSystemOnce;
    use shared::components::{ModelDisplay, Position};
    const CHILD: &str = "TANK_CHASE_TRACE_CAPTURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "diagnostic_tank_chase_trace_records_victim_and_pre_stop_cached_route_at_same_tick",
                "--ignored",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (mut h, tank, _) = spawn_profile_fixture_with_plugins(209057, |_| {});
    let victim = h.npc;
    let data = crate::ground::ground_data_dir();
    h.app
        .insert_resource(crate::navigation::Navigation::new(&data.join("navmesh")));
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&data.join("los")).unwrap());
    let from = Position {
        x: 121.1,
        y: -25.6,
        z: -3.9,
    };
    let cached = Position {
        x: 123.6,
        y: -25.61,
        z: -4.14,
    };
    let map = crate::maps::InMap::continent(34);
    h.app.world_mut().entity_mut(tank).insert((
        from,
        map,
        ModelDisplay { display_id: 113753 },
        crate::creature_ai::SpawnPoint {
            x: from.x,
            y: from.y,
            z: from.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
        },
        crate::creature_ai::CreatureState::Pursuing {
            target: victim.to_bits(),
        },
        crate::networking::CombatTarget(Some(victim)),
    ));
    h.app
        .world_mut()
        .entity_mut(victim)
        .insert((cached, map, ModelDisplay { display_id: 35076 }));
    let request = MoveRequest {
        map_id: 34,
        goal: Goal::Chase(victim.to_bits()),
        from,
        to: cached,
        distance: 0.0,
        reach: 2.0,
    };
    let seed = {
        let mut navigation = h
            .app
            .world_mut()
            .resource_mut::<crate::navigation::Navigation>();
        let _ = navigation.advance(tank, request);
        navigation.drain_until_idle();
        navigation.advance(tank, request)
    };
    assert!(matches!(seed, Step::Moved { .. }));
    *h.app.world_mut().get_mut::<Position>(victim).unwrap() = Position {
        x: 122.66,
        y: -25.61,
        z: -3.91,
    };
    h.app
        .world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(50));
    let tick = h.app.world().resource::<Time>().elapsed_secs_f64();
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = bytes.clone();
    let subscriber = bevy::log::tracing_subscriber::fmt()
        .with_env_filter("game_server::chase_trace=debug")
        .without_time()
        .with_ansi(false)
        .with_writer(move || ChaseTraceWriter(writer.clone()))
        .finish();
    bevy::log::tracing::subscriber::with_default(subscriber, || {
        h.app
            .world_mut()
            .run_system_once(crate::creature_ai::chase_system)
            .unwrap();
    });
    assert_eq!(
        *h.app.world().get::<Position>(tank).unwrap(),
        from,
        "diagnostic trace must not change the current-victim stop"
    );
    let text = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("tank chase frame"),
        "missing tank trace: {text}"
    );
    assert!(
        text.contains(&format!("actor={}", tank.to_bits())),
        "{text}"
    );
    assert!(
        text.contains(&format!("victim=Some({})", victim.to_bits())),
        "{text}"
    );
    assert!(text.contains(&format!("simulation_secs={tick}")), "{text}");
    assert!(
        text.contains("tank cached chase route")
            && text.contains("destination:")
            && text.contains("points:")
            && text.contains("next:")
            && text.contains("terminal_leg:"),
        "missing real pre-stop cache: {text}"
    );
}

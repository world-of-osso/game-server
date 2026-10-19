//! Authored demon summon from an NPC companion, with no character/stable writes.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::spell_cast::test_harness::{Harness, PreparedClassWorld};
use bevy::prelude::*;
use redb::{ReadableDatabase, ReadableTableMetadata};

struct PetFixture {
    h: Harness,
    unit: Entity,
    _source: PreparedClassWorld,
    _database: tempfile::TempDir,
}

fn fixture() -> PetFixture {
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let mut data = prepared.world_data();
    // Keep native defense metadata for the harness's live Kobold target too.
    data.templates.retain(|entry, _| [6, 17252].contains(entry));
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let spells = crate::pet_control::load_pet_levelup_spells(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("npc-pets.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(230861, |app| {
        app.insert_resource(data);
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.add_plugins((
            crate::hunter_pet::HunterPetPlugin,
            crate::pet_control::PetControlPlugin,
            crate::creature_ai::CreatureAiPlugin,
        ));
    });
    // Generic harness has already run Startup; its relative database path is absent
    // in cargo's package cwd. Install authenticated metadata before any pet summon.
    h.app.insert_resource(spells);
    PetFixture {
        h,
        unit,
        _source: prepared,
        _database: directory,
    }
}

fn count_persisted_rows(world: &World) -> u64 {
    let db = world.resource::<crate::persistence::GameDatabase>();
    let read = db.db.begin_read().unwrap();
    read.list_tables()
        .unwrap()
        .map(|table| read.open_untyped_table(table).unwrap().len().unwrap())
        .sum()
}

#[test]
fn follower_lantha_summons_authored_felguard_without_persisting_a_character() {
    let mut f = fixture();
    let before = count_persisted_rows(f.h.app.world());
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .expect("authored companion demon summon");
    f.h.advance(160);
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit)
        .expect("Felguard summoned by Lantha");
    assert_eq!(
        f.h.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252
    );
    assert_eq!(
        f.h.app
            .world()
            .get::<shared::components::UnitSummonedBy>(pet)
            .unwrap()
            .0,
        f.unit.to_bits()
    );
    assert_eq!(
        count_persisted_rows(f.h.app.world()),
        before,
        "runtime NPC pet must not write a persistent character or stable"
    );
}

fn felguard_at_level(level: u8) -> (PetFixture, Entity) {
    let mut f = fixture();
    f.h.app.world_mut().entity_mut(f.h.caster).insert((
        shared::components::Level(level),
        shared::components::UnitLevel(level),
    ));
    let profile =
        f.h.app
            .world()
            .get::<super::Follower>(f.unit)
            .unwrap()
            .profile
            .clone();
    super::spawning::initialize(f.h.app.world_mut(), f.h.caster, f.unit, profile).unwrap();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .unwrap();
    f.h.advance(160);
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).unwrap();
    crate::pet_control::order_companion_pet(f.h.app.world_mut(), f.unit, None);
    f.h.app
        .world_mut()
        .get_mut::<crate::pet_control::PetCharm>(pet)
        .unwrap()
        .react = shared::protocol::REACT_PASSIVE;
    // These kick/readiness fixtures model an already engaged owner, not selection.
    f.h.attack(f.h.npc);
    f.h.advance(3);
    assert!(
        f.h.inbox.attack_starts.iter().any(|event| {
            event.attacker == f.h.caster.to_bits() && event.victim == f.h.npc.to_bits()
        }),
        "Felguard combat fixture requires a real owner AttackStart"
    );
    (f, pet)
}

#[test]
fn follower_lantha_felguard_interrupts_with_learned_axe_toss_without_canceling_owner_cast() {
    let (mut f, pet) = felguard_at_level(17); // SpellLevels89766: learned at17, CreatureFamily29/SkillLine761.
    let before = count_persisted_rows(f.h.app.world());
    let charm =
        f.h.app
            .world()
            .get::<crate::pet_control::PetCharm>(pet)
            .unwrap();
    assert!(
        charm
            .action_bar
            .iter()
            .any(|button| shared::protocol::pet_action_button_action(*button) == 89766),
        "real level17 bar must contain sourced Axe Toss"
    );
    super::utility_tests::start_enemy_fireball(&mut f.h, pet);
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 686, Some(f.h.npc), false)
        .unwrap();
    super::update_ai(f.h.app.world_mut());
    // SpellMisc89766 Speed30: native minimum5yd flight is166ms, then UDP delivery.
    f.h.advance(8);
    assert!(
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.h.npc)
            .is_none(),
        "actual enemy Fireball must be interrupted; pet charm {:?}; pet cast {:?}; pet powers {:?}; failures {:?}; go {:?}; log {:?}",
        f.h.app.world().get::<crate::pet_control::PetCharm>(pet),
        f.h.app.world().get::<shared::casting::CastState>(pet),
        f.h.app.world().get::<shared::components::UnitPowers>(pet),
        f.h.inbox.failed,
        f.h.inbox.go,
        f.h.inbox.log,
    );
    assert_eq!(
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.unit)
            .unwrap()
            .spell_id,
        686,
        "pet kick must not cancel companion's own Shadow Bolt"
    );
    assert!(
        f.h.inbox.log.iter().any(
            |event| event.kind == shared::protocol::CombatLogKind::Interrupt
                && event.source == Some(pet.to_bits())
                && event.target == Some(f.h.npc.to_bits())
                && event.spell_id == Some(347008)
                && event.extra_spell_id == Some(133)
        ),
        "authored Axe Toss child347008 interrupt missing: {:?}",
        f.h.inbox.log
    );
    assert_eq!(
        count_persisted_rows(f.h.app.world()),
        before,
        "NPC pet kick must preserve existing fixture persistence without adding NPC rows"
    );
}

#[test]
fn follower_lantha_felguard_does_not_invent_axe_toss_below_its_authored_level() {
    let (mut f, pet) = felguard_at_level(16);
    super::utility_tests::start_enemy_fireball(&mut f.h, pet);
    super::update_ai(f.h.app.world_mut());
    f.h.advance(3);
    assert!(
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.h.npc)
            .is_some()
    );
    assert!(
        !f.h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == pet.to_bits() && cast.spell_id == 89766)
    );
}

#[test]
fn follower_lantha_felguard_cooldown_preserves_damage_priority() {
    let (mut f, pet) = felguard_at_level(17);
    super::utility_tests::start_enemy_fireball(&mut f.h, pet);
    super::update_ai(f.h.app.world_mut());
    f.h.advance(160);
    super::utility_tests::start_enemy_fireball(&mut f.h, pet);
    super::update_ai(f.h.app.world_mut());
    f.h.advance(8);
    assert!(
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.h.npc)
            .is_some()
    );
    assert_eq!(
        f.h.inbox
            .go
            .iter()
            .filter(|cast| cast.caster == pet.to_bits() && cast.spell_id == 89766)
            .count(),
        1
    );
    assert_lantha_damage_priority(&f);
}

#[test]
fn follower_lantha_felguard_does_not_waste_axe_toss_on_non_interruptible_fireball() {
    let (mut f, pet) = felguard_at_level(17);
    super::utility_tests::start_enemy_fireball(&mut f.h, pet);
    f.h.app
        .world_mut()
        .get_mut::<shared::casting::CastState>(f.h.npc)
        .unwrap()
        .interruptible = false;
    super::update_ai(f.h.app.world_mut());
    f.h.advance(8);
    assert!(
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.h.npc)
            .is_some()
    );
    assert!(
        !f.h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == pet.to_bits() && cast.spell_id == 89766)
    );
}

#[test]
fn follower_lantha_felguard_stun_blocks_axe_toss_without_stopping_owner_damage() {
    let (mut f, pet) = felguard_at_level(17);
    let stunner = crate::spell_cast::test_harness::spawn_npc(&mut f.h.app, 0.0);
    super::utility_tests::prepare_enemy_spec(&mut f.h, stunner, 66);
    crate::spell_cast::creature_cast(f.h.app.world_mut(), stunner, 853, Some(pet), false).unwrap();
    assert!(
        f.h.app
            .world()
            .get::<crate::auras::ActiveAuras>(pet)
            .unwrap()
            .is_stunned()
    );
    super::utility_tests::start_enemy_fireball(&mut f.h, pet);
    super::update_ai(f.h.app.world_mut());
    f.h.advance(8);
    assert!(
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.h.npc)
            .is_some()
    );
    assert!(
        !f.h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == pet.to_bits() && cast.spell_id == 89766)
    );
    assert_lantha_damage_priority(&f);
}

fn assert_lantha_damage_priority(f: &PetFixture) {
    let profile =
        &f.h.app
            .world()
            .get::<super::Follower>(f.unit)
            .unwrap()
            .profile;
    let spells = [profile.attack_spell, profile.cooldown_spell];
    let casting =
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.unit)
            .is_some_and(|cast| spells.contains(&cast.spell_id));
    let launched =
        f.h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == f.unit.to_bits() && spells.contains(&cast.spell_id));
    assert!(
        casting || launched,
        "failed pet readiness must leave the actual sourced damage priority available"
    );
}

fn stockade_pet() -> (PetFixture, Entity, std::path::PathBuf) {
    let mut f = fixture();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .unwrap();
    f.h.advance(160);
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let engine =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    f.h.app.insert_resource(crate::ground::GroundMap::new(
        &engine,
        std::collections::HashMap::from([(34, maps.map(34).unwrap().wdt_file_data_id)]),
    ));
    let nav =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh");
    f.h.app
        .insert_resource(crate::navigation::Navigation::new(&nav));
    let owner = shared::components::Position {
        x: 103.0,
        y: -34.85623,
        z: -76.0,
    };
    let from = shared::components::Position {
        x: 150.557,
        y: -34.85624,
        z: -45.8164,
    };
    for entity in [f.unit, f.h.caster] {
        f.h.app
            .world_mut()
            .entity_mut(entity)
            .insert((owner, crate::maps::InMap::continent(34)));
    }
    f.h.app
        .world_mut()
        .entity_mut(pet)
        .insert((from, crate::maps::InMap::continent(34)));
    (f, pet, engine)
}

fn enable_companion_ai_ticks(f: &mut PetFixture) {
    // Same production ordering as FollowerPlugin, without unrelated admission startup.
    f.h.app.add_systems(
        FixedUpdate,
        super::update_ai
            .after(crate::creature_aggro::update_victim_system)
            .before(crate::creature_ai::chase_system),
    );
}

fn pet_in_hostile_field() -> (PetFixture, Entity, std::path::PathBuf) {
    let (mut f, pet, engine) = stockade_pet();
    f.h.app
        .world_mut()
        .entity_mut(pet)
        .insert(super::hazard_tests::FROM);
    super::hazard_tests::hostile_field_at(&mut f.h, pet, super::hazard_tests::FROM);
    enable_companion_ai_ticks(&mut f);
    (f, pet, engine)
}

#[test]
fn follower_felguard_field_escape_never_uses_unbaked_direct_movement() {
    let (mut f, pet, _) = pet_in_hostile_field();
    let missing = tempfile::tempdir().unwrap();
    f.h.app
        .insert_resource(crate::navigation::Navigation::new(missing.path()));
    f.h.advance(60);
    assert_eq!(
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap(),
        super::hazard_tests::FROM
    );
}

#[test]
fn follower_felguard_leaves_overlapping_hostile_fields_on_real_stockade_routes() {
    let (mut f, pet, engine) = pet_in_hostile_field();
    let first = super::hazard_tests::FROM;
    let second = shared::components::Position {
        x: first.x + 4.0,
        ..first
    };
    super::hazard_tests::hostile_field_at(&mut f.h, pet, second);
    let mut los = los::LosMap::new(&engine.join("los/34"));
    let mut previous = first;
    let mut escaped = false;
    for tick in 0..200 {
        f.h.advance(1);
        let at =
            *f.h.app
                .world()
                .get::<shared::components::Position>(pet)
                .unwrap();
        let a = Vec3::new(previous.x, previous.y, previous.z);
        let b = Vec3::new(at.x, at.y, at.z);
        assert!(
            los.clear(a + Vec3::Y, b + Vec3::Y).unwrap(),
            "pet overlapping-field route crossed collision tick{tick}"
        );
        let crate::ground::GroundProbe::Supported(floor) =
            f.h.app
                .world_mut()
                .resource_mut::<crate::ground::GroundMap>()
                .ground_at(34, b)
        else {
            panic!("pet escape lost actual ground")
        };
        assert!((at.y - floor).abs() < 0.05);
        if (at.x - first.x).hypot(at.z - first.z) > 8.0
            && (at.x - second.x).hypot(at.z - second.z) > 8.0
        {
            assert_eq!(
                crate::spell_cast::scripts::hostile_ground_fields(f.h.app.world_mut(), pet).len(),
                2,
                "escape must occur before either field expires"
            );
            escaped = true;
            break;
        }
        previous = at;
    }
    assert!(
        escaped,
        "pet must escape the union of two actual Consecrations"
    );
    for _ in 0..40 {
        f.h.advance(1);
        let at =
            *f.h.app
                .world()
                .get::<shared::components::Position>(pet)
                .unwrap();
        assert!(
            (at.x - first.x).hypot(at.z - first.z) > 8.0
                && (at.x - second.x).hypot(at.z - second.z) > 8.0,
            "pet attack/follow must not re-enter either live field"
        );
    }
}

#[test]
fn follower_felguard_stun_prevents_displacement_while_field_escape_is_requested() {
    let (mut f, pet, _) = pet_in_hostile_field();
    let stunner = crate::spell_cast::test_harness::spawn_npc(&mut f.h.app, 0.0);
    super::utility_tests::prepare_enemy_spec(&mut f.h, stunner, 66);
    f.h.app
        .world_mut()
        .entity_mut(stunner)
        .insert((super::hazard_tests::FROM, crate::maps::InMap::continent(34)));
    crate::spell_cast::creature_cast(f.h.app.world_mut(), stunner, 853, Some(pet), false).unwrap();
    for _ in 0..20 {
        f.h.advance(1);
        assert!(
            f.h.app
                .world()
                .get::<crate::auras::ActiveAuras>(pet)
                .unwrap()
                .is_stunned()
        );
        assert_eq!(
            *f.h.app
                .world()
                .get::<shared::components::Position>(pet)
                .unwrap(),
            super::hazard_tests::FROM,
            "actual Hammer of Justice must block hazard displacement"
        );
    }
}

fn assert_stockade_pet_route(chase: bool) {
    let (mut f, pet, engine) = stockade_pet();
    if chase {
        let target =
            *f.h.app
                .world()
                .get::<shared::components::Position>(f.unit)
                .unwrap();
        f.h.app
            .world_mut()
            .entity_mut(f.h.npc)
            .insert((target, crate::maps::InMap::continent(34)));
        crate::pet_control::order_companion_pet(f.h.app.world_mut(), f.unit, Some(f.h.npc));
    }
    let mut los = los::LosMap::new(&engine.join("los/34"));
    let mut previous =
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap();
    for tick in 0..500 {
        f.h.advance(1);
        let at =
            *f.h.app
                .world()
                .get::<shared::components::Position>(pet)
                .unwrap();
        let a = Vec3::new(previous.x, previous.y, previous.z);
        let b = Vec3::new(at.x, at.y, at.z);
        assert!(
            los.clear(a + Vec3::Y, b + Vec3::Y).unwrap(),
            "pet route crossed real collision tick{tick}"
        );
        let crate::ground::GroundProbe::Supported(floor) =
            f.h.app
                .world_mut()
                .resource_mut::<crate::ground::GroundMap>()
                .ground_at(34, b)
        else {
            panic!("pet lost ground support")
        };
        assert!((at.y - floor).abs() < 0.05);
        previous = at;
    }
    let owner =
        *f.h.app
            .world()
            .get::<shared::components::Position>(f.unit)
            .unwrap();
    let gap = (previous.x - owner.x).hypot(previous.z - owner.z);
    let expected = if chase {
        crate::creature_ai::CHASE_REACH
    } else {
        4.0
    };
    assert!(
        (gap - expected).abs() < 0.2,
        "pet did not complete real route: gap{gap} expected{expected}"
    );
}

#[test]
#[ignore = "RED: live controlled-pet route must survive fixed pruning until actual despawn"]
fn follower_live_pet_route_survives_fixed_pruning_and_cancels_after_despawn() {
    use crate::navigation::{Goal, MoveRequest, Navigation, Step};
    let (mut f, pet, _) = stockade_pet();
    let from =
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap();
    let owner =
        *f.h.app
            .world()
            .get::<shared::components::Position>(f.unit)
            .unwrap();
    let request = MoveRequest {
        map_id: 34,
        goal: Goal::Follow(f.unit.to_bits()),
        from,
        to: owner,
        distance: 0.25,
        reach: 4.0,
    };
    assert!(matches!(
        f.h.app
            .world_mut()
            .resource_mut::<Navigation>()
            .advance(pet, request),
        Step::Pending
    ));
    crate::navigation::drain_fixture(&mut f.h.app);
    f.h.advance(1);
    let after =
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap();
    assert_ne!(
        after, from,
        "a live controlled pet must retain and consume its published route across fixed pruning"
    );
    assert!(f.h.app.world_mut().despawn(pet));
    f.h.advance(1);
    assert!(
        matches!(
            f.h.app
                .world_mut()
                .resource_mut::<Navigation>()
                .continue_route(pet, 34, after, 0.25),
            Step::Pending
        ),
        "actual despawn must cancel the previously live pet route"
    );
}

#[test]
fn follower_felguard_follow_finishes_real_stockade_route_without_crossing_collision() {
    assert_stockade_pet_route(false);
}

#[test]
fn follower_felguard_chase_finishes_real_stockade_route_without_crossing_collision() {
    assert_stockade_pet_route(true);
}

#[test]
fn follower_felguard_unbaked_follow_does_not_use_direct_movement() {
    let mut f = fixture();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .unwrap();
    f.h.advance(160);
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).unwrap();
    let before =
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap();
    f.h.app
        .world_mut()
        .get_mut::<shared::components::Position>(f.unit)
        .unwrap()
        .x = 15.0;
    f.h.advance(40);
    assert_eq!(
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap(),
        before
    );
}

#[test]
fn follower_felguard_unbaked_chase_does_not_use_direct_movement() {
    let mut f = fixture();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .unwrap();
    f.h.advance(160);
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).unwrap();
    let before =
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap();
    f.h.app
        .world_mut()
        .get_mut::<shared::components::Position>(f.h.npc)
        .unwrap()
        .x = 20.0;
    crate::pet_control::order_companion_pet(f.h.app.world_mut(), f.unit, Some(f.h.npc));
    f.h.advance(40);
    assert_eq!(
        *f.h.app
            .world()
            .get::<shared::components::Position>(pet)
            .unwrap(),
        before
    );
}

#[test]
fn follower_felguard_melee_kill_taps_the_authenticated_player() {
    let mut f = fixture();
    f.h.app
        .world_mut()
        .entity_mut(f.h.caster)
        .insert(crate::persistence::ClientId(30));
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .unwrap();
    f.h.advance(160);
    f.h.app
        .world_mut()
        .entity_mut(f.h.npc)
        .insert(shared::components::Health {
            current: 5.0,
            max: 5.0,
        });
    crate::pet_control::order_companion_pet(f.h.app.world_mut(), f.unit, Some(f.h.npc));
    f.h.advance(120);
    assert_eq!(f.h.health(f.h.npc), 0.0);
    let tap =
        f.h.app
            .world()
            .get::<crate::creature_tap::CreatureTap>(f.h.npc)
            .expect("pet kill tapped actual character");
    assert_eq!(tap.tappers, vec![30]);
    assert_eq!(tap.xp_factor(5.0), 1.0);
}

#[test]
fn follower_felguard_fights_the_companions_engaged_enemy() {
    let mut f = fixture();
    f.h.attack(f.h.npc);
    f.h.advance(3);
    for _ in 0..240 {
        super::update_ai(f.h.app.world_mut());
        f.h.advance(1);
    }
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).unwrap();
    assert!(
        f.h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.target == Some(f.h.npc.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage),
        "actual Felguard damage missing: pos {:?}, target {:?}, state {:?}, charm {:?}, hp {:?}, attack {:?}, flags {:?}, enemy hp {:?}; combat {:?}",
        f.h.app.world().get::<shared::components::Position>(pet),
        f.h.app.world().get::<crate::networking::CombatTarget>(pet),
        f.h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(pet),
        f.h.app.world().get::<crate::pet_control::PetCharm>(pet),
        f.h.app.world().get::<shared::components::Health>(pet),
        f.h.app.world().get::<crate::combat::AutoAttack>(pet),
        f.h.app.world().get::<shared::components::UnitFlags>(pet),
        f.h.app.world().get::<shared::components::Health>(f.h.npc),
        f.h.inbox.log
    );
}

#[test]
fn follower_felguard_leaves_when_runtime_companion_owner_is_removed() {
    let mut f = fixture();
    let before = count_persisted_rows(f.h.app.world());
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.unit, 30146, Some(f.unit), false)
        .unwrap();
    f.h.advance(160);
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).unwrap();
    f.h.app.world_mut().despawn(f.unit);
    f.h.advance(3);
    assert!(!f.h.app.world().entities().contains(pet));
    assert_eq!(count_persisted_rows(f.h.app.world()), before);
}

#[test]
fn follower_lantha_ai_prepares_authored_felguard_before_damage() {
    let mut f = fixture();
    for _ in 0..180 {
        super::update_ai(f.h.app.world_mut());
        f.h.advance(1);
        if crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit).is_some() {
            break;
        }
    }
    let pet = crate::hunter_pet::summoned_pet(f.h.app.world(), f.unit)
        .expect("companion AI prepared Felguard");
    assert_eq!(
        f.h.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252
    );
    assert!(
        !f.h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(f.unit.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage),
        "summon preparation precedes damage"
    );
}

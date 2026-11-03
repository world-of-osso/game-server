//! FOLLOWER_HUNTER_PET_SIMC_DEFAULT: real native125 Waterfowl, ordinary Call Pet.
use super::kit_tests::at_level;
use crate::auras::ActiveAuras;
use crate::hunter_pet::{HunterPet, HunterPetPlugin, PetStable, summoned_pet};
use shared::components::{Health, Npc, UnitLevel};

const AUSTIN: u32 = 209065;
const NATIVE_DUCK: u32 = 174884;
const WATERFOWL: u32 = 125;

pub(super) fn prepare_native_duck(
    level: u8,
) -> (
    crate::spell_cast::test_harness::Harness,
    bevy::prelude::Entity,
    bevy::prelude::Entity,
) {
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let data = prepared.world_data();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("native-follower-duck.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let (mut h, caster) = at_level(AUSTIN, level, |app| {
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.insert_resource(data);
        app.add_plugins((
            HunterPetPlugin,
            crate::pet_control::PetControlPlugin,
            crate::creature_ai::CreatureAiPlugin,
        ));
    });
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::pet_control::load_pet_levelup_spells(&source).unwrap());
    assert!(
        h.app
            .world()
            .get::<shared::components::Player>(caster)
            .is_none()
    );
    crate::hunter_pet::install_follower_pet(h.app.world_mut(), caster, Some(NATIVE_DUCK)).unwrap();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 883, Some(caster), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if summoned_pet(h.app.world(), caster).is_some() {
            break;
        }
    }
    let pet = summoned_pet(h.app.world(), caster)
        .expect("ordinary Call Pet must summon the real native pet");
    assert_eq!(h.app.world().get::<HunterPet>(pet).unwrap().owner, caster);
    assert_eq!(
        h.app.world().get::<Npc>(pet).unwrap().template_id,
        NATIVE_DUCK
    );
    assert_eq!(h.app.world().get::<UnitLevel>(pet).unwrap().0, level);
    let stable = h.app.world().get::<PetStable>(caster).unwrap();
    assert_eq!(
        stable.character_id, None,
        "runtime pet must never invent persistent player identity"
    );
    let stored = stable.stored.active[0].as_ref().unwrap();
    assert_eq!(stored.family, WATERFOWL);
    assert!(
        stored.specialization.is_some(),
        "use the native family specialization, not bespoke follower pet stats"
    );
    assert!(
        h.inbox
            .go
            .iter()
            .any(|event| event.caster == caster.to_bits() && event.spell_id == 883),
        "pet acquisition must use ordinary learned Call Pet"
    );
    (h, caster, pet)
}

#[test]
fn hunter_pet_chase_respects_idle_hostile_sight() {
    use crate::followers::positioning::{CombatMovementExclusions, SightExclusion};
    use crate::maps::InMap;
    use shared::components::Position;
    let (mut h, hunter, pet) = prepare_native_duck(70);
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    h.app.insert_resource(crate::ground::GroundMap::new(
        &root,
        std::collections::HashMap::from([(34, maps.map(34).unwrap().wdt_file_data_id)]),
    ));
    let nav =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh");
    h.app
        .insert_resource(crate::navigation::Navigation::new(&nav));
    let map = InMap::continent(34);
    let target = Position {
        x: 103.0,
        y: -34.85623,
        z: -76.0,
    };
    let start = Position {
        x: 150.557,
        y: -34.85624,
        z: -45.8164,
    };
    for unit in [hunter, h.caster, h.npc] {
        h.app.world_mut().entity_mut(unit).insert((map, target));
    }
    h.app.world_mut().entity_mut(pet).insert((
        map,
        start,
        CombatMovementExclusions(Some(vec![SightExclusion {
            center: bevy::math::Vec3::new(target.x, target.y, target.z),
            radius: 12.0,
            sight: crate::line_of_sight::ProjectedSight::Open,
        }])),
    ));
    crate::pet_control::order_companion_pet(h.app.world_mut(), hunter, Some(h.npc));
    for _ in 0..500 {
        h.advance(1);
    }
    let held = *h.app.world().get::<Position>(pet).unwrap();
    assert!(
        (held.x - target.x).hypot(held.z - target.z) >= 12.0,
        "pet crossed idle hostile sight"
    );
    assert!(
        (held.x - start.x).hypot(held.z - start.z) > 1.0,
        "pet never attempted its chase"
    );
    h.app
        .world_mut()
        .entity_mut(pet)
        .remove::<CombatMovementExclusions>();
    for _ in 0..500 {
        h.advance(1);
    }
    let reached = *h.app.world().get::<Position>(pet).unwrap();
    assert!(
        (reached.x - target.x).hypot(reached.z - target.z) < 4.0,
        "pet failed to resume after sight exclusion was removed"
    );
}

#[test]
fn ruby_taxi_temporarily_unsummons_and_restores_follower_pet() {
    use shared::components::{MovementControl, Position, Rotation};
    let (mut h, follower, old_pet) = prepare_native_duck(70);
    let owner = h.caster;
    let map = crate::maps::InMap {
        map_id: 2521,
        instance_id: 77,
    };
    let start = Position {
        x: 1540.7935,
        y: 131.9674,
        z: 246.033,
    };
    for unit in [owner, follower, old_pet] {
        h.app.world_mut().entity_mut(unit).insert((
            map,
            start,
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ));
    }
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(MovementControl::default());
    let health = h.app.world().get::<Health>(old_pet).unwrap().max * 0.75;
    h.app
        .world_mut()
        .get_mut::<Health>(old_pet)
        .unwrap()
        .current = health;
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::taxi::load_taxi_network(&db).unwrap());
    h.app
        .add_systems(bevy::prelude::FixedUpdate, crate::taxi::fly_taxis);
    h.app
        .init_resource::<crate::scripted_ai::instance_script::InstanceScripts>();
    h.app
        .world_mut()
        .resource_mut::<crate::scripted_ai::instance_script::InstanceScripts>()
        .0
        .insert(
            map,
            crate::scripted_ai::instance_script::InstanceScript {
                name: "ruby",
                bosses: vec![crate::scripted_ai::instance_script::BossInfo {
                    state: crate::scripted_ai::instance_script::EncounterState::Done,
                    encounter_id: 2609,
                }],
                boss_creatures: vec![],
                handled_area_triggers: Default::default(),
                data: Default::default(),
                ramparts_sentry_deaths: 0,
                ramparts_reset_delays: vec![],
            },
        );
    let drake = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 194667,
                name: "Radiant Drake".into(),
            },
            map,
            start,
        ))
        .id();
    crate::taxi::start_ruby_ferry(h.app.world_mut(), owner, drake);
    assert!(
        h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(follower)
            .is_some()
    );
    assert!(summoned_pet(h.app.world(), follower).is_none());
    assert!(h.app.world().get_entity(old_pet).is_err());
    assert_eq!(
        h.app
            .world()
            .get::<PetStable>(follower)
            .unwrap()
            .stored
            .current,
        Some(0)
    );
    for _ in 0..1000 {
        if h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(follower)
            .is_none()
        {
            break;
        }
        super::update_ai(h.app.world_mut());
        h.advance(1);
        if h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(follower)
            .is_some()
        {
            assert!(
                summoned_pet(h.app.world(), follower).is_none(),
                "no pet during flight"
            );
        }
    }
    let pet = summoned_pet(h.app.world(), follower).expect("pet restored after landing");
    assert_ne!(pet, old_pet);
    assert_eq!(
        h.app.world().get::<Npc>(pet).unwrap().template_id,
        NATIVE_DUCK
    );
    assert_eq!(h.app.world().get::<Health>(pet).unwrap().current, health);
    assert_eq!(*h.app.world().get::<crate::maps::InMap>(pet).unwrap(), map);
}

#[test]
fn native_revive_pet_restores_the_dead_follower_pet() {
    let (mut h, caster, pet) = prepare_native_duck(70);
    // Call Pet starts the normal global cooldown; let it expire before Revive Pet.
    h.advance(35);
    let max_health = h.app.world().get::<Health>(pet).unwrap().max;
    let attacker = h.npc;
    crate::combat::deal_direct_damage(h.app.world_mut(), attacker, pet, max_health + 1.0);
    assert_eq!(h.app.world().get::<Health>(pet).unwrap().current, 0.0);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 982, None, false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if h.app.world().get::<Health>(pet).unwrap().current > 0.0 {
            break;
        }
    }
    assert!(
        h.inbox
            .go
            .iter()
            .any(|event| event.caster == caster.to_bits() && event.spell_id == 982),
        "ordinary Revive Pet cast completed"
    );
    assert_eq!(
        h.app.world().get::<Health>(pet).unwrap().current,
        max_health
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(pet)
            .is_none()
    );
}

fn check_native_duck(level: u8) {
    prepare_native_duck(level);
}

#[test]
fn native_level90_duck_is_summoned_via_call_pet() {
    check_native_duck(90);
}

#[test]
fn native_level32_duck_is_summoned_via_call_pet() {
    check_native_duck(32);
}

/// Frames spanning `intervals` consecutive swing clocks; misses also count as real
/// attack attempts. Each reset carries its overshoot (8d99a30ecf), so one interval
/// quantizes to either neighboring frame count while a span stays within one frame.
fn melee_swing_frames(
    h: &mut crate::spell_cast::test_harness::Harness,
    unit: bevy::prelude::Entity,
    intervals: usize,
) -> usize {
    let start = h
        .app
        .world()
        .resource::<crate::combat::CombatLog>()
        .range(0.0, f64::MAX)
        .len();
    let mut first = None;
    let mut observed = 0;
    for frame in 0usize..200 * intervals {
        h.advance(1);
        let log = h.app.world().resource::<crate::combat::CombatLog>();
        let count = log
            .range(0.0, f64::MAX)
            .iter()
            .skip(start)
            .filter(|entry| entry.event.attacker == unit.to_bits() && entry.event.spell_id == 0)
            .count();
        if count > observed {
            observed = count;
            match first {
                Some(first) if observed > intervals => return frame - first,
                Some(_) => {}
                None => first = Some(frame),
            }
        }
    }
    panic!("native unit{unit} did not produce {intervals} melee intervals");
}

#[test]
fn rotation_fidelity_253_native_frenzy_shortens_pet_melee_interval() {
    const FRENZY: u32 = 1_264_934;
    const OWNER_INTERVALS: usize = 10;
    let (mut h, caster, pet) = prepare_native_duck(90);
    assert!(crate::auras::has_aura(h.app.world(), caster, FRENZY));
    let world = h.app.world();
    let raw_amount = world
        .resource::<crate::spell_info::SpellStore>()
        .get(FRENZY)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.aura == 4)
        .unwrap()
        .base_points;
    let native_trait = world
        .get::<crate::class_progression::TraitLoadout>(caster)
        .unwrap()
        .spell_trait(FRENZY)
        .unwrap();
    assert_eq!(native_trait.definition_id, 131223);
    assert_eq!(native_trait.rank, 2);
    let amount = world
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FRENZY)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    // Native points24184 Multiply curve94381 maps ranks1/2 to factors1/2.
    // TC SpellInfo.cpp532-553 applies that curve before the passive is consumed.
    assert_eq!(amount, raw_amount * native_trait.rank as f32);
    println!(
        "NATIVE_FRENZY definition={} rank={} raw={raw_amount} effective={amount}",
        native_trait.definition_id, native_trait.rank
    );
    let target = h.npc;
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .max = 1_000_000_000.0;
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .current = 1_000_000_000.0;
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(target)
        .unwrap();
    h.app
        .world_mut()
        .entity_mut(pet)
        .insert((position, crate::combat::AutoAttack::new(target)));
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::combat::AutoAttack::new(target));
    let owner_with_frenzy = melee_swing_frames(&mut h, caster, OWNER_INTERVALS);
    let accelerated = melee_swing_frames(&mut h, pet, 1);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FRENZY,
    );
    let ordinary = melee_swing_frames(&mut h, pet, 1);
    let owner_without_frenzy = melee_swing_frames(&mut h, caster, OWNER_INTERVALS);
    assert!(
        owner_without_frenzy.abs_diff(owner_with_frenzy) <= 1,
        "owner Frenzy must modify only the pet's attack clock: {OWNER_INTERVALS} owner \
         intervals {owner_with_frenzy} frames with Frenzy, {owner_without_frenzy} without"
    );
    let expected = ordinary as f32 / (1.0 + amount / 100.0);
    assert!(
        (accelerated as f32 - expected).abs() <= 1.0,
        "Frenzy native{amount}% must shorten pet interval: {accelerated} vs ordinary{ordinary}, expected{expected}"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), pet, FRENZY),
        "owner passive must not become a synthetic pet buff"
    );
}

#[test]
fn unsummoning_hunter_follower_removes_its_pet_before_next_fixed_tick() {
    let (mut h, follower, pet) = prepare_native_duck(90);
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert(crate::zones::TempSummon { summoner: h.caster });
    crate::zones::unsummon(h.app.world_mut(), follower);
    assert!(h.app.world().get_entity(follower).is_err());
    assert!(
        h.app.world().get_entity(pet).is_err(),
        "pet must leave with its follower owner"
    );
    h.advance(2);
}

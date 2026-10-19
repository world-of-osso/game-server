//! Native Azure directional pose proof; interrupts remain legitimate, not disabled for the test.
use super::super::{
    Follower, FollowerCatalog, ai, catalog, kit, pull_history, pull_route, spawning,
};
use super::FollowerRun;
use crate::maps::{InMap, WorldLocation};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins, world_db};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{ActiveSpec, Health, Level, Position, UnitLevel};
use shared::group::GroupRole;
use std::collections::{BTreeSet, HashMap};

const MAP: u32 = 2515;
const LEYMOR_GUID: u32 = 29001810;
const PARTY_ORIGIN_GUID: u32 = 29001813;
const LEYMOR: u32 = 186644;
const TENDER: u32 = 191164;
const FISSURE: u32 = 386660;

struct Fixture {
    h: Harness,
    tank: Entity,
    boss: Entity,
    party: Vec<Entity>,
    clean_pose: Vec3,
}

fn initialize_native_owner(
    h: &mut Harness,
    catalog: &FollowerCatalog,
    level: u8,
    map: InMap,
    position: Position,
) {
    let spec = h
        .app
        .world()
        .resource::<crate::spell_info::ClassSpells>()
        .classes
        .iter()
        .find(|class| class.class_id == 8)
        .unwrap()
        .specs
        .iter()
        .find(|spec| spec.spec_id == 63)
        .unwrap();
    let mut known: Vec<_> = spec
        .spells
        .iter()
        .filter(|spell| spell.required_level <= u32::from(level))
        .map(|spell| spell.spell_id)
        .collect();
    let mut passives: Vec<_> = spec
        .spells
        .iter()
        .filter(|spell| spell.passive && spell.required_level <= u32::from(level))
        .map(|spell| spell.spell_id)
        .collect();
    for entry in catalog
        .kits
        .iter()
        .filter(|entry| entry.available(63, level))
    {
        known.push(entry.spell_id);
        if entry.slot == kit::Slot::Passive {
            passives.push(entry.spell_id);
        }
    }
    known.sort_unstable();
    known.dedup();
    passives.sort_unstable();
    passives.dedup();
    h.app.world_mut().entity_mut(h.caster).insert((
        Level(level),
        UnitLevel(level),
        ActiveSpec(63),
        map,
        position,
        crate::class_progression::KnownSpells::fixture(known),
    ));
    crate::auras::sync_passives(
        h.app.world_mut(),
        h.caster,
        crate::auras::AuraSource::KnownSpell,
        &passives,
    );
}

fn source(h: &mut Harness, map: InMap, guid: u32) -> Entity {
    h.app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == guid && **copy == map)
        .unwrap()
        .0
}

fn point(h: &Harness, actor: Entity) -> Vec3 {
    crate::spell_cast::targets::position(h.app.world(), actor).unwrap()
}

fn grounded(h: &mut Harness, map: InMap, at: Vec3) -> Position {
    let crate::ground::GroundProbe::Supported(y) = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, at)
    else {
        panic!("actual Azure pose lacks authenticated native floor support: {at:?}")
    };
    Position {
        x: at.x,
        y,
        z: at.z,
    }
}

fn spawn_native_population(h: &mut Harness, map: InMap) {
    let data = h
        .app
        .world()
        .resource::<crate::world_data::WorldData>()
        .clone();
    let mut ground = h
        .app
        .world_mut()
        .remove_resource::<crate::ground::GroundMap>()
        .unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let count = crate::zones::spawn_map_creatures(
        &mut commands,
        &data,
        crate::zones::MapContent {
            map,
            difficulty_id: catalog::FOLLOWER_DIFFICULTY,
            dead_bosses: vec![],
        },
        false,
        Some(&mut ground),
    );
    assert!(
        count > 3,
        "actual Azure205/Normal-source population prerequisite"
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
}

fn assert_native_bodies_and_stats(h: &mut Harness, actors: &[Entity], boss: Entity, level: u8) {
    for &actor in actors {
        assert_eq!(h.app.world().get::<Level>(actor).unwrap().0, level);
        let stats = h
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap();
        let health = h.app.world().get::<Health>(actor).unwrap();
        assert!(health.current > 0.0);
        assert_eq!(
            health.max, stats.max_health,
            "native class/full-kit stat refresh owns max HP"
        );
    }
    let mut state = bevy::ecs::system::SystemState::<(
        Query<crate::line_of_sight::BodyParts<'static>>,
        Res<crate::line_of_sight::LineOfSight>,
    )>::new(h.app.world_mut());
    let (bodies, los) = state.get(h.app.world()).unwrap();
    for actor in actors.iter().copied().chain([boss]) {
        assert!(
            los.body(bodies.get(actor).unwrap()).is_some(),
            "missing actual native eye/body geometry: {actor:?}"
        );
    }
}

fn finish_native_intro(h: &mut Harness, map: InMap, boss: Entity) {
    h.advance(3);
    assert!(
        crate::auras::has_aura(h.app.world(), boss, 375729),
        "actual Leymor stasis before native intro"
    );
    let tenders: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, npc, copy)| npc.template_id == TENDER && **copy == map)
        .map(|(actor, _, _)| actor)
        .collect();
    assert_eq!(
        tenders.len(),
        3,
        "actual authored tender group, not invented intro actors"
    );
    for tender in tenders {
        crate::combat::kill_unit(h.app.world_mut(), h.caster, tender);
    }
    h.advance(24);
    assert!(
        !crate::auras::has_aura(h.app.world(), boss, 375729),
        "actual tender depletion must finish intro"
    );
}

fn fixture() -> Fixture {
    let assets = crate::ground::ground_data_dir();
    assert!(
        assets.join("navmesh/2515").is_dir(),
        "Azure tank proof requires the receipt-qualified private2515 asset view via GAME_SERVER_GROUND_DIR"
    );
    println!(
        "AZURE_TANK_BAKE_RECEIPT {}",
        std::env::var("AZURE_TANK_BAKE_RECEIPT_SHA256")
            .expect("runner must record and verify the reused bake receipt hash")
    );
    let (_directory, prepared) =
        crate::scripted_ai::tests::data_bosses::prepare_encounter_database("leymor");
    let db = rusqlite::Connection::open_with_flags(
        &prepared,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let native = crate::world_data::load_world_data(&prepared);
    let level = native.templates[&LEYMOR].min_level;
    assert_eq!(
        level, 70,
        "derive fixture level from actual Leymor, never HP tuning"
    );
    let catalog = catalog::load_catalog(&db).unwrap();
    let dungeon = catalog
        .dungeons
        .values()
        .find(|dungeon| dungeon.entrance.map_id == MAP)
        .unwrap()
        .clone();
    let profiles = super::super::select_party(&catalog.profiles, GroupRole::Dps).unwrap();
    assert_eq!(
        profiles
            .iter()
            .find(|profile| profile.role == GroupRole::Tank)
            .unwrap()
            .entry,
        209057
    );
    let bossmod = crate::scripted_ai::bossmod::load_catalog(&db).unwrap();
    let mut h = harness_with_plugins(vec![], &[], 8, true, |app| {
        app.insert_resource(bossmod);
        app.add_plugins(crate::scripted_ai::ScriptedAiPlugin);
    })
    .unwrap();
    h.app.world_mut().despawn(h.npc);
    h.app
        .insert_resource(crate::scripted_ai::creature_text::load_creature_texts(&db).unwrap());
    h.app
        .insert_resource(crate::scripted_ai::smart_ai::load_smart_scripts(&prepared).unwrap());
    h.app
        .insert_resource(crate::spell_cast::area_trigger::load_aura_field_catalog(&db).unwrap());
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let store = crate::maps::store::load_map_store(&db).unwrap();
    let wdt = store.maps[&MAP].wdt_file_data_id;
    let mut instances = crate::maps::InstanceManager::default();
    let owner_id = h.caster.to_bits();
    let map = instances
        .create_map(
            &store,
            MAP,
            crate::maps::instances::InstanceEntrant {
                character_id: owner_id,
                group: None,
                leader: owner_id,
                difficulty_id: catalog::FOLLOWER_DIFFICULTY,
                raid_difficulty: Default::default(),
                raid_group: false,
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_790_510_400,
            },
        )
        .unwrap()
        .map;
    h.app.insert_resource(store);
    h.app.insert_resource(instances);
    h.app.insert_resource(crate::ground::GroundMap::new(
        &assets,
        HashMap::from([(MAP, wdt)]),
    ));
    h.app
        .insert_resource(crate::navigation::Navigation::new(&assets.join("navmesh")));
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&assets.join("los")).unwrap());
    let origin = native
        .spawns
        .iter()
        .find(|spawn| spawn.guid == PARTY_ORIGIN_GUID)
        .unwrap();
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(origin.position_x, origin.position_y, origin.position_z);
    let from = grounded(&mut h, map, Vec3::new(x, y, z));
    initialize_native_owner(&mut h, &catalog, level, map, from);
    h.app.insert_resource(native);
    h.app.insert_resource(catalog);
    spawn_native_population(&mut h, map);
    let members = spawning::spawn_members(h.app.world_mut(), h.caster, &profiles).unwrap();
    let tank = members
        .iter()
        .copied()
        .find(|&unit| h.app.world().get::<Follower>(unit).unwrap().profile.role == GroupRole::Tank)
        .unwrap();
    let boss = source(&mut h, map, LEYMOR_GUID);
    let mut party = members.clone();
    party.push(h.caster);
    let (wx, wy, wz) = crate::terrain_height::bevy_to_wow(from.x, from.y, from.z);
    h.app.world_mut().entity_mut(h.caster).insert(FollowerRun {
        dungeon,
        profiles,
        members,
        copy: Some(map),
        previous_difficulty: 1,
        origin: WorldLocation {
            map_id: MAP,
            position: Vec3::new(wx, wy, wz),
            orientation: 0.0,
        },
        player_role: GroupRole::Dps,
    });
    h.advance(3);
    assert_native_bodies_and_stats(&mut h, &party, boss, level);
    pull_history::sample_all(h.app.world_mut());
    finish_native_intro(&mut h, map, boss);
    let boss_point = point(&h, boss);
    let away = Vec3::new(boss_point.x - from.x, 0.0, boss_point.z - from.z).normalize();
    let clean = grounded(&mut h, map, boss_point + away * ai::MELEE_REACH);
    let clean_pose = Vec3::new(clean.x, clean.y, clean.z);
    let nav = h.app.world().resource::<crate::navigation::Navigation>();
    assert!(
        nav.query_path(MAP, point(&h, tank), clean_pose)
            .unwrap()
            .is_some_and(|path| !path.is_empty()),
        "actual opposite-party pose needs a complete native route"
    );
    Fixture {
        h,
        tank,
        boss,
        party,
        clean_pose,
    }
}

fn fissure_party_targets(f: &mut Fixture, destination: Vec3) -> Vec<Entity> {
    let mut spell =
        f.h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(FISSURE)
            .unwrap()
            .clone();
    spell.cast_destination = Some([destination.x, destination.y, destination.z]);
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.implicit_targets.contains(&104))
        .unwrap()
        .clone();
    let targets = crate::spell_cast::targets::select_targets(
        f.h.app.world_mut(),
        f.boss,
        Some(f.tank),
        &spell,
        &effect,
        0,
        true,
    )
    .unwrap();
    targets
        .into_iter()
        .filter(|target| *target != f.tank && f.party.contains(target))
        .collect()
}

fn engage_native_routes(f: &mut Fixture) {
    let from = *f.h.app.world().get::<Position>(f.tank).unwrap();
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    let mut footprint = BTreeSet::new();
    for origin in [Vec3::new(from.x, from.y, from.z), f.clean_pose] {
        f.h.app.world_mut().entity_mut(f.tank).insert(Position {
            x: origin.x,
            y: origin.y,
            z: origin.z,
        });
        let route = pull_route::read_route(
            f.h.app.world_mut(),
            &pull_route::Request {
                tank: f.tank,
                target_guid: LEYMOR_GUID,
                difficulty: catalog::FOLLOWER_DIFFICULTY,
                cleared: &history.cleared_spawns,
            },
        )
        .expect(
            "native Azure frontal fixture requires complete source-qualified routes on both sides",
        );
        footprint.extend(route.spawns);
    }
    f.h.app.world_mut().entity_mut(f.tank).insert(from);
    let map = *f.h.app.world().get::<InMap>(f.tank).unwrap();
    let spell =
        f.h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(133)
            .unwrap()
            .clone();
    for guid in footprint {
        let actor = source(&mut f.h, map, guid);
        let context = crate::spell_cast::effects::EffectCtx {
            caster: f.tank,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        crate::spell_cast::effects::deal_damage(
            f.h.app.world_mut(),
            &context,
            actor,
            1.0,
            false,
            false,
        );
    }
}

#[test]
fn production_tank_faces_native_leymor_fissure_away_from_the_party() {
    let mut f = fixture();
    let initial = point(&f.h, f.tank);
    let initial_exposure = fissure_party_targets(&mut f, initial);
    assert!(
        !initial_exposure.is_empty(),
        "initial real65-degree/40yd selector104 cone must actually expose party"
    );
    assert!(
        initial_exposure
            .iter()
            .any(|&unit| crate::line_of_sight::sees_in_world(f.h.app.world(), f.boss, unit)),
        "initial native cone must have an actual visible party recipient, not wall-protected geometry"
    );
    let clean = f.clean_pose;
    assert!(
        fissure_party_targets(&mut f, clean).is_empty(),
        "actual native opposite-party candidate must protect the party"
    );
    engage_native_routes(&mut f);
    f.h.select(Some(f.boss));
    crate::creature_ai::install_follower_follow_fixture(&mut f.h.app);
    for _ in 0..160 {
        f.h.advance(1);
        let from = point(&f.h, f.tank);
        let boss = point(&f.h, f.boss);
        if crate::navigation::within_reach(from, boss, ai::MELEE_REACH) {
            break;
        }
    }
    assert!(
        crate::navigation::within_reach(point(&f.h, f.tank), point(&f.h, f.boss), ai::MELEE_REACH),
        "actual native tank approach must reach the living boss before frontal proof"
    );
    assert_eq!(
        (crate::scripted_ai::Ctx {
            world: f.h.app.world_mut(),
            me: f.boss
        })
        .victim(),
        Some(f.tank),
        "real source Fissure must target its actual native current victim"
    );
    let mut started = false;
    for _ in 0..100 {
        match crate::spell_cast::creature_cast(
            f.h.app.world_mut(),
            f.boss,
            FISSURE,
            Some(f.tank),
            false,
        ) {
            Ok(()) => {
                started = true;
                break;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => f.h.advance(1),
            error => panic!("real source Fissure prerequisite refused: {error:?}"),
        }
    }
    assert!(
        started,
        "native cast/GCD must eventually permit the real source Fissure"
    );
    assert_eq!(
        f.h.app.world().get::<CastState>(f.boss).unwrap().spell_id,
        FISSURE
    );
    // Native kicks may interrupt this cast; no cooldown, kit or interruptibility override.
    for _ in 0..80 {
        f.h.advance(1);
    }
    assert!(
        crate::spell_cast::targets::is_alive(f.h.app.world(), f.boss),
        "native boss must remain alive for pose proof"
    );
    assert!(
        f.h.inbox.log.iter().any(
            |event| event.kind == shared::protocol::CombatLogKind::Interrupt
                && event.target == Some(f.boss.to_bits())
                && event.extra_spell_id == Some(FISSURE)
                && event
                    .source
                    .is_some_and(|source| f.party.iter().any(|unit| unit.to_bits() == source))
        ),
        "retained frontal knowledge must be observed after an actual native party kick, not a disabled interrupt or naturally completed cast"
    );
    assert!(
        f.party
            .iter()
            .all(|&unit| crate::spell_cast::targets::is_alive(f.h.app.world(), unit)),
        "dead party members cannot be mistaken for protected frontal targets"
    );
    let at = point(&f.h, f.tank);
    let exposed = fissure_party_targets(&mut f, at);
    assert!(
        exposed.is_empty(),
        "production tank left actual native Fissure cone aimed at party {exposed:?}; tank={at:?}"
    );
}

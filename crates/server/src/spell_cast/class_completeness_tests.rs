//! Shared, source-derived class effects against native Retail rows.
use super::test_harness::*;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::Health;

const PIERCING_CHILL: u32 = 377359;
const CHILL_STREAK: u32 = 204167;
const FROSTBOLT: u32 = 116;
const PALADIN_PASSIVE: u32 = 137026;
const CRUSADER_STRIKE: u32 = 35395;
const CHARGE: u32 = 100;
const FISTS_OF_FURY_DAMAGE: u32 = 117418;

#[test]
fn class_completeness_native_raise_dead_uses_destination_radius_and_owned_ghoul() {
    let mut h = harness(Vec::new(), &[46585], 6).unwrap();
    let data = world_db().unwrap();
    h.app
        .insert_resource(crate::world_data::load_world_data(&data.powers_db));
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    let spell = data.store.get(46585).unwrap().clone();
    let expected_distance = super::targets::radius(&spell.effects[0], 1);
    let destination =
        super::targets::effect_dest(h.app.world(), h.caster, None, &spell.effects[0], None)
            .expect("native destination radius");
    assert!((destination.position.length() - expected_distance).abs() < 0.001);
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None).expect("native Raise Dead");
    let caster = h.caster;
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query::<(
            &shared::components::Npc,
            &shared::components::UnitSummonedBy,
            &shared::components::Position,
        )>()
        .iter(world)
        .filter(|(npc, _, _)| npc.template_id == 26125)
        .collect();
    assert_eq!(summons.len(), 1);
    assert_eq!(summons[0].1.0, caster.to_bits());
    let point = summons[0].2;
    assert!((Vec3::new(point.x, point.y, point.z).length() - expected_distance).abs() < 0.001);
}

#[test]
fn class_completeness_native_real_points_per_level_scales_pick_lock_skill() {
    let mut h = harness(Vec::new(), &[1804], 4).unwrap();
    let spell = world_db().unwrap().store.get(1804).unwrap().clone();
    for (level, skill) in [(28, 14.0), (29, 15.0), (80, 66.0)] {
        h.app.world_mut().entity_mut(h.caster).insert((
            shared::components::Level(level),
            shared::components::UnitLevel(level),
        ));
        let value =
            super::effects::modded_base_points(h.app.world(), h.caster, &spell, &spell.effects[0]);
        assert_eq!(value, skill, "native1804 real-points per level at{level}");
    }
}

#[test]
fn class_completeness_native_caster_rectangle_selects_front_width_not_back_allies_or_far() {
    for (spell_id, class) in [(197214, 7), (392983, 10)] {
        let mut h = harness(Vec::new(), &[spell_id], class).unwrap();
        let spell = world_db().unwrap().store.get(spell_id).unwrap().clone();
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.implicit_targets.contains(&129))
            .unwrap();
        let radius = super::targets::radius(effect, 0);
        let width = spell.target_restrictions.as_ref().unwrap().width;
        let inside = spawn_npc(&mut h.app, 1.0);
        h.app
            .world_mut()
            .get_mut::<shared::components::Position>(inside)
            .unwrap()
            .z = width * 0.5;
        let back = spawn_npc(&mut h.app, -3.0);
        let outside = spawn_npc(&mut h.app, 1.0);
        h.app
            .world_mut()
            .get_mut::<shared::components::Position>(outside)
            .unwrap()
            .z = width + 2.0;
        let far = spawn_npc(&mut h.app, radius + 1.0);
        let ally = spawn_player(&mut h.app, class, 3.0);
        let selected = super::targets::select_targets(
            h.app.world_mut(),
            h.caster,
            Some(back),
            &spell,
            effect,
            0,
            false,
        )
        .expect("native caster-facing rectangle129");
        assert!(selected.contains(&h.npc));
        assert!(selected.contains(&inside));
        for excluded in [back, outside, far, ally] {
            assert!(
                !selected.contains(&excluded),
                "excluded {excluded:?} for {spell_id}"
            );
        }
    }
}

#[test]
fn toyfx54_podium_destination_snaps_to_authored_ground_not_airborne_caster() {
    let mut h = harness(Vec::new(), &[1227063], 8).unwrap();
    let data = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    place_guardian_fixture_on_native_start(&mut h, &conn);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .y += 2.0;
    let effect = &data.store.get(1227063).unwrap().effects[0];
    let selected = super::targets::effect_dest(h.app.world(), h.caster, None, effect, None)
        .expect("source Podium47→138 destination");
    let crate::ground::GroundProbe::Supported(height) = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_at(0, selected.position)
    else {
        panic!("real Northshire destination terrain required");
    };
    let grounded = super::targets::ground_destination(
        h.app.world_mut(),
        h.caster,
        data.store.get(1227063).unwrap(),
        effect,
        Some(selected),
    )
    .unwrap()
    .unwrap();
    assert_eq!(grounded.position.x, selected.position.x);
    assert_eq!(grounded.position.z, selected.position.z);
    assert_eq!(grounded.orientation, selected.orientation);
    assert!(
        (grounded.position.y - height).abs() < 0.01,
        "source138 must use map height {height}, not airborne height {}",
        grounded.position.y,
    );
}

#[test]
fn toyfx54_podium_front_destination_respects_source_player_combat_reach() {
    use shared::components::UnitScale;
    let mut h = harness(Vec::new(), &[1227063], 8).unwrap();
    let data = world_db().unwrap();
    let effect = &data.store.get(1227063).unwrap().effects[0];
    let origin = super::targets::position(h.app.world(), h.caster).unwrap();
    // Authored radius1.25; TC clamps caster-relative offsets to scaled player reach1.5.
    for (scale, expected_distance) in [(1.0, 1.5), (2.0, 3.0), (0.5, 1.25)] {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(UnitScale(scale));
        let destination = super::targets::effect_dest(h.app.world(), h.caster, None, effect, None)
            .expect("source Podium front destination");
        let distance = destination.position.distance(origin);
        assert!(
            (distance - expected_distance).abs() < 0.001,
            "source47 distance at scale{scale}: {distance} != {expected_distance}",
        );
    }
}

#[test]
fn class_completeness_native_darkness_ground_destination_snaps_to_authored_terrain() {
    let mut h = harness(Vec::new(), &[196718], 12).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (x, y, z): (f32, f32, f32) = conn.query_row("SELECT position_x,position_y,position_z FROM tdb_playercreateinfo WHERE race=1 AND class=1", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    let standing = Vec3::new(x, y, z);
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        maps.maps
            .values()
            .map(|map| (map.id, map.wdt_file_data_id))
            .collect(),
    );
    let crate::ground::GroundProbe::Supported(height) =
        ground.ground_at(0, standing + Vec3::Y * 2.0)
    else {
        panic!("native start terrain required");
    };
    h.app.insert_resource(ground);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .x = standing.x;
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .z = standing.z;
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .y = height + 2.0;
    let spell = world_db().unwrap().store.get(196718).unwrap().clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None)
        .expect("native caster-ground target62");
    let world = h.app.world_mut();
    let areas: Vec<_> = world
        .query::<(
            &super::area_trigger::SpellAreaTrigger,
            &shared::components::Position,
        )>()
        .iter(world)
        .filter(|(area, _)| area.spell_id == 196718)
        .collect();
    assert_eq!(areas.len(), 1);
    assert!(
        (areas[0].1.y - height).abs() < 0.01,
        "area rests on native ground, not airborne caster"
    );
}

#[test]
fn class_completeness_native_random_summons_use_authored_caster_or_target_disk() {
    let prepared = prepare_class_world();
    for (spell_id, class, entry, effect_index) in [(1276452, 9, 258584, 1), (123904, 10, 63508, 0)]
    {
        let mut h = harness(Vec::new(), &[spell_id], class).unwrap();
        let data = world_db().unwrap();
        h.app
            .insert_resource(crate::world_data::load_world_data(&prepared.path()));
        let conn = rusqlite::Connection::open_with_flags(
            &data.powers_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
        place_guardian_fixture_on_native_start(&mut h, &conn);
        let spell = data.store.get(spell_id).unwrap().clone();
        let effect = &spell.effects[effect_index];
        let slot = usize::from(spell_id == 123904);
        let origin = if spell_id == 123904 {
            super::targets::position(h.app.world(), h.npc).unwrap()
        } else {
            super::targets::position(h.app.world(), h.caster).unwrap()
        };
        let authored_radius = super::targets::radius(effect, slot);
        let points: Vec<_> = (0..32)
            .map(|_| {
                super::targets::effect_dest(h.app.world(), h.caster, Some(h.npc), effect, None)
                    .expect("native random destination")
                    .position
            })
            .collect();
        for point in &points {
            assert!(point.distance(origin) <= authored_radius + 0.001);
        }
        assert!(
            points
                .iter()
                .any(|point| point.distance(origin) < authored_radius * 0.8),
            "native random radius reaches the disk interior for {spell_id}"
        );
        assert!(points.iter().any(|point| point.distance(points[0]) > 0.1));
        super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(h.npc))
            .expect("native random summon");
        let caster = h.caster;
        let world = h.app.world_mut();
        let owned: Vec<_> = world
            .query::<(
                &shared::components::Npc,
                &shared::components::UnitSummonedBy,
            )>()
            .iter(world)
            .filter(|(npc, owner)| npc.template_id == entry && owner.0 == caster.to_bits())
            .collect();
        assert_eq!(owned.len(), 1, "native summon {spell_id}");
    }
}

fn place_guardian_fixture_on_native_start(h: &mut Harness, conn: &rusqlite::Connection) {
    let (x, y, z): (f32, f32, f32) = conn.query_row(
        "SELECT position_x,position_y,position_z FROM tdb_playercreateinfo WHERE race=1 AND class=1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    let origin = Vec3::new(x, y, z);
    let maps = crate::maps::store::load_map_store(conn).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        maps.maps
            .values()
            .map(|map| (map.id, map.wdt_file_data_id))
            .collect(),
    );
    assert_eq!(ground.area_at(0, origin), Some(9), "source Northshire area");
    for entity in [h.caster, h.npc] {
        let mut position = h
            .app
            .world_mut()
            .get_mut::<shared::components::Position>(entity)
            .unwrap();
        position.x += origin.x;
        position.y += origin.y;
        position.z += origin.z;
    }
    h.app.insert_resource(ground);
}

/// TC a352b1fa SpellInfo.cpp CalcRadius:72/86 use (Max-Min)*sqrt(rand_norm());91 does not.
#[test]
fn toyfx38_native_flute_destination_samples_area_not_ring_or_uniform_distance() {
    let h = harness(Vec::new(), &[142917], 8).unwrap();
    let spell = world_db().unwrap().store.get(142917).unwrap();
    let mut effect = spell.effects[0].clone();
    let native_radius = effect.radius[1].unwrap();
    // Real Flute first; a nonzero minimum distinguishes a span from a shifted annulus.
    for minimum in [native_radius.radius_min, 1.0] {
        effect.radius[1] = Some(crate::spell_info::SpellRadius {
            radius_min: minimum,
            ..native_radius
        });
        let maximum_distance = native_radius.radius - minimum;
        let squared_distances: Vec<_> = (0..512)
            .map(|_| {
                let destination =
                    super::targets::effect_dest(h.app.world(), h.caster, None, &effect, None)
                        .unwrap();
                assert_eq!(destination.position.y, 0.0);
                destination.position.length_squared() / maximum_distance.powi(2)
            })
            .collect();
        assert!(squared_distances.iter().all(|distance| *distance <= 1.0001));
        let mean = squared_distances.iter().sum::<f32>() / squared_distances.len() as f32;
        assert!(
            (0.4..0.6).contains(&mean),
            "uniform disk area, got {mean} (min {minimum})"
        );
    }
}

#[test]
fn toyfx38_native_flute_destination_inherits_radius_target_identity() {
    let h = harness(Vec::new(), &[142917], 8).unwrap();
    let mut effect = world_db().unwrap().store.get(142917).unwrap().effects[0].clone();
    let native_radius = effect.radius[1].unwrap();
    // CalcRadius only changes to TargetB when B has its own radius entry.
    effect.radius = [Some(native_radius), None];
    for _ in 0..32 {
        let destination =
            super::targets::effect_dest(h.app.world(), h.caster, None, &effect, None).unwrap();
        let distance = destination.position.length();
        assert!(
            (distance - native_radius.radius).abs() < 0.001,
            "inherited TargetA18 radius {distance}"
        );
    }
}

#[test]
fn class_completeness_native_destination_radius_uses_random_direction_not_fixed_front() {
    let h = harness(Vec::new(), &[46585], 6).unwrap();
    let spell = world_db().unwrap().store.get(46585).unwrap();
    let effect = &spell.effects[0];
    let native_radius = super::targets::radius(effect, 1);
    let points: Vec<_> = (0..32)
        .map(|_| {
            super::targets::effect_dest(h.app.world(), h.caster, None, effect, None)
                .unwrap()
                .position
        })
        .collect();
    for point in &points {
        assert!((point.length() - native_radius).abs() < 0.001);
    }
    assert!(
        points.iter().any(|point| point.distance(points[0]) > 0.1),
        "native target91 uses random direction"
    );
}

#[test]
fn class_completeness_native_mass_resurrection_offers_to_group_corpse_not_ghost_position() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    use crate::persistence::ClientId;
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::death::{Corpse, DeathState};
    use shared::protocol::{ResurrectionOffer, ResurrectionResponse};

    let mut h = harness(Vec::new(), &[212036], 5).unwrap();
    h.app.insert_resource(
        crate::conditions::load_spell_target_conditions(&world_db().unwrap().powers_db).unwrap(),
    );
    h.app.world_mut().entity_mut(h.caster).insert(ClientId(1));
    let target = spawn_player(&mut h.app, 5, 200.0);
    let stranger = spawn_player(&mut h.app, 5, 3.0);
    for (unit, id) in [(target, 2), (stranger, 3)] {
        h.app.world_mut().entity_mut(unit).insert((
            ClientId(id),
            crate::death::PlayerDeathState {
                state: DeathState::Ghost,
                corpse: Some(Corpse {
                    player: unit.to_bits(),
                    map_id: 0,
                    x: 3.0,
                    y: 0.0,
                    z: 0.0,
                    created_at: 0,
                }),
                graveyard: None,
            },
        ));
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 0.0;
    }
    let online: OnlineView = [(1, h.caster), (2, target)]
        .into_iter()
        .map(|(id, unit)| {
            (
                id,
                OnlineMember {
                    entity: unit.to_bits(),
                    name: format!("Member{id}"),
                    class: 5,
                    level: 80,
                    portrait: Default::default(),
                    dungeon_difficulty: 0,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.invite(1, "Member2", &online);
    groups.respond_invite(2, true);
    assert!(groups.is_in_same_group(1, 2));
    h.app.insert_resource(groups);
    let conn = spawn_peer_connection(&mut h.app, target, 99);
    h.app.world_mut().entity_mut(conn).insert((
        MessageSender::<ResurrectionOffer>::default(),
        MessageReceiver::<ResurrectionOffer>::default(),
    ));
    let spell = world_db().unwrap().store.get(212036).unwrap().clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None)
        .expect("native raid corpse selector");
    h.advance(1);
    let offers = drain::<ResurrectionOffer>(&mut h.app, conn);
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].spell_id, 212036);
    assert!(
        h.app
            .world()
            .get::<crate::death::resurrection::PendingResurrection>(stranger)
            .is_none()
    );
    crate::death::resurrection::respond(
        h.app.world_mut(),
        target,
        ResurrectionResponse {
            caster: h.caster.to_bits(),
            spell_id: 212036,
            accept: true,
        },
    );
    let percent = spell
        .effects
        .iter()
        .find(|effect| crate::death::resurrection::is_resurrection(effect))
        .unwrap()
        .base_points;
    assert_eq!(
        h.health(target),
        (h.app.world().get::<Health>(target).unwrap().max * percent / 100.0).floor()
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(target)
            .unwrap()
            .state,
        DeathState::Alive
    );
}

#[test]
fn class_completeness_native_enemy_cone_hits_front_not_allies_back_or_far() {
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[FISTS_OF_FURY_DAMAGE], 10).unwrap();
    let back = spawn_npc(&mut h.app, -4.0);
    let far = spawn_npc(&mut h.app, 30.0);
    let side = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(side)
        .unwrap()
        .z = 4.0;
    let ally = spawn_player(&mut h.app, 10, 3.0);
    let spell = world_db()
        .unwrap()
        .store
        .get(FISTS_OF_FURY_DAMAGE)
        .unwrap()
        .clone();
    let before = [h.npc, back, far, side, ally].map(|unit| h.health(unit));
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None)
        .expect("native caster-facing enemy cone must be supported");
    h.advance(3);
    assert!(
        h.health(h.npc) < before[0],
        "front hostile takes native AP damage"
    );
    for (unit, initial) in [back, far, side, ally].into_iter().zip(&before[1..]) {
        assert_eq!(h.health(unit), *initial, "excluded unit {unit:?}");
    }
}

#[test]
fn class_completeness_native_enemy_cone_follows_caster_facing_and_target_cap() {
    let mut h = harness(Vec::new(), &[FISTS_OF_FURY_DAMAGE], 10).unwrap();
    let back = spawn_npc(&mut h.app, -4.0);
    let farther_back = spawn_npc(&mut h.app, -5.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        });
    let spell = world_db()
        .unwrap()
        .store
        .get(FISTS_OF_FURY_DAMAGE)
        .unwrap()
        .clone();
    let targets = super::targets::select_targets(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        &spell.effects[0],
        1,
        false,
    )
    .expect("native cone target selection");
    assert_eq!(targets, vec![back]);
    assert!(!targets.contains(&farther_back));
}

fn fixture() -> Harness {
    let data = world_db().expect("real world.db");
    let spells = [PIERCING_CHILL, CHILL_STREAK, FROSTBOLT]
        .into_iter()
        .map(|id| {
            let mut spell = data.store.get(id).expect("native spell row").clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    harness(spells, &[], 6).expect("real world.db")
}

fn apply_chill(h: &mut Harness, caster: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: h.npc,
            spell_id: PIERCING_CHILL,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn hit(h: &mut Harness, spell_id: u32) -> f32 {
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 5000.0;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(h.npc)).unwrap();
    h.advance(10); // Native Chill Streak has a 20 yd/s missile.
    5000.0 - h.health(h.npc)
}

#[test]
fn class_completeness_native_label_vulnerability_increases_owned_chill_streak() {
    let mut h = fixture();
    let base = hit(&mut h, CHILL_STREAK);
    assert!(base > 0.0);
    let caster = h.caster;
    apply_chill(&mut h, caster);
    let amplified = hit(&mut h, CHILL_STREAK);
    assert_eq!(amplified, (base * 1.12).round());
}

#[test]
fn class_completeness_native_label_vulnerability_excludes_other_spell_and_caster() {
    let mut h = fixture();
    let frostbolt = hit(&mut h, FROSTBOLT);
    let chill = hit(&mut h, CHILL_STREAK);
    let other = spawn_player(&mut h.app, 6, 0.0);
    apply_chill(&mut h, other);
    assert_eq!(hit(&mut h, CHILL_STREAK), chill);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == PIERCING_CHILL,
    );
    let caster = h.caster;
    apply_chill(&mut h, caster);
    assert_eq!(hit(&mut h, FROSTBOLT), frostbolt);
}

#[test]
fn class_completeness_native_label_vulnerability_expires() {
    let mut h = fixture();
    let base = hit(&mut h, CHILL_STREAK);
    let caster = h.caster;
    apply_chill(&mut h, caster);
    h.advance(200);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        PIERCING_CHILL
    ));
    assert_eq!(hit(&mut h, CHILL_STREAK), base);
}

fn charge_fixture(passive: bool, haste: f32) -> Harness {
    let ids = if passive {
        vec![PALADIN_PASSIVE, CRUSADER_STRIKE, CHARGE]
    } else {
        vec![CRUSADER_STRIKE, CHARGE]
    };
    let mut h = harness(Vec::new(), &ids, 2).unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = haste;
    h.advance(2);
    h
}

fn cast_charge_button(h: &mut Harness, spell_id: u32) -> shared::protocol::SpellChargesUpdate {
    h.inbox.charges.clear();
    h.cast_and_deliver(spell_id, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .charges
        .iter()
        .find(|update| update.spell_id == spell_id)
        .unwrap()
        .clone()
}

#[test]
fn class_completeness_native_charge_haste_halves_crusader_strike_recovery() {
    let mut base = charge_fixture(false, 100.0);
    let normal = cast_charge_button(&mut base, CRUSADER_STRIKE);
    let mut h = charge_fixture(true, 100.0);
    let fast = cast_charge_button(&mut h, CRUSADER_STRIKE);
    assert_eq!(fast.recharge_ms, normal.recharge_ms / 2);
    h.advance((fast.recharge_ms / 50) as usize);
    let regenerated = cast_charge_button(&mut h, CRUSADER_STRIKE);
    assert_eq!(regenerated.current, regenerated.max - 1);
}

#[test]
fn class_completeness_native_charge_haste_excludes_other_category_and_zero_haste() {
    let mut base = charge_fixture(false, 100.0);
    charge_range(&mut base);
    let normal = cast_charge_button(&mut base, CHARGE);
    let mut h = charge_fixture(true, 100.0);
    charge_range(&mut h);
    assert_eq!(
        cast_charge_button(&mut h, CHARGE).recharge_ms,
        normal.recharge_ms
    );
    let mut zero = charge_fixture(true, 0.0);
    let mut absent = charge_fixture(false, 0.0);
    assert_eq!(
        cast_charge_button(&mut zero, CRUSADER_STRIKE).recharge_ms,
        cast_charge_button(&mut absent, CRUSADER_STRIKE).recharge_ms
    );
}

#[test]
fn class_completeness_native_charge_haste_retimes_inflight_recovery() {
    let mut h = charge_fixture(true, 0.0);
    let normal = cast_charge_button(&mut h, CRUSADER_STRIKE);
    h.advance(40);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = 100.0;
    let faster = cast_charge_button(&mut h, CRUSADER_STRIKE);
    assert_eq!(faster.recharge_ms, normal.recharge_ms / 2);
    // Two seconds plus bounded loopback delivery have elapsed since the first use.
    let expected = normal.remaining_ms.saturating_sub(2000) / 2;
    assert!(
        faster.remaining_ms.abs_diff(expected) <= 150,
        "{:?} vs {expected}",
        faster
    );
}

fn cooldown_fixture(passive: bool, haste: f32) -> Harness {
    let mut ids = vec![12294, 23881, 57755, 117313, 184362, 115804];
    if passive {
        ids.push(137047); // Native Warrior: cooldown haste mask, not every spell.
    }
    let mut h = harness(Vec::new(), &ids, 1).unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = haste;
    h.advance(2);
    h.set_power(shared::components::PowerType::Rage, 1000.0);
    h
}

fn cast_cooldown_button(h: &mut Harness, spell_id: u32) -> u32 {
    if spell_id == 57755 {
        charge_range(h); // Heroic Throw's native minimum range is 8 yards.
    }
    h.inbox.cooldowns.clear();
    h.cast_and_deliver(spell_id, Some(h.npc));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .cooldowns
        .iter()
        .find(|update| update.spell_id == spell_id && !update.is_gcd)
        .unwrap()
        .duration_ms
}

#[test]
fn class_completeness_native_cooldown_haste_scales_spell_and_category_timers() {
    for id in [12294, 23881] {
        let mut base = cooldown_fixture(false, 100.0);
        let normal = cast_cooldown_button(&mut base, id);
        let mut h = cooldown_fixture(true, 100.0);
        assert_eq!(cast_cooldown_button(&mut h, id), normal / 2);
    }
}

fn native80_fixture(ids: &[u32], class: u8, spec: u32) -> Harness {
    let mut h = harness(Vec::new(), ids, class).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.world_mut().entity_mut(h.caster).insert((
        shared::components::Level(80),
        shared::components::ActiveSpec(spec),
    ));
    h.advance(3);
    h
}

#[test]
fn class_completeness_native80_astral_generator_and_spender() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[190984, 78674, 137009, 137013, 197911], 11, 102);
    h.set_power(PowerType::LunarPower, 0.0);
    h.cast_and_deliver(190984, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::LunarPower), 60.0); // Native Balance modifier:6 Astral.
    h.set_power(PowerType::LunarPower, 400.0);
    h.cast_and_deliver(78674, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::LunarPower), 0.0); // Native Starsurge cost40 Astral.
}

#[test]
fn class_completeness_native80_word_of_glory_spends_three_holy_power() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[85673, 115675, 137026, 137028], 2, 66);
    h.set_power(PowerType::HolyPower, 3.0);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.cast_and_deliver(85673, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::HolyPower), 0.0);
    assert!(h.health(h.caster) > 100.0);
}

#[test]
fn class_completeness_native80_rune_runic_generator_and_spender() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 49998, 137005], 6, 251);
    h.set_power(PowerType::RunicPower, 0.0);
    h.cast_and_deliver(49184, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Runes), 5.0);
    assert_eq!(h.power(PowerType::RunicPower), 100.0); // Native negative-cost100 raw.
    h.advance(40);
    h.set_power(PowerType::RunicPower, 450.0);
    h.cast_and_deliver(49998, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::RunicPower), 0.0);
}

#[test]
fn class_completeness_native80_rune_generation_caps_at_native_max() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 137005], 6, 251);
    h.set_power(PowerType::RunicPower, 950.0);
    h.cast_and_deliver(49184, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::RunicPower), 1000.0);
    assert_eq!(h.power(PowerType::Runes), 5.0);
}

#[test]
fn class_completeness_native80_rune_immune_target_spends_neither_runes_nor_runic_gain() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 137005], 6, 251);
    h.set_power(PowerType::RunicPower, 0.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: 45438,
            effect_mask: 1 << 2,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(49184, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(h.npc), 5000.0);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 55095));
    assert_eq!(h.power(PowerType::Runes), 6.0);
    assert_eq!(h.power(PowerType::RunicPower), 0.0);
}

fn apply_native_immunity(h: &mut Harness, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: id,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native80_shadowfrost_partial_immunity_still_generates_runic() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[343294, 137005], 6, 252);
    // Partial-immunity control requires a hit; avoidance is covered separately.
    apply_native_enemy_dodge_reduction(&mut h);
    h.set_power(PowerType::RunicPower, 0.0);
    apply_native_immunity(&mut h, 7940); // Authored Frost16, not Shadow32.
    h.cast_and_deliver(343294, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Runes), 5.0);
    assert_eq!(h.power(PowerType::RunicPower), 100.0);
    assert!(h.health(h.npc) < 5000.0);
}

#[test]
fn class_completeness_native80_shadowfrost_combined_immunity_prevents_rune_costs() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[343294, 137005], 6, 252);
    h.set_power(PowerType::RunicPower, 0.0);
    apply_native_immunity(&mut h, 7940);
    apply_native_immunity(&mut h, 7743); // Separate native Shadow32 makes full48.
    h.cast_and_deliver(343294, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Runes), 6.0);
    assert_eq!(h.power(PowerType::RunicPower), 0.0);
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn class_completeness_native80_rune_rejection_never_grants_runic_power() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[49184, 137005], 6, 251);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(super::runes::Runes {
            cooldowns: vec![10.0; 6],
            order: vec![0, 1, 2, 3, 4, 5],
        });
    h.set_power(PowerType::Runes, 0.0);
    h.set_power(PowerType::RunicPower, 0.0);
    h.cast_and_deliver(49184, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.power(PowerType::RunicPower), 0.0);
    assert_eq!(h.power(PowerType::Runes), 0.0);
    assert!(!h.inbox.go.iter().any(|event| event.spell_id == 49184));
}

#[test]
fn class_completeness_native80_disintegrate_spends_three_essence() {
    use shared::components::PowerType;
    let mut h = native80_fixture(&[356995, 356809], 13, 1467);
    h.set_power(PowerType::Essence, 3.0);
    h.cast_and_deliver(356995, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == 356995));
    // Native0.2/s fractional regeneration resumes after the three-point spend.
    assert!((0.0..=0.1).contains(&h.power(PowerType::Essence)));
}

#[test]
fn class_completeness_native_cooldown_haste_truncates_native_milliseconds() {
    let mut h = cooldown_fixture(true, 40.0);
    assert_eq!(cast_cooldown_button(&mut h, 12294), 4285); // Native6000 / 1.4, truncated.
}

#[test]
fn class_completeness_native_charge_haste_syncs_without_another_cast() {
    let mut h = charge_fixture(true, 0.0);
    cast_charge_button(&mut h, CRUSADER_STRIKE);
    h.advance(40);
    h.inbox.charges.clear();
    h.inbox.go.clear();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = 100.0;
    h.advance(5);
    let changed = h
        .inbox
        .charges
        .last()
        .expect("rate update without a new cast");
    assert_eq!(changed.recharge_ms, 3000);
    assert!(changed.remaining_ms <= 2000);
    assert!(h.inbox.go.is_empty());
    h.advance(45);
    let ready = h.inbox.charges.last().expect("charge replenishment update");
    assert_eq!(ready.current, ready.max);
    assert!(h.inbox.go.is_empty());
}

#[test]
fn class_completeness_native_cooldown_haste_respects_mask_and_zero_haste() {
    let mut base = cooldown_fixture(false, 100.0);
    let mut h = cooldown_fixture(true, 100.0);
    assert_eq!(
        cast_cooldown_button(&mut h, 57755),
        cast_cooldown_button(&mut base, 57755)
    );
    let mut zero = cooldown_fixture(true, 0.0);
    let mut absent = cooldown_fixture(false, 0.0);
    assert_eq!(
        cast_cooldown_button(&mut zero, 12294),
        cast_cooldown_button(&mut absent, 12294)
    );
}

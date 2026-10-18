//! Ordinary205 Frost expiry creates the hazard; never fabricate an area-trigger entity.
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;
use std::path::{Path, PathBuf};

const BOMB: u32 = 386881;
const PARENT: u32 = 386781;
const GROUND: u32 = 387149;
const DAMAGE: u32 = 387150;
const STEP: f32 = 0.05;

fn at(h: &Harness, unit: Entity) -> Vec3 {
    let p = h.app.world().get::<Position>(unit).unwrap();
    Vec3::new(p.x, p.y, p.z)
}

fn source_fixture() -> (Harness, Entity) {
    let (h, telash) = super::azure_frost_bomb::fixture();
    let map = *h.app.world().get::<InMap>(telash).unwrap();
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .copy_difficulty(map)
            .unwrap()
            .difficulty_id,
        205
    );
    let store = h.app.world().resource::<SpellStore>();
    let parent = store.get(GROUND).unwrap();
    assert_eq!(
        (parent.effects[0].effect, parent.effects[0].misc_values[0]),
        (179, 26505)
    );
    assert_eq!(
        parent.misc.as_ref().unwrap().duration.unwrap().duration_ms,
        30000
    );
    let dot = store.get(DAMAGE).unwrap();
    let effects: Vec<_> = dot
        .effects
        .iter()
        .map(|effect| {
            (
                effect.index,
                effect.aura,
                effect.aura_period_ms,
                effect.base_points,
            )
        })
        .collect();
    assert_eq!(
        effects,
        vec![(0, 3, 1000, 8.0), (1, 33, 0, -30.0), (2, 260, 0, 0.0)],
        "ordinary source effects, not Mythic13 or invented magnitudes"
    );
    let db =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let properties: (u32, u32, f32, f32, u32, u32, String) = db.query_row(
        "SELECT AreaTriggerId,Flags,ShapeData0,ShapeData1,ScaleCurveId,TimeToTargetScale,ScriptName FROM tdb_areatrigger_create_properties WHERE Id=26505 AND IsCustom=0 AND Shape=0",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
    ).unwrap();
    assert_eq!(
        properties,
        (30679, 2, 10.0, 10.0, 53094, 150000, String::new())
    );
    let action: (u32, u32, u32) = db.query_row(
        "SELECT ActionType,ActionParam,TargetType FROM tdb_areatrigger_template_actions WHERE AreaTriggerId=30679 AND IsCustom=0",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(action, (0, DAMAGE, 2));
    let curve: (u32, u32) = db
        .query_row("SELECT Type,Flags FROM curve WHERE ID=53094", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(curve, (0, 0));
    let mut statement = db
        .prepare("SELECT Pos_0,Pos_1 FROM curve_point WHERE CurveID=53094 ORDER BY OrderIndex,ID")
        .unwrap();
    let points: Vec<(f32, f32)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(points, vec![(0.0, 0.5), (0.1, 1.0), (1.0, 1.0)]);
    (h, telash)
}

fn fields(h: &mut Harness, telash: Entity) -> Vec<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(h.app.world())
        .filter(|(_, field)| field.caster == telash && field.spell_id == GROUND)
        .map(|(entity, _)| entity)
        .collect()
}

fn has_own_ground_aura(h: &Harness, unit: Entity, telash: Entity) -> bool {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(unit)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == DAMAGE && aura.caster == telash)
        })
}

fn damage_events(h: &Harness, telash: Entity) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.source == Some(telash.to_bits())
                && event.target == Some(h.player().to_bits())
                && event.spell_id == Some(DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        })
        .count()
}

fn expire_first_ordinary_frost(h: &mut Harness, telash: Entity) -> (Entity, f64, f32) {
    h.engage(telash);
    h.advance(3.6);
    let cast = h.app.world().get::<CastState>(telash).unwrap();
    assert_eq!(
        (cast.spell_id, cast.cast_type, cast.duration),
        (PARENT, shared::casting::CastType::Normal, 2.0),
        "actual ordinary first Frost cast"
    );
    h.advance(3.1);
    let player = h.player();
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(player)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == BOMB)
        .expect("real normal cast/missile produced Frost Bomb aura");
    assert_eq!(aura.caster, telash);
    let expires = aura.expires_at.unwrap();
    assert!(
        fields(h, telash).is_empty(),
        "no field before actual Frost expiry"
    );
    let before_speed = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(player)
        .unwrap()
        .movement_speed;
    let remaining = (expires - crate::auras::aura_now(h.app.world())) as f32;
    for _ in 0..(remaining / STEP).ceil() as usize + 2 {
        h.advance(STEP);
        if !h.has_aura(player, BOMB) {
            break;
        }
    }
    assert!(!h.has_aura(player, BOMB));
    let born = crate::auras::aura_now(h.app.world());
    assert!(
        born >= expires,
        "natural expiry, not cancellation/manual hooks"
    );
    assert!(
        h.health(player).current > 0.0,
        "existing controlled native-stat Player survives"
    );
    let actual = fields(h, telash);
    assert_eq!(actual.len(), 1, "real Frost expiry creates one387149 field");
    (actual[0], born, before_speed)
}

fn project_floor(h: &mut Harness, map: InMap, mut position: Position) -> Position {
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, Vec3::new(position.x, position.y, position.z));
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("native Azure ground prerequisite: {probe:?}");
    };
    position.y = height;
    position
}

#[test]
fn azure_frozen_ground_native_expiry_deals_periodic_damage_slow_and_cleans_up_on_exit_and_caster_removal()
 {
    let (mut h, telash) = source_fixture();
    let player = h.player();
    let (field, _, baseline_speed) = expire_first_ordinary_frost(&mut h, telash);
    assert!(
        has_own_ground_aura(&h, player, telash),
        "real expiry-created field must apply native387150 damage/slow aura to its Player member"
    );
    let speed = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(player)
        .unwrap()
        .movement_speed;
    assert!(
        (speed - baseline_speed * 0.7).abs() < 0.000001,
        "derived native aura33 -30% slow"
    );
    assert_eq!(
        damage_events(&h, telash),
        0,
        "native387150 has no extra-initial-period bit"
    );
    h.advance(0.8);
    assert_eq!(
        damage_events(&h, telash),
        0,
        "periodic aura, not invented entry damage"
    );
    h.advance(0.3);
    assert_eq!(
        damage_events(&h, telash),
        1,
        "actual1000ms periodic damage event on the Player"
    );
    let inside = *h.app.world().get::<Position>(player).unwrap();
    let map = *h.app.world().get::<InMap>(player).unwrap();
    let outside = project_floor(
        &mut h,
        map,
        Position {
            x: inside.x + 15.0,
            ..inside
        },
    );
    h.app.world_mut().entity_mut(player).insert(outside);
    h.advance(STEP);
    assert!(
        !has_own_ground_aura(&h, player, telash),
        "exit removes only the field caster's aura"
    );
    let speed = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(player)
        .unwrap()
        .movement_speed;
    assert!(
        (speed - baseline_speed).abs() < 0.000001,
        "exit restores actual movement multiplier"
    );
    let ticks = damage_events(&h, telash);
    h.advance(1.1);
    assert_eq!(damage_events(&h, telash), ticks, "no owned DOT after exit");
    h.app.world_mut().entity_mut(player).insert(inside);
    h.advance(STEP);
    assert!(
        has_own_ground_aura(&h, player, telash),
        "actual re-entry reapplies source aura"
    );
    h.app.world_mut().entity_mut(telash).despawn();
    h.advance(STEP);
    assert!(h.app.world().get_entity(field).is_err());
    assert!(
        !has_own_ground_aura(&h, player, telash),
        "caster disappearance retires field/action aura"
    );
}

#[test]
fn azure_frozen_ground_native_source_curve_grows_five_to_ten_and_normal_field_expires_after_thirty_seconds()
 {
    let (mut h, telash) = source_fixture();
    let (field, born, _) = expire_first_ordinary_frost(&mut h, telash);
    assert_eq!(
        h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius,
        5.0,
        "native property radius10 times curve53094 initial0.5"
    );
    h.advance(7.5);
    assert!((h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius - 7.5).abs() < 0.001);
    h.advance(7.5);
    assert!((h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius - 10.0).abs() < 0.001);
    h.advance(14.9);
    assert!(
        h.app.world().get_entity(field).is_ok(),
        "source Normal30s, not premature cleanup"
    );
    assert!(crate::auras::aura_now(h.app.world()) - born < 30.0);
    h.advance(0.15);
    assert!(
        h.app.world().get_entity(field).is_err(),
        "actual lifetime of this field, not all later Frost fields"
    );
}

fn private_ground() -> PathBuf {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_AZURE_GROUND").expect(
        "prerequisite: explicit private2515 LOS/nav bake, no Stockade/default-data substitution",
    ));
    for directory in [root.join("los/2515"), root.join("navmesh/2515")] {
        assert!(
            directory.is_dir(),
            "missing private Azure bake prerequisite: {}",
            directory.display()
        );
    }
    root
}

fn assert_native_route(h: &mut Harness, unit: Entity, root: &Path) {
    let from = *h.app.world().get::<Position>(unit).unwrap();
    let map = *h.app.world().get::<InMap>(unit).unwrap();
    let to = project_floor(
        h,
        map,
        Position {
            x: from.x + 12.0,
            ..from
        },
    );
    let mut probe = crate::navigation::Navigation::new(&root.join("navmesh"));
    assert!(
        matches!(
            probe.advance(
                unit,
                crate::navigation::MoveRequest {
                    map_id: 2515,
                    goal: crate::navigation::Goal::Point,
                    from,
                    to,
                    distance: 0.1,
                    reach: 0.0,
                }
            ),
            crate::navigation::Step::Moved { .. }
        ),
        "prerequisite: actual2515 nav route from native actor feet"
    );
}

#[test]
fn azure_frozen_ground_native_expiry_hazard_is_escaped_by_a_native_follower_on_private_azure_geometry()
 {
    let root = private_ground();
    let (mut h, telash) = source_fixture();
    h.app
        .insert_resource(crate::navigation::Navigation::new(&root.join("navmesh")));
    h.app.add_systems(
        FixedUpdate,
        crate::followers::update_ai
            .after(crate::creature_aggro::update_victim_system)
            .before(crate::creature_ai::chase_system),
    );
    let (field, _, _) = expire_first_ordinary_frost(&mut h, telash);
    let owner = h.player();
    let unit = crate::followers::native_fixture::spawn(h.app.world_mut(), owner, 209057)
        .expect("real imported Garrick profile/template and production native initializer");
    assert!(
        h.app.world().get::<Player>(unit).is_none(),
        "follower remains NPC"
    );
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), unit)
        .expect("authored Garrick combat reach prerequisite, not guessed body geometry");
    assert_native_route(&mut h, unit, &root);
    h.advance(STEP);
    assert!(
        has_own_ground_aura(&h, owner, telash),
        "actual Player damage/slow producer before escape RED"
    );
    let center = at(&h, field);
    let mut collision = los::LosMap::new(&root.join("los/2515"));
    let mut previous = at(&h, unit);
    let mut escaped = false;
    for tick in 0..120 {
        h.advance(STEP);
        let position = at(&h, unit);
        assert!(
            collision
                .clear(previous + Vec3::Y, position + Vec3::Y)
                .expect("actual private2515 LOS tile"),
            "escape tick{tick} crossed native collision"
        );
        let radius = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
        if position.distance(center) > radius + reach {
            escaped = true;
            break;
        }
        previous = position;
    }
    assert!(
        escaped,
        "follower must escape the actual Frost-expiry field, not a manufactured hazard"
    );
    assert!(
        h.health(unit).current > 0.0,
        "unchanged native follower survives"
    );
    assert!(!has_own_ground_aura(&h, unit, telash));
    for _ in 0..40 {
        h.advance(STEP);
        let radius = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
        assert!(
            at(&h, unit).distance(center) > radius + reach,
            "normal follow/combat does not re-enter growing native affect range"
        );
    }
}

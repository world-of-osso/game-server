//! Player casts and their line of sight witness ray (docs/specs/line-of-sight.md, Player
//! casts) against the real map 0 bake (`los-bake --map 0`, the ground directory's `los/`)
//! and world.db spells.

use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{ModelDisplay, Position, Rotation};
use shared::protocol::{CastFailed, WitnessRay};
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::line_of_sight::{LineOfSight, WITNESS_END_TOLERANCE};
use crate::maps::InMap;

const FIREBALL: u32 = 133;
const ARCANE_EXPLOSION: u32 = 1449;
/// Kobold Vermin's display: an eye row of the bake.
const KOBOLD_DISPLAY: u32 = 10913;

/// The mage at WoW `caster`, a kobold at WoW `target`, with the bake loaded.
fn harness_at(caster: (f32, f32, f32), target: (f32, f32, f32)) -> Option<Harness> {
    let dir = crate::ground::ground_data_dir().join("los");
    crate::test_data::require(dir.join(los::eye::EYE_TABLE_FILE));
    let mut h = harness(Vec::new(), &[FIREBALL], 8)?;
    h.app.insert_resource(LineOfSight::load(&dir).unwrap());
    let place = |(x, y, z): (f32, f32, f32)| {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
        Position { x, y, z }
    };
    let (from, to) = (place(caster), place(target));
    // Facing each other (`Rotation.y`: forward is Bevy `(sin y, 0, cos y)`).
    let yaw = (to.x - from.x).atan2(to.z - from.z);
    let world = h.app.world_mut();
    world.entity_mut(h.caster).insert((from, facing(yaw)));
    world.entity_mut(h.npc).insert((
        to,
        facing(yaw + std::f32::consts::PI),
        ModelDisplay {
            display_id: KOBOLD_DISPLAY,
        },
    ));
    Some(h)
}

fn facing(yaw: f32) -> Rotation {
    Rotation {
        x: 0.0,
        y: yaw,
        z: 0.0,
    }
}

/// The witness ray the client's search finds: the caster's eyes to the first clear one
/// of the kobold's eyes, chest and feet.
fn search(h: &Harness) -> (Option<WitnessRay>, WitnessRay) {
    let world = h.app.world();
    let los = world.resource::<LineOfSight>();
    let body = |id: Entity| {
        let unit = world.entity(id);
        los.body((
            id,
            unit.get::<shared::components::Npc>(),
            unit.get::<Position>().unwrap(),
            unit.get::<Rotation>(),
            unit.get::<ModelDisplay>(),
            None,
            unit.get::<shared::components::Player>(),
            unit.get::<shared::components::PlayerStandState>(),
        ))
        .unwrap()
    };
    let (caster, target) = (body(h.caster), body(h.npc));
    let found = los.witness(InMap::continent(0), &caster, &target);
    let ray = |end: Vec3| WitnessRay {
        start: caster.eyes.to_array(),
        end: end.to_array(),
    };
    // The straight eye-to-chest ray, clear or not.
    (found.map(ray), ray(target.chest()))
}

/// Cast Fireball at the kobold with `witness`: the failure, if any.
fn cast(h: &mut Harness, witness: Option<WitnessRay>) -> Option<CastFailReason> {
    h.app.world_mut().entity_mut(h.caster).remove::<CastState>();
    h.cast_witnessed(FIREBALL, Some(h.npc), witness);
    h.advance(3);
    let failed: Vec<CastFailed> = std::mem::take(&mut h.inbox.failed);
    assert!(failed.len() <= 1, "{failed:?}");
    match failed.first() {
        Some(failed) => Some(failed.reason),
        None => {
            assert!(h.app.world().get::<CastState>(h.caster).is_some());
            None
        }
    }
}

#[test]
fn a_cast_at_a_kobold_behind_the_stormwind_wall_fails_with_line_of_sight() {
    // Stormwind's outer wall west of the main gate: the mage on the Elwynn side, the
    // kobold on the street behind it, 36 yd away (floor heights from the bake).
    let Some(mut h) = harness_at((-8862.27, 471.05, 83.10), (-8835.97, 489.89, 98.26)) else {
        return;
    };
    let (found, straight) = search(&h);
    assert_eq!(found, None, "no clear ray reaches the kobold");
    assert_eq!(cast(&mut h, None), Some(CastFailReason::LineOfSight));
    // A forged witness: the straight ray through the wall.
    assert_eq!(
        cast(&mut h, Some(straight)),
        Some(CastFailReason::LineOfSight)
    );
}

#[test]
fn a_cast_through_the_northshire_abbey_doorway_needs_its_witness_ray() {
    // The kobold in the hall, the mage on the steps of the west door 30 yd away.
    let Some(mut h) = harness_at((-8909.2, -146.6, 81.95), (-8896.5, -173.8, 81.58)) else {
        return;
    };
    let (found, _) = search(&h);
    let found = found.expect("a clear ray through the doorway");
    assert_eq!(cast(&mut h, None), Some(CastFailReason::LineOfSight));
    // Forged: the same ray ending 3 yd past the latency tolerance above the kobold.
    let above = WitnessRay {
        end: [
            found.end[0],
            found.end[1] + WITNESS_END_TOLERANCE + 3.0,
            found.end[2],
        ],
        ..found
    };
    assert_eq!(cast(&mut h, Some(above)), Some(CastFailReason::LineOfSight));
    // Forged: starting 3 yd beside the mage's eyes.
    let beside = WitnessRay {
        start: [found.start[0] + 3.0, found.start[1], found.start[2]],
        ..found
    };
    assert_eq!(
        cast(&mut h, Some(beside)),
        Some(CastFailReason::LineOfSight)
    );
    assert_eq!(cast(&mut h, Some(found)), None, "the cast starts");
}

#[test]
fn the_wall_beside_the_abbey_door_blocks_the_same_line_moved_aside() {
    // The doorway line moved 4 yd to the side runs through the wall beside the door.
    let Some(mut h) = harness_at((-8905.575, -144.91, 81.96), (-8892.875, -172.11, 81.58)) else {
        return;
    };
    let (found, straight) = search(&h);
    assert_eq!(found, None);
    assert_eq!(
        cast(&mut h, Some(straight)),
        Some(CastFailReason::LineOfSight)
    );
}

fn area_wall_harness() -> Harness {
    let mut h = harness(Vec::new(), &[ARCANE_EXPLOSION], 8).unwrap();
    let mut sight = LineOfSight::load(&crate::ground::ground_data_dir().join("los")).unwrap();
    let wall = std::sync::Arc::new(los::LosTile::build(
        32,
        32,
        vec![
            Vec3::new(2.0, -5.0, -10.0),
            Vec3::new(2.0, 10.0, -10.0),
            Vec3::new(2.0, 10.0, 10.0),
            Vec3::new(2.0, -5.0, 10.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    ));
    sight.set_live_hulls(std::collections::HashMap::from([(
        InMap::continent(0),
        vec![wall],
    )]));
    h.app.insert_resource(sight);
    h.app.world_mut().entity_mut(h.npc).insert(ModelDisplay {
        display_id: KOBOLD_DISPLAY,
    });
    let visible = spawn_npc(&mut h.app, -3.0);
    h.app.world_mut().entity_mut(visible).insert(ModelDisplay {
        display_id: KOBOLD_DISPLAY,
    });
    assert!(!crate::line_of_sight::sees_in_world(
        h.app.world(),
        h.npc,
        h.caster
    ));
    assert!(crate::line_of_sight::sees_in_world(
        h.app.world(),
        visible,
        h.caster
    ));
    h
}

/// TC a352b1fa SelectImplicitAreaTargets -> AddUnitTarget(center) ->
/// CheckEffectTarget: the hit must have LOS to the area center.
#[test]
fn arcane_explosion_hits_visible_units_but_not_units_behind_a_wall() {
    let mut h = area_wall_harness();
    let visible = h
        .app
        .world_mut()
        .query::<(Entity, &Position)>()
        .iter(h.app.world())
        .find(|(_, at)| at.x == -3.0)
        .unwrap()
        .0;
    h.cast_and_deliver(ARCANE_EXPLOSION, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<shared::components::Health>(visible)
            .unwrap()
            .current
            < 5000.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap()
            .current,
        5000.0,
        "area damage must not pass through the wall"
    );
}

#[test]
fn area_hit_sight_uses_destination_center_and_authored_los_flags() {
    let mut h = area_wall_harness();
    let native = world_db()
        .unwrap()
        .store
        .get(ARCANE_EXPLOSION)
        .unwrap()
        .clone();
    assert_eq!(
        super::attributes(&native, 2) & 4,
        0,
        "native Arcane Explosion does not ignore LOS"
    );
    let mut spell = native.clone();
    // Isolate the destination selector with native radius/damage metadata.
    let mut effect = spell
        .effects
        .iter()
        .find(|effect| effect.implicit_targets.contains(&15))
        .unwrap()
        .clone();
    effect.implicit_targets = [87, 16];
    spell.cast_destination = Some([3.0, 0.0, 0.0]);
    let select = |h: &mut Harness,
                  spell: &crate::spell_info::SpellInfo,
                  effect: &crate::spell_info::SpellEffect| {
        super::targets::select_targets(h.app.world_mut(), h.caster, None, spell, effect, 0, false)
            .unwrap()
    };
    let units = select(&mut h, &spell, &effect);
    assert_eq!(
        units,
        vec![h.npc],
        "recipient sees center even though caster cannot see recipient"
    );
    spell.attributes[5] |= 0x0400_0000;
    assert!(
        select(&mut h, &spell, &effect).is_empty(),
        "authored always-AOE-LOS also requires caster sight"
    );
    spell.attributes[5] = native.attributes[5];
    effect.attributes |= 0x20;
    assert!(
        select(&mut h, &spell, &effect).is_empty(),
        "effect always-AOE-LOS also requires caster sight"
    );
    spell.attributes[2] |= 4;
    assert_eq!(
        select(&mut h, &spell, &effect).len(),
        2,
        "ignore-LOS bypasses both checks"
    );
}

/// Template display 251 has no `CreatureDisplayInfo` row, so the bake skips it: a
/// creature a script shows it with is out of line of sight (fail closed), not skipped.
#[test]
fn a_creature_showing_a_display_without_an_eye_row_is_out_of_line_of_sight() {
    let Some(mut h) = harness_at((-8909.2, -146.6, 81.95), (-8896.5, -173.8, 81.58)) else {
        return;
    };
    let sees = |h: &Harness| crate::line_of_sight::sees_in_world(h.app.world(), h.npc, h.caster);
    assert!(sees(&h), "the kobold sees the mage through the doorway");
    let witness = search(&h).0.expect("a clear ray through the doorway");
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(ModelDisplay { display_id: 251 });
    assert!(!sees(&h));
    assert_eq!(
        cast(&mut h, Some(witness)),
        Some(CastFailReason::LineOfSight)
    );
}

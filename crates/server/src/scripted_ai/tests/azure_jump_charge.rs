//! Real Telash effect254 casts; no movement component or arrival hook is fabricated.
use super::*;

const OUTBOUND_JUMP: u32 = 387981;
const RETURN_JUMP: u32 = 388082;
const FIXTURE_STEP_SECONDS: f32 = 0.05;
const POSITION_TOLERANCE: f32 = 0.001;

fn read_source_jump_seconds(parameter: u32) -> f32 {
    let db = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("published jump-charge database prerequisite");
    db.query_row(
        "SELECT speed FROM tdb_jump_charge_params WHERE id=?1 \
         AND treatSpeedAsMoveTimeSeconds=1 AND unlimitedSpeed=0 \
         AND minHeight IS NULL AND maxHeight IS NULL AND spellVisualId IS NULL \
         AND progressCurveId IS NULL AND parabolicCurveId IS NULL AND triggerSpellId IS NULL",
        [parameter],
        |row| row.get::<_, f32>(0),
    )
    .expect("actual713/714 time mode and NULL-option prerequisites")
}

fn position(h: &Harness, unit: Entity) -> Vec3 {
    let at = h.app.world().get::<Position>(unit).unwrap();
    Vec3::new(at.x, at.y, at.z)
}

fn assert_native_jump(spell_id: u32, parameter: u32, wow_destination: Vec3) {
    let seconds = read_source_jump_seconds(parameter);
    assert_eq!(
        seconds, 1.0,
        "exact published server SQL, not a guessed speed"
    );
    let (mut h, telash) = super::azure_frost_bomb::fixture();
    assert!(h.app.world().get::<Health>(telash).unwrap().current > 0.0);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    let effect = &spell.effects[0];
    assert_eq!(
        (effect.effect, effect.misc_values[0]),
        (254, parameter as i32)
    );
    assert_eq!(effect.trigger_spell, 0);
    let misc = spell.misc.as_ref().expect("native SpellMisc prerequisite");
    assert_eq!(misc.attributes[9], 0x0010_0000);
    assert_eq!(misc.speed, 0.0, "no missile flight before native movement");
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(wow_destination.x, wow_destination.y, wow_destination.z);
    let destination = Vec3::new(x, y, z);
    let planned = crate::spell_cast::targets::effect_dest(
        h.app.world(),
        telash,
        None,
        effect,
        Some(destination.to_array()),
    );
    assert_eq!(
        planned.map(|dest| dest.position),
        Some(destination),
        "source SpellMgr387981/388082 exact-destination prerequisite, not a motion RED"
    );
    let from = position(&h, telash);
    let distance = from.distance(destination);
    assert!(
        distance > 1.0,
        "actual source spawn and authored endpoint differ"
    );
    let map = *h.app.world().get::<InMap>(telash).unwrap();
    let ground = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, destination);
    assert!(
        matches!(ground, crate::ground::GroundProbe::Supported(_)),
        "actual destination floor prerequisite: {ground:?}"
    );
    let los =
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .expect("native LOS inputs, not a missing-resource motion failure");
    h.app.insert_resource(los);
    crate::spell_cast::cast_triggered_at_destination(
        h.app.world_mut(),
        telash,
        spell_id,
        destination,
        1,
    );
    assert_eq!(
        position(&h, telash),
        from,
        "effect254 must not teleport at launch"
    );
    let speed = (distance / seconds).min(50.0);
    let travel_seconds = distance / speed;
    let duration_ms = 1 + (distance * (1000.0 / speed)) as u32;
    let height = (shared::skyriding::GRAVITY * travel_seconds.powi(2) / 8.0).clamp(0.5, 1000.0);
    h.advance(0.5);
    let t = 500.0 / duration_ms as f32;
    let mut midpoint = from.lerp(destination, t);
    midpoint.y += height * 4.0 * t * (1.0 - t);
    assert!(
        position(&h, telash).distance(midpoint) < POSITION_TOLERANCE,
        "native half-second linear/parabolic interpolation, got {:?}, expected {midpoint:?}",
        position(&h, telash)
    );
    let before_landing_frames = (duration_ms - 1) / 50;
    let remaining_frames = before_landing_frames - 10;
    h.advance(remaining_frames as f32 * FIXTURE_STEP_SECONDS);
    assert!(
        position(&h, telash).distance(destination) > POSITION_TOLERANCE,
        "not landed before the source spline duration including1ms base"
    );
    h.advance(FIXTURE_STEP_SECONDS);
    assert_eq!(
        position(&h, telash),
        destination,
        "exact authored endpoint, not ground-snapped or LOS-clamped"
    );
}

#[test]
#[ignore = "native254 motion RED pending repaired-master gate"]
fn azure_telash_387981_jumps_to_authored_elevated_center_without_teleport() {
    // TC boss_telash_greywing.cpp TelashJumpPosition, not a derived floor point.
    assert_native_jump(
        OUTBOUND_JUMP,
        713,
        Vec3::new(-5336.8003, 1066.6493, 344.32678),
    );
}

#[test]
#[ignore = "native254 motion RED pending repaired-master gate"]
fn azure_telash_388082_jumps_to_authored_return_endpoint_without_teleport() {
    // Proves real388082 motion to source return point0, not the later selector/hook.
    assert_native_jump(RETURN_JUMP, 714, Vec3::new(-5350.0835, 1089.8976, 340.5709));
}

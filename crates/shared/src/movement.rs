//! Shared movement constants and validation used by both client and server.

/// Max walkable slope angle (radians). WoW uses ~50°.
pub const MAX_SLOPE_ANGLE: f32 = 50.0 * std::f32::consts::PI / 180.0;

/// Gravity in yards/s² (~2g for snappy game feel).
pub const GRAVITY: f32 = 19.6;

/// Distance below which we snap to ground.
pub const GROUND_SNAP_THRESHOLD: f32 = 0.3;

/// Run speed in yards/sec.
pub const RUN_SPEED: f32 = 7.0;

/// Walk speed in yards/sec.
pub const WALK_SPEED: f32 = 2.5;

/// Swim speed in yards/sec (67.5% of run speed, WoW default).
pub const SWIM_SPEED: f32 = 4.7222;

/// Backward movement speed multiplier (WoW: ~60% of forward).
pub const BACKPEDAL_MULTIPLIER: f32 = 0.6;

/// Strafing movement speed multiplier.
pub const STRAFE_MULTIPLIER: f32 = 0.8;

/// Jump impulse velocity in yards/sec.
pub const JUMP_IMPULSE: f32 = 9.0;

/// Returns true if a mounted player should be dismounted (entering water).
pub fn should_dismount(swimming: bool, mounted: bool) -> bool {
    swimming && mounted
}

/// Minimum fall distance before damage applies (yards).
pub const FALL_DAMAGE_THRESHOLD: f32 = 2.7;

/// Calculate fall damage as a percentage of max HP (0.0..=1.0).
///
/// WoW-style formula: no damage below threshold, then scales quadratically.
/// Fatal at ~65+ yards.
pub fn fall_damage_percent(fall_distance: f32) -> f32 {
    if fall_distance <= FALL_DAMAGE_THRESHOLD {
        return 0.0;
    }
    let excess = fall_distance - FALL_DAMAGE_THRESHOLD;
    // Quadratic scaling: ~2.5% per yard² above threshold
    (excess * excess * 0.00025).min(1.0)
}

/// WMO group flags for indoor/outdoor detection.
/// Ref: WoW WMO format — group header flags field.
pub const WMO_FLAG_OUTDOOR: u32 = 0x08;
pub const WMO_FLAG_INDOOR: u32 = 0x2000;

/// Determine if a WMO area is indoors based on group flags.
///
/// If INDOOR flag is set → indoors. If OUTDOOR flag is set → outdoors.
/// If neither or both → default to outdoor.
pub fn is_indoor(wmo_group_flags: u32) -> bool {
    let indoor = wmo_group_flags & WMO_FLAG_INDOOR != 0;
    let outdoor = wmo_group_flags & WMO_FLAG_OUTDOOR != 0;
    indoor && !outdoor
}

/// Calculate water depth at a position.
///
/// Returns 0.0 if no water or terrain is above water.
/// `water_surface` and `terrain_height` are both in world Z.
pub fn water_depth(water_surface: Option<f32>, terrain_height: f32) -> f32 {
    match water_surface {
        Some(ws) if ws > terrain_height => ws - terrain_height,
        _ => 0.0,
    }
}

/// Check if a slope is walkable given height difference and horizontal distance.
pub fn is_walkable_slope(height_diff: f32, horizontal_dist: f32) -> bool {
    if horizontal_dist < 0.001 {
        return true;
    }
    let slope = (height_diff / horizontal_dist).abs().atan();
    slope <= MAX_SLOPE_ANGLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_is_walkable() {
        assert!(is_walkable_slope(0.0, 10.0));
    }

    #[test]
    fn gentle_slope_is_walkable() {
        assert!(is_walkable_slope(0.577, 1.0)); // ~30°
    }

    #[test]
    fn steep_slope_is_rejected() {
        assert!(!is_walkable_slope(1.732, 1.0)); // ~60°
    }

    #[test]
    fn slope_at_49_degrees_is_walkable() {
        // tan(49°) ≈ 1.1504
        let height = 49.0_f32.to_radians().tan();
        assert!(is_walkable_slope(height, 1.0));
    }

    #[test]
    fn slope_at_exactly_50_degrees_is_walkable() {
        // tan(50°) ≈ 1.1918 — boundary, should pass (<=)
        let height = 50.0_f32.to_radians().tan();
        assert!(is_walkable_slope(height, 1.0));
    }

    #[test]
    fn slope_at_51_degrees_is_rejected() {
        // tan(51°) ≈ 1.2349
        let height = 51.0_f32.to_radians().tan();
        assert!(!is_walkable_slope(height, 1.0));
    }

    #[test]
    fn vertical_slope_is_rejected() {
        assert!(!is_walkable_slope(1000.0, 1.0)); // ~90°
    }

    #[test]
    fn fall_damage_zero_below_threshold() {
        assert_eq!(fall_damage_percent(0.0), 0.0);
        assert_eq!(fall_damage_percent(1.0), 0.0);
        assert_eq!(fall_damage_percent(FALL_DAMAGE_THRESHOLD), 0.0);
    }

    #[test]
    fn fall_damage_scales_with_distance() {
        let short = fall_damage_percent(5.0);
        let medium = fall_damage_percent(15.0);
        let long = fall_damage_percent(30.0);

        assert!(short > 0.0, "short fall deals damage");
        assert!(medium > short, "longer fall deals more");
        assert!(long > medium, "even longer deals even more");
    }

    #[test]
    fn fall_damage_caps_at_100_percent() {
        assert_eq!(fall_damage_percent(100.0), 1.0);
        assert_eq!(fall_damage_percent(1000.0), 1.0);
    }

    #[test]
    fn fall_damage_fatal_around_65_yards() {
        let at_60 = fall_damage_percent(60.0);
        let at_65 = fall_damage_percent(65.0);
        assert!(at_60 < 1.0, "60yd not quite fatal: {at_60}");
        assert!((at_65 - 1.0).abs() < 0.05, "~65yd is fatal: {at_65}");
    }

    #[test]
    fn dismount_on_entering_water() {
        assert!(should_dismount(true, true));
        assert!(!should_dismount(false, true));
        assert!(!should_dismount(true, false));
        assert!(!should_dismount(false, false));
    }

    #[test]
    fn wmo_indoor_outdoor_detection() {
        // Indoor flag only → indoor
        assert!(is_indoor(WMO_FLAG_INDOOR));

        // Outdoor flag only → outdoor
        assert!(!is_indoor(WMO_FLAG_OUTDOOR));

        // Both flags → outdoor wins
        assert!(!is_indoor(WMO_FLAG_INDOOR | WMO_FLAG_OUTDOOR));

        // Neither flag → default outdoor
        assert!(!is_indoor(0));

        // Indoor with other flags set → still indoor
        assert!(is_indoor(WMO_FLAG_INDOOR | 0x01));

        // Outdoor with other flags → still outdoor
        assert!(!is_indoor(WMO_FLAG_OUTDOOR | 0x01));
    }

    #[test]
    fn water_depth_calculation() {
        // Submerged: water at 10, terrain at -5 → depth 15
        assert!((water_depth(Some(10.0), -5.0) - 15.0).abs() < 0.01);

        // Shallow: water at 5, terrain at 3 → depth 2
        assert!((water_depth(Some(5.0), 3.0) - 2.0).abs() < 0.01);

        // Terrain above water: water at 5, terrain at 8 → depth 0
        assert_eq!(water_depth(Some(5.0), 8.0), 0.0);

        // Terrain at water level → depth 0
        assert_eq!(water_depth(Some(5.0), 5.0), 0.0);

        // No water → depth 0
        assert_eq!(water_depth(None, -20.0), 0.0);
    }
}

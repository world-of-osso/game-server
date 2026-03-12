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

/// Jump impulse velocity in yards/sec.
pub const JUMP_IMPULSE: f32 = 9.0;

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
}

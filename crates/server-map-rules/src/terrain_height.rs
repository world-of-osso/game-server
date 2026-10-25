/// Convert WoW coordinates (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
pub fn wow_to_bevy(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    (x, z, -y)
}

/// Convert Bevy coordinates (X-right, Y-up, Z-back) to WoW (X-right, Y-forward, Z-up).
pub fn bevy_to_wow(bx: f32, by: f32, bz: f32) -> (f32, f32, f32) {
    (bx, -bz, by)
}

#[cfg(test)]
#[path = "terrain_height_tests.rs"]
mod tests;

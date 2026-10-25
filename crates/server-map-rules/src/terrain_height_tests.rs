use super::*;

#[test]
fn wow_bevy_coordinate_round_trip() {
    let cases = [
        (0.0, 0.0, 0.0),
        (-8949.0, -132.0, 83.0),
        (100.0, -50.0, 200.0),
        (-1000.0, 5000.0, -300.0),
    ];
    for (wx, wy, wz) in cases {
        let (bx, by, bz) = wow_to_bevy(wx, wy, wz);
        let (rx, ry, rz) = bevy_to_wow(bx, by, bz);
        assert!(
            (rx - wx).abs() < 1e-6 && (ry - wy).abs() < 1e-6 && (rz - wz).abs() < 1e-6,
            "wow→bevy→wow failed for ({wx},{wy},{wz}): got ({rx},{ry},{rz})",
        );

        // Also test bevy→wow→bevy
        let (bx2, by2, bz2) = wow_to_bevy(rx, ry, rz);
        assert!(
            (bx2 - bx).abs() < 1e-6 && (by2 - by).abs() < 1e-6 && (bz2 - bz).abs() < 1e-6,
            "bevy→wow→bevy failed",
        );
    }
}

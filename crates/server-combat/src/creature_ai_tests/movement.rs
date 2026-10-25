use super::*;

#[test]
fn pick_point_within_radius() {
    for seed in 0..100 {
        let (px, py) = pick_wander_point(10.0, 20.0, 5.0, seed);
        let dx = px - 10.0;
        let dy = py - 20.0;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!(dist <= 5.01, "seed {seed}: dist {dist} exceeds radius 5.0");
    }
}

#[test]
fn pick_point_zero_radius_stays_at_center() {
    let (px, py) = pick_wander_point(5.0, 5.0, 0.0, 42);
    assert!((px - 5.0).abs() < 0.001);
    assert!((py - 5.0).abs() < 0.001);
}

#[test]
fn move_toward_reaches_close_target() {
    let (reached, x, y) = move_toward(0.0, 0.0, 0.5, 0.0, 1.0);
    assert!(reached);
    assert!((x - 0.5).abs() < 0.001);
    assert!(y.abs() < 0.001);
}

#[test]
fn move_toward_partial_step() {
    let (reached, x, y) = move_toward(0.0, 0.0, 10.0, 0.0, 3.0);
    assert!(!reached);
    assert!((x - 3.0).abs() < 0.001);
    assert!(y.abs() < 0.001);
}

#[test]
fn move_toward_diagonal() {
    let (reached, x, y) = move_toward(0.0, 0.0, 3.0, 4.0, 2.5);
    assert!(!reached);
    assert!((x - 1.5).abs() < 0.001);
    assert!((y - 2.0).abs() < 0.001);
}

#[test]
fn lcg_produces_values_in_range() {
    for seed in 0..1000 {
        let v = lcg_float(seed);
        assert!(
            (0.0..1.0).contains(&v),
            "seed {seed}: value {v} out of range"
        );
    }
}

#[test]
fn random_idle_in_range() {
    for seed in 0..100 {
        let t = random_idle_secs(seed);
        assert!(
            (IDLE_MIN_SECS..=IDLE_MAX_SECS).contains(&t),
            "seed {seed}: idle {t}"
        );
    }
}

//! Native rand distributions; only an explicit, test-only scope supplies a seed.
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) use ::rand::*;

#[cfg(any(test, feature = "test-support"))]
pub(crate) use ::rand::{Rng, SeedableRng, distributions, rngs, seq};

#[cfg(any(test, feature = "test-support"))]
thread_local! {
    static SEEDED: std::cell::RefCell<Option<rngs::StdRng>> = const { std::cell::RefCell::new(None) };
}

/// Restores the caller's RNG mode when this explicit simulation scope ends.
#[cfg(any(test, feature = "test-support"))]
pub struct SeedScope(Option<rngs::StdRng>);

#[cfg(any(test, feature = "test-support"))]
pub fn seed_scope(seed: u64) -> SeedScope {
    let rng = rngs::StdRng::seed_from_u64(seed);
    SeedScope(SEEDED.with(|state| state.replace(Some(rng))))
}

#[cfg(any(test, feature = "test-support"))]
impl Drop for SeedScope {
    fn drop(&mut self) {
        SEEDED.with(|state| state.replace(self.0.take()));
    }
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) struct ThreadRng;

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn thread_rng() -> ThreadRng {
    ThreadRng
}

#[cfg(any(test, feature = "test-support"))]
fn with_rng<T>(draw: impl FnOnce(&mut dyn ::rand::RngCore) -> T) -> T {
    SEEDED.with(|state| match state.borrow_mut().as_mut() {
        Some(rng) => draw(rng),
        None => draw(&mut ::rand::thread_rng()),
    })
}

#[cfg(any(test, feature = "test-support"))]
impl ::rand::RngCore for ThreadRng {
    fn next_u32(&mut self) -> u32 {
        with_rng(|rng| rng.next_u32())
    }
    fn next_u64(&mut self) -> u64 {
        with_rng(|rng| rng.next_u64())
    }
    fn fill_bytes(&mut self, destination: &mut [u8]) {
        with_rng(|rng| rng.fill_bytes(destination));
    }
    fn try_fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), ::rand::Error> {
        with_rng(|rng| rng.try_fill_bytes(destination))
    }
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn random<T>() -> T
where
    distributions::Standard: distributions::Distribution<T>,
{
    Rng::r#gen(&mut thread_rng())
}

/// SimC76f8118 proc_rng.hpp170–236: default-cap PRD constant for a mean rate.
/// Roll `constant * attempts_since_success` and reset attempts after success.
pub fn accumulated_proc_constant(probability: f64) -> f64 {
    capped_accumulated_proc_constant(probability, 100_000)
}

/// SimC76f8118 proc_rng.hpp201–236: the cap is part of the mean-rate solver.
pub fn capped_accumulated_proc_constant(probability: f64, cap: u32) -> f64 {
    let cap = if cap == 0 { 100_000 } else { cap.min(100_000) };
    assert!(probability.is_finite(), "proc probability must be finite");
    if probability <= 0.0 {
        return 0.0;
    }
    if probability <= 1.0 / f64::from(cap) {
        return 1e-300;
    }
    if probability >= 1.0 {
        return 1.0;
    }
    let expected = 1.0 / probability;
    let mut constant = (1.25 * probability * probability).min(0.99);
    for _ in 0..20 {
        let (attempts, derivative) = accumulated_expected_attempts(constant, cap);
        let error = attempts - expected;
        if error.abs() < 1e-12 {
            break;
        }
        constant -= error / derivative;
    }
    constant
}

fn accumulated_expected_attempts(constant: f64, cap: u32) -> (f64, f64) {
    let mut chain = 1.0;
    let mut chain_derivative = 0.0;
    let mut expected = 0.0;
    let mut derivative = 0.0;
    for attempt in 1..=cap {
        expected += chain;
        derivative += chain_derivative;
        let chance = f64::from(attempt) * constant;
        if chance >= 1.0 {
            break;
        }
        chain_derivative = chain_derivative * (1.0 - chance) - chain * f64::from(attempt);
        chain *= 1.0 - chance;
    }
    (expected, derivative)
}

#[test]
fn accumulated_proc_constants_match_expected_proc_rates() {
    // Independent survival-sum roots for SimC76f8118 proc_rng.hpp170–236.
    for (probability, expected) in [
        (0.0, 0.0),
        (0.00001, 1e-300),
        (0.01, 0.00015604169167653424),
        (0.15, 0.03222091437308769),
        (0.25, 0.08474409185231696),
        (0.5, 0.3021030253487419),
        (1.0, 1.0),
    ] {
        let actual = accumulated_proc_constant(probability);
        if expected == 0.0 || expected == 1e-300 || expected == 1.0 {
            assert_eq!(actual, expected);
        } else {
            assert!(
                (actual - expected).abs() < 1e-12,
                "p={probability}: {actual}"
            );
        }
    }
}

#[test]
fn same_seed_replays_native_random_draws() {
    let draw = |seed| {
        let _scope = seed_scope(seed);
        (0..16)
            .map(|_| (thread_rng().gen_range(0..10_000), random::<f32>()))
            .collect::<Vec<_>>()
    };
    let first = draw(34);
    assert_eq!(first, draw(34));
    assert_ne!(first, draw(35));
}

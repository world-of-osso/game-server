//! Native rand distributions; only an explicit, test-only scope supplies a seed.
#[cfg(not(test))]
pub(crate) use ::rand::*;

#[cfg(test)]
pub(crate) use ::rand::{Rng, SeedableRng, distributions, rngs, seq};

#[cfg(test)]
thread_local! {
    static SEEDED: std::cell::RefCell<Option<rngs::StdRng>> = const { std::cell::RefCell::new(None) };
}

/// Restores the caller's RNG mode when this explicit simulation scope ends.
#[cfg(test)]
pub(crate) struct SeedScope(Option<rngs::StdRng>);

#[cfg(test)]
pub(crate) fn seed_scope(seed: u64) -> SeedScope {
    let rng = rngs::StdRng::seed_from_u64(seed);
    SeedScope(SEEDED.with(|state| state.replace(Some(rng))))
}

#[cfg(test)]
impl Drop for SeedScope {
    fn drop(&mut self) {
        SEEDED.with(|state| state.replace(self.0.take()));
    }
}

#[cfg(test)]
pub(crate) struct ThreadRng;

#[cfg(test)]
pub(crate) fn thread_rng() -> ThreadRng {
    ThreadRng
}

#[cfg(test)]
fn with_rng<T>(draw: impl FnOnce(&mut dyn ::rand::RngCore) -> T) -> T {
    SEEDED.with(|state| match state.borrow_mut().as_mut() {
        Some(rng) => draw(rng),
        None => draw(&mut ::rand::thread_rng()),
    })
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn random<T>() -> T
where
    distributions::Standard: distributions::Distribution<T>,
{
    Rng::r#gen(&mut thread_rng())
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

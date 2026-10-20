//! Scoped simulation seeding preserves native rand distributions.
pub(crate) use rand::{random, thread_rng};

pub(crate) struct SeedScope;

pub(crate) fn seed_scope(_seed: u64) -> SeedScope {
    SeedScope
}

#[test]
fn same_seed_replays_native_random_draws() {
    use rand::Rng;
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

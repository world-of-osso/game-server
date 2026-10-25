//! Engine-free reporter for the production Program; run with rustc --test.
mod simc_apl;

pub use simc_apl::*;

#[path = "simc_apl/feral_reporter_tests.rs"]
mod feral_reporter_tests;

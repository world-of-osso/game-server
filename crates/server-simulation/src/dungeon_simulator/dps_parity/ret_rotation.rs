//! Literal Ret adapter qualification; native facts and execution stay shared.
use crate::simc_apl_native::Bindings;

fn bindings() -> Bindings {
    Bindings::default()
}

#[cfg(test)]
mod tests;

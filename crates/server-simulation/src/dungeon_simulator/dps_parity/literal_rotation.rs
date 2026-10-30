//! Fixture side of the production literal driver: actor capture and driver tests.
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;

pub(crate) use server::test_support::LiteralParityFixture;
pub(crate) use server_follower_apl::literal_rotation::*;
// The driver tests below read these through `use super::*`.
use equipment::Equipment;
use execution::{Execution, Prepared};
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(crate) fn capture_fixture(
    world: &mut World,
    actor: Entity,
    encounter: Encounter,
) -> Result<(), String> {
    if matches!(encounter, Encounter::Dungeon { .. }) {
        // The live follower brain keeps its owner-follow and positioning behavior.
        return Ok(());
    }
    // Literal control must not keep the retired kit's owner-follow movement.
    world
        .entity_mut(actor)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(LiteralParityFixture);
    Ok(())
}

mod channel_tests;
mod composite_tests;
mod equipment_tests;
mod gating_tests;
mod hostile_item_tests;
mod interrupt_tests;
mod precombat_poll_tests;
mod tests;
mod use_items_tests;

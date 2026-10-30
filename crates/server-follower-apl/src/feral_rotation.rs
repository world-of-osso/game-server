//! Literal pinned Feral rows over the shared native driver, for either actor identity.
pub mod bindings;
pub mod context;
pub mod driver;
pub mod facts;
pub mod geometry;
pub mod source;

use bevy::prelude::*;
pub use driver::Rotation;

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        Rotation::poll_precombat(self, world, actor, targets)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: server::simc_apl::Action,
        execution: crate::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        Rotation::start_combat_at(self, started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        Rotation::drive(self, world, actor, targets)
    }
}

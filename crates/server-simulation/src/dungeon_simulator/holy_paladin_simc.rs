//! Holy Paladin fixture policy over the production healer driver.
#[cfg(test)]
#[path = "holy_paladin_simc_tests.rs"]
mod tests;
use super::healer_simc_replay::Policy;
use crate::simc_apl_native::{Encounter, FightStyle};
use bevy::prelude::{Entity, World};
use server_follower_apl::holy_paladin_rotation::HolyPaladin;

pub(super) fn prepare(
    run: &mut super::dps_parity::Run,
    healer: Entity,
) -> Result<Box<dyn Policy>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let policy = HolyPaladin::load(run.app.world_mut(), healer, encounter)?;
    super::dps_parity::literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    Ok(Box::new(policy))
}

impl Policy for HolyPaladin {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
        // The fixture's damage target is the Protection Paladin tank's target.
        let enemy = party.iter().find_map(|&unit| {
            let spec = world.get::<shared::components::ActiveSpec>(unit)?;
            (spec.0 == 66)
                .then(|| world.get::<crate::networking::CombatTarget>(unit)?.0)
                .flatten()
        });
        HolyPaladin::drive(self, world, healer, party, enemy)
    }
}

//! Read-only authored concentration display. No grant or recharge clock is initialized.
use super::PlayerProfessions;
use crate::{
    auth::{Authenticated, InWorld},
    currency::PlayerCurrencies,
    networking::PlayerLink,
};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::{
    concentration::ConcentrationCatalog,
    protocol::{ProfessionChannel, ProfessionConcentrationRequest, ProfessionConcentrationUpdate},
};

pub(super) fn receive_queries(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<ProfessionConcentrationRequest>,
    ), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, request.skill_line))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, skill_line) in requests {
        let result = query_concentration(world, player, skill_line);
        let (snapshot, error) = match result {
            Ok(snapshot) => (snapshot, None),
            Err(error) => (None, Some(error)),
        };
        world
            .get_mut::<MessageSender<ProfessionConcentrationUpdate>>(connection)
            .expect("registered concentration response sender")
            .send::<ProfessionChannel>(ProfessionConcentrationUpdate {
                skill_line,
                snapshot,
                error,
            });
    }
}

fn query_concentration(
    world: &World,
    player: Entity,
    skill: u32,
) -> Result<Option<shared::concentration::ConcentrationSnapshot>, String> {
    let professions = world
        .get::<PlayerProfessions>(player)
        .ok_or("Profession state unavailable")?;
    if professions.rank(skill) == 0 {
        return Err("Profession skill line is not learned".into());
    }
    let catalog = world
        .get_resource::<ConcentrationCatalog>()
        .ok_or("Authored concentration catalog unavailable")?;
    let wallet = world
        .get::<PlayerCurrencies>(player)
        .ok_or("Currency wallet unavailable")?;
    Ok(catalog.snapshot(skill, &wallet.0))
}

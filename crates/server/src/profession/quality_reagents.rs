//! Non-consuming owner-only quality reagent validation. Outcome composition is unavailable.
use super::{PlayerProfessions, with_bags};
use crate::{
    auth::{Authenticated, InWorld},
    networking::PlayerLink,
};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{CraftingOperationRequest, CraftingOperationStateUpdate, ProfessionChannel};
use shared::{crafting_reagents::CraftingReagentCatalog, item_data::ItemDefinitionSource::Retail};
use std::collections::BTreeMap;

pub(super) fn receive_allocations(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<CraftingOperationRequest>,
    ), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, request) in requests {
        let validation = validate_owned_allocation(world, player, &request);
        let update = CraftingOperationStateUpdate {
            spell_id: request.spell_id,
            allocations: if validation.is_ok() {
                request.allocations
            } else {
                vec![]
            },
            validation_error: validation.err(),
            quality: shared::profession::evaluate_crafted_quality(),
        };
        world
            .get_mut::<MessageSender<CraftingOperationStateUpdate>>(connection)
            .expect("registered quality operation response sender")
            .send::<ProfessionChannel>(update);
    }
}

fn validate_owned_allocation(
    world: &World,
    player: Entity,
    request: &CraftingOperationRequest,
) -> Result<(), String> {
    let knows = world
        .get::<PlayerProfessions>(player)
        .is_some_and(|profs| profs.knows(request.spell_id));
    if !knows {
        return Err("Recipe is not learned".into());
    }
    let catalog = world
        .get_resource::<CraftingReagentCatalog>()
        .ok_or("Authored reagent catalog unavailable")?;
    with_bags(world, player, |bags| {
        let model = bags.load()?;
        let banks = bags.load_banks();
        let owned_count = |item| model.bag_count(Retail, item) + banks.count(item);
        let owned: BTreeMap<_, _> = request
            .allocations
            .iter()
            .map(|allocation| (allocation.item_id, owned_count(allocation.item_id)))
            .collect();
        catalog.validate(request.spell_id, &request.allocations, &owned)?;
        let data = world
            .get_resource::<super::ProfessionData>()
            .ok_or("Profession requirements unavailable")?;
        for &(item, needed) in data.reagents(request.spell_id) {
            if owned_count(item) < needed {
                return Err(format!("Insufficient required reagent {item}"));
            }
        }
        Ok(())
    })
    .ok_or_else(|| "Character inventory unavailable".to_string())?
}

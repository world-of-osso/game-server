//! Fill existing concrete copies before new matchmaking; named project policies in PvP spec.
use shared::battleground::{BgPhase, BgTemplate, RANDOM_BATTLEGROUND_ID, template_by_id};

use super::{InvitedBattleground, PvpServerState, random_bg};
use crate::maps::InstanceManager;

pub(super) struct Invitations {
    pub match_id: u64,
    pub characters: Vec<u64>,
}

fn compatible_template(
    matched: &InvitedBattleground,
    queue_id: u32,
    catalog: &random_bg::Catalog,
) -> Option<BgTemplate> {
    if matched.arena.is_some() || matched.instance.phase == BgPhase::Ended {
        return None;
    }
    let template =
        template_by_id(matched.instance.bg_id).expect("existing concrete battleground template");
    let compatible = if queue_id == RANDOM_BATTLEGROUND_ID {
        catalog.contains_template(template.id)
    } else {
        queue_id == template.id
    };
    compatible.then_some(template)
}

pub(super) fn fill_existing_copy(
    state: &mut PvpServerState,
    instances: &mut InstanceManager,
    catalog: &random_bg::Catalog,
    queue_id: u32,
) -> Result<Option<Invitations>, String> {
    // BG_BACKFILL_MATCH_ORDER: BTreeMap's ascending match IDs, not TC push_front order.
    for (&match_id, matched) in &mut state.battleground_matches {
        let Some(template) = compatible_template(matched, queue_id, catalog) else {
            continue;
        };
        if let Some(batch) = reserve_and_fill_copy(
            &mut state.battleground_queue,
            matched,
            instances,
            &template,
            queue_id,
            match_id,
        )? {
            return Ok(Some(batch));
        }
    }
    Ok(None)
}

fn reserve_and_fill_copy(
    queue: &mut shared::battleground::BgQueue,
    matched: &mut InvitedBattleground,
    instances: &mut InstanceManager,
    template: &BgTemplate,
    queue_id: u32,
    match_id: u64,
) -> Result<Option<Invitations>, String> {
    let mut pending_queue = queue.clone();
    let mut pending_instance = matched.instance.clone();
    let characters = pending_queue.fill_existing(queue_id, &mut pending_instance, template);
    if characters.is_empty() {
        return Ok(None);
    }
    if let Some(map) = matched.map {
        instances
            .reserve_battleground_players(map, &characters)
            .map_err(|reason| {
                format!("backfill match{match_id} queue{queue_id} reservation refused: {reason:?}")
            })?;
    }
    matched.instance = pending_instance;
    matched
        .original_queues
        .extend(characters.iter().map(|&character| (character, queue_id)));
    *queue = pending_queue;
    Ok(Some(Invitations {
        match_id,
        characters,
    }))
}

#[cfg(test)]
#[path = "pvp_bg_backfill_tests.rs"]
mod tests;

//! Profession trainers: `TrainerList` when `npc_interaction` opens the trainer
//! role, `TrainerBuySpell` for the open trainer (TrinityCore `Trainer::SendSpells`,
//! `TeachSpell`, `GetSpellState`, `CanTeachSpell`; Trainer.cpp at a352b1fa).
//!
//! The trainer is the creature entry's single `tdb_creature_trainer` row;
//! creatures with several (gossip-menu-specific) trainers are refused with a
//! warning. A purchase takes the gold in one `InventoryModel` save, casts the
//! spell when it has a `LEARN_SPELL` effect and learns it otherwise, then sends
//! the list again with the new states (TrinityCore leaves that to the client's
//! own state refresh). No reputation price discount (as `crate::merchant`).

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, Npc};
use shared::protocol::{
    NpcRole, TrainerBuyFailed, TrainerBuySpell, TrainerChannel, TrainerFailReason, TrainerList,
    TrainerService, TrainerServiceState,
};

use super::data::{Trainer, TrainerSpell};
use super::{EFFECT_LEARN_SPELL, Learner, PlayerProfessions, with_learner};
use crate::auth::InWorld;
use crate::gold::GoldUpdateQueue;
use crate::inventory::load_model;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::GameDatabase;
use crate::persistence::inventory_store::save_inventory_record;

pub(crate) struct TrainerPlugin;

impl Plugin for TrainerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (send_trainer_lists, process_trainer_buys).chain());
    }
}

/// TrinityCore `Trainer::GetSpellState`.
fn spell_state(
    learner: &Learner,
    profs: &PlayerProfessions,
    level: u8,
    spell: &TrainerSpell,
) -> TrainerServiceState {
    if profs.knows(spell.spell) {
        return TrainerServiceState::Known;
    }
    let skill_short =
        spell.req_skill_line != 0 && profs.rank(spell.req_skill_line) < spell.req_skill_rank;
    let ability_missing = spell
        .req_abilities
        .iter()
        .any(|&ability| !profs.knows(ability));
    if skill_short || ability_missing || level < spell.req_level {
        return TrainerServiceState::Unavailable;
    }
    let learned: Vec<u32> = learner
        .spells
        .get(spell.spell)
        .into_iter()
        .flat_map(|info| &info.effects)
        .filter(|effect| effect.effect == EFFECT_LEARN_SPELL)
        .map(|effect| effect.trigger_spell)
        .collect();
    if !learned.is_empty() && learned.iter().all(|&spell| profs.knows(spell)) {
        return TrainerServiceState::Known;
    }
    TrainerServiceState::Available
}

/// Services the character sees (`Trainer::SendSpells` skips spells that do not
/// fit its race and class).
pub(crate) fn trainer_services(
    learner: &Learner,
    profs: &PlayerProfessions,
    level: u8,
    trainer: &Trainer,
) -> Vec<TrainerService> {
    trainer
        .spells
        .iter()
        .filter(|spell| {
            learner
                .data
                .spell_fits(spell.spell, learner.race, learner.class)
        })
        .map(|spell| TrainerService {
            spell_id: spell.spell,
            cost: spell.cost,
            state: spell_state(learner, profs, level, spell),
            req_level: spell.req_level,
            req_skill_line: spell.req_skill_line,
            req_skill_rank: spell.req_skill_rank,
            req_abilities: spell.req_abilities.clone(),
            profession: learner.adds_primary_profession(profs, spell.spell),
        })
        .collect()
}

fn open_trainer(interaction: Option<&ActiveInteraction>) -> Option<Entity> {
    interaction
        .filter(|interaction| interaction.role == Some(NpcRole::Trainer))
        .map(|interaction| interaction.npc)
}

/// The trainer of `npc`'s creature entry.
fn trainer_for(world: &World, npc: Entity) -> Option<Trainer> {
    let entry = world.get::<Npc>(npc)?.template_id;
    let data = world.get_resource::<super::ProfessionData>()?;
    match data.trainers_for_creature(entry) {
        [id] => data.trainer(*id).cloned(),
        [] => {
            warn!("Creature {entry} has the trainer role but no tdb_creature_trainer row");
            None
        }
        ids => {
            warn!("Creature {entry} has gossip-menu-specific trainers {ids:?}; not supported");
            None
        }
    }
}

fn level_of(world: &World, player: Entity) -> u8 {
    world.get::<Level>(player).map_or(1, |level| level.0)
}

fn build_list(world: &mut World, player: Entity, npc: Entity) -> Option<TrainerList> {
    let trainer = trainer_for(world, npc)?;
    let level = level_of(world, player);
    let services = with_learner(world, player, |learner, profs| {
        trainer_services(learner, profs, level, &trainer)
    })?;
    Some(TrainerList {
        npc: npc.to_bits(),
        trainer_id: trainer.id,
        greeting: trainer.greeting,
        services,
    })
}

fn send<M: lightyear::prelude::Message>(world: &mut World, connection: Entity, message: M) {
    if let Some(mut sender) = world.get_mut::<MessageSender<M>>(connection) {
        sender.send::<TrainerChannel>(message);
    }
}

/// Opening a trainer frame sends its list (TrinityCore `SendTrainerList` when the
/// trainer gossip option is picked).
fn send_trainer_lists(world: &mut World) {
    let mut opened = world
        .query_filtered::<(Entity, &PlayerLink, &ActiveInteraction), Changed<ActiveInteraction>>();
    let opened: Vec<(Entity, Entity, Entity)> = opened
        .iter(world)
        .filter_map(|(connection, link, interaction)| {
            open_trainer(Some(interaction)).map(|npc| (connection, link.0, npc))
        })
        .collect();
    for (connection, player, npc) in opened {
        if let Some(list) = build_list(world, player, npc) {
            send(world, connection, list);
        }
    }
}

struct BuyRequest {
    connection: Entity,
    player: Entity,
    character_id: u64,
    trainer: Option<Entity>,
    request: TrainerBuySpell,
}

fn process_trainer_buys(world: &mut World) {
    let mut receivers = world.query::<(
        Entity,
        &InWorld,
        &PlayerLink,
        Option<&ActiveInteraction>,
        &mut MessageReceiver<TrainerBuySpell>,
    )>();
    let requests: Vec<BuyRequest> = receivers
        .iter_mut(world)
        .flat_map(|(connection, in_world, link, interaction, mut receiver)| {
            let trainer = open_trainer(interaction);
            let (player, character_id) = (link.0, in_world.character_id);
            receiver
                .receive()
                .map(move |request| BuyRequest {
                    connection,
                    player,
                    character_id,
                    trainer,
                    request,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    for buy in requests {
        let request = buy.request;
        match teach(world, &buy) {
            Ok(npc) => {
                if let Some(list) = build_list(world, buy.player, npc) {
                    send(world, buy.connection, list);
                }
            }
            Err(reason) => send(
                world,
                buy.connection,
                TrainerBuyFailed {
                    npc: request.npc,
                    spell_id: request.spell_id,
                    reason,
                },
            ),
        }
    }
}

/// TrinityCore `Trainer::TeachSpell`; returns the trainer NPC.
fn teach(world: &mut World, buy: &BuyRequest) -> Result<Entity, TrainerFailReason> {
    let npc = buy
        .trainer
        .filter(|npc| npc.to_bits() == buy.request.npc)
        .ok_or(TrainerFailReason::Unavailable)?;
    let trainer = trainer_for(world, npc).ok_or(TrainerFailReason::Unavailable)?;
    let spell = trainer
        .spells
        .iter()
        .find(|spell| spell.spell == buy.request.spell_id)
        .cloned()
        .ok_or(TrainerFailReason::Unavailable)?;
    let level = level_of(world, buy.player);
    let teachable = with_learner(world, buy.player, |learner, profs| {
        learner
            .data
            .spell_fits(spell.spell, learner.race, learner.class)
            && spell_state(learner, profs, level, &spell) == TrainerServiceState::Available
            && !learner.blocked_by_primary_limit(profs, spell.spell)
    });
    if teachable != Some(true) {
        return Err(TrainerFailReason::Unavailable);
    }
    pay(world, buy.character_id, spell.cost)?;
    let castable = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(spell.spell))
        .is_some_and(|info| {
            info.effects
                .iter()
                .any(|effect| effect.effect == EFFECT_LEARN_SPELL)
        });
    if castable {
        crate::spell_cast::cast_triggered(world, buy.player, spell.spell, Some(buy.player), 0);
    } else {
        with_learner(world, buy.player, |learner, profs| {
            learner.learn_spell(profs, spell.spell, 0)
        });
    }
    Ok(npc)
}

/// Take `cost` copper in one inventory save (`Player::ModifyMoney`).
fn pay(world: &mut World, character_id: u64, cost: u64) -> Result<(), TrainerFailReason> {
    let (Some(db), Some(templates)) = (
        world.get_resource::<GameDatabase>(),
        world.get_resource::<ItemTemplates>(),
    ) else {
        return Err(TrainerFailReason::Unavailable);
    };
    let mut model =
        load_model(&db.db, templates, character_id).map_err(|_| TrainerFailReason::Unavailable)?;
    if model.gold() < cost {
        return Err(TrainerFailReason::NotEnoughMoney);
    }
    model.set_gold(model.gold() - cost);
    let gold = model.gold();
    save_inventory_record(&db.db, character_id, &model.into_record());
    world
        .resource_mut::<GoldUpdateQueue>()
        .record(character_id, gold);
    Ok(())
}

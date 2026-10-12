//! TC a352b1fa spell_item.cpp dice/compass scripts and Player::DoRandomRoll.
//! BroadcastText26147/26152 and GlobalStrings RANDOM_ROLL_RESULT are authored text.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::{Player, Position, Rotation};
use shared::protocol::{ChatMessage, ChatType};

use crate::group::GroupRegistry;
use crate::maps::InMap;
use crate::persistence::ClientId;
use crate::scripted_ai::creature_text::send_chat;
use crate::spell_info::{SpellEffect, SpellInfo};

use super::CastFailure;
use super::effects::{CastEffectHandler, EffectCtx};

/// TC worldserver.conf.dist ListenRange.TextEmote, not the ordinary /say range.
const TEXT_EMOTE_RANGE: f32 = 40.0;
const UNUSUAL_COMPASS: u32 = 64385;
const EFFECT_DUMMY: u32 = 3;
const EFFECT_SCRIPT: u32 = 77;

#[derive(Resource)]
pub(crate) struct DiceTexts(HashMap<u32, (String, String)>);

pub(crate) fn load_dice_texts(conn: &Connection) -> rusqlite::Result<DiceTexts> {
    let mut statement = conn.prepare(
        "SELECT ID, MaleText, FemaleText FROM content_broadcast_text WHERE ID IN (26147, 26152)",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map(DiceTexts)
}

#[derive(Clone, Copy)]
struct DicePlan {
    text_id: u32,
    maximum: u32,
    rolls: usize,
}

fn dice_plan(spell: u32) -> Option<DicePlan> {
    match spell {
        47770 => Some(DicePlan {
            text_id: 26147,
            maximum: 100,
            rolls: 1,
        }),
        47776 => Some(DicePlan {
            text_id: 26152,
            maximum: 6,
            rolls: 2,
        }),
        _ => None,
    }
}

fn is_caster_script_effect(effect: &SpellEffect, effect_type: u32) -> bool {
    let caster_script = (
        effect.index,
        effect.effect,
        effect.aura,
        effect.implicit_targets,
    ) == (0, effect_type, 0, [1, 0]);
    let no_trigger_payload = effect.trigger_spell == 0 && effect.misc_values == [0, 0];
    let no_gameplay_payload =
        effect.base_points == 0.0 && effect.item_type == 0 && effect.mechanic == 0;
    caster_script && no_trigger_payload && no_gameplay_payload
}

fn has_source_signature(spell: &SpellInfo, effect_type: u32) -> bool {
    let mut effects = spell.effects.iter().filter(|effect| effect.effect != 0);
    let source_effect = effects
        .next()
        .is_some_and(|effect| is_caster_script_effect(effect, effect_type));
    source_effect && effects.next().is_none()
}

pub(crate) fn unsupported(world: &World, spell: &SpellInfo) -> Option<String> {
    if spell.id == UNUSUAL_COMPASS {
        return (!has_source_signature(spell, EFFECT_DUMMY))
            .then(|| "compass source effect signature differs".into());
    }
    let plan = dice_plan(spell.id)?;
    if !has_source_signature(spell, EFFECT_SCRIPT) {
        return Some("dice source effect signature differs".into());
    }
    if !world.contains_resource::<GroupRegistry>() {
        return Some("dice group authority is unavailable".into());
    }
    let text = world
        .get_resource::<DiceTexts>()
        .and_then(|texts| texts.0.get(&plan.text_id));
    match text {
        Some((male, female)) if !male.is_empty() && !female.is_empty() => None,
        _ => Some(format!(
            "dice BroadcastText {} is unavailable",
            plan.text_id
        )),
    }
}

pub(crate) fn supports_effect(spell: u32, effect: &SpellEffect) -> bool {
    if spell == UNUSUAL_COMPASS {
        return is_caster_script_effect(effect, EFFECT_DUMMY);
    }
    dice_plan(spell).is_some() && is_caster_script_effect(effect, EFFECT_SCRIPT)
}

pub(crate) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if spell.id == UNUSUAL_COMPASS {
        return check_compass_cast(world, caster, spell);
    }
    if dice_plan(spell.id).is_none() {
        return Ok(());
    }
    if let Some(reason) = unsupported(world, spell) {
        return Err(CastFailure::Unsupported(reason));
    }
    let player_identity =
        world.get::<Player>(caster).is_some() && world.get::<ClientId>(caster).is_some();
    let located_in_world =
        world.get::<Position>(caster).is_some() && world.get::<InMap>(caster).is_some();
    if player_identity && located_in_world {
        Ok(())
    } else {
        Err(CastFailure::InvalidTarget)
    }
}

pub(super) fn cast_effect(spell: u32, index: u8) -> Option<CastEffectHandler> {
    if spell == UNUSUAL_COMPASS && index == 0 {
        return Some(turn_compass_caster_randomly);
    }
    (dice_plan(spell).is_some() && index == 0)
        .then_some(emit_dice_emote_and_rolls as CastEffectHandler)
}

fn check_compass_cast(world: &World, caster: Entity, spell: &SpellInfo) -> Result<(), CastFailure> {
    if let Some(reason) = unsupported(world, spell) {
        return Err(CastFailure::Unsupported(reason));
    }
    let can_adopt_facing = world.get::<Rotation>(caster).is_some()
        && world
            .get::<shared::components::MovementControl>(caster)
            .is_some();
    if can_adopt_facing {
        Ok(())
    } else {
        Err(CastFailure::InvalidTarget)
    }
}

fn turn_compass_caster_randomly(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let orientation = rand::Rng::gen_range(&mut rand::thread_rng(), 0.0..=std::f32::consts::TAU);
    let yaw = orientation + std::f32::consts::FRAC_PI_2;
    crate::networking::face_player(world, ctx.caster, yaw);
}

fn emit_dice_emote_and_rolls(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let plan = dice_plan(ctx.spell.id).expect("registered dice script");
    let emote = read_dice_emote(world, ctx.caster, plan.text_id);
    for recipient in emote_hearers(world, ctx.caster, &emote.sender) {
        send_chat(world, recipient, emote.clone());
    }
    emit_dice_rolls(world, ctx.caster, &emote.sender, plan);
}

fn read_dice_emote(world: &World, caster: Entity, text_id: u32) -> ChatMessage {
    let player = world.get::<Player>(caster).expect("validated dice player");
    let female = player.appearance.sex == 1;
    let texts = &world.resource::<DiceTexts>().0[&text_id];
    let content = if female {
        texts.1.clone()
    } else {
        texts.0.clone()
    };
    ChatMessage {
        sender: player.name.clone(),
        content,
        channel: ChatType::Emote,
    }
}

fn emit_dice_rolls(world: &mut World, caster: Entity, name: &str, plan: DicePlan) {
    let recipients = roll_recipients(world, caster);
    for _ in 0..plan.rolls {
        let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 1..=plan.maximum);
        // Retail69933 GlobalStrings11691: "%s rolls %d (%d-%d)".
        let message = ChatMessage {
            sender: name.to_string(),
            content: format!("{name} rolls {roll} (1-{})", plan.maximum),
            channel: ChatType::System,
        };
        for &recipient in &recipients {
            send_chat(world, recipient, message.clone());
        }
    }
}

fn emote_hearers(world: &mut World, caster: Entity, name: &str) -> Vec<Entity> {
    let origin = *world
        .get::<Position>(caster)
        .expect("validated dice position");
    let map = *world.get::<InMap>(caster).expect("validated dice map");
    let origin = Vec3::new(origin.x, origin.y, origin.z);
    world
        .query_filtered::<(
            Entity,
            &Position,
            &InMap,
            Option<&crate::ignore_list::PlayerIgnoreListState>,
        ), With<Player>>()
        .iter(world)
        .filter(|listener| hears_dice_emote(caster, origin, map, name, *listener))
        .map(|(player, ..)| player)
        .collect()
}

fn hears_dice_emote(
    caster: Entity,
    origin: Vec3,
    map: InMap,
    name: &str,
    listener: (
        Entity,
        &Position,
        &InMap,
        Option<&crate::ignore_list::PlayerIgnoreListState>,
    ),
) -> bool {
    let (player, position, candidate_map, ignored) = listener;
    let location = Vec3::new(position.x, position.y, position.z);
    let in_range = origin.distance_squared(location) <= TEXT_EMOTE_RANGE.powi(2);
    let ignores_sender = ignored.is_some_and(|state| {
        state
            .names
            .iter()
            .any(|ignored| ignored.eq_ignore_ascii_case(name))
    });
    let hears_sender = player == caster || !ignores_sender;
    *candidate_map == map && in_range && hears_sender
}

fn roll_recipients(world: &mut World, caster: Entity) -> Vec<Entity> {
    let character = world
        .get::<ClientId>(caster)
        .expect("validated dice character")
        .0;
    let members = world.resource::<GroupRegistry>().group_members(character);
    if members.is_empty() {
        return vec![caster];
    }
    let members: HashSet<_> = members.into_iter().collect();
    world
        .query_filtered::<(Entity, &ClientId), With<Player>>()
        .iter(world)
        .filter(|(_, id)| members.contains(&id.0))
        .map(|(player, _)| player)
        .collect()
}

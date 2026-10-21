//! TC a352b1fa spell_item.cpp dice/compass/coin/weather/wormhole and spell_generic.cpp
//! foam sword scripts, and Player::DoRandomRoll.
//! BroadcastText26147/26152/32638/32663/32664 and GlobalStrings RANDOM_ROLL_RESULT are authored text.
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
use super::toy_coin::{self, COIN_TEXTS, COIN_TOSS};
use super::toy_foam_sword::{self, CREATE_FOAM_SWORD};
use super::toy_weather::{self, WEATHER_MACHINE};
use super::toy_wormhole::{self, WORMHOLE_PANDARIA};

/// TC worldserver.conf.dist ListenRange.TextEmote, not the ordinary /say range.
const TEXT_EMOTE_RANGE: f32 = 40.0;
const UNUSUAL_COMPASS: u32 = 64385;
const EFFECT_DUMMY: u32 = 3;
const EFFECT_SCRIPT: u32 = 77;

#[derive(Resource)]
pub(crate) struct ToyTexts(HashMap<u32, (String, String)>);

pub(crate) fn load_toy_texts(conn: &Connection) -> rusqlite::Result<ToyTexts> {
    let mut statement = conn.prepare(
        "SELECT ID, MaleText, FemaleText FROM content_broadcast_text
        WHERE ID IN (26147, 26152, 28008, 32638, 32663, 32664)",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map(ToyTexts)
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

/// The exact first-effect type/targets/trigger with no misc or gameplay payload.
pub(super) fn has_effect_shape(
    effect: &SpellEffect,
    effect_type: u32,
    targets: [u32; 2],
    trigger_spell: u32,
) -> bool {
    let shape = (
        effect.index,
        effect.effect,
        effect.aura,
        effect.implicit_targets,
        effect.trigger_spell,
    ) == (0, effect_type, 0, targets, trigger_spell);
    let no_misc_payload = effect.misc_values == [0, 0];
    let no_gameplay_payload =
        effect.base_points == 0.0 && effect.item_type == 0 && effect.mechanic == 0;
    shape && no_misc_payload && no_gameplay_payload
}

fn is_caster_script_effect(effect: &SpellEffect, effect_type: u32) -> bool {
    has_effect_shape(effect, effect_type, [1, 0], 0)
}

/// The spell's only nonzero effect.
pub(super) fn single_effect(spell: &SpellInfo) -> Option<&SpellEffect> {
    let mut effects = spell.effects.iter().filter(|effect| effect.effect != 0);
    let first = effects.next();
    effects.next().is_none().then_some(first).flatten()
}

fn has_source_signature(spell: &SpellInfo, effect_type: u32) -> bool {
    single_effect(spell).is_some_and(|effect| is_caster_script_effect(effect, effect_type))
}

pub(super) fn texts_available(world: &World, ids: &[u32]) -> bool {
    let Some(texts) = world.get_resource::<ToyTexts>() else {
        return false;
    };
    ids.iter().all(|id| {
        texts
            .0
            .get(id)
            .is_some_and(|(male, female)| !male.is_empty() && !female.is_empty())
    })
}

/// The coin script resolves its own triggered catch, not the generic area-entry target.
pub(crate) fn owns_triggered_spell(spell: u32) -> bool {
    spell == COIN_TOSS
}

fn unsupported_coin(world: &World, spell: &SpellInfo) -> Option<String> {
    if !toy_coin::has_source_signature(world, spell) {
        return Some("coin source effect signature differs".into());
    }
    (!texts_available(world, &COIN_TEXTS)).then(|| "coin BroadcastText is unavailable".into())
}

pub(crate) fn unsupported(world: &World, spell: &SpellInfo) -> Option<String> {
    if spell.id == super::toy_flag::TARGETING {
        return super::toy_flag::unsupported(world, spell);
    }
    if spell.id == COIN_TOSS {
        return unsupported_coin(world, spell);
    }
    if spell.id == WEATHER_MACHINE {
        if !has_source_signature(spell, EFFECT_SCRIPT) {
            return Some("weather machine source effect signature differs".into());
        }
        return toy_weather::unsupported(world);
    }
    if spell.id == CREATE_FOAM_SWORD {
        return toy_foam_sword::unsupported(world, spell);
    }
    if spell.id == WORMHOLE_PANDARIA {
        if !has_source_signature(spell, EFFECT_DUMMY) {
            return Some("wormhole source effect signature differs".into());
        }
        return toy_wormhole::unsupported(world);
    }
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
    (!texts_available(world, &[plan.text_id]))
        .then(|| format!("dice BroadcastText {} is unavailable", plan.text_id))
}

pub(crate) fn supports_effect(spell: u32, effect: &SpellEffect) -> bool {
    if spell == super::toy_flag::TARGETING {
        return has_effect_shape(effect, EFFECT_DUMMY, [22, 93], 0);
    }
    if spell == UNUSUAL_COMPASS || spell == WORMHOLE_PANDARIA {
        return is_caster_script_effect(effect, EFFECT_DUMMY);
    }
    if spell == COIN_TOSS {
        return toy_coin::is_toss_effect(effect);
    }
    if spell == CREATE_FOAM_SWORD {
        return toy_foam_sword::is_script_effect(effect);
    }
    if spell == WEATHER_MACHINE {
        return is_caster_script_effect(effect, EFFECT_SCRIPT);
    }
    dice_plan(spell).is_some() && is_caster_script_effect(effect, EFFECT_SCRIPT)
}

pub(crate) fn check_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if spell.id == super::toy_flag::TARGETING {
        return super::toy_flag::check_cast(world, caster, spell);
    }
    if spell.id == UNUSUAL_COMPASS {
        return check_compass_cast(world, caster, spell);
    }
    if [WEATHER_MACHINE, WORMHOLE_PANDARIA, CREATE_FOAM_SWORD].contains(&spell.id) {
        return unsupported(world, spell)
            .map_or(Ok(()), |reason| Err(CastFailure::Unsupported(reason)));
    }
    let text_script = dice_plan(spell.id).is_some() || spell.id == COIN_TOSS;
    if !text_script {
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
    if spell == COIN_TOSS && index == 0 {
        return Some(toy_coin::toss_coin);
    }
    if spell == WEATHER_MACHINE && index == 0 {
        return Some(toy_weather::cast_personal_weather);
    }
    if spell == CREATE_FOAM_SWORD && index == 0 {
        return Some(toy_foam_sword::create_one_foam_sword);
    }
    if spell == WORMHOLE_PANDARIA && index == 0 {
        return Some(toy_wormhole::teleport_to_random_pandaria_point);
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
    let emote = send_text_emote(world, ctx.caster, plan.text_id);
    emit_dice_rolls(world, ctx.caster, &emote.sender, plan);
}

/// `Unit::TextEmote(text, caster)`: the caster's gendered text to its TextEmote hearers.
pub(super) fn send_text_emote(world: &mut World, caster: Entity, text_id: u32) -> ChatMessage {
    let emote = read_text_emote(world, caster, text_id);
    for recipient in emote_hearers(world, caster, &emote.sender) {
        send_chat(world, recipient, emote.clone());
    }
    emote
}

pub(super) fn send_targeted_text_emote(
    world: &mut World,
    caster: Entity,
    target: Entity,
    text_id: u32,
) {
    let mut emote = read_text_emote(world, caster, text_id);
    let target_name = &world
        .get::<Player>(target)
        .expect("selected corpse owner")
        .name;
    emote.content = emote
        .content
        .replace("%s", &emote.sender)
        .replace("$n", target_name);
    for recipient in emote_hearers(world, caster, &emote.sender) {
        send_chat(world, recipient, emote.clone());
    }
}

fn read_text_emote(world: &World, caster: Entity, text_id: u32) -> ChatMessage {
    let player = world
        .get::<Player>(caster)
        .expect("validated text-emote player");
    let female = player.appearance.sex == 1;
    let texts = &world.resource::<ToyTexts>().0[&text_id];
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

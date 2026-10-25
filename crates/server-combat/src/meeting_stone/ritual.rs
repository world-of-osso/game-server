//! The summoning portal: a `GAMEOBJECT_TYPE_RITUAL` object a channel spawns
//! (TrinityCore `Spell::EffectTransmitted`, SpellEffects.cpp:4474-4560) and
//! other party members click (`GameObject::Use`, GameObject.cpp:3053-3150).
//!
//! `SPELL_EFFECT_TRANS_DOOR` spawns the template of its MiscValue on the
//! caster, facing its way, with the caster as first participant. The object
//! belongs to the caster's channel of that spell: it goes when the channel
//! ends (`Unit::AddGameObject`, "will be removed at spell cancel") and at the
//! latest after the spell's duration (`SetRespawnTime`).
//!
//! A click by another player joins the ritual when that player shares the
//! owner's party or raid (for grouped rituals) and the owner is still
//! channeling. Once `casters` distinct players took part, the owner's channel
//! finishes, which removes the portal (no spell-spawned ritual is persistent),
//! and the owner casts the ritual's spell triggered, i.e. without cast time
//! or reported errors. The ritual's `animSpell` (32783 Arcane Channeling, a
//! self-channel visual on each clicker) is not cast.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::{Position, Rotation};

use super::MeetingStoneData;
use crate::interest::NetworkVisibility;
use crate::networking::CombatTarget;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_cast::{cast_triggered, duration_ms, end_cast, now};
use crate::spell_info::SpellEffect;

/// A ritual in progress on its object.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct Ritual {
    pub owner: Entity,
    /// The owner's channel holding the object.
    pub channel_spell: u32,
    pub casters: u32,
    pub spell: u32,
    pub grouped: bool,
    /// Distinct participants, the owner first (`AddUniqueUse`).
    pub users: Vec<Entity>,
    pub expires_at: f64,
}

/// `SPELL_EFFECT_TRANS_DOOR` for ritual templates, once per cast.
pub(crate) fn effect_trans_door(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let entry = effect.misc_values[0] as u32;
    let Some(template) = world.resource::<MeetingStoneData>().ritual(entry).cloned() else {
        warn!(
            "Spell {}: TRANS_DOOR object {entry} is not a ritual, not spawned",
            ctx.spell.id
        );
        return;
    };
    let caster = ctx.caster;
    let (Some(position), Some(rotation), Some(map)) = (
        world.get::<Position>(caster).copied(),
        world.get::<Rotation>(caster).copied(),
        world.get::<crate::maps::InMap>(caster).copied(),
    ) else {
        return;
    };
    let expires_at = now(world) + duration_ms(world, caster, ctx.spell).max(0) as f64 / 1000.0;
    let object = world
        .spawn((
            position,
            rotation,
            map,
            template.bounds(rotation.y - FRAC_PI_2),
            template.info.clone(),
            // EffectTransmitted creates the ritual portal GO_STATE_READY.
            shared::protocol::GameObjectCollision(true),
            Ritual {
                owner: caster,
                channel_spell: ctx.spell.id,
                casters: template.casters,
                spell: template.spell,
                grouped: template.grouped,
                users: vec![caster],
                expires_at,
            },
            Replicate::to_clients(NetworkTarget::All),
            NetworkVisibility::default(),
        ))
        .id();
    info!(
        "Spell {}: ritual {entry} {object} opened by {caster}",
        ctx.spell.id
    );
}

/// The owner's channel of `spell_id` ended: remove the objects it held.
pub(crate) fn channel_ended(world: &mut World, owner: Entity, spell_id: u32) {
    let mut rituals = world.query::<(Entity, &Ritual)>();
    let ended: Vec<Entity> = rituals
        .iter(world)
        .filter(|(_, ritual)| ritual.owner == owner && ritual.channel_spell == spell_id)
        .map(|(entity, _)| entity)
        .collect();
    for object in ended {
        world.despawn(object);
    }
}

pub(crate) fn expire_rituals(world: &mut World) {
    let now = now(world);
    let mut rituals = world.query::<(Entity, &Ritual)>();
    let expired: Vec<Entity> = rituals
        .iter(world)
        .filter(|(_, ritual)| ritual.expires_at <= now)
        .map(|(entity, _)| entity)
        .collect();
    for object in expired {
        world.despawn(object);
    }
}

fn channeling(world: &World, owner: Entity, spell_id: u32) -> bool {
    world
        .get::<CastState>(owner)
        .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == spell_id)
}

pub(crate) fn use_ritual(world: &mut World, user: Entity, object: Entity) {
    let Some(ritual) = world.get::<Ritual>(object).cloned() else {
        return;
    };
    if user == ritual.owner
        || (ritual.grouped && !super::same_group(world, user, ritual.owner))
        || !channeling(world, ritual.owner, ritual.channel_spell)
    {
        return;
    }
    let mut users = ritual.users;
    if !users.contains(&user) {
        users.push(user);
    }
    if (users.len() as u32) < ritual.casters {
        if let Some(mut stored) = world.get_mut::<Ritual>(object) {
            stored.users = users;
        }
        return;
    }
    // The summoning channel hands over to the summon spell: no failure is reported.
    end_cast(world, ritual.owner);
    let target = world
        .get::<CombatTarget>(ritual.owner)
        .and_then(|target| target.0);
    cast_triggered(world, ritual.owner, ritual.spell, target, 0);
}

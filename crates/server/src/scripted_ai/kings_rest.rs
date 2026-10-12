//! Partial Golden Serpent companion port from TC a352b1fa boss_golden_serpent.cpp.
//! Primary intro/events, follow/absorption and molten area triggers remain gaps.
use super::{CreatureAi, Ctx, DamageTaken};
use bevy::prelude::*;
use shared::components::UnitFlags;

pub(crate) const ANIMATED_GOLD_SCRIPT: &str = "npc_animated_gold";
const INSTANCE: &str = "instance_kings_rest";
const MOLTEN_GOLD: u32 = 265915;
const BREAK_TARGETING: u32 = 140562;
const SPIT_GOLD: u32 = 265773;
const ANIMATED_GOLD: u32 = 135406;

pub(crate) fn is_creature_script(name: &str) -> bool {
    name == ANIMATED_GOLD_SCRIPT
}

pub(crate) fn creature_ai(world: &World, actor: Entity, name: &str) -> Option<Box<dyn CreatureAi>> {
    if !is_creature_script(name) {
        return None;
    }
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if maps.instance_scripts.get(&map.map_id).map(String::as_str) != Some(INSTANCE) {
        return None;
    }
    Some(Box::new(AnimatedGold::default()))
}

/// Source DamageTaken's pure loss adjustment, before any health/death/loot commit.
/// Unlike Unit::SetUnkillable, this script also intercepts self-inflicted loss.
pub(crate) fn limit_incoming_loss(health: f32, loss: f32) -> f32 {
    if loss >= health {
        (health - 1.0).max(0.0)
    } else {
        loss
    }
}

#[derive(Default)]
struct AnimatedGold {
    molten_cast: bool,
}

impl CreatureAi for AnimatedGold {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_self(BREAK_TARGETING);
        ctx.set_react_state(super::ReactState::Passive);
        ctx.world
            .entity_mut(ctx.me)
            .insert(super::ScriptedVictimUpdatePaused);
    }
    fn damage_taken(&mut self, ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if damage.damage >= damage.health_before && !self.molten_cast {
            ctx.do_cast_self(MOLTEN_GOLD);
            self.molten_cast = true;
        }
    }
    // TC companion UpdateAI only advances its task scheduler, never UpdateVictim.
    fn update_ai(&mut self, _ctx: &mut Ctx, _diff_ms: u32) {}
}

pub(crate) fn register_aura_scripts(mut scripts: ResMut<crate::auras::scripts::AuraScripts>) {
    scripts.0.insert(
        SPIT_GOLD,
        crate::auras::scripts::AuraScript {
            on_remove: Some(summon_gold_after_spit),
            ..Default::default()
        },
    );
    scripts.0.insert(
        MOLTEN_GOLD,
        crate::auras::scripts::AuraScript {
            on_apply: Some(apply_molten_immunity),
            on_remove: Some(remove_molten_immunity),
            ..Default::default()
        },
    );
}

/// TC Spit Gold effect0/periodic-damage AfterEffectRemove: only death/expiry.
fn summon_gold_after_spit(
    world: &mut World,
    target: Entity,
    aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    if !matches!(
        reason,
        crate::auras::RemoveReason::Expired | crate::auras::RemoveReason::Death
    ) {
        return;
    }
    let periodic = aura.effects.iter().any(|effect| {
        effect.index == 0 && effect.aura_type == crate::auras::handlers::PERIODIC_DAMAGE
    });
    if !periodic || world.get_entity(aura.caster).is_err() {
        return;
    }
    let position = world
        .get::<shared::components::Position>(target)
        .expect("Spit Gold target position");
    let orientation = world
        .get::<shared::components::Rotation>(target)
        .map_or(0.0, |rotation| rotation.y - std::f32::consts::FRAC_PI_2);
    let location = [position.x, -position.z, position.y, orientation];
    crate::zones::summon_creature(world, aura.caster, ANIMATED_GOLD, location);
}

fn has_transform(aura: &crate::auras::AuraInstance) -> bool {
    aura.effects
        .iter()
        .any(|effect| effect.index == 1 && effect.aura_type == 56)
}

fn apply_molten_immunity(world: &mut World, actor: Entity, aura: &crate::auras::AuraInstance) {
    if has_transform(aura) {
        set_molten_flags(world, actor, true);
    }
}
fn remove_molten_immunity(
    world: &mut World,
    actor: Entity,
    aura: &crate::auras::AuraInstance,
    _reason: crate::auras::RemoveReason,
) {
    if has_transform(aura) {
        set_molten_flags(world, actor, false);
    }
}
fn set_molten_flags(world: &mut World, actor: Entity, apply: bool) {
    let mask = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;
    let mut flags = world
        .get::<UnitFlags>(actor)
        .copied()
        .unwrap_or(UnitFlags(0));
    if apply {
        flags.0 |= mask;
    } else {
        flags.0 &= !mask;
    }
    world.entity_mut(actor).insert(flags);
}

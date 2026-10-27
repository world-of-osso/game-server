//! SimC76f8118 sc_rogue.cpp3627–3665/8229–8249/8668–8701:
//! earned Ancient Arts clones snapshot the finisher and strike after 500 ms.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{ActiveAuras, handlers, spell_mods};
use combat::random;
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::{self, SpellScriptHook},
    scripts::script_effect_value,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const TALENT: u32 = 1268932;

#[derive(Component, Default)]
struct PendingClones(Vec<Clone>);
struct Clone {
    target: Entity,
    spell: SpellInfo,
    crit: f32,
    due: f64,
}
#[derive(Component)]
struct ActiveClone {
    spell: u32,
    crit: f32,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterQueuedTriggers(launch_due),
    SpellScriptHook::CriticalChance(snapshot_crit),
];

pub(super) fn schedule(world: &mut World, ctx: &EffectCtx, target: Entity, consumed: u8) {
    let child = match ctx.spell.id {
        196819 => 1269205,
        319175 => 1269565,
        280719 => 1269560,
        _ => return,
    };
    let points = ctx
        .spell
        .cast_policy_context
        .as_ref()
        .expect("finisher resource snapshot")
        .resources
        .iter()
        .find(|(power, _)| *power == PowerType::ComboPoints)
        .expect("finisher combo-point snapshot")
        .1;
    queue_clone(world, ctx.caster, target, child, consumed, points, 0.5);
}

// sc_rogue.cpp8175–8194: builder collection triggers its own 200 ms clone.
pub(super) fn schedule_builder(
    world: &mut World,
    caster: Entity,
    source: &SpellInfo,
    consumed: u8,
) {
    let child = match source.id {
        53 => 1269209,
        200758 => 1269214,
        185438 => 1269336,
        197835 => 1269340,
        426591 => 1269217,
        _ => return,
    };
    let target = source
        .cast_primary_target
        .expect("native builder primary target");
    queue_clone(world, caster, target, child, consumed, 1.0, 0.2);
}

fn queue_clone(
    world: &mut World,
    caster: Entity,
    target: Entity,
    child: u32,
    consumed: u8,
    points: f32,
    delay: f64,
) {
    if !has_talent(world, caster, TALENT) {
        return;
    }
    let chance = script_effect_value(world, caster, TALENT, 0) * f32::from(consumed) / 100.0;
    if random::random::<f32>() >= chance {
        return;
    }
    let mut spell = world
        .resource::<SpellStore>()
        .get(child)
        .expect("native shadow clone")
        .clone();
    spell.cast_damage_multiplier = Some(points * 0.5);
    spell.cast_charge_mods = Some(spell_mods::snapshot_charged_modifiers(
        world, caster, &spell,
    ));
    let crit = effects::crit_chance(world, caster, target, &spell);
    let due = spell_cast::now(world) + delay;
    if world.get::<PendingClones>(caster).is_none() {
        world.entity_mut(caster).insert(PendingClones::default());
    }
    world
        .get_mut::<PendingClones>(caster)
        .unwrap()
        .0
        .push(Clone {
            target,
            spell,
            crit,
            due,
        });
}

fn launch_due(world: &mut World) {
    let now = spell_cast::now(world);
    let mut due = Vec::new();
    for (caster, mut queue) in world
        .query::<(Entity, &mut PendingClones)>()
        .iter_mut(world)
    {
        let mut i = 0;
        while i < queue.0.len() {
            if queue.0[i].due <= now {
                due.push((caster, queue.0.remove(i)));
            } else {
                i += 1;
            }
        }
    }
    for (caster, clone) in due {
        if world.get_entity(clone.target).is_err() {
            continue;
        }
        world.entity_mut(caster).insert(ActiveClone {
            spell: clone.spell.id,
            crit: clone.crit,
        });
        spell_cast::launch_triggered_spell(world, caster, &clone.spell, Some(clone.target), 1);
        world.entity_mut(caster).remove::<ActiveClone>();
    }
}

fn snapshot_crit(world: &World, caster: Entity, target: Entity, spell: u32) -> Option<f32> {
    let clone = world.get::<ActiveClone>(caster)?;
    if clone.spell != spell {
        return None;
    }
    let target_modifier = world.get::<ActiveAuras>(target).map_or(0.0, |auras| {
        auras.modifier(handlers::MOD_ATTACKER_MELEE_CRIT_CHANCE)
    });
    let native = world
        .resource::<SpellStore>()
        .get(spell)
        .expect("native shadow clone");
    let guaranteed = if script_registry::guarantees_crit(world, caster, native) {
        100.0
    } else {
        0.0
    };
    Some(clone.crit - target_modifier - guaranteed)
}

//! SimC76f8118 Demonbolt-earned Doom and Hand of Gul'dan expiry.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{DeclaredReach, ScriptReach, SpellScriptHook},
    targets,
};
use combat::spell_info::SpellInfo;

const CORE: u32 = 264173;
const DEMONBOLT: u32 = 264178;
const TALENT: u32 = 460551;
const DOOM: u32 = 460553;
const DAMAGE: u32 = 460555;
const HAND_OF_GULDAN: u32 = 105174;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::BeforeDamageSnapshot(snapshot_core),
    SpellScriptHook::ResolvedDamage(apply_doom),
    SpellScriptHook::AfterForegroundSpecial(shorten_doom),
];

fn declared_reach(world: &World, caster: Entity, spell: &SpellInfo) -> ScriptReach {
    if !combat::spell_cast::script_registry::reach_source_selected(world, caster, TALENT) {
        return ScriptReach::None;
    }
    if spell.id == DEMONBOLT {
        return ScriptReach::Child(DAMAGE);
    }
    if spell.id != HAND_OF_GULDAN {
        return ScriptReach::None;
    }
    let Some(origin) = targets::position(world, caster) else {
        return ScriptReach::None;
    };
    let radius = world
        .resource::<combat::spell_info::SpellStore>()
        .get(DAMAGE)
        .and_then(|damage| damage.effects.iter().find(|effect| effect.index == 0))
        .map(|effect| targets::radius(effect, 1))
        .expect("native Doom damage radius");
    let map = world.get::<combat::maps::InMap>(caster);
    let maximum = world
        .iter_entities()
        .filter(|unit| unit.get::<combat::maps::InMap>() == map)
        .filter(|unit| {
            unit.get::<ActiveAuras>().is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.caster == caster && aura.spell_id == DOOM)
            })
        })
        .filter_map(|unit| targets::position(world, unit.id()))
        .map(|position| origin.distance(position) + radius)
        .reduce(f32::max);
    maximum.map_or(ScriptReach::None, |caster_radius| {
        ScriptReach::Area(DeclaredReach {
            caster_radius,
            target_radius: 0.0,
            ..Default::default()
        })
    })
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(DOOM).or_default().on_natural_end = Some(explode);
}

fn snapshot_core(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != DEMONBOLT {
        return;
    }
    // Native effect2 unconditionally triggers Doom. SimC only admits it
    // after a landed Core-powered Demonbolt and does not refresh active Doom.
    if let Some(effect) = spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == 2 && effect.trigger_spell == DOOM)
    {
        effect.effect = 0;
    }
    if auras::has_aura(world, caster, CORE) {
        spell.cast_launch_auras.push(CORE);
    }
}

fn apply_doom(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if ctx.depth != 0
        || ctx.spell.id != DEMONBOLT
        || damage <= 0.0
        || !ctx.spell.cast_launch_auras.contains(&CORE)
        || !auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    let present = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == DOOM && aura.caster == ctx.caster)
    });
    if !present {
        spell_cast::cast_triggered(world, ctx.caster, DOOM, Some(target), 1);
    }
}

fn explode(world: &mut World, target: Entity, aura: &AuraInstance) {
    spell_cast::cast_triggered(world, aura.caster, DAMAGE, Some(target), 1);
}

fn shorten_doom(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != HAND_OF_GULDAN || !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let now = auras::aura_now(world);
    let reduction = f64::from(spell_cast::scripts::script_effect_value(
        world, caster, TALENT, 0,
    )) / 1000.0;
    let targets: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .flat_map(|(target, auras)| {
            auras
                .iter()
                .filter(move |aura| aura.caster == caster && aura.spell_id == DOOM)
                .filter_map(move |aura| {
                    aura.expires_at
                        .map(|expiry| (target, aura.instance_id, expiry - now))
                })
        })
        .collect();
    for (target, instance, remaining) in targets {
        if remaining <= reduction {
            auras::remove_instance(world, target, instance, RemoveReason::Expired);
        } else {
            auras::set_owned_duration(
                world,
                caster,
                target,
                DOOM,
                ((remaining - reduction) * 1000.0).round() as u32,
            );
        }
    }
}

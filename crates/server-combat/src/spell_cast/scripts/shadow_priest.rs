//! Sourced Shadow Dummy relationships and target-dependent mastery.
//! SimC76f8118 sc_priest_shadow.cpp1758–1871/sc_priest.cpp4285–4341.
//! Native payloads, mastery and talent effects own all amounts.
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::{ActiveSpec, Position};

use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::random as rand;
use crate::spell_info::SpellStore;

pub(super) const SLAM: u32 = 1227280;
const DAMAGE: u32 = 1227621;
const VT: u32 = 34914;
const MADDENING_TENTACLES: u32 = 1279353;
const MADNESS: u32 = 335467;
const SHADOW_WEAVING: u32 = 343690;
const MADNESS_WEAVING: u32 = 1240394;
const VOIDFORM: u32 = 194249;
const DOTS: &[u32] = &[589, VT, MADNESS];
// Current native Shadow APL damage; not item/racial procs or reflected self damage.
// sc_priest.cpp194/1220; sc_priest_shadow.cpp40/561/756/835/1186/1435/1709.
const WEAVING_PAYLOADS: &[u32] = &[
    589, 8092, 15407, 32379, VT, 263165, MADNESS, 391403, 450983, DAMAGE, 1242189,
];

/// SimC sc_priest_shadow.cpp1819–1826 explicitly authors500ms, absent in data.
/// Keep the override in script policy, not in fabricated native spell records.
pub(in crate::spell_cast) fn launch_delay(spell: u32, native: f32) -> f32 {
    if spell == SLAM { 0.5 } else { native }
}

pub(super) fn slam_impact(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    let depth = depth + 1;
    let Some(damage) = load_slam_damage(world, primary, depth) else {
        return;
    };
    let plan =
        match super::super::plan_triggered_spell(world, caster, &damage, Some(primary), depth) {
            Ok(plan) => plan,
            Err(reason) => {
                warn!(?reason, "Tentacle Slam native area selection failed");
                return;
            }
        };
    let units = super::super::selected_units(&plan);
    apply_slam_dots(world, caster, units, depth);
    // sc_priest_shadow.cpp1853–1869: VT before damage, then weak Madness.
    super::super::launch_triggered_spell(world, caster, &damage, Some(primary), depth);
    cast_maddening_tentacles(world, caster, primary, depth);
}

fn load_slam_damage(
    world: &World,
    primary: Entity,
    depth: u8,
) -> Option<crate::spell_info::SpellInfo> {
    let mut damage = super::super::load_triggered_spell(world, DAMAGE, depth)?;
    let Some(position) = world.get::<Position>(primary) else {
        warn!(?primary, "Tentacle Slam primary position unavailable");
        return None;
    };
    damage.cast_destination = Some([position.x, position.y, position.z]);
    Some(damage)
}

fn apply_slam_dots(world: &mut World, caster: Entity, mut units: Vec<Entity>, depth: u8) {
    let cap = world
        .resource::<SpellStore>()
        .get(SLAM)
        .expect("native Tentacle Slam root")
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("native Tentacle Slam VT cap")
        .base_points
        .max(0.0) as usize;
    // sc_priest_shadow.cpp1758–1785: shuffle only above cap, then prefer fresh VT.
    if units.len() > cap {
        units.shuffle(&mut rand::thread_rng());
        units.sort_by_key(|&target| owns_dot(world, caster, target, VT));
        units.truncate(cap);
    }
    for target in units {
        super::super::cast_triggered(world, caster, VT, Some(target), depth);
    }
}

fn owns_dot(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    let now = super::super::now(world);
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            let owned = aura.caster == caster && aura.spell_id == spell;
            let live = aura.expires_at.is_none_or(|expiry| expiry > now);
            let periodic = aura.effects.iter().any(|effect| effect.period_ms > 0);
            owned && live && periodic
        })
    })
}

fn cast_maddening_tentacles(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    if !selected(world, caster, MADDENING_TENTACLES) {
        return;
    }
    let Some(percent) = owned_effect_amount(world, caster, MADDENING_TENTACLES, 0) else {
        warn!(?caster, "Maddening Tentacles native effect unavailable");
        return;
    };
    let Some(mut child) = super::super::load_triggered_spell(world, MADNESS, depth) else {
        return;
    };
    // sc_priest_shadow.cpp1272–1275: a background Madness at talent E0 percent.
    // Native rolling-periodic refresh retains unpaid damage from the previous DoT.
    child.cast_damage_multiplier = Some(percent / 100.0);
    super::super::launch_triggered_spell(world, caster, &child, Some(primary), depth);
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    })
}

fn owned_effect_amount(world: &World, caster: Entity, spell: u32, index: u8) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == spell)?;
    let effect = aura.effects.iter().find(|effect| effect.index == index)?;
    Some(effect.amount)
}

/// SimC76f8118 sc_priest.cpp4285–4341: live target DoTs, not a flat school bonus.
pub(in crate::spell_cast) fn weaving_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: u32,
) -> f32 {
    let shadow = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 258);
    let eligible = shadow && WEAVING_PAYLOADS.contains(&spell);
    if !eligible {
        return 1.0;
    }
    let Some(mastery) = owned_effect_amount(world, caster, SHADOW_WEAVING, 0) else {
        return 1.0;
    };
    1.0 + weaving_dots(world, caster, target, spell) * mastery / 100.0
}

fn weaving_dots(world: &World, caster: Entity, target: Entity, spell: u32) -> f32 {
    let voidform = crate::auras::has_aura(world, caster, VOIDFORM);
    let madness = spell == MADNESS || owns_dot(world, caster, target, MADNESS);
    // An initial DoT hit benefits before its own aura has been installed.
    let dots = if voidform {
        3.0
    } else {
        DOTS.iter()
            .filter(|&&dot| spell == dot || owns_dot(world, caster, target, dot))
            .count() as f32
    };
    let extra = if (voidform || madness) && selected(world, caster, MADNESS_WEAVING) {
        super::script_effect_value(world, caster, MADNESS_WEAVING, 0) / 100.0
    } else {
        0.0
    };
    dots + extra
}
